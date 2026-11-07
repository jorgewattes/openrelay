//! Export the drawing (everything or the selection) as PNG, JPEG, SVG or PDF,
//! on a white, grid or transparent background, to a file or the clipboard.
//! Pictures: the circuit is redrawn light, a screenshot is taken and cropped.
//! SVG: the same drawing is recorded as vector shapes and written as SVG.

use super::*;

/// Tag put on the screenshots taken for exports.
const EXPORT_TAG: &str = "simueletro-export";

#[derive(Clone, Copy, PartialEq)]
pub enum ExportFormat {
    Png,
    Jpeg,
    Svg,
    Pdf,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ExportBg {
    White,
    Grid,
    Transparent,
}

/// Choices in the export window.
#[derive(Clone, Copy)]
pub struct ExportOpts {
    pub scope: ExportScope,
    pub format: ExportFormat,
    pub bg: ExportBg,
    /// Copy to the clipboard instead of saving / downloading.
    pub clipboard: bool,
}

impl Default for ExportOpts {
    fn default() -> Self {
        ExportOpts { scope: ExportScope::All, format: ExportFormat::Png, bg: ExportBg::White, clipboard: false }
    }
}

pub(super) fn is_export_shot(u: &egui::UserData) -> bool {
    u.data
        .as_ref()
        .and_then(|d| d.downcast_ref::<&'static str>())
        .is_some_and(|s| *s == EXPORT_TAG)
}

impl App {
    /// Open the export window (PDF preselected for Ctrl+P).
    pub(super) fn open_export_dialog(&mut self, pdf: bool) {
        let has_sel = !self.sel.is_empty() || !self.sel_w.is_empty();
        self.export_opts.scope = if has_sel { ExportScope::Selection } else { ExportScope::All };
        if pdf {
            self.export_opts.format = ExportFormat::Pdf;
            self.export_opts.clipboard = false;
        }
        self.export_dialog = true;
        self.sheet = Sheet::None;
        self.lib_open = false;
    }

    pub(super) fn export_dialog_ui(&mut self, ctx: &egui::Context) {
        if !self.export_dialog {
            return;
        }
        let mut open = true;
        let mut go = false;
        let has_sel = !self.sel.is_empty() || !self.sel_w.is_empty();
        let width = (ctx.screen_rect().width() - 32.0).clamp(260.0, 420.0);
        egui::Window::new(format!("🖶 {}", t("Exportar / imprimir", "Export / print")))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .fixed_size(Vec2::new(width, 0.0))
            .show(ctx, |ui| {
                let o = &mut self.export_opts;
                ui.label(RichText::new(t("O que exportar", "What to export")).strong());
                ui.horizontal(|ui| {
                    ui.radio_value(&mut o.scope, ExportScope::All, t("Diagrama inteiro", "Whole diagram"));
                    ui.add_enabled_ui(has_sel, |ui| {
                        ui.radio_value(&mut o.scope, ExportScope::Selection, t("Só a seleção", "Selection only"));
                    });
                });
                if !has_sel {
                    o.scope = ExportScope::All;
                }
                ui.add_space(4.0);
                ui.label(RichText::new(t("Formato", "Format")).strong());
                ui.horizontal(|ui| {
                    ui.radio_value(&mut o.format, ExportFormat::Png, "PNG");
                    ui.radio_value(&mut o.format, ExportFormat::Jpeg, "JPEG");
                    ui.radio_value(&mut o.format, ExportFormat::Svg, "SVG");
                    ui.radio_value(&mut o.format, ExportFormat::Pdf, t("PDF (imprimir)", "PDF (print)"));
                });
                ui.add_space(4.0);
                ui.label(RichText::new(t("Fundo", "Background")).strong());
                let transp_ok = matches!(o.format, ExportFormat::Png | ExportFormat::Svg);
                ui.horizontal(|ui| {
                    ui.radio_value(&mut o.bg, ExportBg::White, t("Branco", "White"));
                    ui.radio_value(&mut o.bg, ExportBg::Grid, t("Com grade", "With grid"));
                    ui.add_enabled_ui(transp_ok, |ui| {
                        ui.radio_value(&mut o.bg, ExportBg::Transparent, t("Transparente", "Transparent"));
                    });
                });
                if !transp_ok && o.bg == ExportBg::Transparent {
                    o.bg = ExportBg::White;
                }
                ui.add_space(4.0);
                ui.label(RichText::new(t("Destino", "Destination")).strong());
                let clip_ok = o.format != ExportFormat::Pdf;
                ui.horizontal(|ui| {
                    ui.radio_value(&mut o.clipboard, false, t("Baixar / salvar arquivo", "Download / save file"));
                    ui.add_enabled_ui(clip_ok, |ui| {
                        ui.radio_value(&mut o.clipboard, true, t("Copiar para a área de transferência", "Copy to clipboard"));
                    });
                });
                if !clip_ok {
                    o.clipboard = false;
                }
                ui.add_space(8.0);
                let label = if o.clipboard { t("📋 Copiar", "📋 Copy") } else { t("💾 Exportar", "💾 Export") };
                let b = egui::Button::new(RichText::new(label).strong().color(Color32::WHITE))
                    .fill(ACCENT)
                    .min_size(Vec2::new(ui.available_width(), 34.0));
                if ui.add(b).clicked() {
                    go = true;
                }
            });
        if go {
            self.export_dialog = false;
            let opts = self.export_opts;
            self.start_export(opts, ctx);
        } else if !open {
            self.export_dialog = false;
        }
    }

    /// Begin an export with the chosen options.
    pub(super) fn start_export(&mut self, opts: ExportOpts, ctx: &egui::Context) {
        if self.export.is_some() {
            return;
        }
        if opts.scope == ExportScope::Selection && self.sel.is_empty() && self.sel_w.is_empty() {
            self.status = t("Selecione algo para exportar.", "Select something to export.").into();
            return;
        }
        if self.circ.comps.is_empty() && self.circ.wires.is_empty() {
            self.status = t("O circuito está vazio.", "The circuit is empty.").into();
            return;
        }
        self.sheet = Sheet::None;
        self.lib_open = false;
        self.share_link = None;
        self.help = false;
        if opts.format == ExportFormat::Svg {
            self.export_svg(opts, ctx);
            return;
        }
        self.export = Some(Export { opts, stage: 0, view: (self.off, self.zoom) });
        self.status = t("Exportando…", "Exporting…").into();
    }

    /// Area (grid units) the export shows, with room for tags and numbers.
    fn export_bounds(&self, scope: ExportScope) -> Option<(f32, f32, f32, f32)> {
        if scope == ExportScope::All {
            return self.content_bounds();
        }
        let mut b: Option<(f32, f32, f32, f32)> = None;
        let mut grow = |x0: f32, y0: f32, x1: f32, y1: f32| {
            b = Some(match b {
                None => (x0, y0, x1, y1),
                Some((a, bb, c, d)) => (a.min(x0), bb.min(y0), c.max(x1), d.max(y1)),
            });
        };
        for c in self.circ.comps.iter().filter(|c| self.sel.contains(&c.id)) {
            let (x0, y0, x1, y1) = c.world_bounds();
            grow(x0 - 4.0, y0, x1, y1);
        }
        for (_, w) in self.circ.wires.iter().enumerate().filter(|(i, _)| self.sel_w.contains(i)) {
            let (ax, ay, bx, by) = (w.a.x as f32, w.a.y as f32, w.b.x as f32, w.b.y as f32);
            grow(ax.min(bx), ay.min(by), ax.max(bx), ay.max(by));
        }
        b.map(|(a, bb, c, d)| (a - 2.0, bb - 3.0, c + 2.0, d + 2.0))
    }

    /// Runs once per frame, before the panels are drawn.
    pub(super) fn export_step(&mut self, ctx: &egui::Context) {
        let Some(ex) = &self.export else { return };
        let (opts, stage) = (ex.opts, ex.stage);
        match stage {
            0 => {
                // Fit the drawing to the canvas for this and the next frames.
                if let Some((x0, y0, x1, y1)) = self.export_bounds(opts.scope) {
                    let r = self.canvas;
                    let (w, h) = ((x1 - x0).max(4.0), (y1 - y0).max(4.0));
                    let z = ((r.width() - 24.0) / w).min((r.height() - 24.0) / h).clamp(2.0, 30.0);
                    self.zoom = z;
                    self.off = Vec2::new((r.width() - w * z) / 2.0 - x0 * z, (r.height() - h * z) / 2.0 - y0 * z);
                }
            }
            2 => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(EXPORT_TAG)));
            }
            s if s > 2 => {
                let shot = ctx.input(|i| {
                    i.raw.events.iter().find_map(|e| match e {
                        egui::Event::Screenshot { image, user_data, .. } if is_export_shot(user_data) => {
                            Some(image.clone())
                        }
                        _ => None,
                    })
                });
                if let Some(image) = shot {
                    let crop = self.export_crop(ctx, opts.scope);
                    self.finish_export();
                    match crop_image(&image, crop) {
                        Some(mut img) => {
                            if opts.bg == ExportBg::Transparent && opts.format == ExportFormat::Png {
                                unblend_white(&mut img);
                            }
                            self.save_export(img, opts, ctx);
                        }
                        None => self.status = t("Não foi possível exportar.", "Could not export.").into(),
                    }
                    return;
                }
                if s > 120 {
                    self.finish_export();
                    self.status = t("A exportação não respondeu.", "The export did not respond.").into();
                    return;
                }
            }
            _ => {}
        }
        if let Some(ex) = &mut self.export {
            ex.stage += 1;
        }
    }

    /// Background of the canvas while exporting a picture.
    pub(super) fn paint_export_bg(&self, painter: &egui::Painter, th: &draw::Theme) {
        let r = self.canvas;
        painter.rect_filled(r, 0.0, Color32::WHITE);
        if self.export.as_ref().is_some_and(|e| e.opts.bg == ExportBg::Grid) {
            let z = self.zoom;
            let step = if z >= 7.0 { 1 } else { 2 };
            let rad = (z * 0.07).clamp(0.6, 1.3);
            let (ax, ay) = self.to_world(r.min);
            let (bx, by) = self.to_world(r.max);
            let mut y = ay.ceil() as i32;
            while y as f32 <= by {
                let mut x = ax.ceil() as i32;
                while x as f32 <= bx {
                    if x % step == 0 && y % step == 0 {
                        painter.circle_filled(self.to_screen(x as f32, y as f32), rad, th.dot);
                    }
                    x += 1;
                }
                y += 1;
            }
        }
    }

    /// Screen area to keep: the drawing (plus a small margin) inside the canvas, in pixels.
    fn export_crop(&self, ctx: &egui::Context, scope: ExportScope) -> Rect {
        let mut r = self.canvas;
        if let Some((x0, y0, x1, y1)) = self.export_bounds(scope) {
            let b = Rect::from_min_max(self.to_screen(x0, y0), self.to_screen(x1, y1)).expand(12.0);
            r = r.intersect(b);
        }
        let ppp = ctx.pixels_per_point();
        Rect::from_min_max((r.min.to_vec2() * ppp).to_pos2(), (r.max.to_vec2() * ppp).to_pos2())
    }

    fn finish_export(&mut self) {
        if let Some(ex) = self.export.take() {
            self.off = ex.view.0;
            self.zoom = ex.view.1;
        }
    }

    fn save_export(&mut self, img: image::RgbaImage, opts: ExportOpts, ctx: &egui::Context) {
        let base = file_stem(&self.title);
        let (w, h) = (img.width(), img.height());
        if opts.clipboard {
            let pixels: Vec<Color32> = img
                .pixels()
                .map(|p| Color32::from_rgba_unmultiplied(p.0[0], p.0[1], p.0[2], p.0[3]))
                .collect();
            ctx.copy_image(egui::ColorImage::new([w as usize, h as usize], pixels));
            self.status = format!("{} ({w}×{h} px)", t("Imagem copiada para a área de transferência", "Picture copied to the clipboard"));
            return;
        }
        let (bytes, name, mime) = match opts.format {
            ExportFormat::Pdf => {
                let title = format!("SimuEletro – {}", self.title);
                (make_pdf(&img, &title), format!("{base}.pdf"), "application/pdf")
            }
            ExportFormat::Jpeg => {
                let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
                let mut buf = Vec::new();
                if rgb.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Jpeg).is_err() {
                    self.status = t("Não foi possível criar a imagem.", "Could not create the picture.").into();
                    return;
                }
                (buf, format!("{base}.jpg"), "image/jpeg")
            }
            _ => {
                let mut buf = Vec::new();
                if img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png).is_err() {
                    self.status = t("Não foi possível criar a imagem.", "Could not create the picture.").into();
                    return;
                }
                (buf, format!("{base}.png"), "image/png")
            }
        };
        self.finish_save(&bytes, &name, mime, Some((w, h)));
    }

    fn finish_save(&mut self, bytes: &[u8], name: &str, mime: &str, size: Option<(u32, u32)>) {
        match save_bytes(bytes, name, mime) {
            Ok(Some(where_)) => {
                self.status = match size {
                    Some((w, h)) => format!("{} {where_} ({w}×{h} px)", t("Arquivo salvo:", "File saved:")),
                    None => format!("{} {where_}", t("Arquivo salvo:", "File saved:")),
                }
            }
            Ok(None) => self.status = t("Exportação cancelada.", "Export cancelled.").into(),
            Err(e) => self.status = format!("{} {e}", t("Erro ao salvar:", "Could not save:")),
        }
    }

    /// SVG: redraw the circuit on a hidden layer, read the shapes back and write them as SVG.
    fn export_svg(&mut self, opts: ExportOpts, ctx: &egui::Context) {
        let Some((x0, y0, x1, y1)) = self.export_bounds(opts.scope) else { return };
        let z = 10.0;
        let m = 12.0;
        let (w, h) = ((x1 - x0) * z + 2.0 * m, (y1 - y0) * z + 2.0 * m);
        let saved = (self.canvas, self.off, self.zoom);
        self.canvas = Rect::from_min_size(Pos2::ZERO, Vec2::new(w, h));
        self.off = Vec2::new(m - x0 * z, m - y0 * z);
        self.zoom = z;
        self.export = Some(Export { opts, stage: 255, view: (saved.1, saved.2) });
        let layer = egui::LayerId::new(egui::Order::Background, egui::Id::new("simueletro_svg_export"));
        let painter = egui::Painter::new(ctx.clone(), layer, Rect::NOTHING);
        let th = draw::Theme::get(false);
        self.paint_circuit(&painter, &th);
        let shapes: Vec<egui::Shape> = ctx.graphics(|g| {
            g.get(layer).map(|l| l.all_entries().map(|cs| cs.shape.clone()).collect()).unwrap_or_default()
        });
        self.export = None;
        self.canvas = saved.0;
        self.off = saved.1;
        self.zoom = saved.2;

        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.0}\" height=\"{h:.0}\" viewBox=\"0 0 {w:.1} {h:.1}\" font-family=\"Ubuntu, 'Segoe UI', Arial, sans-serif\">\n"
        );
        match opts.bg {
            ExportBg::White => svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n"),
            ExportBg::Grid => svg.push_str(&format!(
                "<defs><pattern id=\"g\" width=\"{z}\" height=\"{z}\" patternUnits=\"userSpaceOnUse\" x=\"{ox:.2}\" y=\"{oy:.2}\"><circle cx=\"0\" cy=\"0\" r=\"0.8\" fill=\"#c8cdd6\"/></pattern></defs>\n<rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n<rect width=\"100%\" height=\"100%\" fill=\"url(#g)\"/>\n",
                ox = (m - x0 * z).rem_euclid(z),
                oy = (m - y0 * z).rem_euclid(z)
            )),
            ExportBg::Transparent => {}
        }
        for s in &shapes {
            svg_shape(s, &mut svg);
        }
        svg.push_str("</svg>\n");
        if opts.clipboard {
            ctx.copy_text(svg);
            self.status = t("SVG copiado para a área de transferência.", "SVG copied to the clipboard.").into();
        } else {
            let name = format!("{}.svg", file_stem(&self.title));
            self.finish_save(svg.as_bytes(), &name, "image/svg+xml", None);
        }
    }
}

