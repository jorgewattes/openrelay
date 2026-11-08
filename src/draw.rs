//! Drawing of the electrical symbols (IEC style) on an egui painter.

use crate::circuit::{pin_numbers, rot_f, t, volt_badge, Comp, Kind, Pot, LAMP_COLORS};
use eframe::egui::{self, Align2, Color32, FontId, Painter, Pos2, Shape, Stroke};
use std::f32::consts::PI;

pub const INK: Color32 = Color32::from_rgb(28, 32, 38);
pub const SELECT: Color32 = Color32::from_rgb(0, 122, 255);
pub const DEAD: Color32 = Color32::from_rgb(150, 156, 165);
pub const NUM: Color32 = Color32::from_rgb(90, 96, 110);
/// Teal used for tags and terminal numbers in the dark (phone) theme.
pub const TEAL: Color32 = Color32::from_rgb(45, 212, 160);

pub fn pot_color(p: Pot) -> Color32 {
    match p {
        Pot::L1 | Pot::L2 | Pot::L3 | Pot::P24 => Color32::from_rgb(222, 40, 40),
        Pot::N | Pot::M0 => Color32::from_rgb(25, 95, 220),
        Pot::PE => Color32::from_rgb(30, 150, 60),
    }
}

/// Maps local symbol coordinates (grid units) to the screen.
#[derive(Clone, Copy)]
pub struct Xf {
    pub o: Pos2,
    pub s: f32,
    pub r: u8,
}

impl Xf {
    pub fn p(&self, x: f32, y: f32) -> Pos2 {
        let (rx, ry) = rot_f(x, y, self.r);
        Pos2::new(self.o.x + rx * self.s, self.o.y + ry * self.s)
    }
}

pub struct DrawCtx {
    pub ink: Color32,
    /// Device worked (contact switched, button pressed, breaker tripped...).
    pub act: bool,
    /// Kind of the device a contact follows.
    pub linked: Option<Kind>,
    /// Live colour per pin during simulation.
    pub pin_col: Vec<Option<Color32>>,
    pub sim: bool,
    pub time: f32,
    pub numbers: bool,
    pub tag: bool,
    /// Fill inside symbols (the sheet colour).
    pub paper: Color32,
    /// Terminal number colour.
    pub num: Color32,
    /// Tag (-KM1) colour.
    pub tag_col: Color32,
    /// Dark theme (phone layout).
    pub dark: bool,
}

impl DrawCtx {
    /// Plain, non-simulated drawing in the given theme.
    pub fn plain(ink: Color32, dark: bool) -> Self {
        let t = Theme::get(dark);
        DrawCtx {
            ink,
            act: false,
            linked: None,
            pin_col: vec![],
            sim: false,
            time: 0.0,
            numbers: false,
            tag: false,
            paper: t.paper,
            num: t.num,
            tag_col: t.tag,
            dark,
        }
    }
}

/// Colours of the drawing sheet: light (computer, like the classic simulators)
/// or dark (phone, like the SimuRelay app).
#[derive(Clone, Copy)]
pub struct Theme {
    pub dark: bool,
    pub canvas: Color32,
    pub paper: Color32,
    pub ink: Color32,
    pub num: Color32,
    pub tag: Color32,
    pub grid: Color32,
    pub grid_major: Color32,
    /// Dots of the computer sheet's grid.
    pub dot: Color32,
    /// Wires (a colour of their own, different from the symbols).
    pub wire: Color32,
    pub select: Color32,
    pub dead: Color32,
}

impl Theme {
    pub fn get(dark: bool) -> Theme {
        if dark {
            Theme {
                dark,
                canvas: Color32::from_rgb(12, 12, 12),
                paper: Color32::from_rgb(21, 21, 21),
                ink: Color32::from_rgb(232, 232, 232),
                num: TEAL,
                tag: TEAL,
                grid: Color32::from_rgb(33, 33, 33),
                grid_major: Color32::from_rgb(48, 48, 48),
                dot: Color32::from_rgb(62, 62, 62),
                wire: Color32::from_rgb(110, 190, 235),
                select: Color32::from_rgb(52, 140, 255),
                dead: Color32::from_rgb(110, 110, 110),
            }
        } else {
            Theme {
                dark,
                canvas: Color32::from_rgb(214, 219, 226),
                paper: Color32::WHITE,
                ink: INK,
                num: NUM,
                tag: INK,
                grid: Color32::from_rgb(234, 237, 242),
                grid_major: Color32::from_rgb(214, 219, 227),
                dot: Color32::from_rgb(200, 205, 214),
                wire: Color32::from_rgb(0, 96, 150),
                select: SELECT,
                dead: DEAD,
            }
        }
    }
}

struct D<'a> {
    p: &'a Painter,
    xf: Xf,
    w: f32,
    ink: Color32,
}

