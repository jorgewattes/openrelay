//! Simulation: builds electrical nets from wires and closed contacts, finds
//! which loads have voltage (from fixed, tabulated values only), compares it
//! with each part's rated voltage and runs contactors, timers and motors.

use crate::circuit::{
    nominal, t, Circuit, Comp, Cur, Kind, Pot, AC_SYSTEMS, DC_LEVELS, MOTOR12_NOMINALS, MOTOR1D_NOMINALS, MOTOR6_NOMINALS, P,
};
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
            k if k.is_breaker() => !comp.st.on,
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
        ContactNo | ContactNc | ContactCo | Main3 => act.get(&c.tag).copied().unwrap_or(false),
        PushNo | PushNc => c.st.pressed,
        Switch | EStop | LimitNo | LimitNc => c.st.on,
        Thermal3 => c.st.on,
        k if k.is_breaker() => c.st.on,
        _ => false,
    }
}

fn closed_pairs(c: &Comp, act: &HashMap<String, bool>) -> &'static [(usize, usize)] {
    use Kind::*;
    const ONE: &[(usize, usize)] = &[(0, 1)];
    const TWO: &[(usize, usize)] = &[(0, 2), (1, 3)];
    const THREE: &[(usize, usize)] = &[(0, 3), (1, 4), (2, 5)];
    const FOUR: &[(usize, usize)] = &[(0, 4), (1, 5), (2, 6), (3, 7)];
    const NONE: &[(usize, usize)] = &[];
    let a = actuated(c, act);
    match c.kind {
        Fuse => ONE,
        Breaker1 => if a { NONE } else { ONE },
        Breaker2 | Rcd2 => if a { NONE } else { TWO },
        Breaker3 | MotorBreaker3 => if a { NONE } else { THREE },
        Rcd4 => if a { NONE } else { FOUR },
        Thermal3 => THREE,
        Main3 => if a { THREE } else { NONE },
        PushNo | Switch | LimitNo | ContactNo => if a { ONE } else { NONE },
        PushNc | EStop | LimitNc | ContactNc => if a { NONE } else { ONE },
        // Changeover: common (0) with NC (1) at rest, with NO (2) when worked.
        ContactCo => if a { &[(0, 2)] } else { &[(0, 1)] },
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
    /// Last RCD trip, shown until the simulation restarts.
    pub rcd_msg: Option<String>,
}

impl Sim {
    pub fn start(&mut self, circ: &mut Circuit) {
        circ.reset_state();
        self.running = true;
        self.paused = false;
        self.time = 0.0;
        self.fault = None;
        self.rcd_msg = None;
        self.step(circ, 0.0);
    }

