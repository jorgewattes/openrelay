//! Simulation: builds electrical nets from wires and closed contacts, finds
//! which loads have voltage (from fixed, tabulated values only), compares it
//! with each part's rated voltage and runs contactors, timers and motors.

use crate::circuit::{nominal, t, Circuit, Comp, Cur, Kind, Pot, AC_SYSTEMS, DC_LEVELS, MOTOR6_NOMINALS, P};
use std::collections::{HashMap, HashSet};

#[derive(Default, Clone)]
pub struct Nets {
    idx: HashMap<P, usize>,
    parent: Vec<usize>,
    pot: HashMap<usize, Pot>,
    /// Voltage option of the source feeding the net (AC system or DC level).
    level: HashMap<usize, u8>,
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
    /// Source on a point: (potential, voltage option of that source).
    fn src_at(&self, p: P) -> Option<(Pot, u8)> {
        let r = self.find(*self.idx.get(&p)?);
        Some((*self.pot.get(&r)?, self.level.get(&r).copied().unwrap_or(0)))
    }
    /// Both points belong to the same net.
    fn same(&self, a: P, b: P) -> bool {
        match (self.idx.get(&a), self.idx.get(&b)) {
            (Some(&x), Some(&y)) => self.find(x) == self.find(y),
            _ => false,
        }
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
    let mut level: HashMap<usize, u8> = HashMap::new();
    let mut short = HashSet::new();
    let mut short_pair = None;
    for c in &circ.comps {
        if let Some(s) = c.kind.source() {
            let r = uf.find(idx[&c.pin(0)]);
            match pot.get(&r).copied() {
                None => {
                    pot.insert(r, s);
                    level.insert(r, c.volt);
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
    Nets { idx, parent, pot, level, short, short_pair }
}

/// Phase-neutral and line voltage of an AC system option.
fn ac_system(level: u8) -> (f32, f32) {
    *AC_SYSTEMS.get(level as usize).unwrap_or(&AC_SYSTEMS[0])
}

/// Voltage across two points, from the tabulated source values only:
/// Some((AC/DC, volts, direction)) or None when there is no voltage.
fn volts(nets: &Nets, a: P, b: P) -> Option<(Cur, f32, i8)> {
    if nets.shorted(a) || nets.shorted(b) {
        return None;
    }
    let ((pa, la), (pb, lb)) = (nets.src_at(a)?, nets.src_at(b)?);
    if pa == pb {
        return None;
    }
    if pa.is_ac() && pb.is_ac() {
        let v = if pa == Pot::N {
            ac_system(lb).0
        } else if pb == Pot::N {
            ac_system(la).0
        } else {
            ac_system(la).1
        };
        return Some((Cur::Ac, v, 1));
    }
    let dc = |l: u8| *DC_LEVELS.get(l as usize).unwrap_or(&DC_LEVELS[0]);
    match (pa, pb) {
        (Pot::P24, Pot::M0) => Some((Cur::Dc, dc(la), 1)),
        (Pot::M0, Pot::P24) => Some((Cur::Dc, dc(lb), -1)),
        _ => None,
    }
}

/// Compare applied with rated voltage: 0 ok, -1 under, 1 over, 2 wrong kind (AC/DC).
pub fn check(rated: (Cur, f32), got: (Cur, f32)) -> i8 {
    if rated.0 != got.0 {
        return 2;
    }
    let r = got.1 / rated.1.max(0.1);
    if r > 1.1 {
        1
    } else if r < 0.9 {
        -1
    } else {
        0
    }
}

/// Phase order of three points: Some(1) L1-L2-L3 order, Some(-1) reversed,
/// None when they are not three different phases.
fn rotation(nets: &Nets, pts: [P; 3]) -> Option<(i8, u8)> {
    let mut ph = vec![];
    let mut lvl = 0;
    for p in pts {
        if nets.shorted(p) {
            return None;
        }
        let (pot, l) = nets.src_at(p)?;
        if !pot.is_phase() {
            return None;
        }
        lvl = l;
        ph.push(pot as i32);
    }
    let (a, b, c) = (ph[0], ph[1], ph[2]);
    if a == b || b == c || a == c {
        return None;
    }
    let fwd = (b - a).rem_euclid(3) == 1 && (c - b).rem_euclid(3) == 1;
    Some((if fwd { 1 } else { -1 }, lvl))
}

/// How many different phases reach these points (for the "missing phase" warning).
fn phase_count(nets: &Nets, pts: &[P]) -> usize {
    let mut v: Vec<i32> = pts
        .iter()
        .filter_map(|p| nets.src_at(*p))
        .filter(|(p, _)| p.is_phase())
        .map(|(p, _)| p as i32)
        .collect();
    v.sort();
    v.dedup();
    v.len()
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
                    // A coil pulls in only at its rated voltage: under-voltage,
                    // over-voltage or the wrong kind of current leave it open.
                    let (e, volt, vstat) = match (volts(&nets, c.pin(0), c.pin(1)), nominal(c)) {
                        (Some((k, v, _)), Some(n)) => {
                            let s = check(n, (k, v));
                            (s == 0, v, s)
                        }
                        _ => (false, 0.0, 0),
                    };
                    c.st.volt = volt;
                    c.st.vstat = vstat;
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
                    if c.st.burnt {
                        c.st.energized = false;
                        c.st.bright = 0.0;
                        c.st.vstat = 1;
                        c.st.volt = volts(nets, c.pin(0), c.pin(1)).map(|v| v.1).unwrap_or(0.0);
                        continue;
                    }
                    match (volts(nets, c.pin(0), c.pin(1)), nominal(c)) {
                        (Some((k, v, _)), Some(n)) => {
                            let s = check(n, (k, v));
                            c.st.volt = v;
                            c.st.vstat = s;
                            match s {
                                1 => {
                                    // Over-voltage burns the lamp / LED.
                                    c.st.burnt = true;
                                    c.st.energized = false;
                                    c.st.bright = 0.0;
                                }
                                2 => {
                                    c.st.energized = false;
                                    c.st.bright = 0.0;
                                }
                                -1 => {
                                    // Under-voltage: lit, but clearly paler.
                                    c.st.energized = true;
                                    c.st.bright = ((v / n.1) * 0.6).clamp(0.12, 0.5);
                                }
                                _ => {
                                    c.st.energized = true;
                                    c.st.bright = 1.0;
                                }
                            }
                        }
                        _ => {
                            c.st.energized = false;
                            c.st.bright = 0.0;
                            c.st.volt = 0.0;
                            c.st.vstat = 0;
                        }
                    }
                }
                Kind::Motor1 => match (volts(nets, c.pin(0), c.pin(1)), nominal(c)) {
                    (Some((k, v, d)), Some(n)) => {
                        let s = check(n, (k, v));
                        c.st.volt = v;
                        c.st.vstat = s;
                        // Over-voltage or wrong AC/DC: the motor doesn't run.
                        let runs = s == 0 || s == -1;
                        c.st.energized = runs;
                        c.st.motor = if runs { d } else { 0 };
                    }
                    _ => {
                        c.st.energized = false;
                        c.st.motor = 0;
                        c.st.volt = 0.0;
                        c.st.vstat = 0;
                    }
                },
                Kind::Motor3 => {
                    let pts = [c.pin(0), c.pin(1), c.pin(2)];
                    match rotation(nets, pts) {
                        Some((dir, lvl)) => {
                            let vl = ac_system(lvl).1;
                            let s = nominal(c).map(|n| check(n, (Cur::Ac, vl))).unwrap_or(0);
                            let runs = s != 1;
                            c.st.motor = if runs { dir } else { 0 };
                            c.st.energized = runs;
                            c.st.hum = false;
                            c.st.volt = vl;
                            c.st.vstat = s;
                        }
                        None => {
                            c.st.motor = 0;
                            c.st.energized = false;
                            c.st.hum = phase_count(nets, &pts) == 2;
                            c.st.volt = 0.0;
                            c.st.vstat = 0;
                        }
                    }
                }
                Kind::Motor6 => {
                    // Windings U1-U2, V1-V2, W1-W2; the outside links decide star or delta.
                    let mut w: Vec<f32> = (0..3)
                        .map(|i| match volts(nets, c.pin(i), c.pin(i + 3)) {
                            Some((Cur::Ac, v, _)) => v,
                            _ => 0.0,
                        })
                        .collect();
                    let top = [c.pin(0), c.pin(1), c.pin(2)];
                    let star = nets.same(c.pin(3), c.pin(4)) && nets.same(c.pin(4), c.pin(5));
                    // A floating star point sits at neutral: each winding gets the phase voltage.
                    if star && nets.src_at(c.pin(3)).is_none() {
                        if let Some((_, lvl)) = rotation(nets, top) {
                            w = vec![ac_system(lvl).0; 3];
                        }
                    }
                    let delta = nets.same(c.pin(0), c.pin(5)) && nets.same(c.pin(1), c.pin(3)) && nets.same(c.pin(2), c.pin(4));
                    c.st.conn = if star { 1 } else if delta { 2 } else { 0 };
                    match rotation(nets, top) {
                        Some((dir, lvl)) if w.iter().all(|v| *v > 0.0) => {
                            let winding = w.iter().cloned().fold(0.0f32, f32::max);
                            let rated = MOTOR6_NOMINALS.get(c.volt as usize).unwrap_or(&MOTOR6_NOMINALS[0]).0;
                            let s = check((Cur::Ac, rated), (Cur::Ac, winding));
                            let runs = s != 1;
                            c.st.motor = if runs { dir } else { 0 };
                            c.st.energized = runs;
                            c.st.hum = false;
                            c.st.volt = ac_system(lvl).1;
                            c.st.vstat = s;
                        }
                        _ => {
                            c.st.motor = 0;
                            c.st.energized = false;
                            c.st.hum = w.iter().any(|v| *v > 0.0) || phase_count(nets, &top) == 2;
                            c.st.volt = 0.0;
                            c.st.vstat = 0;
                        }
                    }
                }
                _ => {}
            }
            if c.st.motor != 0 {
                // Under-voltage turns slower.
                let speed = if c.st.vstat == -1 { 1.8 } else { 6.0 };
                c.st.angle += dt * speed * c.st.motor as f32;
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
    /// Lamp between L1 and N with the given AC system and lamp rating.
    fn lamp_circuit(sys: u8, rating: u8) -> Circuit {
        let mut c = Circuit::default();
        let l = c.add(Kind::TermL1, (0, 0).into(), 0, "");
        c.comp_mut(l).unwrap().volt = sys;
        let h = c.add(Kind::Lamp, (0, 2).into(), 0, "H1");
        c.comp_mut(h).unwrap().volt = rating;
        c.add(Kind::TermN, (0, 10).into(), 2, "");
        c.wire((0, 0), (0, 2));
        c.wire((0, 8), (0, 10));
        c
    }

    #[test]
    fn lamp_voltage_rules() {
        // 220 V lamp on 220/380 system (phase-neutral 220 V): normal.
        let mut c = lamp_circuit(0, 0);
        let mut s = Sim::default();
        s.start(&mut c);
        let h = get(&c, Kind::Lamp, "H1");
        assert!(h.st.energized && h.st.vstat == 0 && h.st.bright == 1.0);
        // 220 V lamp on 127/220 system: dim, under-voltage.
        let mut c = lamp_circuit(1, 0);
        s.start(&mut c);
        let h = get(&c, Kind::Lamp, "H1");
        assert!(h.st.energized && h.st.vstat == -1 && h.st.bright < 0.7);
        assert_eq!(h.st.volt, 127.0);
        // 127 V lamp on 220/380: burns and stays burnt.
        let mut c = lamp_circuit(0, 1);
        s.start(&mut c);
        s.step(&mut c, 0.1);
        let h = get(&c, Kind::Lamp, "H1");
        assert!(h.st.burnt && !h.st.energized);
        // 24 V DC LED on AC: wrong kind, no light.
        let mut c = lamp_circuit(0, 4);
        s.start(&mut c);
        let h = get(&c, Kind::Lamp, "H1");
        assert!(!h.st.energized && h.st.vstat == 2);
    }

    #[test]
    fn coil_needs_rated_voltage() {
        let mut c = Circuit::default();
        let l = c.add(Kind::TermL1, (0, 0).into(), 0, "");
        c.comp_mut(l).unwrap().volt = 1; // 127/220 system
        let k = c.add(Kind::Coil, (0, 2).into(), 0, "KM1");
        c.comp_mut(k).unwrap().volt = 0; // 220 V coil
        c.add(Kind::TermN, (0, 10).into(), 2, "");
        c.wire((0, 0), (0, 2));
        c.wire((0, 8), (0, 10));
        let mut s = Sim::default();
        s.start(&mut c);
        let k = get(&c, Kind::Coil, "KM1");
        assert!(!k.st.energized, "under-voltage coil must not pull in");
        assert_eq!(k.st.vstat, -1);
        // Same coil rated 127 V: pulls in.
        c.comps.iter_mut().find(|x| x.kind == Kind::Coil).unwrap().volt = 1;
        s.start(&mut c);
        assert!(get(&c, Kind::Coil, "KM1").st.energized);
        // 127 V coil on a 220/380 system (220 V): over-voltage, does not pull in.
        c.comps.iter_mut().find(|x| x.kind == Kind::TermL1).unwrap().volt = 0;
        s.start(&mut c);
        let k = get(&c, Kind::Coil, "KM1");
        assert!(!k.st.energized, "over-voltage coil must not pull in");
        assert_eq!(k.st.vstat, 1);
    }

    fn motor6(delta: bool, rating: u8) -> Circuit {
        let mut c = Circuit::default();
        c.add(Kind::TermL1, (0, 0).into(), 0, "");
        c.add(Kind::TermL2, (3, 0).into(), 0, "");
        c.add(Kind::TermL3, (6, 0).into(), 0, "");
        let m = c.add(Kind::Motor6, (0, 4).into(), 0, "M1");
        c.comp_mut(m).unwrap().volt = rating;
        for x in [0, 3, 6] {
            c.wire((x, 0), (x, 4));
        }
        if delta {
            c.wire((0, 4), (6, 15));
            c.wire((3, 4), (0, 15));
            c.wire((6, 4), (3, 15));
        } else {
            c.wire((0, 15), (3, 15));
            c.wire((3, 15), (6, 15));
        }
        c
    }

    #[test]
    fn six_terminal_motor_star_delta() {
        let mut s = Sim::default();
        // Δ220/Y380 motor on a 380 V line: star is right, delta is over-voltage.
        let mut c = motor6(false, 0);
        s.start(&mut c);
        let m = get(&c, Kind::Motor6, "M1");
        assert_eq!((m.st.motor != 0, m.st.conn, m.st.vstat), (true, 1, 0));
        let mut c = motor6(true, 0);
        s.start(&mut c);
        let m = get(&c, Kind::Motor6, "M1");
        // Over-voltage: the motor does not run.
        assert_eq!((m.st.motor != 0, m.st.conn, m.st.vstat), (false, 2, 1));
        // Δ380/Y660 motor in delta on 380 V: right.
        let mut c = motor6(true, 1);
        s.start(&mut c);
        assert_eq!(get(&c, Kind::Motor6, "M1").st.vstat, 0);
    }
}
