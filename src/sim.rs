//! Simulation: builds electrical nets from wires and closed contacts, finds
//! which loads have voltage, and runs contactors, timers and motors.

use crate::circuit::{t, Circuit, Comp, Kind, Pot, P};
use std::collections::{HashMap, HashSet};

#[derive(Default, Clone)]
pub struct Nets {
    idx: HashMap<P, usize>,
    parent: Vec<usize>,
    pot: HashMap<usize, Pot>,
    short: HashSet<usize>,
    pub short_pair: Option<(Pot, Pot)>,
}

impl Nets {
    fn find(&self, mut i: usize) -> usize {
        while self.parent[i] != i {
            i = self.parent[i];
        }
        i
    }
    pub fn pot_at(&self, p: P) -> Option<Pot> {
        let i = *self.idx.get(&p)?;
        self.pot.get(&self.find(i)).copied()
    }
    pub fn shorted(&self, p: P) -> bool {
        match self.idx.get(&p) {
            Some(&i) => self.short.contains(&self.find(i)),
            None => false,
        }
    }
}

struct Uf {
    parent: Vec<usize>,
}

impl Uf {
    fn find(&mut self, mut i: usize) -> usize {
        while self.parent[i] != i {
            self.parent[i] = self.parent[self.parent[i]];
            i = self.parent[i];
        }
        i
    }
    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.parent[ra] = rb;
        }
    }
}

/// Which devices (by tag) currently hold their contacts in the worked position.
pub fn device_active(c: &Circuit) -> HashMap<String, bool> {
    let mut m: HashMap<String, bool> = HashMap::new();
    for comp in &c.comps {
        let a = match comp.kind {
            Kind::Coil => comp.st.energized,
            Kind::TimerOn | Kind::TimerOff => comp.st.out,
            Kind::Thermal3 => comp.st.on,
            Kind::Breaker1 | Kind::Breaker3 => !comp.st.on,
            _ => continue,
        };
        *m.entry(comp.tag.clone()).or_insert(false) |= a;
    }
    m
}

/// Visual / electrical "worked" state of a component.
pub fn actuated(c: &Comp, act: &HashMap<String, bool>) -> bool {
    use Kind::*;
    match c.kind {
        ContactNo | ContactNc | Main3 => act.get(&c.tag).copied().unwrap_or(false),
        PushNo | PushNc => c.st.pressed,
        Switch | EStop | LimitNo | LimitNc => c.st.on,
        Breaker1 | Breaker3 | Thermal3 => c.st.on,
        _ => false,
    }
}

fn closed_pairs(c: &Comp, act: &HashMap<String, bool>) -> &'static [(usize, usize)] {
    use Kind::*;
    const ONE: &[(usize, usize)] = &[(0, 1)];
    const THREE: &[(usize, usize)] = &[(0, 3), (1, 4), (2, 5)];
    const NONE: &[(usize, usize)] = &[];
    let a = actuated(c, act);
    match c.kind {
        Fuse => ONE,
        Breaker1 => if a { NONE } else { ONE },
        Breaker3 => if a { NONE } else { THREE },
        Thermal3 => THREE,
        Main3 => if a { THREE } else { NONE },
        PushNo | Switch | LimitNo | ContactNo => if a { ONE } else { NONE },
        PushNc | EStop | LimitNc | ContactNc => if a { NONE } else { ONE },
        _ => NONE,
    }
}

fn conflict(a: Pot, b: Pot) -> bool {
    if a == b {
        return false;
    }
    let earthy = |p: Pot| matches!(p, Pot::N | Pot::PE | Pot::M0);
    !(earthy(a) && earthy(b))
}

pub fn build_nets(circ: &Circuit, act: &HashMap<String, bool>) -> Nets {
    let mut idx: HashMap<P, usize> = HashMap::new();
    let add = |p: P, idx: &mut HashMap<P, usize>| {
        let n = idx.len();
        *idx.entry(p).or_insert(n)
    };
    for w in &circ.wires {
        add(w.a, &mut idx);
        add(w.b, &mut idx);
    }
    for c in &circ.comps {
        for p in c.pins() {
            add(p, &mut idx);
        }
    }
    let mut uf = Uf { parent: (0..idx.len()).collect() };
    for w in &circ.wires {
        uf.union(idx[&w.a], idx[&w.b]);
    }
    // T-junctions: a wire end resting on another wire's middle.
    for w in &circ.wires {
        for o in &circ.wires {
            for e in [o.a, o.b] {
                if w.interior(e) {
                    uf.union(idx[&e], idx[&w.a]);
                }
            }
        }
    }
    for c in &circ.comps {
        for &(i, j) in closed_pairs(c, act) {
            uf.union(idx[&c.pin(i)], idx[&c.pin(j)]);
        }
    }
    let mut pot: HashMap<usize, Pot> = HashMap::new();
    let mut short = HashSet::new();
    let mut short_pair = None;
    for c in &circ.comps {
        if let Some(s) = c.kind.source() {
            let r = uf.find(idx[&c.pin(0)]);
            match pot.get(&r).copied() {
                None => {
                    pot.insert(r, s);
                }
                Some(o) if conflict(o, s) => {
                    short.insert(r);
                    short_pair.get_or_insert((o, s));
                }
                Some(o) => {
                    // N joined with PE: keep N as the net's name.
                    if s == Pot::N && o == Pot::PE {
                        pot.insert(r, s);
                    }
                }
            }
        }
    }
    let n = idx.len();
    let parent = (0..n).map(|i| uf.find(i)).collect();
    Nets { idx, parent, pot, short, short_pair }
}

