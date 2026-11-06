//! Circuit model: components, wires and the parts library.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

/// UI language switch (false = Portuguese, true = English).
pub static EN: AtomicBool = AtomicBool::new(false);

/// Pick the Portuguese or English text.
pub fn t(pt: &'static str, en: &'static str) -> &'static str {
    if EN.load(Ordering::Relaxed) {
        en
    } else {
        pt
    }
}

/// A point on the drawing grid (grid units).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, Default)]
pub struct P {
    pub x: i32,
    pub y: i32,
}

impl P {
    pub const fn new(x: i32, y: i32) -> Self {
        P { x, y }
    }
}

impl From<(i32, i32)> for P {
    fn from(v: (i32, i32)) -> P {
        P::new(v.0, v.1)
    }
}

impl std::ops::Add for P {
    type Output = P;
    fn add(self, o: P) -> P {
        P::new(self.x + o.x, self.y + o.y)
    }
}

impl std::ops::Sub for P {
    type Output = P;
    fn sub(self, o: P) -> P {
        P::new(self.x - o.x, self.y - o.y)
    }
}

pub fn rot_i(x: i32, y: i32, r: u8) -> P {
    match r % 4 {
        0 => P::new(x, y),
        1 => P::new(-y, x),
        2 => P::new(-x, -y),
        _ => P::new(y, -x),
    }
}

pub fn rot_f(x: f32, y: f32, r: u8) -> (f32, f32) {
    match r % 4 {
        0 => (x, y),
        1 => (-y, x),
        2 => (-x, -y),
        _ => (y, -x),
    }
}

/// Electrical potential delivered by a source terminal.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Pot {
    L1,
    L2,
    L3,
    N,
    PE,
    P24,
    M0,
}