fn svg_color(c: Color32) -> String {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    if a == 255 {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        format!("rgba({r},{g},{b},{:.3})", a as f32 / 255.0)
    }
}

fn svg_paint(fill: Color32, stroke: Option<(f32, Color32)>) -> String {
    let mut s = if fill.a() == 0 { " fill=\"none\"".to_string() } else { format!(" fill=\"{}\"", svg_color(fill)) };
    if let Some((w, c)) = stroke {
        if w > 0.0 && c.a() > 0 {
            s.push_str(&format!(
                " stroke=\"{}\" stroke-width=\"{w:.2}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"",
                svg_color(c)
            ));
        }
    }
    s
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// One egui shape as SVG.
fn svg_shape(s: &egui::Shape, out: &mut String) {
    use egui::epaint::ColorMode;
    use egui::Shape as S;
    match s {
        S::Vec(v) => v.iter().for_each(|x| svg_shape(x, out)),
        S::LineSegment { points, stroke } => {
            if stroke.color.a() > 0 {
                out.push_str(&format!(
                    "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\"{}/>\n",
                    points[0].x,
                    points[0].y,
                    points[1].x,
                    points[1].y,
                    svg_paint(Color32::TRANSPARENT, Some((stroke.width, stroke.color)))
                ));
            }
        }
        S::Path(p) => {
            let pts: Vec<String> = p.points.iter().map(|q| format!("{:.2},{:.2}", q.x, q.y)).collect();
            let col = match p.stroke.color {
                ColorMode::Solid(c) => c,
                _ => Color32::BLACK,
            };
            let tag = if p.closed { "polygon" } else { "polyline" };
            let fill = if p.closed { p.fill } else { Color32::TRANSPARENT };
            out.push_str(&format!(
                "<{tag} points=\"{}\"{}/>\n",
                pts.join(" "),
                svg_paint(fill, Some((p.stroke.width, col)))
            ));
        }
        S::Circle(c) => {
            out.push_str(&format!(
                "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\"{}/>\n",
                c.center.x,
                c.center.y,
                c.radius,
                svg_paint(c.fill, Some((c.stroke.width, c.stroke.color)))
            ));
        }
        S::Ellipse(e) => {
            out.push_str(&format!(
                "<ellipse cx=\"{:.2}\" cy=\"{:.2}\" rx=\"{:.2}\" ry=\"{:.2}\"{}/>\n",
                e.center.x,
                e.center.y,
                e.radius.x,
                e.radius.y,
                svg_paint(e.fill, Some((e.stroke.width, e.stroke.color)))
            ));
        }
        S::Rect(r) => {
            out.push_str(&format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{:.2}\"{}/>\n",
                r.rect.min.x,
                r.rect.min.y,
                r.rect.width(),
                r.rect.height(),
                r.corner_radius.nw as f32,
                svg_paint(r.fill, Some((r.stroke.width, r.stroke.color)))
            ));
        }
        S::Text(ts) => {
            let g = &ts.galley;
            for row in &g.rows {
                if row.glyphs.is_empty() {
                    continue;
                }
                let first = &row.glyphs[0];
                let Some(sec) = g.job.sections.first() else { continue };
                let fmt = &sec.format;
                let mut col = ts.override_text_color.unwrap_or(fmt.color);
                if col == Color32::PLACEHOLDER {
                    col = ts.fallback_color;
                }
                let xs: Vec<String> =
                    row.glyphs.iter().map(|gl| format!("{:.2}", ts.pos.x + row.pos.x + gl.pos.x)).collect();
                let text: String = row.glyphs.iter().map(|gl| gl.chr).collect();
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{:.2}\" font-size=\"{:.2}\" fill=\"{}\">{}</text>\n",
                    xs.join(" "),
                    ts.pos.y + row.pos.y + first.pos.y,
                    fmt.font_id.size,
                    svg_color(col),
                    xml(&text)
                ));
            }
        }
        _ => {}
    }
}