/// Voltage across a two-terminal load: Some(direction) when it works.
fn across(nets: &Nets, a: P, b: P) -> Option<i8> {
    let (pa, pb) = (nets.pot_at(a)?, nets.pot_at(b)?);
    if nets.shorted(a) || nets.shorted(b) {
        return None;
    }
    if pa.is_ac() && pb.is_ac() && pa != pb {
        return Some(1);
    }
    match (pa, pb) {
        (Pot::P24, Pot::M0) => Some(1),
        (Pot::M0, Pot::P24) => Some(-1),
        _ => None,
    }
}

#[derive(Default)]
pub struct Sim {
    pub running: bool,
    pub paused: bool,
    pub nets: Nets,
    pub time: f32,
    pub fault: Option<String>,
}

impl Sim {
    pub fn start(&mut self, circ: &mut Circuit) {
        circ.reset_state();
        self.running = true;
        self.paused = false;
        self.time = 0.0;
        self.fault = None;
        self.step(circ, 0.0);
    }

    pub fn stop(&mut self, circ: &mut Circuit) {
        circ.reset_state();
        self.running = false;
        self.paused = false;
        self.fault = None;
        self.nets = Nets::default();
    }

    pub fn step(&mut self, circ: &mut Circuit, dt: f32) {
        if self.paused {
            return;
        }
        self.time += dt;
        // Let relays settle: coils switch contacts, which may switch more coils.
        for _ in 0..16 {
            let act = device_active(circ);
            let nets = build_nets(circ, &act);
            let mut changed = false;
            for c in circ.comps.iter_mut() {
                if matches!(c.kind, Kind::Coil | Kind::TimerOn | Kind::TimerOff) {
                    let e = across(&nets, c.pin(0), c.pin(1)).is_some();
                    if e != c.st.energized {
                        c.st.energized = e;
                        changed = true;
                    }
                    if c.kind == Kind::TimerOff && e && !c.st.out {
                        c.st.out = true;
                        c.st.timer = 0.0;
                        changed = true;
                    }
                }
            }
            self.nets = nets;
            if !changed {
                break;
            }
        }
        // Timers count real time.
        for c in circ.comps.iter_mut() {
            match c.kind {
                Kind::TimerOn => {
                    if c.st.energized {
                        c.st.timer = (c.st.timer + dt).min(c.preset);
                        if c.st.timer >= c.preset {
                            c.st.out = true;
                        }
                    } else {
                        c.st.timer = 0.0;
                        c.st.out = false;
                    }
                }
                Kind::TimerOff => {
                    if c.st.energized {
                        c.st.timer = 0.0;
                        c.st.out = true;
                    } else if c.st.out {
                        c.st.timer += dt;
                        if c.st.timer >= c.preset {
                            c.st.out = false;
                            c.st.timer = 0.0;
                        }
                    }
                }
                _ => {}
            }
        }
        // Loads.
        let nets = &self.nets;
        for c in circ.comps.iter_mut() {
            match c.kind {
                Kind::Lamp | Kind::Buzzer => {
                    c.st.energized = across(nets, c.pin(0), c.pin(1)).is_some();
                }
                Kind::Motor1 => {
                    let d = across(nets, c.pin(0), c.pin(1));
                    c.st.energized = d.is_some();
                    c.st.motor = d.unwrap_or(0);
                }
                Kind::Motor3 => {
                    let ps: Vec<Option<Pot>> = (0..3)
                        .map(|i| {
                            let p = c.pin(i);
                            if nets.shorted(p) { None } else { nets.pot_at(p) }
                        })
                        .collect();
                    let phases: Vec<Pot> = ps.iter().flatten().copied().filter(|p| p.is_phase()).collect();
                    let mut uniq = phases.clone();
                    uniq.sort_by_key(|p| *p as u8);
                    uniq.dedup();
                    if phases.len() == 3 && uniq.len() == 3 {
                        let k = |p: Pot| p as i32;
                        let (a, b, cc) = (k(phases[0]), k(phases[1]), k(phases[2]));
                        // Cyclic order L1-L2-L3 turns one way, any swap reverses.
                        let fwd = (b - a).rem_euclid(3) == 1 && (cc - b).rem_euclid(3) == 1;
                        c.st.motor = if fwd { 1 } else { -1 };
                        c.st.energized = true;
                        c.st.hum = false;
                    } else {
                        c.st.motor = 0;
                        c.st.energized = false;
                        c.st.hum = uniq.len() == 2;
                    }
                }
                _ => {}
            }
            if c.st.motor != 0 {
                c.st.angle += dt * 6.0 * c.st.motor as f32;
            }
        }
        if let Some((a, b)) = self.nets.short_pair {
            self.fault = Some(format!(
                "{} {} – {}. {}",
                t("Curto-circuito entre", "Short circuit between"),
                a.name(),
                b.name(),
                t("Pare a simulação e corrija a ligação.", "Stop the simulation and fix the wiring.")
            ));
        } else {
            self.fault = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::examples;

    fn press(c: &mut Circuit, tag: &str, on: bool) {
        for comp in c.comps.iter_mut() {
            if matches!(comp.kind, Kind::PushNo | Kind::PushNc) && comp.tag == tag {
                comp.st.pressed = on;
            }
        }
    }
    fn get<'a>(c: &'a Circuit, kind: Kind, tag: &str) -> &'a Comp {
        c.comps.iter().find(|x| x.kind == kind && x.tag == tag).unwrap()
    }

