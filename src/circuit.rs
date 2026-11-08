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
    /// Motor protective circuit breaker, three poles (thermal + magnetic trip, manual operator).
    MotorBreaker3,
    /// Six-terminal motor drawn with crossed windings (U1→U2, V1→V2, W1→W2)
    /// and the bottom terminals in W2 U2 V2 order (delta links are straight).
    Motor6X,
    /// Twelve-terminal motor: two half-windings per phase (series / parallel, star / delta).
    Motor12,
    /// Single-phase motor: main winding U1-U2 and auxiliary start winding Z1-Z2.
    Motor1S,
    /// Single-phase dual-voltage motor: main windings U1-U2 and U3-U4 (parallel / series)
    /// and auxiliary start winding Z1-Z2.
    Motor1D,
    /// Changeover auxiliary contact: common, NC and NO.
    ContactCo,
    /// Free text on the drawing.
    Text,
    /// Two-speed Dahlander motor: 1U 1V 1W (low speed, Δ) and 2U 2V 2W (high speed, YY).
    MotorDahlander,
    /// Two-speed motor with two separate windings: 1U 1V 1W (low) and 2U 2V 2W (high).
    Motor2W,
    /// Rectifier module for an electromagnetic brake: AC in (~ ~), DC out (+ −).
    Rectifier,
    /// Electromagnetic brake coil (DC), released while it has its rated voltage.
    Brake,
    /// Three-phase supply block: L1 L2 L3 side by side.
    Supply3,
    /// Supply block: L1 L2 L3 N PE side by side.
    Supply5,
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

