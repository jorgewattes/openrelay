//! Drawing of the electrical symbols (IEC style) on an egui painter.

use crate::circuit::{pin_numbers, rot_f, Comp, Kind, Pot, LAMP_COLORS};
use eframe::egui::{self, Align2, Color32, FontId, Painter, Pos2, Shape, Stroke};
use std::f32::consts::PI;

pub const INK: Color32 = Color32::from_rgb(28, 32, 38);
pub const SELECT: Color32 = Color32::from_rgb(0, 122, 255);
pub const DEAD: Color32 = Color32::from_rgb(150, 156, 165);
pub const NUM: Color32 = Color32::from_rgb(90, 96, 110);

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
                Color32::WHITE
            };
            d.circle(0.0, -1.0, 0.5, Some(fill));
            let label = match c.kind.source() {
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
        Breaker1 => {
            let m = contact(&d, 0.0, !cx.act, false, pc(0), pc(1));
            breaker_mark(&d, 0.0);
            d.dash(m.0, m.1, -2.6, 3.0);
            let fill = if cx.act { Some(Color32::from_rgb(255, 210, 205)) } else { None };
            d.rect(-4.4, 2.2, -2.6, 3.8, fill);
            d.text(-3.5, 3.0, "I>", 0.9, Align2::CENTER_CENTER, cx.ink);
        }
        Breaker3 => {
            let mut last = (0.0, 0.0);
            for (k, x) in [0.0f32, 3.0, 6.0].into_iter().enumerate() {
                last = contact(&d, x, !cx.act, false, pc(k), pc(k + 3));
                breaker_mark(&d, x);
            }
            d.dash(last.0, last.1, -2.6, 3.0);
            let fill = if cx.act { Some(Color32::from_rgb(255, 210, 205)) } else { None };
            d.rect(-4.4, 2.2, -2.6, 3.8, fill);
            d.text(-3.5, 3.0, "I>", 0.9, Align2::CENTER_CENTER, cx.ink);
        }
        Thermal3 => {
            let fill = if cx.act { Some(Color32::from_rgb(255, 210, 205)) } else { None };
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
        ContactNo | ContactNc => {
            let nc = c.kind == ContactNc;
            let closed = if nc { !cx.act } else { cx.act };
            let m = contact(&d, 0.0, closed, nc, pc(0), pc(1));
            match cx.linked {
                Some(TimerOn) | Some(TimerOff) => {
                    d.dash(m.0, m.1, -1.9, 3.0);
                    if cx.linked == Some(TimerOn) {
                        d.arc_line(-2.5, 3.0, 0.6, -PI * 0.5, PI * 0.5);
                    } else {
                        d.arc_line(-1.9, 3.0, 0.6, PI * 0.5, PI * 1.5);
                    }
                }
                Some(Thermal3) => {
                    d.dash(m.0, m.1, -2.0, 3.0);
                    d.poly(&[(-2.0, 2.4), (-2.6, 2.4), (-2.6, 3.6), (-2.0, 3.6)], false, None);
                }
                _ => {}
            }
        }
        Coil | TimerOn | TimerOff => {
            d.line_c(0.0, 0.0, 0.0, 2.0, pc(0));
            d.line_c(0.0, 4.0, 0.0, 6.0, pc(1));
            let fill = if cx.sim && cx.act { Some(Color32::from_rgb(140, 225, 160)) } else { Some(Color32::WHITE) };
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
            if on {
                let glow = Color32::from_rgba_unmultiplied(r, g, b, 70);
                p.circle_filled(xf.p(0.0, 3.0), 2.3 * xf.s, glow);
            }
            let fill = if on { Some(Color32::from_rgb(r, g, b)) } else { Some(Color32::WHITE) };
            d.circle(0.0, 3.0, 1.3, fill);
            let k = 1.3 * 0.7071;
            d.line(-k, 3.0 - k, k, 3.0 + k);
            d.line(-k, 3.0 + k, k, 3.0 - k);
        }
        Buzzer => {
            d.line_c(0.0, 0.0, 0.0, 2.2, pc(0));
            d.line_c(0.0, 3.8, 0.0, 6.0, pc(1));
            let fill = if cx.sim && cx.act { Some(Color32::from_rgb(255, 214, 120)) } else { Some(Color32::WHITE) };
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
            d.line_c(0.0, 0.0, 0.0, 2.2, pc(0));
            d.line_c(0.0, 2.2, 1.7, 3.25, pc(0));
            d.line_c(3.0, 0.0, 3.0, 2.9, pc(1));
            d.line_c(6.0, 0.0, 6.0, 2.2, pc(2));
            d.line_c(6.0, 2.2, 4.3, 3.25, pc(2));
            let pe = pc(3);
            d.line_c(9.0, 0.0, 9.0, 5.5, pe);
            d.line_c(9.0, 5.5, 5.6, 5.5, pe);
            motor_body(&d, 3.0, 5.5, 2.6, c, cx, "3~");
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
            let (x, y) = if c.kind == Motor3 { (px + 0.3, 1.0) } else { (px + 0.35, y) };
            d.text(x, y, s, 0.85, Align2::LEFT_CENTER, NUM);
        }
    }
    if cx.tag && !c.tag.is_empty() {
        let (x0, y0, x1, _y1) = c.world_bounds();
        let s = xf.s;
        let origin = xf.o - egui::vec2(c.pos.x as f32 * s, c.pos.y as f32 * s);
        let text = format!("-{}", c.tag);
        let font = FontId::proportional(1.15 * s);
        if c.rot % 2 == 0 {
            let mid = match c.kind {
                Motor3 => c.pos.y as f32 + if c.rot == 0 { 5.5 } else { -5.5 },
                _ => c.pos.y as f32 + if c.rot == 0 { 3.0 } else { -3.0 },
            };
            let pos = origin + egui::vec2((x0 - 0.4) * s, mid * s);
            p.text(pos, Align2::RIGHT_CENTER, text, font, cx.ink);
        } else {
            let pos = origin + egui::vec2((x0 + x1) * 0.5 * s, (y0 - 0.3) * s);
            p.text(pos, Align2::CENTER_BOTTOM, text, font, cx.ink);
        }
    }
}