impl Pot {
    pub fn name(self) -> &'static str {
        match self {
            Pot::L1 => "L1",
            Pot::L2 => "L2",
            Pot::L3 => "L3",
            Pot::N => "N",
            Pot::PE => "PE",
            Pot::P24 => "+24V",
            Pot::M0 => "0V",
        }
    }
    pub fn is_ac(self) -> bool {
        matches!(self, Pot::L1 | Pot::L2 | Pot::L3 | Pot::N)
    }
    pub fn is_phase(self) -> bool {
        matches!(self, Pot::L1 | Pot::L2 | Pot::L3)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Kind {
    TermL1,
    TermL2,
    TermL3,
    TermN,
    TermPE,
    TermP24,
    Term0V,
    Fuse,
    Breaker1,
    Breaker3,
    Thermal3,
    PushNo,
    PushNc,
    Switch,
    EStop,
    LimitNo,
    LimitNc,
    Coil,
    TimerOn,
    TimerOff,
    ContactNo,
    ContactNc,
    Main3,
    Lamp,
    Buzzer,
    Motor1,
    Motor3,
    /// Three-phase motor with six terminals (star or delta closed outside).
    Motor6,
    /// Two-pole circuit breaker.
    Breaker2,
    /// Residual-current device (DR), two poles (phase + neutral).
    Rcd2,
    /// Residual-current device (DR), four poles (3 phases + neutral).
    Rcd4,
}

/// Palette categories, in library order, each with its marker colour
/// (the coloured line under each part in the library).
pub fn categories() -> Vec<(&'static str, Vec<Kind>)> {
    CATEGORY_DEFS
        .iter()
        .map(|(pt, en, _, kinds)| (t(pt, en), kinds.to_vec()))
        .collect()
}

type CategoryDef = (&'static str, &'static str, [u8; 3], &'static [Kind]);

pub const CATEGORY_DEFS: [CategoryDef; 8] = {
    use Kind::*;
    [
        ("Fontes", "Sources", [76, 175, 80], &[TermL1, TermL2, TermL3, TermN, TermPE, TermP24, Term0V]),
        ("Bobinas", "Coils", [212, 194, 43], &[Coil, TimerOn, TimerOff]),
        ("Auxiliares", "Auxiliaries", [47, 134, 214], &[ContactNo, ContactNc]),
        ("Comandos", "Switches", [156, 72, 190], &[PushNo, PushNc, Switch, EStop, LimitNo, LimitNc]),
        ("Cargas", "Loads", [141, 122, 110], &[Lamp, Buzzer]),
        ("Contatores", "Contactors", [47, 198, 214], &[Main3]),
        ("Proteções", "Protections", [214, 69, 58], &[Fuse, Breaker1, Breaker2, Breaker3, Rcd2, Rcd4, Thermal3]),
        ("Motores", "Motors", [31, 150, 130], &[Motor1, Motor3, Motor6]),
    ]
};

/// Marker colour of the category a part belongs to.
pub fn category_color(k: Kind) -> [u8; 3] {
    CATEGORY_DEFS
        .iter()
        .find(|(_, _, _, kinds)| kinds.contains(&k))
        .map(|d| d.2)
        .unwrap_or([150, 150, 150])
}

const PINS_1: &[(i32, i32)] = &[(0, 0)];
const PINS_2: &[(i32, i32)] = &[(0, 0), (0, 6)];
const PINS_3P: &[(i32, i32)] = &[(0, 0), (3, 0), (6, 0), (0, 6), (3, 6), (6, 6)];
const PINS_M3: &[(i32, i32)] = &[(0, 0), (3, 0), (6, 0), (9, 0)];
const PINS_2P: &[(i32, i32)] = &[(0, 0), (3, 0), (0, 6), (3, 6)];
const PINS_4P: &[(i32, i32)] = &[(0, 0), (3, 0), (6, 0), (9, 0), (0, 6), (3, 6), (6, 6), (9, 6)];
/// U1 V1 W1 (top), U2 V2 W2 (bottom), PE.
const PINS_M6: &[(i32, i32)] = &[(0, 0), (3, 0), (6, 0), (0, 11), (3, 11), (6, 11), (9, 0)];

impl Kind {
    pub fn pins(self) -> &'static [(i32, i32)] {
        use Kind::*;
        match self {
            TermL1 | TermL2 | TermL3 | TermN | TermPE | TermP24 | Term0V => PINS_1,
            Breaker3 | Thermal3 | Main3 => PINS_3P,
            Motor3 => PINS_M3,
            Motor6 => PINS_M6,
            Breaker2 | Rcd2 => PINS_2P,
            Rcd4 => PINS_4P,
            _ => PINS_2,
        }
    }

    /// Number of switching poles (top pins i connect to bottom pins i + poles).
    pub fn poles(self) -> usize {
        use Kind::*;
        match self {
            Breaker2 | Rcd2 => 2,
            Breaker3 | Thermal3 | Main3 => 3,
            Rcd4 => 4,
            _ => 1,
        }
    }

    /// Breakers and DRs: closed when set, open when tripped or switched off.
    pub fn is_breaker(self) -> bool {
        matches!(self, Kind::Breaker1 | Kind::Breaker2 | Kind::Breaker3 | Kind::Rcd2 | Kind::Rcd4)
    }

    /// Parts the student operates with a click during the simulation.
    pub fn operable(self) -> bool {
        use Kind::*;
        matches!(self, PushNo | PushNc | Switch | EStop | LimitNo | LimitNc | Thermal3) || self.is_breaker()
    }

    /// Contacts of one button / switch: parts with the same tag and family move together.
    pub fn operator_family(self) -> Option<u8> {
        use Kind::*;
        match self {
            PushNo | PushNc => Some(0),
            Switch => Some(1),
            EStop => Some(2),
            LimitNo | LimitNc => Some(3),
            _ => None,
        }
    }

    /// Local bounding box (min x, min y, max x, max y) used for picking.
    pub fn bounds(self) -> (f32, f32, f32, f32) {
        use Kind::*;
        match self {
            TermL1 | TermL2 | TermL3 | TermN | TermPE | TermP24 | Term0V => (-1.2, -3.2, 1.2, 0.3),
            Breaker3 => (-5.0, 0.0, 7.0, 6.0),
            Thermal3 | Main3 => (-1.5, 0.0, 7.5, 6.0),
            Motor3 => (-0.5, 0.0, 9.5, 9.0),
            Motor6 => (-0.5, 0.0, 9.5, 11.0),
            Breaker1 => (-5.0, 0.0, 1.5, 6.0),
            Breaker2 => (-5.0, 0.0, 4.5, 6.0),
            Rcd2 => (-5.0, 0.0, 4.5, 6.0),
            Rcd4 => (-5.0, 0.0, 10.5, 6.0),
            PushNo | PushNc | Switch | EStop | LimitNo | LimitNc => (-4.2, 0.0, 1.5, 6.0),
            Coil | TimerOn | TimerOff => (-3.2, 0.0, 1.8, 6.0),
            Motor1 => (-2.4, 0.0, 2.4, 6.0),
            _ => (-1.8, 0.0, 1.8, 6.0),
        }
    }

    pub fn source(self) -> Option<Pot> {
        use Kind::*;
        match self {
            TermL1 => Some(Pot::L1),
            TermL2 => Some(Pot::L2),
            TermL3 => Some(Pot::L3),
            TermN => Some(Pot::N),
            TermPE => Some(Pot::PE),
            TermP24 => Some(Pot::P24),
            Term0V => Some(Pot::M0),
            _ => None,
        }
    }

    pub fn is_terminal(self) -> bool {
        self.source().is_some()
    }

    /// Contacts and power contacts follow another device by its tag.
    pub fn is_linked(self) -> bool {
        matches!(self, Kind::ContactNo | Kind::ContactNc | Kind::Main3)
    }

    /// Devices whose state can be followed by contacts.
    pub fn is_device(self) -> bool {
        matches!(
            self,
            Kind::Coil | Kind::TimerOn | Kind::TimerOff | Kind::Thermal3
        ) || self.is_breaker()
    }

    pub fn prefix(self) -> &'static str {
        use Kind::*;
        match self {
            Fuse | Thermal3 => "F",
            Breaker1 | Breaker2 | Breaker3 => "Q",
            Rcd2 | Rcd4 => "DR",
            PushNo | PushNc | Switch | EStop => "S",
            LimitNo | LimitNc => "SQ",
            Coil => "KM",
            TimerOn | TimerOff => "KT",
            Lamp => "H",
            Buzzer => "HA",
            Motor1 | Motor3 | Motor6 => "M",
            _ => "",
        }
    }

    pub fn name(self) -> &'static str {
        use Kind::*;
        match self {
            TermL1 => t("Fase L1", "Phase L1"),
            TermL2 => t("Fase L2", "Phase L2"),
            TermL3 => t("Fase L3", "Phase L3"),
            TermN => t("Neutro N", "Neutral N"),
            TermPE => t("Terra PE", "Earth PE"),
            TermP24 => t("+V CC (12/24 V)", "+V DC (12/24 V)"),
            Term0V => t("0 V CC", "0 V DC"),
            Fuse => t("Fusível", "Fuse"),
            Breaker1 => t("Disjuntor 1 polo", "Breaker 1-pole"),
            Breaker3 => t("Disjuntor motor 3 polos", "Motor breaker 3-pole"),
            Thermal3 => t("Relé térmico", "Overload relay"),
            PushNo => t("Botão NA (liga)", "Push button NO"),
            PushNc => t("Botão NF (desliga)", "Push button NC"),
            Switch => t("Chave seletora", "Selector switch"),
            EStop => t("Emergência", "Emergency stop"),
            LimitNo => t("Fim de curso NA", "Limit switch NO"),
            LimitNc => t("Fim de curso NF", "Limit switch NC"),
            Coil => t("Bobina contator", "Contactor coil"),
            TimerOn => t("Temporizador ao trabalho", "On-delay timer"),
            TimerOff => t("Temporizador ao repouso", "Off-delay timer"),
            ContactNo => t("Contato auxiliar NA", "Aux contact NO"),
            ContactNc => t("Contato auxiliar NF", "Aux contact NC"),
            Main3 => t("Contatos principais", "Main contacts"),
            Lamp => t("Lâmpada sinalizadora", "Pilot lamp"),
            Buzzer => t("Sirene", "Buzzer"),
            Motor1 => t("Motor monofásico / CC", "Motor 1~ / DC"),
            Motor3 => t("Motor trifásico (fechamento interno)", "Motor 3~ (internal connection)"),
            Motor6 => t("Motor trifásico 6 terminais (Y/Δ)", "Motor 3~ 6 terminals (Y/Δ)"),
            Breaker2 => t("Disjuntor 2 polos (bipolar)", "Breaker 2-pole"),
            Rcd2 => t("DR bipolar (2 polos)", "RCD 2-pole"),
            Rcd4 => t("DR tetrapolar (4 polos)", "RCD 4-pole"),
        }
    }
}