pub const CATEGORY_DEFS: [CategoryDef; 9] = {
    use Kind::*;
    [
        ("Fontes", "Sources", [76, 175, 80], &[TermL1, Supply3, Supply5, TermN, TermPE, TermP24, Term0V]),
        ("Bobinas", "Coils", [212, 194, 43], &[Coil, TimerOn, TimerOff]),
        ("Auxiliares", "Auxiliaries", [47, 134, 214], &[ContactNo, ContactNc, ContactCo]),
        ("Comandos", "Switches", [156, 72, 190], &[PushNo, PushNc, Switch, EStop, LimitNo, LimitNc]),
        ("Cargas", "Loads", [141, 122, 110], &[Lamp, Buzzer, Brake]),
        ("Contatores", "Contactors", [47, 198, 214], &[Main3]),
        (
            "Proteções",
            "Protections",
            [214, 69, 58],
            &[Fuse, Breaker1, Breaker2, Breaker3, MotorBreaker3, Rcd2, Rcd4, Thermal3],
        ),
        (
            "Motores",
            "Motors",
            [31, 150, 130],
            &[Motor1, Motor1S, Motor1D, Motor3, Motor6X, Motor6, Motor12, MotorDahlander, Motor2W],
        ),
        ("Outros", "Others", [150, 150, 150], &[Rectifier, Text]),
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
const PINS_S3: &[(i32, i32)] = &[(0, 0), (3, 0), (6, 0)];
const PINS_S5: &[(i32, i32)] = &[(0, 0), (3, 0), (6, 0), (9, 0), (12, 0)];
const PINS_2: &[(i32, i32)] = &[(0, 0), (0, 6)];
const PINS_3P: &[(i32, i32)] = &[(0, 0), (3, 0), (6, 0), (0, 6), (3, 6), (6, 6)];
const PINS_M3: &[(i32, i32)] = &[(0, 0), (3, 0), (6, 0), (9, 0)];
const PINS_2P: &[(i32, i32)] = &[(0, 0), (3, 0), (0, 6), (3, 6)];
const PINS_4P: &[(i32, i32)] = &[(0, 0), (3, 0), (6, 0), (9, 0), (0, 6), (3, 6), (6, 6), (9, 6)];
/// U1 V1 W1 (top), U2 V2 W2 (bottom), PE.
const PINS_M6: &[(i32, i32)] = &[(0, 0), (3, 0), (6, 0), (0, 11), (3, 11), (6, 11), (9, 0)];
/// Crossed windings: U1 V1 W1 on top, bottom row W2 U2 V2 (so U2 sits under V1...).
const PINS_M6X: &[(i32, i32)] = &[(0, 0), (3, 0), (6, 0), (3, 11), (6, 11), (0, 11), (9, 0)];
/// U1 V1 W1 U2 V2 W2 U5 V5 W5 U6 V6 W6 PE.
const PINS_M12: &[(i32, i32)] = &[
    (0, 0),
    (3, 0),
    (6, 0),
    (0, 12),
    (3, 12),
    (6, 12),
    (9, 0),
    (12, 0),
    (15, 0),
    (9, 12),
    (12, 12),
    (15, 12),
    (18, 0),
];
/// U1 U2 (main) Z1 Z2 (auxiliary).
const PINS_M1S: &[(i32, i32)] = &[(0, 0), (0, 8), (4, 0), (4, 8)];
/// U1 U2 U3 U4 (main halves) Z1 Z2 (auxiliary).
const PINS_M1D: &[(i32, i32)] = &[(0, 0), (0, 10), (3, 0), (3, 10), (6, 0), (6, 10)];
/// Common (top, centred), NC (bottom left), NO (bottom right).
const PINS_CO: &[(i32, i32)] = &[(0, 0), (-2, 6), (2, 6)];
/// Rectifier: ~ ~ (top), + − (bottom).
const PINS_RECT: &[(i32, i32)] = &[(0, 0), (3, 0), (0, 6), (3, 6)];
const PINS_0: &[(i32, i32)] = &[];

impl Kind {
    pub fn pins(self) -> &'static [(i32, i32)] {
        use Kind::*;
        match self {
            TermL1 | TermL2 | TermL3 | TermN | TermPE | TermP24 | Term0V => PINS_1,
            Supply3 => PINS_S3,
            Supply5 => PINS_S5,
            Breaker3 | MotorBreaker3 | Thermal3 | Main3 => PINS_3P,
            Motor3 => PINS_M3,
            Motor6 | MotorDahlander | Motor2W => PINS_M6,
            Motor6X => PINS_M6X,
            Rectifier => PINS_RECT,
            Motor12 => PINS_M12,
            Motor1S => PINS_M1S,
            Motor1D => PINS_M1D,
            ContactCo => PINS_CO,
            Text => PINS_0,
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
            Breaker3 | MotorBreaker3 | Thermal3 | Main3 => 3,
            Rcd4 => 4,
            _ => 1,
        }
    }

    /// Breakers and DRs: closed when set, open when tripped or switched off.
    pub fn is_breaker(self) -> bool {
        matches!(
            self,
            Kind::Breaker1 | Kind::Breaker2 | Kind::Breaker3 | Kind::MotorBreaker3 | Kind::Rcd2 | Kind::Rcd4
        )
    }

    /// Three-phase motors with six windings ends outside (star / delta closed by the student).
    pub fn is_motor6(self) -> bool {
        matches!(self, Kind::Motor6 | Kind::Motor6X)
    }

    pub fn is_motor(self) -> bool {
        use Kind::*;
        matches!(self, Motor1 | Motor1S | Motor1D | Motor3 | Motor6 | Motor6X | Motor12 | MotorDahlander | Motor2W)
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
            Supply3 => (-1.4, -3.4, 7.4, 0.3),
            Supply5 => (-1.4, -3.4, 13.4, 0.3),
            Breaker3 => (-5.0, 0.0, 7.0, 6.0),
            MotorBreaker3 => (-5.6, 0.0, 7.8, 6.0),
            Thermal3 | Main3 => (-1.5, 0.0, 7.5, 6.0),
            Motor3 => (-2.0, 0.0, 9.5, 10.4),
            Motor6 | Motor6X | MotorDahlander | Motor2W => (-2.0, 0.0, 9.5, 11.0),
            Motor12 => (-2.0, 0.0, 18.5, 12.0),
            Rectifier => (-1.4, 0.0, 4.4, 6.0),
            Brake => (-1.8, 0.0, 1.8, 6.0),
            Motor1S => (-2.2, 0.0, 6.2, 8.0),
            Motor1D => (-2.4, 0.0, 8.4, 10.0),
            ContactNo | ContactNc => (-3.6, 0.0, 1.6, 6.0),
            ContactCo => (-5.0, 0.0, 2.8, 6.0),
            Text => (0.0, -1.0, 6.0, 1.0),
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

    /// Source potential of each pin (terminals and supply blocks).
    pub fn sources(self) -> &'static [Pot] {
        use Kind::*;
        match self {
            TermL1 => &[Pot::L1],
            TermL2 => &[Pot::L2],
            TermL3 => &[Pot::L3],
            TermN => &[Pot::N],
            TermPE => &[Pot::PE],
            TermP24 => &[Pot::P24],
            Term0V => &[Pot::M0],
            Supply3 => &[Pot::L1, Pot::L2, Pot::L3],
            Supply5 => &[Pot::L1, Pot::L2, Pot::L3, Pot::N, Pot::PE],
            _ => &[],
        }
    }

    pub fn is_terminal(self) -> bool {
        !self.sources().is_empty()
    }

    /// One phase terminal (L1, L2 or L3, chosen in the properties).
    pub fn is_phase(self) -> bool {
        matches!(self, Kind::TermL1 | Kind::TermL2 | Kind::TermL3)
    }

    /// Name in the parts library (the single phase terminal covers L1/L2/L3).
    pub fn lib_name(self) -> &'static str {
        if self.is_phase() {
            t("Fase (L1 / L2 / L3)", "Phase (L1 / L2 / L3)")
        } else {
            self.name()
        }
    }

    /// Contacts and power contacts follow another device by its tag.
    pub fn is_linked(self) -> bool {
        matches!(self, Kind::ContactNo | Kind::ContactNc | Kind::ContactCo | Kind::Main3)
    }

    /// Auxiliary contacts (numbered per device).
    pub fn is_aux_contact(self) -> bool {
        matches!(self, Kind::ContactNo | Kind::ContactNc | Kind::ContactCo)
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
            Breaker1 | Breaker2 | Breaker3 | MotorBreaker3 => "Q",
            Rcd2 | Rcd4 => "DR",
            Rectifier => "U",
            Brake => "YB",
            PushNo | PushNc | Switch | EStop => "S",
            LimitNo | LimitNc => "SQ",
            Coil => "KM",
            TimerOn | TimerOff => "KT",
            Lamp => "H",
            Buzzer => "HA",
            k if k.is_motor() => "M",
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
            Breaker1 => t("Disjuntor termomagnético monopolar (1P)", "Thermal-magnetic breaker 1P"),
            Breaker3 => t("Disjuntor termomagnético tripolar (3P)", "Thermal-magnetic breaker 3P"),
            Thermal3 => t("Relé térmico de sobrecarga (3P)", "Thermal overload relay 3P"),
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
            Motor1 => t("Motor monofásico / CC (2 terminais)", "Motor 1~ / DC (2 terminals)"),
            Motor3 => t("Motor trifásico 3 terminais (estrela interna)", "Motor 3~ 3 terminals (internal star)"),
            Motor6 => t("Motor trifásico 6 terminais – bobinas verticais", "Motor 3~ 6 terminals – vertical windings"),
            Motor6X => t("Motor trifásico 6 terminais (Y/Δ)", "Motor 3~ 6 terminals (Y/Δ)"),
            Motor12 => t("Motor trifásico 12 terminais (4 tensões)", "Motor 3~ 12 terminals (4 voltages)"),
            Motor1S => t("Motor monofásico com bobina auxiliar de partida", "Motor 1~ with auxiliary start winding"),
            Motor1D => t("Motor monofásico 2 tensões (127/220 V)", "Motor 1~ dual voltage (127/220 V)"),
            Breaker2 => t("Disjuntor termomagnético bipolar (2P)", "Thermal-magnetic breaker 2P"),
            MotorBreaker3 => t("Disjuntor-motor tripolar (3P)", "Motor protective breaker 3P"),
            Rcd2 => t("DR bipolar (2P)", "RCD 2P"),
            Rcd4 => t("DR tetrapolar (4P)", "RCD 4P"),
            ContactCo => t("Contato auxiliar comutador (NF/NA)", "Aux changeover contact (NC/NO)"),
            Text => t("Texto", "Text"),
            MotorDahlander => t("Motor Dahlander – 2 velocidades", "Dahlander motor – 2 speeds"),
            Motor2W => t("Motor de dois enrolamentos – 2 velocidades", "Two-winding motor – 2 speeds"),
            Rectifier => t("Módulo retificador do freio", "Brake rectifier module"),
            Brake => t("Freio eletromagnético (bobina CC)", "Electromagnetic brake (DC coil)"),
            Supply3 => t("3 fases L1 L2 L3", "3 phases L1 L2 L3"),
            Supply5 => t("L1 L2 L3 N PE", "L1 L2 L3 N PE"),
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
    /// Motor connection found: 6 terminals 1 star / 2 delta; 12 terminals 1 YY, 2 ΔΔ, 3 Y, 4 Δ;
    /// dual-voltage single-phase 1 parallel / 2 series; 0 unknown.
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
pub const AC_SYSTEMS: [(f32, f32); 4] = [(220.0, 380.0), (127.0, 220.0), (254.0, 440.0), (440.0, 760.0)];
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
/// Twelve-terminal motor: (ΔΔ, YY, Δ, Y) voltages; each half-winding is rated for the ΔΔ value.
pub const MOTOR12_NOMINALS: [(f32, f32, f32, f32); 1] = [(220.0, 380.0, 440.0, 760.0)];
/// Single-phase motor with start winding (main and auxiliary rated the same).
pub const MOTOR1S_NOMINALS: [f32; 2] = [220.0, 127.0];
/// Two-speed motors (Dahlander, two windings): line voltage.
pub const MOTOR2S_NOMINALS: [f32; 3] = [380.0, 220.0, 440.0];
/// Brake coil (DC) rated voltages.
pub const BRAKE_NOMINALS: [f32; 3] = [180.0, 99.0, 198.0];
/// Rectifier: DC output as a fraction of the AC input (half / full wave).
pub const RECT_FACTORS: [f32; 2] = [0.45, 0.9];
/// Dual-voltage single-phase motor: (parallel, series) supply voltage.
pub const MOTOR1D_NOMINALS: [(f32, f32); 1] = [(127.0, 220.0)];

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
        TermL1 | TermL2 | TermL3 | Supply3 | Supply5 => AC_SYSTEMS
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
        Motor6 | Motor6X => MOTOR6_NOMINALS
            .iter()
            .map(|(d, y)| format!("Δ {d:.0} V / Y {y:.0} V"))
            .collect(),
        Motor12 => MOTOR12_NOMINALS
            .iter()
            .map(|(a, b, c, d)| format!("ΔΔ {a:.0} / YY {b:.0} / Δ {c:.0} / Y {d:.0} V"))
            .collect(),
        Motor1S => MOTOR1S_NOMINALS.iter().map(|v| format!("{} CA", vtxt(*v))).collect(),
        MotorDahlander | Motor2W => MOTOR2S_NOMINALS.iter().map(|v| format!("{} CA ({})", vtxt(*v), t("entre fases", "line"))).collect(),
        Brake => BRAKE_NOMINALS.iter().map(|v| format!("{} CC", vtxt(*v))).collect(),
        Rectifier => vec![t("Meia onda (CC = 0,45 × CA)", "Half wave (DC = 0.45 × AC)").into(), t("Onda completa (CC = 0,9 × CA)", "Full wave (DC = 0.9 × AC)").into()],
        Motor1D => MOTOR1D_NOMINALS.iter().map(|(p, s)| format!("{p:.0} V {} / {s:.0} V {}", t("paralelo", "parallel"), t("série", "series"))).collect(),
        _ => vec![],
    }
}