impl D<'_> {
    fn st(&self, c: Color32) -> Stroke {
        Stroke::new(self.w, c)
    }
    fn line_c(&self, x1: f32, y1: f32, x2: f32, y2: f32, c: Color32) {
        self.p.line_segment([self.xf.p(x1, y1), self.xf.p(x2, y2)], self.st(c));
    }
    fn line(&self, x1: f32, y1: f32, x2: f32, y2: f32) {
        self.line_c(x1, y1, x2, y2, self.ink);
    }
    fn dash(&self, x1: f32, y1: f32, x2: f32, y2: f32) {
        let d = (self.xf.s * 0.45).max(2.0);
        self.p.extend(Shape::dashed_line(
            &[self.xf.p(x1, y1), self.xf.p(x2, y2)],
            Stroke::new(self.w * 0.8, self.ink),
            d,
            d * 0.8,
        ));
    }
    fn circle(&self, x: f32, y: f32, r: f32, fill: Option<Color32>) {
        let c = self.xf.p(x, y);
        if let Some(f) = fill {
            self.p.circle_filled(c, r * self.xf.s, f);
        }
        self.p.circle_stroke(c, r * self.xf.s, self.st(self.ink));
    }
    fn poly(&self, pts: &[(f32, f32)], closed: bool, fill: Option<Color32>) {
        let v: Vec<Pos2> = pts.iter().map(|&(x, y)| self.xf.p(x, y)).collect();
        if let Some(f) = fill {
            self.p.add(Shape::convex_polygon(v.clone(), f, Stroke::NONE));
        }
        if closed {
            self.p.add(Shape::closed_line(v, self.st(self.ink)));
        } else {
            self.p.add(Shape::line(v, self.st(self.ink)));
        }
    }
    fn rect(&self, x1: f32, y1: f32, x2: f32, y2: f32, fill: Option<Color32>) {
        self.poly(&[(x1, y1), (x2, y1), (x2, y2), (x1, y2)], true, fill);
    }
    fn arc(&self, cx: f32, cy: f32, r: f32, a0: f32, a1: f32) -> Vec<Pos2> {
        let n = 20;
        (0..=n)
            .map(|i| {
                let a = a0 + (a1 - a0) * i as f32 / n as f32;
                self.xf.p(cx + r * a.cos(), cy + r * a.sin())
            })
            .collect()
    }
    fn arc_line(&self, cx: f32, cy: f32, r: f32, a0: f32, a1: f32) {
        let v = self.arc(cx, cy, r, a0, a1);
        self.p.add(Shape::line(v, self.st(self.ink)));
    }
    fn text(&self, x: f32, y: f32, s: &str, size: f32, align: Align2, c: Color32) {
        self.p.text(self.xf.p(x, y), align, s, FontId::proportional(size * self.xf.s), c);
    }
}

/// Two-terminal switching element between (x,0) and (x,6).
/// `closed` decides the blade; `nc` draws the normally-closed tab.
/// Returns the blade midpoint (for actuator links).
fn contact(d: &D, x: f32, closed: bool, nc: bool, top: Color32, bot: Color32) -> (f32, f32) {
    d.line_c(x, 0.0, x, 2.0, top);
    d.line_c(x, 4.0, x, 6.0, bot);
    if nc {
        d.line_c(x, 2.0, x + 0.9, 2.0, top);
    }
    let tip = if closed {
        if nc {
            (x + 1.05, 1.75)
        } else {
            (x - 0.55, 2.0)
        }
    } else {
        (x - 1.3, 2.0)
    };
    let blade = if closed { bot } else { d.ink };
    d.line_c(x, 4.0, tip.0, tip.1, blade);
    if closed && !nc {
        // Short bar showing the blade touching the fixed contact.
        d.line_c(tip.0, 2.0, x, 2.0, top);
    }
    ((x + tip.0) / 2.0, (4.0 + tip.1) / 2.0)
}

fn breaker_mark(d: &D, x: f32) {
    let k = 0.35;
    d.line(x - k, 2.0 - k, x + k, 2.0 + k);
    d.line(x - k, 2.0 + k, x + k, 2.0 - k);
}