/// Runtime state used during simulation (never saved).
#[derive(Clone, Debug, Default)]
pub struct St {
    /// Momentary push button held down.
    pub pressed: bool,
    /// Latched mechanical state: switch on, e-stop pushed, limit actuated,
    /// breaker / overload tripped.
    pub on: bool,
    /// Load has voltage across it.
    pub energized: bool,
    pub timer: f32,
    /// Timer output contacts switched.
    pub out: bool,
    /// Motor direction (-1, 0, 1).
    pub motor: i8,
    pub angle: f32,
    /// 3-phase motor missing a phase.
    pub hum: bool,
    /// Voltage applied to the load (V, magnitude).
    pub volt: f32,
    /// Voltage check: 0 ok, -1 under, 1 over, 2 wrong kind (AC/DC).
    pub vstat: i8,
    /// Lamp burnt by overvoltage (stays burnt until the simulation stops).
    pub burnt: bool,
    /// Lamp brightness 0..1 (dim on undervoltage).
    pub bright: f32,
    /// 6-terminal motor connection: 1 star, 2 delta, 0 unknown.
    pub conn: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Comp {
    pub id: u32,
    pub kind: Kind,
    pub pos: P,
    #[serde(default)]
    pub rot: u8,
    #[serde(default)]
    pub tag: String,
    /// Timer preset in seconds.
    #[serde(default = "default_preset")]
    pub preset: f32,
    /// Lamp colour index / contact numbering index.
    #[serde(default)]
    pub color: u8,
    #[serde(default)]
    pub num: u8,
    /// Chosen voltage, as an index into the part's table (see `volt_choices`):
    /// supply system for phases, +12/+24 V for DC, nominal voltage for loads.
    #[serde(default)]
    pub volt: u8,
    #[serde(skip)]
    pub st: St,
}

/// Alternating or direct current.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cur {
    Ac,
    Dc,
}