/// Label of the voltage setting in the properties panel.
pub fn volt_label(k: Kind) -> &'static str {
    use Kind::*;
    match k {
        TermL1 | TermL2 | TermL3 | Supply3 | Supply5 => t("Sistema", "System"),
        TermP24 => t("Tensão", "Voltage"),
        Rectifier => t("Tipo", "Type"),
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
        Motor6 | Motor6X => Some((Cur::Ac, MOTOR6_NOMINALS.get(i).unwrap_or(&MOTOR6_NOMINALS[0]).0)),
        Motor12 => Some((Cur::Ac, MOTOR12_NOMINALS.get(i).unwrap_or(&MOTOR12_NOMINALS[0]).0)),
        Motor1S => Some((Cur::Ac, *MOTOR1S_NOMINALS.get(i).unwrap_or(&MOTOR1S_NOMINALS[0]))),
        Motor1D => Some((Cur::Ac, MOTOR1D_NOMINALS.get(i).unwrap_or(&MOTOR1D_NOMINALS[0]).0)),
        MotorDahlander | Motor2W => Some((Cur::Ac, *MOTOR2S_NOMINALS.get(i).unwrap_or(&MOTOR2S_NOMINALS[0]))),
        Brake => Some((Cur::Dc, *BRAKE_NOMINALS.get(i).unwrap_or(&BRAKE_NOMINALS[0]))),
        _ => None,
    }
}