fn motor_body(d: &D, x: f32, y: f32, r: f32, c: &Comp, cx: &DrawCtx, kind: &str) {
    let running = cx.sim && c.st.motor != 0;
    let fill = if running {
        Color32::from_rgb(214, 222, 232)
    } else if cx.sim && c.st.hum {
        Color32::from_rgb(255, 225, 190)
    } else {
        Color32::WHITE
    };
    d.circle(x, y, r, Some(fill));
    d.text(x, y - r * 0.28, "M", 1.25 * r / 2.0, Align2::CENTER_CENTER, cx.ink);
    d.text(x, y + r * 0.38, kind, 0.95 * r / 2.0, Align2::CENTER_CENTER, cx.ink);
    if running {
        // Rotating arrow around the rotor.
        let a0 = c.st.angle;
        let span = PI * 1.2;
        let rr = r * 0.82;
        let pts = d.arc(x, y, rr, a0, a0 + span * c.st.motor as f32);
        let col = Color32::from_rgb(0, 110, 220);
        d.p.add(Shape::line(pts, Stroke::new(d.w * 1.2, col)));
        let ae = a0 + span * c.st.motor as f32;
        let tip = (x + rr * ae.cos(), y + rr * ae.sin());
        let dir = c.st.motor as f32;
        let tx = -ae.sin() * dir;
        let ty = ae.cos() * dir;
        let k = 0.5;
        let back = (tip.0 - tx * k, tip.1 - ty * k);
        let nx = -ty * k * 0.6;
        let ny = tx * k * 0.6;
        let tri = vec![
            d.xf.p(tip.0 + tx * 0.2, tip.1 + ty * 0.2),
            d.xf.p(back.0 + nx, back.1 + ny),
            d.xf.p(back.0 - nx, back.1 - ny),
        ];
        d.p.add(Shape::convex_polygon(tri, col, Stroke::NONE));
    }
}