/// AC supply systems: (phase-neutral, line-line) volts.
pub const AC_SYSTEMS: [(f32, f32); 2] = [(220.0, 380.0), (127.0, 220.0)];
/// DC positive terminal options.
pub const DC_LEVELS: [f32; 2] = [24.0, 12.0];
/// Nominal voltages of lamps, buzzers and coils (first is the default).
pub const LOAD_NOMINALS: [(Cur, f32); 6] = [
    (Cur::Ac, 220.0),
    (Cur::Ac, 127.0),
    (Cur::Ac, 380.0),
    (Cur::Ac, 24.0),
    (Cur::Dc, 24.0),
    (Cur::Dc, 12.0),
];
/// Single-phase / DC motor nominal voltages.
pub const MOTOR1_NOMINALS: [(Cur, f32); 4] = [(Cur::Ac, 220.0), (Cur::Ac, 127.0), (Cur::Dc, 24.0), (Cur::Dc, 12.0)];
/// Three-phase motor with internal connection: line voltage.
pub const MOTOR3_NOMINALS: [f32; 3] = [380.0, 220.0, 440.0];
/// Six-terminal motor: (delta, star) line voltages. Each winding is rated for the delta value.
pub const MOTOR6_NOMINALS: [(f32, f32); 3] = [(220.0, 380.0), (380.0, 660.0), (127.0, 220.0)];