/// Short text of a part's chosen voltage (shown on the sheet), e.g. "220V~", "+12V".
pub fn volt_badge(c: &Comp) -> Option<String> {
    use Kind::*;
    let i = c.volt as usize;
    match c.kind {
        TermL1 | TermL2 | TermL3 | Supply3 | Supply5 => AC_SYSTEMS.get(i).map(|(f, l)| format!("{f:.0}/{l:.0}V")),
        Lamp | Buzzer | Coil | TimerOn | TimerOff | Motor1 | Motor3 | Motor1S | MotorDahlander | Motor2W | Brake => nominal(c).map(|(k, v)| {
            format!("{v:.0}V{}", if k == Cur::Ac { "~" } else { "=" })
        }),
        Motor6 | Motor6X => MOTOR6_NOMINALS.get(i).map(|(d, y)| format!("Δ{d:.0}/Y{y:.0}V")),
        Motor12 => Some("220/380/440/760V".into()),
        Motor1D => Some("127/220V".into()),
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
    /// Letter height of a text item, in grid units (size chosen in the properties).
    pub fn text_size(&self) -> f32 {
        [1.3, 1.9, 2.8][(self.num as usize).min(2)]
    }

    /// World-space bounding box in grid units.
    pub fn world_bounds(&self) -> (f32, f32, f32, f32) {
        if self.kind == Kind::Text {
            let h = self.text_size();
            let lines: Vec<&str> = self.tag.split('\n').collect();
            let w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(1).max(1) as f32 * 0.58 * h;
            let (x, y) = (self.pos.x as f32, self.pos.y as f32);
            return (x, y - h * 0.6, x + w, y - h * 0.6 + h * 1.25 * lines.len() as f32);
        }
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
    /// Page size: 0 A4, 1 A3, 2 A2, 3 A1.
    #[serde(default)]
    pub page: u8,
    /// Portrait page (landscape otherwise).
    #[serde(default)]
    pub portrait: bool,
}

/// Page sizes (grid units, landscape): A4 … A1.
pub const PAGES: [(&str, f32, f32); 4] = [("A4", 100.0, 70.0), ("A3", 141.0, 100.0), ("A2", 200.0, 141.0), ("A1", 283.0, 200.0)];

impl Circuit {
    /// Sheet width and height in grid units.
    pub fn sheet(&self) -> (f32, f32) {
        let (_, w, h) = PAGES[(self.page as usize).min(PAGES.len() - 1)];
        if self.portrait {
            (h, w)
        } else {
            (w, h)
        }
    }

    /// Area used by the drawing (parts with room for their tags and labels, and wires).
    pub fn drawing_box(&self) -> Option<(f32, f32, f32, f32)> {
        let mut b: Option<(f32, f32, f32, f32)> = None;
        let mut grow = |x0: f32, y0: f32, x1: f32, y1: f32| {
            b = Some(match b {
                None => (x0, y0, x1, y1),
                Some((a, c, d, e)) => (a.min(x0), c.min(y0), d.max(x1), e.max(y1)),
            });
        };
        for c in &self.comps {
            let (x0, y0, x1, y1) = c.world_bounds();
            if c.kind == Kind::Text {
                grow(x0, y0, x1, y1);
            } else {
                // Tag on the left, terminal names above, labels below / right.
                grow(x0 - 3.5, y0 - 0.5, x1 + 2.5, y1 + 0.5);
            }
        }
        for w in &self.wires {
            for p in [w.a, w.b] {
                grow(p.x as f32 - 0.5, p.y as f32 - 0.5, p.x as f32 + 0.5, p.y as f32 + 0.5);
            }
        }
        b
    }

    /// Choose the smallest page (and the orientation where the drawing fits best)
    /// and centre the drawing inside the page frame.
    pub fn fit_page(&mut self) {
        let Some((x0, y0, x1, y1)) = self.drawing_box() else { return };
        let (w, h) = (x1 - x0, y1 - y0);
        // Frame interior: x 4 … W-2, y 4 … H-2, with a little air around the drawing.
        let mut best: Option<(u8, bool)> = None;
        'pages: for page in 0..PAGES.len() as u8 {
            let mut choice: Option<(f32, bool)> = None;
            for portrait in [false, true] {
                let (_, pw, ph) = PAGES[page as usize];
                let (sw, sh) = if portrait { (ph, pw) } else { (pw, ph) };
                // The drawing box already holds room for tags and labels.
                let (aw, ah) = (sw - 6.0, sh - 6.0);
                if w <= aw && h <= ah {
                    let s = (aw / w.max(1.0)).min(ah / h.max(1.0));
                    // Landscape wins ties.
                    if choice.is_none_or(|(bs, _)| s > bs + 0.05) {
                        choice = Some((s, portrait));
                    }
                }
            }
            if let Some((_, portrait)) = choice {
                best = Some((page, portrait));
                break 'pages;
            }
        }
        let (page, portrait) = best.unwrap_or((PAGES.len() as u8 - 1, false));
        self.page = page;
        self.portrait = portrait;
        let (sw, sh) = self.sheet();
        let (cx, cy) = ((4.0 + sw - 2.0) / 2.0, (4.0 + sh - 2.0) / 2.0);
        let dx = (cx - (x0 + x1) / 2.0).round() as i32;
        let dy = (cy - (y0 + y1) / 2.0).round() as i32;
        if dx != 0 || dy != 0 {
            for c in self.comps.iter_mut() {
                c.pos.x += dx;
                c.pos.y += dy;
            }
            for wr in self.wires.iter_mut() {
                wr.a.x += dx;
                wr.a.y += dy;
                wr.b.x += dx;
                wr.b.y += dy;
            }
        }
    }

    pub fn add(&mut self, kind: Kind, pos: P, rot: u8, tag: &str) -> u32 {
        self.next_id = self.next_id.max(self.comps.iter().map(|c| c.id + 1).max().unwrap_or(0));
        let id = self.next_id;
        self.next_id += 1;
        let mut num = self.next_num(kind, tag);
        // Timer / overload-relay contacts: number 9 by default (95-96, 97-98).
        if kind.is_aux_contact() && matches!(self.device_kind(tag), Some(Kind::TimerOn | Kind::TimerOff | Kind::Thermal3)) {
            num = 8;
        }
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
            if k.is_aux_contact() {
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
            // No coil / device on the drawing yet: leave "Segue" empty.
            return dev.map(|c| c.tag.clone()).unwrap_or_default();
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

/// `num` value meaning "no contact number" (terminals just 1-2 / 3-4).
pub const NO_NUM: u8 = 255;

/// Terminal names of the motors that draw them inside their own symbol.
pub fn motor_terminals(k: Kind) -> &'static [&'static str] {
    use Kind::*;
    match k {
        Motor3 => &["U1", "V1", "W1", "PE"],
        Motor6 | Motor6X => &["U1", "V1", "W1", "U2", "V2", "W2", "PE"],
        Motor12 => &["U1", "V1", "W1", "U2", "V2", "W2", "U5", "V5", "W5", "U6", "V6", "W6", "PE"],
        Motor1S => &["U1", "U2", "Z1", "Z2"],
        Motor1D => &["U1", "U2", "U3", "U4", "Z1", "Z2"],
        MotorDahlander | Motor2W => &["1U", "1V", "1W", "2U", "2V", "2W", "PE"],
        _ => &[],
    }
}

/// Terminal numbers printed next to a component's pins.
pub fn pin_numbers(c: &Comp, linked: Option<Kind>) -> Vec<String> {
    use Kind::*;
    let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    // Contact number prefix: "1" → 13-14, "2" → 23-24…; none → 3-4 / 1-2.
    let n = if c.num == NO_NUM { String::new() } else { (c.num as usize + 1).to_string() };
    // Timer and overload-relay contacts use the function digits 5-6 (NC), 7-8 (NO),
    // 5-6-8 (changeover); the contact number in front is 9 by default (95-96, 97-98).
    let special = matches!(linked, Some(TimerOn) | Some(TimerOff) | Some(Thermal3));
    match c.kind {
        ContactNc if special => vec![format!("{n}5"), format!("{n}6")],
        ContactNo if special => vec![format!("{n}7"), format!("{n}8")],
        ContactCo if special => vec![format!("{n}5"), format!("{n}6"), format!("{n}8")],
        ContactNo => vec![format!("{n}3"), format!("{n}4")],
        ContactNc => vec![format!("{n}1"), format!("{n}2")],
        ContactCo => vec![format!("{n}1"), format!("{n}2"), format!("{n}4")],
        // Button / switch contacts: contact 1 = 13-14 or 11-12, contact 2 = 23-24 or 21-22...
        PushNo | Switch | LimitNo => vec![format!("{n}3"), format!("{n}4")],
        PushNc | EStop | LimitNc => vec![format!("{n}1"), format!("{n}2")],
        Coil | TimerOn | TimerOff => v(&["A1", "A2"]),
        Lamp | Buzzer => v(&["X1", "X2"]),
        Fuse | Breaker1 => v(&["1", "2"]),
        Breaker2 => v(&["1", "3", "2", "4"]),
        Rcd2 => v(&["1", "N", "2", "N"]),
        Rcd4 => v(&["1", "3", "5", "N", "2", "4", "6", "N"]),
        Breaker3 | MotorBreaker3 | Main3 | Thermal3 => v(&["1", "3", "5", "2", "4", "6"]),
        Motor1 => v(&["U1", "U2"]),
        Rectifier => v(&["~", "~", "+", "−"]),
        Brake => v(&["1", "2"]),
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