pub fn draw_comp(p: &Painter, xf: Xf, c: &Comp, cx: &DrawCtx) {
    let w = (xf.s * 0.16).clamp(1.0, 3.0);
    let d = D { p, xf, w, ink: cx.ink };
    let pc = |i: usize| -> Color32 { cx.pin_col.get(i).copied().flatten().unwrap_or(cx.ink) };
    use Kind::*;
    match c.kind {
        TermL1 | TermL2 | TermL3 | TermN | TermPE | TermP24 | Term0V => {
            let col = pc(0);
            d.line_c(0.0, -0.5, 0.0, 0.0, col);
            let fill = if cx.sim && cx.pin_col.first().copied().flatten().is_some() {
                col
            } else {
                cx.paper
            };
            d.circle(0.0, -1.0, 0.5, Some(fill));
            let label = match c.kind.source() {
                Some(Pot::P24) if c.volt == 1 => "+12V",
                Some(pt) => pt.name(),
                None => "",
            };
            d.text(0.0, -2.3, label, 1.1, Align2::CENTER_CENTER, cx.ink);
        }
        Fuse => {
            d.line_c(0.0, 0.0, 0.0, 1.4, pc(0));
            d.line_c(0.0, 4.6, 0.0, 6.0, pc(1));
            d.rect(-0.7, 1.4, 0.7, 4.6, None);
            d.line(0.0, 1.4, 0.0, 4.6);
        }
        Breaker1 | Breaker2 | Breaker3 | MotorBreaker3 => {
            // Thermal-magnetic breaker: poles with the breaker "x", linked to the
            // thermal (bimetal) and magnetic (I>) trip units.
            let n = c.kind.poles();
            let mut last = (0.0, 0.0);
            for k in 0..n {
                let x = 3.0 * k as f32;
                last = contact(&d, x, !cx.act, false, pc(k), pc(k + n));
                breaker_mark(&d, x);
                if c.kind == MotorBreaker3 {
                    // Motor breaker: thermal element in each pole.
                    d.poly(&[(x, 4.4), (x - 0.55, 4.4), (x - 0.55, 5.4), (x, 5.4)], false, None);
                }
            }
            let mb = c.kind == MotorBreaker3;
            let bx = if mb { -2.9 } else { -2.6 };
            d.dash(last.0, last.1, bx, 3.0);
            trip_units(&d, bx, cx);
            if mb {
                // Manual rotary operator.
                d.dash(bx - 2.0, 3.0, bx - 2.4, 3.0);
                d.circle(bx - 2.65, 3.0, 0.3, Some(cx.paper));
                d.line(bx - 2.65, 2.4, bx - 2.65, 3.6);
            }
        }
        Rcd2 | Rcd4 => {
            let n = c.kind.poles();
            let mut last = (0.0, 0.0);
            for k in 0..n {
                let x = 3.0 * k as f32;
                last = contact(&d, x, !cx.act, false, pc(k), pc(k + n));
            }
            d.dash(last.0, last.1, -2.6, 3.0);
            let fill = trip_fill(cx);
            d.rect(-4.4, 2.2, -2.6, 3.8, fill);
            d.text(-3.5, 3.0, "IΔ", 0.9, Align2::CENTER_CENTER, cx.ink);
            // Toroid (current sensor) around all conductors, linked to the trip box.
            let (cxm, rx) = (1.5 * (n as f32 - 1.0), 1.5 * (n as f32 - 1.0) + 1.0);
            let pts: Vec<egui::Pos2> = (0..32)
                .map(|i| {
                    let a = i as f32 / 32.0 * std::f32::consts::TAU;
                    d.xf.p(cxm + rx * a.cos(), 4.7 + 0.4 * a.sin())
                })
                .collect();
            p.add(Shape::closed_line(pts, Stroke::new(w, cx.ink)));
            d.dash(cxm - rx, 4.7, -3.5, 3.8);
        }        Thermal3 => {
            let fill = if cx.act { Some(if cx.dark { Color32::from_rgb(110, 40, 36) } else { Color32::from_rgb(255, 210, 205) }) } else { None };
            d.rect(-1.0, 1.9, 7.4, 4.1, fill);
            for (k, x) in [0.0f32, 3.0, 6.0].into_iter().enumerate() {
                d.line_c(x, 0.0, x, 2.5, pc(k));
                d.line_c(x, 3.5, x, 6.0, pc(k + 3));
                d.poly(&[(x, 2.5), (x + 0.6, 2.5), (x + 0.6, 3.5), (x, 3.5)], false, None);
            }
        }
        Main3 => {
            let mut mids = vec![];
            for (k, x) in [0.0f32, 3.0, 6.0].into_iter().enumerate() {
                mids.push(contact(&d, x, cx.act, false, pc(k), pc(k + 3)));
            }
            d.dash(mids[0].0 - 0.8, mids[0].1, mids[2].0, mids[2].1);
        }
        PushNo | PushNc | Switch | EStop | LimitNo | LimitNc => {
            let nc = matches!(c.kind, PushNc | EStop | LimitNc);
            let closed = if nc { !cx.act } else { cx.act };
            let m = contact(&d, 0.0, closed, nc, pc(0), pc(1));
            let off = if cx.act && matches!(c.kind, PushNo | PushNc | EStop) { 0.5 } else { 0.0 };
            let ax = -2.6 + off;
            d.dash(m.0, m.1, ax, 3.0);
            match c.kind {
                PushNo | PushNc => {
                    d.poly(&[(ax - 0.6, 2.2), (ax, 2.2), (ax, 3.8), (ax - 0.6, 3.8)], false, None);
                }
                Switch => {
                    d.poly(&[(ax - 0.6, 2.2), (ax, 2.2), (ax, 3.8), (ax - 0.6, 3.8)], false, None);
                    d.line(ax - 0.6, 2.2, ax - 0.9, 2.7);
                    d.line(ax - 0.6, 3.8, ax - 0.9, 3.3);
                }
                EStop => {
                    d.line(ax, 2.2, ax, 3.8);
                    let pts = d.arc(ax, 3.0, 0.9, PI * 0.5, PI * 1.5);
                    let fill = Color32::from_rgb(230, 60, 50);
                    let mut poly = pts.clone();
                    poly.push(d.xf.p(ax, 2.1));
                    p.add(Shape::convex_polygon(poly, fill, Stroke::new(w, cx.ink)));
                }
                _ => {
                    d.poly(&[(ax, 3.0), (ax - 0.8, 2.3), (ax - 0.8, 3.7)], true, None);
                }
            }
        }
        ContactNo | ContactNc | ContactCo => {
            let co = c.kind == ContactCo;
            let m = if co {
                // Changeover: common on top (centred), NC bottom-left, NO bottom-right.
                d.line_c(0.0, 0.0, 0.0, 2.0, pc(0));
                d.line_c(-2.0, 6.0, -2.0, 4.2, pc(1));
                d.line_c(-2.0, 4.2, -0.9, 4.2, pc(1));
                d.line_c(2.0, 6.0, 2.0, 4.2, pc(2));
                d.line_c(2.0, 4.2, 0.9, 4.2, pc(2));
                let tip = if cx.act { (0.95, 4.1) } else { (-0.95, 4.1) };
                d.line_c(0.0, 2.0, tip.0, tip.1, pc(0));
                (tip.0 / 2.0, (2.0 + tip.1) / 2.0)
            } else {
                let nc = c.kind == ContactNc;
                let closed = if nc { !cx.act } else { cx.act };
                contact(&d, 0.0, closed, nc, pc(0), pc(1))
            };
            // Actuators sit further left on the changeover (its NC terminal is there).
            let ax = if co { -1.0 } else { 0.0 };
            match cx.linked {
                Some(TimerOn) | Some(TimerOff) => {
                    d.dash(m.0, m.1, -1.9 + ax, 3.0);
                    if cx.linked == Some(TimerOn) {
                        d.arc_line(-2.5 + ax, 3.0, 0.6, -PI * 0.5, PI * 0.5);
                    } else {
                        d.arc_line(-1.9 + ax, 3.0, 0.6, PI * 0.5, PI * 1.5);
                    }
                }
                Some(Thermal3) => {
                    // Thermal (bimetal) actuator, clear of the tag on its left.
                    d.dash(m.0, m.1, -1.8 + ax, 3.0);
                    bimetal(&d, -3.4 + ax, -1.8 + ax, 3.0);
                }
                _ => {}
            }
        }
        Coil | TimerOn | TimerOff => {
            d.line_c(0.0, 0.0, 0.0, 2.0, pc(0));
            d.line_c(0.0, 4.0, 0.0, 6.0, pc(1));
            let fill = if cx.sim && cx.act { Some(if cx.dark { Color32::from_rgb(30, 120, 66) } else { Color32::from_rgb(140, 225, 160) }) } else { Some(cx.paper) };
            d.rect(-1.4, 2.0, 1.4, 4.0, fill);
            match c.kind {
                TimerOn => {
                    d.rect(-2.8, 2.0, -1.4, 4.0, None);
                    d.line(-2.8, 2.0, -1.4, 4.0);
                    d.line(-2.8, 4.0, -1.4, 2.0);
                }
                TimerOff => {
                    d.rect(-2.8, 2.0, -1.4, 4.0, Some(cx.ink));
                }
                _ => {}
            }
        }
        Lamp => {
            d.line_c(0.0, 0.0, 0.0, 1.7, pc(0));
            d.line_c(0.0, 4.3, 0.0, 6.0, pc(1));
            let [r, g, b] = LAMP_COLORS[c.color as usize % LAMP_COLORS.len()].2;
            let on = cx.sim && cx.act;
            // Brightness follows the applied voltage (dim on under-voltage).
            let bright = if on { c.st.bright.clamp(0.0, 1.0) } else { 0.0 };
            if on {
                let glow = Color32::from_rgba_unmultiplied(r, g, b, (70.0 * bright) as u8);
                p.circle_filled(xf.p(0.0, 3.0), (1.5 + 0.8 * bright) * xf.s, glow);
            }
            let burnt = cx.sim && c.st.burnt;
            let fill = if burnt {
                Some(Color32::from_rgb(60, 52, 48))
            } else if on {
                Some(cx.paper.lerp_to_gamma(Color32::from_rgb(r, g, b), 0.25 + 0.75 * bright))
            } else {
                Some(cx.paper)
            };
            d.circle(0.0, 3.0, 1.3, fill);
            let k = 1.3 * 0.7071;
            d.line(-k, 3.0 - k, k, 3.0 + k);
            d.line(-k, 3.0 + k, k, 3.0 - k);
            if burnt {
                // Broken filament / crack.
                let red = Color32::from_rgb(235, 70, 60);
                d.line_c(-0.4, 1.4, 0.2, 2.4, red);
                d.line_c(0.2, 2.4, -0.3, 3.2, red);
                d.line_c(-0.3, 3.2, 0.35, 4.4, red);
            }
        }
        Buzzer => {
            d.line_c(0.0, 0.0, 0.0, 2.2, pc(0));
            d.line_c(0.0, 3.8, 0.0, 6.0, pc(1));
            let fill = if cx.sim && cx.act { Some(Color32::from_rgb(255, 214, 120)) } else { Some(cx.paper) };
            d.rect(-1.0, 2.2, 1.0, 3.8, fill);
            d.arc_line(1.0, 3.0, 0.8, -PI * 0.5, PI * 0.5);
            if cx.sim && cx.act {
                let ph = (cx.time * 4.0).fract();
                d.arc_line(1.0, 3.0, 1.4 + ph * 0.6, -PI * 0.3, PI * 0.3);
            }
        }
        Motor1 => {
            d.line_c(0.0, 0.0, 0.0, 1.0, pc(0));
            d.line_c(0.0, 5.0, 0.0, 6.0, pc(1));
            motor_body(&d, 0.0, 3.0, 2.0, c, cx, "1~");
        }
        Motor3 => {
            // Same look as the 6-terminal motor, with the star closed inside.
            motor_ring(&d, c, cx, 3.0, 5.5, 4.6);
            let tops = [(0.0, 2.4), (3.0, 1.9), (6.0, 2.4)];
            let star = (3.0, 7.4);
            for (i, &(x, y)) in tops.iter().enumerate() {
                d.line_c(x, 0.0, x, y, pc(i));
                winding(&d, (x, y), star, phase_col(i, cx));
            }
            d.circle(star.0, star.1, 0.22, Some(cx.ink));
            pe_lead(&d, pc(3), 7.6);
            for (i, &(x, y)) in tops.iter().enumerate() {
                tlabel(&d, x, y - 0.15, ["U1", "V1", "W1"][i], 3.0, cx);
            }
            motor_name(&d, c, cx, 7.3, 9.8, "M3~");
        }
        Motor6 | Motor6X => {
            motor_ring(&d, c, cx, 3.0, 5.5, 4.6);
            let tops = [(0.0, 2.4), (3.0, 1.9), (6.0, 2.4)];
            let bots = [(0.0, 8.6), (3.0, 9.1), (6.0, 8.6)];
            let crossed = c.kind == Motor6X;
            // Winding i runs from its top terminal to its "2" terminal.
            for i in 0..3 {
                let (tx, ty) = tops[i];
                let pin2 = c.kind.pins()[i + 3].0 as f32;
                let b = bots[(pin2 / 3.0).round() as usize];
                d.line_c(tx, 0.0, tx, ty, pc(i));
                d.line_c(b.0, b.1, b.0, 11.0, pc(i + 3));
                if crossed {
                    winding(&d, (tx, ty), b, phase_col(i, cx));
                } else {
                    winding(&d, (tx, ty), (tx, b.1), phase_col(i, cx));
                }
            }
            pe_lead(&d, pc(6), 7.6);
            let names = ["U1", "V1", "W1", "U2", "V2", "W2"];
            for i in 0..6 {
                let (px, _) = c.kind.pins()[i];
                let (x, y) = if i < 3 { tops[i] } else { bots[(px as f32 / 3.0).round() as usize] };
                let dy = if i < 3 { -0.15 } else { 0.15 };
                tlabel(&d, x, y + dy, names[i], 3.0, cx);
            }
            let label = if cx.sim {
                match c.st.conn {
                    1 => "M3~ Y",
                    2 => "M3~ Δ",
                    _ => "M3~",
                }
            } else {
                "M3~"
            };
            motor_name(&d, c, cx, 7.3, 10.0, label);
        }
        Motor12 => {
            // Oval motor body: six half-windings (two per phase), every end brought out.
            motor_oval(&d, c, cx, 7.5, 6.0, 9.4, 6.4);
            let names = ["U1", "V1", "W1", "U2", "V2", "W2", "U5", "V5", "W5", "U6", "V6", "W6"];
            for i in 0..12 {
                let (px, py) = c.kind.pins()[i];
                let (x, y) = (px as f32, if py == 0 { 2.7 } else { 9.3 });
                d.line_c(x, py as f32, x, y, pc(i));
            }
            for top in [0, 1, 2, 6, 7, 8] {
                let x = c.kind.pins()[top].0 as f32;
                winding(&d, (x, 2.7), (x, 9.3), phase_col(top % 3, cx));
            }
            for i in 0..12 {
                let (px, py) = c.kind.pins()[i];
                let y = if py == 0 { 2.0 } else { 10.0 };
                tlabel(&d, px as f32, y, names[i], 14.9, cx);
            }
            d.text(7.5, 5.3, "M", 1.0, Align2::CENTER_CENTER, cx.ink);
            d.text(7.5, 6.7, "3~", 0.8, Align2::CENTER_CENTER, cx.ink);
            let pe = pc(12);
            d.line_c(18.0, 0.0, 18.0, 6.0, pe);
            d.line_c(18.0, 6.0, 16.9, 6.0, pe);
            d.text(18.35, 1.0, "PE", 0.72, Align2::LEFT_CENTER, cx.num);
            let label = if cx.sim {
                match c.st.conn {
                    1 => "M3~ YY",
                    2 => "M3~ ΔΔ",
                    3 => "M3~ Y",
                    4 => "M3~ Δ",
                    _ => "M3~",
                }
            } else {
                "M3~ 12"
            };
            motor_name(&d, c, cx, 16.6, 10.6, label);
        }
        Motor1S | Motor1D => {
            // Main winding(s) in blue, auxiliary start winding in red.
            let dual = c.kind == Motor1D;
            let (cxm, cym, r) = if dual { (3.0, 5.0, 4.6) } else { (2.0, 4.0, 3.6) };
            motor_ring(&d, c, cx, cxm, cym, r);
            let names: &[&str] = crate::circuit::motor_terminals(c.kind);
            let pins = c.kind.pins();
            let bottom = if dual { 10.0 } else { 8.0 };
            let inset = |x: f32| {
                let dx = x - cxm;
                (r * r - dx * dx).max(0.0).sqrt() - 0.7
            };
            let mut i = 0;
            while i + 1 < pins.len() {
                let x = pins[i].0 as f32;
                let h = inset(x);
                let (ya, yb) = (cym - h, cym + h);
                d.line_c(x, 0.0, x, ya, pc(i));
                d.line_c(x, yb, x, bottom, pc(i + 1));
                let aux = names[i].starts_with('Z');
                let col = if aux { Color32::from_rgb(220, 50, 40) } else { Color32::from_rgb(40, 110, 230) };
                winding(&d, (x, ya), (x, yb), col);
                tlabel(&d, x, ya - 0.1, names[i], cxm, cx);
                tlabel(&d, x, yb + 0.1, names[i + 1], cxm, cx);
                i += 2;
            }
            let label = if dual && cx.sim {
                match c.st.conn {
                    1 => "M1~ //",
                    2 => "M1~ ser.",
                    _ => "M1~",
                }
            } else {
                "M1~"
            };
            motor_name(&d, c, cx, cxm + r * 0.75, cym + r * 0.95, label);
        }
        MotorDahlander | Motor2W => {
            motor_ring(&d, c, cx, 3.0, 5.5, 4.6);
            let names = ["1U", "1V", "1W", "2U", "2V", "2W"];
            let tops = [(0.0, 2.4), (3.0, 1.9), (6.0, 2.4)];
            let bots = [(0.0, 8.6), (3.0, 9.1), (6.0, 8.6)];
            for i in 0..3 {
                let (x, ty) = tops[i];
                let by = bots[i].1;
                d.line_c(x, 0.0, x, ty, pc(i));
                d.line_c(x, by, x, 11.0, pc(i + 3));
                let col = phase_col(i, cx);
                if c.kind == MotorDahlander {
                    // One winding per phase with a centre tap (Δ/YY).
                    winding(&d, (x, ty), (x, 5.5), col);
                    winding(&d, (x, 5.5), (x, by), col);
                    d.circle(x, 5.5, 0.14, Some(cx.ink));
                } else {
                    // Two independent star windings: low speed on top, high speed below.
                    winding(&d, (x, ty), (3.0, 4.6), col);
                    winding(&d, (x, by), (3.0, 6.6), col);
                }
                tlabel(&d, x, ty - 0.15, names[i], 3.0, cx);
                tlabel(&d, x, by + 0.15, names[i + 3], 3.0, cx);
            }
            if c.kind == Motor2W {
                d.circle(3.0, 4.6, 0.18, Some(cx.ink));
                d.circle(3.0, 6.6, 0.18, Some(cx.ink));
            }
            pe_lead(&d, pc(6), 7.6);
            let label = match (c.kind, cx.sim, c.st.conn) {
                (_, true, 1) => t("M3~ lenta", "M3~ low"),
                (_, true, 2) => t("M3~ rápida", "M3~ high"),
                (MotorDahlander, _, _) => "M3~ Dahl.",
                _ => t("M3~ 2 enr.", "M3~ 2 wnd."),
            };
            motor_name(&d, c, cx, 7.3, 10.0, label);
        }
        Rectifier => {
            d.line_c(0.0, 0.0, 0.0, 1.2, pc(0));
            d.line_c(3.0, 0.0, 3.0, 1.2, pc(1));
            d.line_c(0.0, 4.8, 0.0, 6.0, pc(2));
            d.line_c(3.0, 4.8, 3.0, 6.0, pc(3));
            d.rect(-1.0, 1.2, 4.0, 4.8, Some(cx.paper));
            // Diode mark.
            d.poly(&[(0.9, 2.4), (0.9, 3.6), (1.9, 3.0)], true, Some(cx.ink));
            d.line(1.95, 2.4, 1.95, 3.6);
            d.line(0.4, 3.0, 2.6, 3.0);
        }
        Brake => {
            d.line_c(0.0, 0.0, 0.0, 2.0, pc(0));
            d.line_c(0.0, 4.0, 0.0, 6.0, pc(1));
            let released = cx.sim && c.st.energized;
            let fill = if released {
                Some(if cx.dark { Color32::from_rgb(30, 120, 66) } else { Color32::from_rgb(140, 225, 160) })
            } else {
                Some(cx.paper)
            };
            d.rect(-1.4, 2.0, 1.4, 4.0, fill);
            // Brake shoes: apart when released, closed when braking.
            let gap = if released { 0.55 } else { 0.25 };
            d.line(-gap, 2.35, -gap, 3.65);
            d.line(gap, 2.35, gap, 3.65);
        }
        Text => {
            let h = c.text_size();
            for (i, line) in c.tag.split('\n').enumerate() {
                d.text(0.0, -h * 0.6 + h * 1.25 * i as f32, line, h, Align2::LEFT_TOP, cx.ink);
            }
        }
    }

    if cx.numbers {
        let nums = pin_numbers(c, cx.linked);
        let pins = c.kind.pins();
        for (i, s) in nums.iter().enumerate() {
            if i >= pins.len() {
                break;
            }
            let (px, py) = (pins[i].0 as f32, pins[i].1 as f32);
            let y = if py < 3.0 { py + 0.9 } else { py - 0.9 };
            // Changeover NO terminal sits to the right; keep its number clear of the hook.
            let x = if c.kind == ContactCo && i == 2 { px + 0.4 } else { px + 0.35 };
            d.text(x, y, s, 0.85, Align2::LEFT_CENTER, cx.num);
        }
    }
    if cx.tag {
        voltage_marks(p, xf, c, cx);
    }
    if cx.tag && !c.tag.is_empty() && c.kind != Text {
        let (x0, y0, x1, _y1) = c.world_bounds();
        // Plain coil contacts have no actuator drawn on their left: keep the tag
        // close, so it doesn't land on a neighbouring button.
        let plain = c.kind.is_aux_contact() && !matches!(cx.linked, Some(TimerOn) | Some(TimerOff) | Some(Thermal3));
        let x0 = if plain && c.rot == 0 { c.pos.x as f32 - if c.kind == ContactCo { 2.6 } else { 1.6 } } else { x0 };
        let s = xf.s;
        let origin = xf.o - egui::vec2(c.pos.x as f32 * s, c.pos.y as f32 * s);
        let text = format!("-{}", c.tag);
        let font = FontId::proportional(1.15 * s);
        if c.rot % 2 == 0 {
            let mid = match c.kind {
                k if k.is_motor() && k != Motor1 => (_y1 + y0) / 2.0,
                _ => c.pos.y as f32 + if c.rot == 0 { 3.0 } else { -3.0 },
            };
            let pos = origin + egui::vec2((x0 - 0.4) * s, mid * s);
            p.text(pos, Align2::RIGHT_CENTER, text, font, cx.tag_col);
        } else {
            let pos = origin + egui::vec2((x0 + x1) * 0.5 * s, (y0 - 0.3) * s);
            p.text(pos, Align2::CENTER_BOTTOM, text, font, cx.tag_col);
        }
    }
}