/// Undo the white background: alpha from how far each pixel is from white.
fn unblend_white(img: &mut image::RgbaImage) {
    for p in img.pixels_mut() {
        let [r, g, b, _] = p.0;
        let a = 255 - r.min(g).min(b);
        if a == 0 {
            p.0 = [0, 0, 0, 0];
            continue;
        }
        let af = a as f32 / 255.0;
        let un = |c: u8| (((c as f32 / 255.0 - (1.0 - af)) / af).clamp(0.0, 1.0) * 255.0).round() as u8;
        p.0 = [un(r), un(g), un(b), a];
    }
}
/// A file name from the circuit title.
fn file_stem(title: &str) -> String {
    let s: String = title
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let s = s.trim_matches('_').to_string();
    if s.is_empty() { "circuito".into() } else { s }
}

fn crop_image(image: &egui::ColorImage, r: Rect) -> Option<image::RgbaImage> {
    let [w, h] = image.size;
    let x0 = (r.min.x.round().max(0.0) as usize).min(w);
    let y0 = (r.min.y.round().max(0.0) as usize).min(h);
    let x1 = (r.max.x.round().max(0.0) as usize).min(w);
    let y1 = (r.max.y.round().max(0.0) as usize).min(h);
    if x1 <= x0 + 4 || y1 <= y0 + 4 {
        return None;
    }
    let mut out = image::RgbaImage::new((x1 - x0) as u32, (y1 - y0) as u32);
    for y in y0..y1 {
        for x in x0..x1 {
            let c = image.pixels[y * w + x];
            out.put_pixel((x - x0) as u32, (y - y0) as u32, image::Rgba([c.r(), c.g(), c.b(), 255]));
        }
    }
    Some(out)
}