fn vtxt(v: f32) -> String {
    format!("{v:.0} V")
}

fn cur_txt(c: Cur) -> &'static str {
    match c {
        Cur::Ac => t("CA", "AC"),
        Cur::Dc => t("CC", "DC"),
    }
}

/// The fixed list a student picks from (no free typing), for this kind of part.
/// Empty when the part has no voltage setting.
pub fn volt_choices(k: Kind) -> Vec<String> {
    use Kind::*;
    match k {
        TermL1 | TermL2 | TermL3 => AC_SYSTEMS
            .iter()
            .map(|(f, l)| format!("{f:.0} V / {l:.0} V CA ({})", t("fase/linha", "phase/line")))
            .collect(),
        TermP24 => DC_LEVELS.iter().map(|v| format!("+{v:.0} V CC")).collect(),
        Lamp | Buzzer | Coil | TimerOn | TimerOff => LOAD_NOMINALS
            .iter()
            .map(|(c, v)| format!("{} {}", vtxt(*v), cur_txt(*c)))
            .collect(),
        Motor1 => MOTOR1_NOMINALS
            .iter()
            .map(|(c, v)| format!("{} {}", vtxt(*v), cur_txt(*c)))
            .collect(),
        Motor3 => MOTOR3_NOMINALS.iter().map(|v| format!("{} CA ({})", vtxt(*v), t("entre fases", "line"))).collect(),
        Motor6 => MOTOR6_NOMINALS
            .iter()
            .map(|(d, y)| format!("Δ {d:.0} V / Y {y:.0} V"))
            .collect(),
        _ => vec![],
    }
}

/// Label of the voltage setting in the properties panel.
pub fn volt_label(k: Kind) -> &'static str {
    use Kind::*;
    match k {
        TermL1 | TermL2 | TermL3 => t("Sistema", "System"),
        TermP24 => t("Tensão", "Voltage"),
        _ => t("Tensão nominal", "Rated voltage"),
    }
}

/// Nominal (kind of current, volts) of a load; for 6-terminal motors the delta (winding) value.
pub fn nominal(c: &Comp) -> Option<(Cur, f32)> {
    use Kind::*;
    let i = c.volt as usize;
    match c.kind {
        Lamp | Buzzer | Coil | TimerOn | TimerOff => LOAD_NOMINALS.get(i).or(LOAD_NOMINALS.first()).copied(),
        Motor1 => MOTOR1_NOMINALS.get(i).or(MOTOR1_NOMINALS.first()).copied(),
        Motor3 => Some((Cur::Ac, *MOTOR3_NOMINALS.get(i).unwrap_or(&MOTOR3_NOMINALS[0]))),
        Motor6 => Some((Cur::Ac, MOTOR6_NOMINALS.get(i).unwrap_or(&MOTOR6_NOMINALS[0]).0)),
        _ => None,
    }
}

/// Short text of a part's chosen voltage (shown on the sheet), e.g. "220V~", "+12V".
pub fn volt_badge(c: &Comp) -> Option<String> {
    use Kind::*;
    let i = c.volt as usize;
    match c.kind {
        TermL1 | TermL2 | TermL3 => AC_SYSTEMS.get(i).map(|(f, l)| format!("{f:.0}/{l:.0}V")),
        Lamp | Buzzer | Coil | TimerOn | TimerOff | Motor1 | Motor3 => nominal(c).map(|(k, v)| {
            format!("{v:.0}V{}", if k == Cur::Ac { "~" } else { "=" })
        }),
        Motor6 => MOTOR6_NOMINALS.get(i).map(|(d, y)| format!("Δ{d:.0}/Y{y:.0}V")),
        _ => None,
    }
}

fn default_preset() -> f32 {
    3.0
}