/// Rated voltage under the tag, and during simulation a warning badge beside
/// the part when the applied voltage is wrong (under / over / burnt / AC-DC).
fn voltage_marks(p: &Painter, xf: Xf, c: &Comp, cx: &DrawCtx) {
    let s = xf.s;
    let origin = xf.o - egui::vec2(c.pos.x as f32 * s, c.pos.y as f32 * s);
    let (x0, y0, x1, y1) = c.world_bounds();
    let w = |x: f32, y: f32| origin + egui::vec2(x * s, y * s);
    let mid = match c.kind {
        k if k.is_motor() && k != Kind::Motor1 => (y0 + y1) / 2.0,
        _ if c.rot % 2 == 0 => c.pos.y as f32 + if c.rot == 0 { 3.0 } else { -3.0 },
        _ => (y0 + y1) / 2.0,
    };
    // Rated voltage, small, under the tag (left of the part).
    if cx.numbers && c.rot % 2 == 0 {
        if let Some(b) = volt_badge(c) {
            if !c.kind.is_terminal() {
                p.text(
                    w(x0 - 0.4, mid + 1.3),
                    Align2::RIGHT_CENTER,
                    b,
                    FontId::proportional((0.85 * s).max(5.5)),
                    cx.num,
                );
            }
        }
    }
    if !cx.sim {
        return;
    }
    let (text, col) = if c.st.burnt {
        (format!("⚠ {} {:.0} V", t("QUEIMADA", "BURNT"), c.st.volt), Color32::from_rgb(200, 40, 30))
    } else {
        match c.st.vstat {
            -1 if c.st.volt > 0.0 => (
                format!("⚠ {} {:.0} V", t("Subtensão", "Under-voltage"), c.st.volt),
                Color32::from_rgb(214, 130, 0),
            ),
            1 => (
                format!("⚠ {} {:.0} V", t("Sobretensão", "Over-voltage"), c.st.volt),
                Color32::from_rgb(200, 40, 30),
            ),
            2 => (
                format!("⚠ {} ({:.0} V)", t("CA/CC errado", "Wrong AC/DC"), c.st.volt),
                Color32::from_rgb(130, 60, 190),
            ),
            _ => return,
        }
    };
    let font = FontId::proportional((0.95 * s).clamp(7.0, 16.0));
    let g = p.layout_no_wrap(text, font, Color32::WHITE);
    let pad = egui::vec2(5.0, 2.0);
    // Centred just under the part, so neighbours in a row don't get covered.
    let _ = mid;
    let pos = w((x0 + x1) / 2.0, y1 + 0.5) - egui::vec2(g.size().x / 2.0 + pad.x, 0.0);
    let r = egui::Rect::from_min_size(pos, g.size() + pad * 2.0);
    p.rect_filled(r, 4.0, col);
    p.galley(pos + pad, g, Color32::WHITE);
}