    pub fn stop(&mut self, circ: &mut Circuit) {
        circ.reset_state();
        self.running = false;
        self.paused = false;
        self.fault = None;
        self.rcd_msg = None;
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
                Kind::Motor12 => {
                    // Pins: 0 U1 1 V1 2 W1 3 U2 4 V2 5 W2 6 U5 7 V5 8 W5 9 U6 10 V6 11 W6.
                    let p = |i: usize| c.pin(i);
                    let same = |a: usize, b: usize| nets.same(p(a), p(b));
                    let top = [p(0), p(1), p(2)];
                    let par = same(0, 6) && same(1, 7) && same(2, 8) && same(3, 9) && same(4, 10) && same(5, 11);
                    let ser = same(3, 6) && same(4, 7) && same(5, 8);
                    // (connection code, half-winding voltage as fraction of (phase, line) voltage)
                    let conn: Option<(u8, bool, f32)> = if par && same(3, 4) && same(4, 5) {
                        Some((1, false, 1.0)) // YY: phase voltage
                    } else if par && same(0, 5) && same(1, 3) && same(2, 4) {
                        Some((2, true, 1.0)) // ΔΔ: line voltage
                    } else if ser && same(9, 10) && same(10, 11) {
                        Some((3, false, 0.5)) // Y series: half the phase voltage
                    } else if ser && same(0, 11) && same(1, 9) && same(2, 10) {
                        Some((4, true, 0.5)) // Δ series: half the line voltage
                    } else {
                        None
                    };
                    c.st.conn = conn.map(|x| x.0).unwrap_or(0);
                    match (rotation(nets, top), conn) {
                        (Some((dir, lvl)), Some((_, line, frac))) => {
                            let (vf, vl) = ac_system(lvl);
                            let half = if line { vl } else { vf } * frac;
                            let rated = MOTOR12_NOMINALS.get(c.volt as usize).unwrap_or(&MOTOR12_NOMINALS[0]).0;
                            let s = check((Cur::Ac, rated), (Cur::Ac, half));
                            let runs = s != 1;
                            c.st.motor = if runs { dir } else { 0 };
                            c.st.energized = runs;
                            c.st.hum = false;
                            c.st.volt = vl;
                            c.st.vstat = s;
                        }
                        (rot, _) => {
                            c.st.motor = 0;
                            c.st.energized = false;
                            c.st.hum = rot.is_some() || phase_count(nets, &top) == 2;
                            c.st.volt = 0.0;
                            c.st.vstat = 0;
                        }
                    }
                }
                Kind::Motor1S | Kind::Motor1D => {
                    // Main winding supply and start winding; reversing Z1/Z2 reverses rotation.
                    let pins = c.pins();
                    let p = |i: usize| pins[i];
                    let (supply, rated, aux_ok, z1_with_start) = if c.kind == Kind::Motor1S {
                        let aux = volts(nets, p(2), p(3)).is_some();
                        (volts(nets, p(0), p(1)), nominal(c).map(|n| n.1).unwrap_or(220.0), aux, nets.same(p(2), p(0)))
                    } else {
                        let (par_v, ser_v) = *MOTOR1D_NOMINALS.get(c.volt as usize).unwrap_or(&MOTOR1D_NOMINALS[0]);
                        let par = nets.same(p(0), p(2)) && nets.same(p(1), p(3));
                        let ser = nets.same(p(1), p(2));
                        c.st.conn = if par { 1 } else if ser { 2 } else { 0 };
                        let aux1 = nets.same(p(4), p(0)) && nets.same(p(5), p(1));
                        let aux2 = nets.same(p(4), p(2)) && nets.same(p(5), p(3));
                        let aux1r = nets.same(p(4), p(1)) && nets.same(p(5), p(0));
                        let aux2r = nets.same(p(4), p(3)) && nets.same(p(5), p(2));
                        let aux = aux1 || aux2 || aux1r || aux2r;
                        if par {
                            (volts(nets, p(0), p(1)), par_v, aux, aux1 || aux2)
                        } else if ser {
                            (volts(nets, p(0), p(3)), ser_v, aux, aux1 || aux2)
                        } else {
                            (None, par_v, aux, true)
                        }
                    };
                    match supply {
                        Some((Cur::Ac, v, _)) => {
                            let s = check((Cur::Ac, rated), (Cur::Ac, v));
                            c.st.volt = v;
                            c.st.vstat = s;
                            // Without the start winding it only hums and does not start.
                            let runs = s != 1 && aux_ok;
                            c.st.hum = !aux_ok;
                            c.st.energized = runs;
                            c.st.motor = if runs { if z1_with_start { 1 } else { -1 } } else { 0 };
                        }
                        Some((Cur::Dc, v, _)) => {
                            c.st.volt = v;
                            c.st.vstat = 2;
                            c.st.hum = false;
                            c.st.energized = false;
                            c.st.motor = 0;
                        }
                        None => {
                            c.st.volt = 0.0;
                            c.st.vstat = 0;
                            c.st.hum = false;
                            c.st.energized = false;
                            c.st.motor = 0;
                        }
                    }
                }
                Kind::Motor6 | Kind::Motor6X => {
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
        // DR (residual-current device): trips when a load fed through it leaks to
        // earth (one side on a live output of the DR, the other on PE).
        let nets = &self.nets;
        let mut trips: Vec<(usize, String, String)> = vec![];
        for (ri, r) in circ.comps.iter().enumerate() {
            if !matches!(r.kind, Kind::Rcd2 | Kind::Rcd4) || r.st.on {
                continue;
            }
            let n = r.kind.poles();
            let outs: Vec<P> = (n..2 * n).map(|i| r.pin(i)).collect();
            for l in &circ.comps {
                let pins: Vec<P> = match l.kind {
                    Kind::Lamp
                    | Kind::Buzzer
                    | Kind::Coil
                    | Kind::TimerOn
                    | Kind::TimerOff
                    | Kind::Motor1
                    | Kind::Motor1S
                    | Kind::Motor1D => l.pins(),
                    Kind::Motor3 => l.pins()[..3].to_vec(),
                    Kind::Motor6 | Kind::Motor6X => l.pins()[..6].to_vec(),
                    Kind::Motor12 => l.pins()[..12].to_vec(),
                    _ => continue,
                };
                let earth = pins.iter().any(|p| matches!(nets.src_at(*p), Some((Pot::PE, _))));
                let live = pins.iter().any(|p| {
                    outs.iter().any(|o| nets.same(*p, *o))
                        && nets.src_at(*p).is_some_and(|(pot, _)| pot.is_phase() || pot == Pot::P24)
                });
                if earth && live {
                    trips.push((ri, r.tag.clone(), l.tag.clone()));
                    break;
                }
            }
        }
        for (ri, dr, load) in trips {
            circ.comps[ri].st.on = true;
            self.rcd_msg = Some(format!(
                "{} {dr}: {} {load}.",
                t("O DR desarmou", "The RCD tripped"),
                t("fuga de corrente para a terra em", "earth leakage at")
            ));
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
            self.fault = self.rcd_msg.clone();
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
        c.comps.iter_mut().find(|x| x.kind == Kind::MotorBreaker3).unwrap().st.on = true;
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
    #[test]
    fn star_delta_switches_after_delay() {
        let mut c = examples::star_delta();
        let mut s = Sim::default();
        s.start(&mut c);
        assert!(s.fault.is_none(), "{:?}", s.fault);
        press(&mut c, "S1", true);
        s.step(&mut c, 0.02);
        press(&mut c, "S1", false);
        s.step(&mut c, 0.02);
        assert!(get(&c, Kind::Coil, "KM1").st.energized);
        assert!(get(&c, Kind::Coil, "KM2").st.energized, "star first");
        assert!(!get(&c, Kind::Coil, "KM3").st.energized);
        let m = get(&c, Kind::Motor6X, "M1");
        assert_eq!((m.st.motor != 0, m.st.conn), (true, 1), "runs in star");
        assert_eq!(m.st.vstat, -1, "reduced voltage in star");
        for _ in 0..300 {
            s.step(&mut c, 0.02);
        }
        assert!(!get(&c, Kind::Coil, "KM2").st.energized);
        assert!(get(&c, Kind::Coil, "KM3").st.energized, "then delta");
        let m = get(&c, Kind::Motor6X, "M1");
        assert_eq!((m.st.motor != 0, m.st.conn, m.st.vstat), (true, 2, 0));
        assert!(s.fault.is_none(), "{:?}", s.fault);
        // Stop.
        press(&mut c, "S0", true);
        s.step(&mut c, 0.02);
        assert!(!get(&c, Kind::Coil, "KM1").st.energized);
        assert_eq!(get(&c, Kind::Motor6X, "M1").st.motor, 0);
    }

    #[test]
    fn rcd_trips_on_earth_leak() {
        let mut c = Circuit::default();
        c.add(Kind::TermL1, (0, 0).into(), 0, "");
        c.add(Kind::TermN, (3, 0).into(), 0, "");
        c.add(Kind::Rcd2, (0, 2).into(), 0, "DR1");
        c.wire((0, 0), (0, 2));
        c.wire((3, 0), (3, 2));
        // Lamp between the DR's phase output and earth: a leak.
        c.add(Kind::Lamp, (0, 10).into(), 0, "H1");
        c.wire((0, 8), (0, 10));
        c.add(Kind::TermPE, (0, 18).into(), 2, "");
        c.wire((0, 16), (0, 18));
        let mut s = Sim::default();
        s.start(&mut c);
        s.step(&mut c, 0.02);
        let dr = c.comps.iter().find(|x| x.kind == Kind::Rcd2).unwrap();
        assert!(dr.st.on, "DR must trip");
        assert!(s.fault.as_deref().unwrap_or("").contains("DR1"));
    }
    fn run(c: &mut Circuit, s: &mut Sim, press_tag: &str, steps: usize) {
        press(c, press_tag, true);
        s.step(c, 0.02);
        press(c, press_tag, false);
        for _ in 0..steps {
            s.step(c, 0.02);
        }
    }

    #[test]
    fn corrections_do_not_work() {
        let list = examples::corrections();
        // 1: holding contact wrong → motor stops when S2 is released.
        let mut c = (list[0].2)();
        let mut s = Sim::default();
        s.start(&mut c);
        run(&mut c, &mut s, "S2", 2);
        assert!(!get(&c, Kind::Coil, "KM1").st.energized);
        // 2: coil on, motor never turns.
        let mut c = (list[1].2)();
        s.start(&mut c);
        run(&mut c, &mut s, "S2", 2);
        assert!(get(&c, Kind::Coil, "KM1").st.energized);
        assert_eq!(get(&c, Kind::Motor3, "M1").st.motor, 0);
        // 3: both directions the same.
        let mut c = (list[2].2)();
        s.start(&mut c);
        run(&mut c, &mut s, "S1", 2);
        let d1 = get(&c, Kind::Motor3, "M1").st.motor;
        run(&mut c, &mut s, "S0", 2);
        run(&mut c, &mut s, "S2", 2);
        let d2 = get(&c, Kind::Motor3, "M1").st.motor;
        assert!(d1 != 0 && d1 == d2, "{d1} {d2}");
        // 4: lamp lit before the delay.
        let mut c = (list[3].2)();
        s.start(&mut c);
        run(&mut c, &mut s, "S1", 5);
        assert!(get(&c, Kind::Lamp, "H1").st.energized);
        // 5: starts in delta.
        let mut c = (list[4].2)();
        s.start(&mut c);
        run(&mut c, &mut s, "S1", 2);
        assert!(get(&c, Kind::Coil, "KM3").st.energized);
        assert!(!get(&c, Kind::Coil, "KM2").st.energized);
        assert!(s.fault.is_none());
    }

    /// Three phases into U1 V1 W1 of a 12-terminal motor, with the links given.
    fn motor12(links: &[((i32, i32), (i32, i32))]) -> Circuit {
        let mut c = Circuit::default();
        c.add(Kind::TermL1, (0, 0).into(), 0, "");
        c.add(Kind::TermL2, (3, 0).into(), 0, "");
        c.add(Kind::TermL3, (6, 0).into(), 0, "");
        c.add(Kind::Motor12, (0, 4).into(), 0, "M1");
        for x in [0, 3, 6] {
            c.wire((x, 0), (x, 4));
        }
        for (a, b) in links {
            c.wire(*a, *b);
        }
        c
    }

    #[test]
    fn twelve_terminal_motor_connections() {
        let mut s = Sim::default();
        // Top row y=4: U1 0, V1 3, W1 6, U5 9, V5 12, W5 15; bottom y=16: U2 0, V2 3, W2 6, U6 9, V6 12, W6 15.
        // YY on 380 V: parallel (U1-U5... and U2-U6...), U2-V2-W2 tied → 220 V per half: right.
        let mut c = motor12(&[
            ((0, 4), (0, 2)), ((0, 2), (9, 2)), ((9, 2), (9, 4)),
            ((3, 4), (3, 1)), ((3, 1), (12, 1)), ((12, 1), (12, 4)),
            ((6, 4), (6, 0)), ((6, 0), (15, 0)), ((15, 0), (15, 4)),
            ((0, 16), (3, 16)), ((3, 16), (6, 16)), ((6, 16), (9, 16)), ((9, 16), (12, 16)), ((12, 16), (15, 16)),
        ]);
        s.start(&mut c);
        let m = get(&c, Kind::Motor12, "M1");
        assert_eq!((m.st.conn, m.st.motor != 0, m.st.vstat), (1, true, 0));
        // Y series on 380 V: U2-U5, V2-V5, W2-W5 and U6-V6-W6 tied → 110 V per half: under-voltage.
        let mut c = motor12(&[
            ((0, 16), (0, 18)), ((0, 18), (9, 18)), ((9, 18), (9, 4)),
            ((3, 16), (3, 19)), ((3, 19), (12, 19)), ((12, 19), (12, 4)),
            ((6, 16), (6, 20)), ((6, 20), (15, 20)), ((15, 20), (15, 4)),
            ((9, 16), (12, 16)), ((12, 16), (15, 16)),
        ]);
        s.start(&mut c);
        let m = get(&c, Kind::Motor12, "M1");
        assert_eq!((m.st.conn, m.st.motor != 0, m.st.vstat), (3, true, -1));
    }

    #[test]
    fn single_phase_needs_start_winding() {
        let mut c = Circuit::default();
        c.add(Kind::TermL1, (0, 0).into(), 0, "");
        c.add(Kind::TermN, (0, 20).into(), 2, "");
        let m = c.add(Kind::Motor1S, (0, 4).into(), 0, "M1");
        c.comp_mut(m).unwrap().volt = 0; // 220 V
        c.wire((0, 0), (0, 4));
        c.wire((0, 12), (0, 20));
        let mut s = Sim::default();
        s.start(&mut c);
        let m = get(&c, Kind::Motor1S, "M1");
        assert!(m.st.motor == 0 && m.st.hum, "no start winding: hums");
        // Start winding in parallel: runs.
        c.wire((0, 2), (4, 2));
        c.wire((4, 2), (4, 4));
        c.wire((4, 12), (4, 14));
        c.wire((4, 14), (0, 14));
        s.start(&mut c);
        assert_ne!(get(&c, Kind::Motor1S, "M1").st.motor, 0);
    }

    #[test]
    fn changeover_contact_switches() {
        let mut c = Circuit::default();
        c.add(Kind::TermL1, (0, 0).into(), 0, "");
        c.add(Kind::TermN, (20, 40).into(), 2, "");
        c.add(Kind::PushNo, (0, 2).into(), 0, "S1");
        c.add(Kind::Coil, (0, 10).into(), 0, "KM1");
        c.wire((0, 0), (0, 2));
        c.wire((0, 8), (0, 10));
        c.wire((0, 16), (0, 40));
        c.wire((0, 40), (20, 40));
        // Changeover KM1: common fed from L1, NC → H1, NO → H2.
        c.add(Kind::ContactCo, (10, 4).into(), 0, "KM1");
        c.wire((0, 1), (10, 1));
        c.wire((10, 1), (10, 10));
        c.add(Kind::Lamp, (10, 20).into(), 0, "H1");
        c.add(Kind::Lamp, (13, 20).into(), 0, "H2");
        // NC at (10,4)? pins: C (10,10), NC (10,4), NO (13,4).
        c.wires.retain(|w| !(w.a == P::new(10, 1) && w.b == P::new(10, 10)));
        c.wire((10, 1), (10, 0));
        c.wire((10, 0), (8, 0));
        c.wire((8, 0), (8, 12));
        c.wire((8, 12), (10, 12));
        c.wire((10, 12), (10, 10));
        c.wire((10, 4), (10, 3));
        c.wire((10, 3), (11, 3));
        c.wire((11, 3), (11, 20));
        c.wire((11, 20), (10, 20));
        c.wire((13, 4), (13, 20));
        c.wire((10, 26), (10, 40));
        c.wire((13, 26), (13, 40));
        let mut s = Sim::default();
        s.start(&mut c);
        assert!(get(&c, Kind::Lamp, "H1").st.energized && !get(&c, Kind::Lamp, "H2").st.energized);
        press(&mut c, "S1", true);
        s.step(&mut c, 0.02);
        assert!(!get(&c, Kind::Lamp, "H1").st.energized && get(&c, Kind::Lamp, "H2").st.energized);
    }
}