impl Comp {
    pub fn pin(&self, i: usize) -> P {
        let (x, y) = self.kind.pins()[i];
        self.pos + rot_i(x, y, self.rot)
    }
    pub fn pins(&self) -> Vec<P> {
        (0..self.kind.pins().len()).map(|i| self.pin(i)).collect()
    }
    /// World-space bounding box in grid units.
    pub fn world_bounds(&self) -> (f32, f32, f32, f32) {
        let (a, b, c, d) = self.kind.bounds();
        let pts = [(a, b), (c, b), (a, d), (c, d)];
        let mut mn = (f32::MAX, f32::MAX);
        let mut mx = (f32::MIN, f32::MIN);
        for (x, y) in pts {
            let (rx, ry) = rot_f(x, y, self.rot);
            let (wx, wy) = (rx + self.pos.x as f32, ry + self.pos.y as f32);
            mn.0 = mn.0.min(wx);
            mn.1 = mn.1.min(wy);
            mx.0 = mx.0.max(wx);
            mx.1 = mx.1.max(wy);
        }
        (mn.0, mn.1, mx.0, mx.1)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Wire {
    pub a: P,
    pub b: P,
}

impl Wire {
    /// True if `p` lies on the segment but is not one of its end points.
    pub fn interior(&self, p: P) -> bool {
        if p == self.a || p == self.b {
            return false;
        }
        let (dx, dy) = (self.b.x - self.a.x, self.b.y - self.a.y);
        let (px, py) = (p.x - self.a.x, p.y - self.a.y);
        if dx as i64 * py as i64 - dy as i64 * px as i64 != 0 {
            return false;
        }
        p.x >= self.a.x.min(self.b.x)
            && p.x <= self.a.x.max(self.b.x)
            && p.y >= self.a.y.min(self.b.y)
            && p.y <= self.a.y.max(self.b.y)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Circuit {
    pub comps: Vec<Comp>,
    pub wires: Vec<Wire>,
    #[serde(default)]
    pub next_id: u32,
}

impl Circuit {
    pub fn add(&mut self, kind: Kind, pos: P, rot: u8, tag: &str) -> u32 {
        self.next_id = self.next_id.max(self.comps.iter().map(|c| c.id + 1).max().unwrap_or(0));
        let id = self.next_id;
        self.next_id += 1;
        let num = self.next_num(kind, tag);
        self.comps.push(Comp {
            id,
            kind,
            pos,
            rot,
            tag: tag.to_string(),
            preset: 3.0,
            color: 0,
            num,
            volt: 0,
            st: St::default(),
        });
        id
    }

    pub fn wire(&mut self, a: (i32, i32), b: (i32, i32)) {
        let (a, b) = (P::new(a.0, a.1), P::new(b.0, b.1));
        if a != b {
            self.wires.push(Wire { a, b });
        }
    }

    pub fn comp(&self, id: u32) -> Option<&Comp> {
        self.comps.iter().find(|c| c.id == id)
    }

    pub fn comp_mut(&mut self, id: u32) -> Option<&mut Comp> {
        self.comps.iter_mut().find(|c| c.id == id)
    }

    /// Kind of the device a tag refers to (coil, timer, breaker, overload).
    pub fn device_kind(&self, tag: &str) -> Option<Kind> {
        self.comps
            .iter()
            .find(|c| c.kind.is_device() && c.tag == tag)
            .map(|c| c.kind)
    }

    pub fn device_tags(&self) -> Vec<(String, Kind)> {
        let mut v: Vec<(String, Kind)> = Vec::new();
        for c in &self.comps {
            if c.kind.is_device() && !v.iter().any(|(t, _)| *t == c.tag) {
                v.push((c.tag.clone(), c.kind));
            }
        }
        v
    }

    /// Next free contact numbering index for a contact following `tag`.
    fn next_num(&self, kind: Kind, tag: &str) -> u8 {
        self.free_num(u32::MAX, kind, tag)
    }

    /// First contact number not yet used by another contact of the same device
    /// (auxiliary contacts of a coil, or contacts of one button / switch).
    pub fn free_num(&self, except: u32, kind: Kind, tag: &str) -> u8 {
        let group = |k: Kind| -> u8 {
            if matches!(k, Kind::ContactNo | Kind::ContactNc) {
                100
            } else {
                k.operator_family().unwrap_or(200)
            }
        };
        let g = group(kind);
        if g == 200 || tag.is_empty() {
            return 0;
        }
        let used: Vec<u8> = self
            .comps
            .iter()
            .filter(|c| c.id != except && group(c.kind) == g && c.tag == tag)
            .map(|c| c.num)
            .collect();
        (0..9u8).find(|n| !used.contains(n)).unwrap_or(0)
    }

    /// Suggest a tag for a new component of this kind.
    pub fn next_tag(&self, kind: Kind) -> String {
        if kind.is_linked() {
            // Follow the most recently placed coil-like device.
            let want_main = kind == Kind::Main3;
            let dev = self.comps.iter().rev().find(|c| {
                if want_main {
                    c.kind == Kind::Coil
                } else {
                    c.kind.is_device()
                }
            });
            return dev.map(|c| c.tag.clone()).unwrap_or_else(|| "KM1".into());
        }
        let pre = kind.prefix();
        if pre.is_empty() {
            return String::new();
        }
        let mut max = 0;
        for c in &self.comps {
            if let Some(rest) = c.tag.strip_prefix(pre) {
                if let Ok(n) = rest.parse::<i32>() {
                    max = max.max(n);
                }
            }
        }
        format!("{pre}{}", max + 1)
    }

    pub fn reset_state(&mut self) {
        for c in &mut self.comps {
            c.st = St::default();
        }
    }
}

/// Terminal numbers printed next to a component's pins.
pub fn pin_numbers(c: &Comp, linked: Option<Kind>) -> Vec<String> {
    use Kind::*;
    let n = c.num as usize + 1;
    let timer = matches!(linked, Some(TimerOn) | Some(TimerOff));
    match c.kind {
        ContactNo if timer => vec![format!("{}7", n + 5), format!("{}8", n + 5)],
        ContactNc if timer => vec![format!("{}5", n + 4), format!("{}6", n + 4)],
        ContactNo => vec![format!("{n}3"), format!("{n}4")],
        ContactNc => vec![format!("{n}1"), format!("{n}2")],
        // Button / switch contacts: contact 1 = 13-14 or 11-12, contact 2 = 23-24 or 21-22...
        PushNo | Switch | LimitNo => vec![format!("{n}3"), format!("{n}4")],
        PushNc | EStop | LimitNc => vec![format!("{n}1"), format!("{n}2")],
        Coil | TimerOn | TimerOff => vec!["A1".into(), "A2".into()],
        Lamp | Buzzer => vec!["X1".into(), "X2".into()],
        Fuse | Breaker1 => vec!["1".into(), "2".into()],
        Breaker2 => ["1", "3", "2", "4"].iter().map(|s| s.to_string()).collect(),
        Rcd2 => ["1", "N", "2", "N"].iter().map(|s| s.to_string()).collect(),
        Rcd4 => ["1", "3", "5", "N", "2", "4", "6", "N"].iter().map(|s| s.to_string()).collect(),
        Breaker3 | Main3 | Thermal3 => ["1", "3", "5", "2", "4", "6"].iter().map(|s| s.to_string()).collect(),
        Motor3 => ["U1", "V1", "W1", "PE"].iter().map(|s| s.to_string()).collect(),
        Motor6 => ["U1", "V1", "W1", "U2", "V2", "W2", "PE"].iter().map(|s| s.to_string()).collect(),
        Motor1 => vec!["U1".into(), "U2".into()],
        _ => vec![],
    }
}

/// Lamp colours available in the properties panel.
pub const LAMP_COLORS: [(&str, &str, [u8; 3]); 5] = [
    ("Verde", "Green", [46, 204, 113]),
    ("Vermelho", "Red", [235, 64, 52]),
    ("Amarelo", "Amber", [250, 190, 40]),
    ("Azul", "Blue", [52, 152, 235]),
    ("Branco", "White", [236, 240, 241]),
];