fn trip_fill(cx: &DrawCtx) -> Option<Color32> {
    if cx.act {
        Some(if cx.dark { Color32::from_rgb(110, 40, 36) } else { Color32::from_rgb(255, 210, 205) })
    } else {
        None
    }
}

/// Bimetal (thermal effect) mark between x0 and x1 around height y.
fn bimetal(d: &D, x0: f32, x1: f32, y: f32) {
    let m = (x0 + x1) / 2.0;
    d.poly(
        &[(x1, y), (m + 0.35, y), (m + 0.35, y - 0.55), (m - 0.35, y - 0.55), (m - 0.35, y), (x0, y)],
        false,
        None,
    );
}

/// Thermal-magnetic trip units at the left of a breaker: thermal (bimetal) box
/// on top, magnetic (I>) box below, linked at `x` (right edge).
fn trip_units(d: &D, x: f32, cx: &DrawCtx) {
    let fill = trip_fill(cx);
    d.rect(x - 1.9, 1.4, x, 2.95, fill);
    bimetal(d, x - 1.7, x - 0.2, 2.45);
    d.rect(x - 1.9, 3.05, x, 4.6, fill);
    d.text(x - 0.95, 3.82, "I>", 0.85, Align2::CENTER_CENTER, d.ink);
}

/// Winding colours: U blue, V red, W black (the drawing colour).
fn phase_col(i: usize, cx: &DrawCtx) -> Color32 {
    match i % 3 {
        0 => Color32::from_rgb(40, 110, 230),
        1 => Color32::from_rgb(220, 50, 40),
        _ => cx.ink,
    }
}

