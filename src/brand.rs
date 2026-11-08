//! OpenRelay brand: name, tagline and the logo mark.
//!
//! The mark is a green rounded square with a relay's normally-open contact (white)
//! inside an open ring (light green): an electrical command that is "open",
//! like open-source software.

use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Shape, Stroke, Vec2};

pub const NAME: &str = "OpenRelay";
pub const REPO_URL: &str = "https://github.com/jorgewattes/openrelay";

pub fn tagline() -> &'static str {
    crate::circuit::t(
        "Simulador de comandos elétricos de código aberto",
        "Free and open-source electrical control simulator",
    )
}

const BG: [u8; 3] = [14, 124, 90];
const RING: [u8; 3] = [159, 240, 207];
const INK: [u8; 3] = [255, 255, 255];

// Geometry in a 64 × 64 box.
const RING_C: (f32, f32) = (32.0, 33.0);
const RING_R: f32 = 20.0;
const STROKE: f32 = 5.0;
/// Ring arcs (degrees, y down): left and right halves, leaving the top and
/// bottom open where the contact passes.
const ARCS: [(f32, f32); 2] = [(112.0, 248.0), (-68.0, 68.0)];
const SEGS: [((f32, f32), (f32, f32)); 3] = [
    ((32.0, 7.0), (32.0, 22.0)),  // fixed contact lead
    ((32.0, 44.0), (21.5, 24.5)), // open blade
    ((32.0, 44.0), (32.0, 58.0)), // lower lead
];
const PIVOT: ((f32, f32), f32) = ((32.0, 44.0), 3.6);

fn c32(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

/// Paint the logo mark into `rect` (square).
pub fn paint_logo(p: &egui::Painter, rect: Rect) {
    let s = rect.width().min(rect.height()) / 64.0;
    let o = rect.center() - Vec2::splat(32.0 * s);
    let at = |x: f32, y: f32| Pos2::new(o.x + x * s, o.y + y * s);
    p.rect_filled(
        Rect::from_min_size(at(0.0, 0.0), Vec2::splat(64.0 * s)),
        CornerRadius::same((14.0 * s).round().clamp(0.0, 255.0) as u8),
        c32(BG),
    );
    for (a0, a1) in ARCS {
        let pts: Vec<Pos2> = (0..=24)
            .map(|i| {
                let a = (a0 + (a1 - a0) * i as f32 / 24.0).to_radians();
                at(RING_C.0 + RING_R * a.cos(), RING_C.1 + RING_R * a.sin())
            })
            .collect();
        p.add(Shape::line(pts, Stroke::new(STROKE * s, c32(RING))));
    }
    for (a, b) in SEGS {
        p.line_segment([at(a.0, a.1), at(b.0, b.1)], Stroke::new(STROKE * s, c32(INK)));
        // Round caps.
        p.circle_filled(at(a.0, a.1), STROKE * s / 2.0, c32(INK));
        p.circle_filled(at(b.0, b.1), STROKE * s / 2.0, c32(INK));
    }
    p.circle_filled(at(PIVOT.0 .0, PIVOT.0 .1), PIVOT.1 * s, c32(INK));
}

/// Logo followed by the name, as one widget (menu bar, sheets, help).
pub fn logo_title(ui: &mut egui::Ui, size: f32, color: Color32) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
        paint_logo(ui.painter(), r);
        ui.add_space(2.0);
        ui.label(egui::RichText::new(NAME).strong().size(size * 0.78).color(color));
    });
}

fn seg_dist(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy).max(1e-6)).clamp(0.0, 1.0);
    let (qx, qy) = (a.0 + dx * t - p.0, a.1 + dy * t - p.1);
    (qx * qx + qy * qy).sqrt()
}

/// The logo as RGBA pixels (window icon).
pub fn icon_rgba(size: u32) -> Vec<u8> {
    let k = 64.0 / size as f32;
    let cov = |d: f32| (0.5 - d / k).clamp(0.0, 1.0);
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    for py in 0..size {
        for px in 0..size {
            let p = ((px as f32 + 0.5) * k, (py as f32 + 0.5) * k);
            // Rounded square (signed distance).
            let r = 14.0;
            let q = ((p.0 - 32.0).abs() - (32.0 - r), (p.1 - 32.0).abs() - (32.0 - r));
            let outside = (q.0.max(0.0).powi(2) + q.1.max(0.0).powi(2)).sqrt() + q.0.max(q.1).min(0.0) - r;
            let alpha = cov(outside);
            let mut col = [BG[0] as f32, BG[1] as f32, BG[2] as f32];
            let mut blend = |c: [u8; 3], a: f32| {
                for i in 0..3 {
                    col[i] += (c[i] as f32 - col[i]) * a;
                }
            };
            // Ring arcs.
            let (dx, dy) = (p.0 - RING_C.0, p.1 - RING_C.1);
            let ang = dy.atan2(dx).to_degrees();
            let in_arc = ARCS.iter().any(|&(a0, a1)| {
                let a = if a0 > 90.0 && ang < 0.0 { ang + 360.0 } else { ang };
                a >= a0 && a <= a1
            });
            if in_arc {
                let d = ((dx * dx + dy * dy).sqrt() - RING_R).abs() - STROKE / 2.0;
                blend(RING, cov(d));
            }
            let mut d = f32::MAX;
            for (a, b) in SEGS {
                d = d.min(seg_dist(p, a, b) - STROKE / 2.0);
            }
            let (c, r) = PIVOT;
            d = d.min(((p.0 - c.0).powi(2) + (p.1 - c.1).powi(2)).sqrt() - r);
            blend(INK, cov(d));
            out.extend_from_slice(&[col[0] as u8, col[1] as u8, col[2] as u8, (alpha * 255.0) as u8]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    /// `cargo test save_icon -- --ignored` writes the icon to target/icon.png.
    #[test]
    #[ignore]
    fn save_icon() {
        let px = super::icon_rgba(256);
        image::save_buffer("target/icon.png", &px, 256, 256, image::ColorType::Rgba8).unwrap();
    }

    #[test]
    fn icon_has_logo_colours() {
        let px = super::icon_rgba(64);
        assert_eq!(px.len(), 64 * 64 * 4);
        // Corner transparent, centre of the lower lead white.
        assert_eq!(px[3], 0);
        let i = ((54 * 64 + 32) * 4) as usize;
        assert!(px[i] > 240 && px[i + 1] > 240 && px[i + 3] > 240);
    }
}
