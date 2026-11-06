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
        ("Proteções", "Protections", [214, 69, 58], &[Fuse, Breaker1, Breaker3, Thermal3]),
        ("Motores", "Motors", [31, 150, 130], &[Motor1, Motor3]),
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

impl Kind {
    pub fn pins(self) -> &'static [(i32, i32)] {
        use Kind::*;
        match self {
            TermL1 | TermL2 | TermL3 | TermN | TermPE | TermP24 | Term0V => PINS_1,
            Breaker3 | Thermal3 | Main3 => PINS_3P,
            Motor3 => PINS_M3,
            _ => PINS_2,
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
            Breaker1 => (-5.0, 0.0, 1.5, 6.0),
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
            Kind::Coil | Kind::TimerOn | Kind::TimerOff | Kind::Thermal3 | Kind::Breaker1 | Kind::Breaker3
        )
    }

    pub fn prefix(self) -> &'static str {
        use Kind::*;
        match self {
            Fuse | Thermal3 => "F",
            Breaker1 | Breaker3 => "Q",
            PushNo | PushNc | Switch | EStop => "S",
            LimitNo | LimitNc => "SQ",
            Coil => "KM",
            TimerOn | TimerOff => "KT",
            Lamp => "H",
            Buzzer => "HA",
            Motor1 | Motor3 => "M",
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
            TermP24 => t("+24 V CC", "+24 V DC"),
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
            Motor3 => t("Motor trifásico", "Motor 3~"),
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
    #[serde(skip)]
    pub st: St,
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
        if !matches!(kind, Kind::ContactNo | Kind::ContactNc) {
            return 0;
        }
        let used: Vec<u8> = self
            .comps
            .iter()
            .filter(|c| matches!(c.kind, Kind::ContactNo | Kind::ContactNc) && c.tag == tag)
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
        PushNo | Switch | LimitNo => vec!["13".into(), "14".into()],
        PushNc | EStop | LimitNc => vec!["11".into(), "12".into()],
        Coil | TimerOn | TimerOff => vec!["A1".into(), "A2".into()],
        Lamp | Buzzer => vec!["X1".into(), "X2".into()],
        Fuse | Breaker1 => vec!["1".into(), "2".into()],
        Breaker3 | Main3 | Thermal3 => ["1", "3", "5", "2", "4", "6"].iter().map(|s| s.to_string()).collect(),
        Motor3 => ["U1", "V1", "W1", "PE"].iter().map(|s| s.to_string()).collect(),
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