    #[test]
    fn direct_start_latches_and_stops() {
        let mut c = examples::direct_start();
        let mut s = Sim::default();
        s.start(&mut c);
        assert_eq!(get(&c, Kind::Motor3, "M1").st.motor, 0);
        press(&mut c, "S2", true);
        s.step(&mut c, 0.02);
        press(&mut c, "S2", false);
        s.step(&mut c, 0.02);
        assert!(get(&c, Kind::Coil, "KM1").st.energized);
        assert_eq!(get(&c, Kind::Motor3, "M1").st.motor, 1);
        assert!(get(&c, Kind::Lamp, "H1").st.energized);
        assert!(!get(&c, Kind::Lamp, "H2").st.energized);
        press(&mut c, "S1", true);
        s.step(&mut c, 0.02);
        press(&mut c, "S1", false);
        s.step(&mut c, 0.02);
        assert!(!get(&c, Kind::Coil, "KM1").st.energized);
        assert_eq!(get(&c, Kind::Motor3, "M1").st.motor, 0);
        // Trip the motor breaker: tripped lamp lights.
        c.comps.iter_mut().find(|x| x.kind == Kind::Breaker3).unwrap().st.on = true;
        s.step(&mut c, 0.02);
        assert!(get(&c, Kind::Lamp, "H2").st.energized);
        assert!(s.fault.is_none());
    }

    #[test]
    fn reversing_runs_both_ways_with_interlock() {
        let mut c = examples::reversing();
        let mut s = Sim::default();
        s.start(&mut c);
        press(&mut c, "S1", true);
        s.step(&mut c, 0.02);
        press(&mut c, "S1", false);
        s.step(&mut c, 0.02);
        let fwd = get(&c, Kind::Motor3, "M1").st.motor;
        assert_ne!(fwd, 0);
        press(&mut c, "S2", true);
        s.step(&mut c, 0.02);
        assert!(!get(&c, Kind::Coil, "KM2").st.energized, "interlock");
        press(&mut c, "S2", false);
        press(&mut c, "S0", true);
        s.step(&mut c, 0.02);
        press(&mut c, "S0", false);
        s.step(&mut c, 0.02);
        press(&mut c, "S2", true);
        s.step(&mut c, 0.02);
        press(&mut c, "S2", false);
        s.step(&mut c, 0.02);
        assert_eq!(get(&c, Kind::Motor3, "M1").st.motor, -fwd);
        assert!(s.fault.is_none());
    }

    #[test]
    fn timer_lights_lamp_after_delay() {
        let mut c = examples::timed_lamp();
        let mut s = Sim::default();
        s.start(&mut c);
        press(&mut c, "S1", true);
        s.step(&mut c, 0.02);
        press(&mut c, "S1", false);
        for _ in 0..100 {
            s.step(&mut c, 0.02);
        }
        assert!(!get(&c, Kind::Lamp, "H1").st.energized);
        for _ in 0..200 {
            s.step(&mut c, 0.02);
        }
        assert!(get(&c, Kind::Lamp, "H1").st.energized);
    }

    #[test]
    fn short_circuit_is_reported() {
        let mut c = Circuit::default();
        c.add(Kind::TermL1, (0, 0).into(), 0, "");
        c.add(Kind::TermN, (5, 0).into(), 0, "");
        c.wire((0, 0), (5, 0));
        let mut s = Sim::default();
        s.start(&mut c);
        assert!(s.fault.is_some());
    }
}