/// A coil drawn as a wavy line between two points (straight ends, waves in the middle).
fn winding(d: &D, a: (f32, f32), b: (f32, f32), col: Color32) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = (dx * dx + dy * dy).sqrt().max(0.01);
    let (ux, uy) = (dx / len, dy / len);
    let (nx, ny) = (-uy, ux);
    let n = 48;
    let pts: Vec<Pos2> = (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let wave = if (0.18..=0.82).contains(&t) {
                let k = (t - 0.18) / 0.64;
                (k * PI * 2.0 * 4.0).sin() * 0.32
            } else {
                0.0
            };
            d.xf.p(a.0 + dx * t + nx * wave, a.1 + dy * t + ny * wave)
        })
        .collect();
    d.p.add(Shape::line(pts, Stroke::new(d.w * 1.1, col)));
}

/// Terminal name beside a winding end: on the right, or on the left for
/// terminals right of the body's centre (so it stays inside the motor).
fn tlabel(d: &D, x: f32, y: f32, s: &str, centre: f32, cx: &DrawCtx) {
    if x > centre + 0.1 {
        d.text(x - 0.42, y, s, 0.72, Align2::RIGHT_CENTER, cx.num);
    } else {
        d.text(x + 0.42, y, s, 0.72, Align2::LEFT_CENTER, cx.num);
    }
}