/// A one-page PDF (A4, turned to suit the picture) with a title and the drawing.
pub(super) fn make_pdf(img: &image::RgbaImage, title: &str) -> Vec<u8> {
    let (iw, ih) = (img.width() as f32, img.height() as f32);
    let (pw, ph) = if iw >= ih { (842.0f32, 595.0f32) } else { (595.0, 842.0) };
    let margin = 32.0;
    let title_h = 28.0;
    let (aw, ah) = (pw - 2.0 * margin, ph - 2.0 * margin - title_h);
    let s = (aw / iw).min(ah / ih);
    let (dw, dh) = (iw * s, ih * s);
    let (x, y) = (margin + (aw - dw) / 2.0, margin + (ah - dh) / 2.0);

    let mut rgb = Vec::with_capacity((img.width() * img.height() * 3) as usize);
    for p in img.pixels() {
        rgb.extend_from_slice(&p.0[..3]);
    }
    let data = miniz_oxide::deflate::compress_to_vec_zlib(&rgb, 6);

    // Title in the standard Helvetica font (WinAnsi text).
    let mut text = Vec::new();
    for ch in title.chars() {
        let b = match ch {
            '(' | ')' | '\\' => {
                text.push(b'\\');
                ch as u8
            }
            '–' => 0x96,
            c if (c as u32) < 256 => c as u8,
            _ => b'?',
        };
        text.push(b);
    }
    let mut content = format!(
        "q {dw:.2} 0 0 {dh:.2} {x:.2} {y:.2} cm /Im1 Do Q\n0.6 0.65 0.7 RG 0.5 w {m:.2} {ly:.2} m {r:.2} {ly:.2} l S\nBT /F1 13 Tf 0.1 0.12 0.15 rg {m:.2} {ty:.2} Td (",
        m = margin,
        r = pw - margin,
        ly = ph - margin - title_h + 8.0,
        ty = ph - margin - 14.0
    )
    .into_bytes();
    content.extend_from_slice(&text);
    content.extend_from_slice(b") Tj ET\n");

    let mut out: Vec<u8> = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::new();
    let mut obj = |out: &mut Vec<u8>, body: &[u8]| {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", offsets.len()).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    };
    obj(&mut out, b"<< /Type /Catalog /Pages 2 0 R >>");
    obj(&mut out, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    obj(
        &mut out,
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {pw:.0} {ph:.0}] /Resources << /XObject << /Im1 5 0 R >> /Font << /F1 6 0 R >> >> /Contents 4 0 R >>"
        )
        .as_bytes(),
    );
    let mut c = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    c.extend_from_slice(&content);
    c.extend_from_slice(b"\nendstream");
    obj(&mut out, &c);
    let mut im = format!(
        "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode /Length {} >>\nstream\n",
        img.width(),
        img.height(),
        data.len()
    )
    .into_bytes();
    im.extend_from_slice(&data);
    im.extend_from_slice(b"\nendstream");
    obj(&mut out, &im);
    obj(&mut out, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>");
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes());
    for o in &offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", offsets.len() + 1).as_bytes(),
    );
    out
}