/// Earth lead of a three-phase motor (pin at (9,0)) to the body edge at x_edge.
fn pe_lead(d: &D, col: Color32, x_edge: f32) {
    d.line_c(9.0, 0.0, 9.0, 5.5, col);
    d.line_c(9.0, 5.5, x_edge, 5.5, col);
    d.text(9.35, 1.0, "PE", 0.72, Align2::LEFT_CENTER, col);
}

fn motor_fill(c: &Comp, cx: &DrawCtx) -> Color32 {
    if cx.sim && c.st.motor != 0 {
        // Very light tint: the windings stay easy to read while it turns.
        if cx.dark { Color32::from_rgb(26, 40, 56) } else { Color32::from_rgb(238, 245, 253) }
    } else if cx.sim && c.st.hum {
        if cx.dark { Color32::from_rgb(96, 64, 20) } else { Color32::from_rgb(255, 228, 196) }
    } else {
        cx.paper
    }
}

/// Round motor body with the turning arrow while it runs.
fn motor_ring(d: &D, c: &Comp, cx: &DrawCtx, x: f32, y: f32, r: f32) {
    d.circle(x, y, r, Some(motor_fill(c, cx)));
    if cx.sim && c.st.motor != 0 {
        spin(d, c, x, y, r + 0.75, r + 0.75);
    }
}