/// Save the file: a save dialog on computers (or `SIMU_EXPORT_DIR` for tests),
/// a download in the browser. Ok(None) = the person cancelled.
#[cfg(not(target_arch = "wasm32"))]
fn save_bytes(bytes: &[u8], name: &str, _mime: &str) -> Result<Option<String>, String> {
    let path = if let Ok(dir) = std::env::var("SIMU_EXPORT_DIR") {
        std::path::Path::new(&dir).join(name)
    } else {
        let ext = name.rsplit('.').next().unwrap_or("png");
        let label = ext.to_uppercase();
        match rfd::FileDialog::new().add_filter(&label, &[ext]).set_file_name(name).save_file() {
            Some(p) => p,
            None => return Ok(None),
        }
    };
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(Some(path.display().to_string()))
}

#[cfg(target_arch = "wasm32")]
fn save_bytes(bytes: &[u8], name: &str, mime: &str) -> Result<Option<String>, String> {
    use eframe::wasm_bindgen::JsCast as _;
    let err = |_| "download".to_string();
    let arr = js_sys::Uint8Array::from(bytes);
    let parts = js_sys::Array::new();
    parts.push(&arr.buffer());
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type(mime);
    let blob = web_sys::Blob::new_with_buffer_source_sequence_and_options(&parts, &opts).map_err(err)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(err)?;
    let doc = web_sys::window().and_then(|w| w.document()).ok_or("document")?;
    let a = doc
        .create_element("a")
        .map_err(err)?
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .map_err(|_| "anchor".to_string())?;
    a.set_href(&url);
    a.set_download(name);
    a.click();
    let _ = web_sys::Url::revoke_object_url(&url);
    Ok(Some(name.to_string()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn pdf_is_well_formed() {
        let img = image::RgbaImage::from_pixel(40, 20, image::Rgba([255, 255, 255, 255]));
        let pdf = super::make_pdf(&img, "SimuEletro – Partida (teste)");
        let s = String::from_utf8_lossy(&pdf);
        assert!(s.starts_with("%PDF-1.4"));
        assert!(s.contains("/Subtype /Image /Width 40 /Height 20"));
        assert!(s.trim_end().ends_with("%%EOF"));
    }
}