/// Oval body (12-terminal motor) with the turning arrow while it runs.
fn motor_oval(d: &D, c: &Comp, cx: &DrawCtx, x: f32, y: f32, rx: f32, ry: f32) {
    let pts: Vec<Pos2> = (0..64)
        .map(|i| {
            let a = i as f32 / 64.0 * std::f32::consts::TAU;
            d.xf.p(x + rx * a.cos(), y + ry * a.sin())
        })
        .collect();
    d.p.add(Shape::convex_polygon(pts.clone(), motor_fill(c, cx), Stroke::NONE));
    d.p.add(Shape::closed_line(pts, Stroke::new(d.w, cx.ink)));
    if cx.sim && c.st.motor != 0 {
        spin(d, c, x, y, rx + 0.75, ry + 0.75);
    }
}

/// Small motor name ("M3~") outside the body, so it never covers the windings.
fn motor_name(d: &D, _c: &Comp, cx: &DrawCtx, x: f32, y: f32, s: &str) {
    d.text(x, y, s, 0.85, Align2::LEFT_CENTER, cx.ink);
}

/// Rotation symbol drawn just outside the motor body (so it never covers the
/// windings): two turning arrows on an ellipse around the body, moving with
/// the simulated angle; the arrow heads show the direction.
fn spin(d: &D, c: &Comp, x: f32, y: f32, rx: f32, ry: f32) {
    let dir = c.st.motor as f32;
    let col = Color32::from_rgb(0, 130, 245);
    let span = PI * 0.62;
    for k in 0..2 {
        let a0 = c.st.angle + PI * k as f32;
        let a1 = a0 + span * dir;
        let n = 24;
        let pts: Vec<Pos2> = (0..=n)
            .map(|i| {
                let a = a0 + (a1 - a0) * i as f32 / n as f32;
                d.xf.p(x + rx * a.cos(), y + ry * a.sin())
            })
            .collect();
        d.p.add(Shape::line(pts, Stroke::new(d.w * 1.5, col)));
        // Arrow head along the tangent of the ellipse at the end.
        let tip = (x + rx * a1.cos(), y + ry * a1.sin());
        let (mut tx, mut ty) = (-rx * a1.sin() * dir, ry * a1.cos() * dir);
        let l = (tx * tx + ty * ty).sqrt().max(0.001);
        tx /= l;
        ty /= l;
        let s = 0.62;
        let back = (tip.0 - tx * s, tip.1 - ty * s);
        let (nx, ny) = (-ty * s * 0.55, tx * s * 0.55);
        let tri = vec![
            d.xf.p(tip.0 + tx * 0.25, tip.1 + ty * 0.25),
            d.xf.p(back.0 + nx, back.1 + ny),
            d.xf.p(back.0 - nx, back.1 - ny),
        ];
        d.p.add(Shape::convex_polygon(tri, col, Stroke::NONE));
    }
}

fn motor_body(d: &D, x: f32, y: f32, r: f32, c: &Comp, cx: &DrawCtx, kind: &str) {
    d.circle(x, y, r, Some(motor_fill(c, cx)));
    d.text(x, y - r * 0.28, "M", 1.25 * r / 2.0, Align2::CENTER_CENTER, cx.ink);
    d.text(x, y + r * 0.38, kind, 0.95 * r / 2.0, Align2::CENTER_CENTER, cx.ink);
    if cx.sim && c.st.motor != 0 {
        spin(d, c, x, y, r + 0.6, r + 0.6);
    }
}
