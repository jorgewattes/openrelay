//! Export the drawing as a PNG picture (everything or the selection) or as a
//! printable PDF page. The circuit is redrawn light on white, a screenshot of
//! the window is taken and cropped to the drawing.

use super::*;

/// Tag put on the screenshots taken for exports.
const EXPORT_TAG: &str = "simueletro-export";

pub(super) fn is_export_shot(u: &egui::UserData) -> bool {
    u.data
        .as_ref()
        .and_then(|d| d.downcast_ref::<&'static str>())
        .is_some_and(|s| *s == EXPORT_TAG)
}

impl App {
    /// Begin an export: `pdf` makes a printable page, otherwise a PNG.
    pub(super) fn start_export(&mut self, scope: ExportScope, pdf: bool) {
        if self.export.is_some() {
            return;
        }
        if scope == ExportScope::Selection && self.sel.is_empty() && self.sel_w.is_empty() {
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
        self.export = Some(Export { scope, pdf, stage: 0, view: (self.off, self.zoom) });
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
        let (scope, pdf, stage) = (ex.scope, ex.pdf, ex.stage);
        match stage {
            0 => {
                // Fit the drawing to the canvas for this and the next frames.
                if let Some((x0, y0, x1, y1)) = self.export_bounds(scope) {
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
                    let crop = self.export_crop(ctx, scope);
                    self.finish_export();
                    match crop_image(&image, crop) {
                        Some(img) => self.save_export(img, pdf),
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

    fn save_export(&mut self, img: image::RgbaImage, pdf: bool) {
        let base = file_stem(&self.title);
        let (bytes, name, mime) = if pdf {
            let title = format!("SimuEletro – {}", self.title);
            (make_pdf(&img, &title), format!("{base}.pdf"), "application/pdf")
        } else {
            let mut buf = Vec::new();
            if img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png).is_err() {
                self.status = t("Não foi possível criar a imagem.", "Could not create the picture.").into();
                return;
            }
            (buf, format!("{base}.png"), "image/png")
        };
        match save_bytes(&bytes, &name, mime, pdf) {
            Ok(Some(where_)) => {
                self.status = format!(
                    "{} {where_} ({}×{} px)",
                    if pdf { t("PDF salvo:", "PDF saved:") } else { t("Imagem salva:", "Picture saved:") },
                    img.width(),
                    img.height()
                )
            }
            Ok(None) => self.status = t("Exportação cancelada.", "Export cancelled.").into(),
            Err(e) => self.status = format!("{} {e}", t("Erro ao salvar:", "Could not save:")),
        }
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
fn save_bytes(bytes: &[u8], name: &str, _mime: &str, pdf: bool) -> Result<Option<String>, String> {
    let path = if let Ok(dir) = std::env::var("SIMU_EXPORT_DIR") {
        std::path::Path::new(&dir).join(name)
    } else {
        let (label, ext) = if pdf { ("PDF", "pdf") } else { ("PNG", "png") };
        match rfd::FileDialog::new().add_filter(label, &[ext]).set_file_name(name).save_file() {
            Some(p) => p,
            None => return Ok(None),
        }
    };
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(Some(path.display().to_string()))
}

#[cfg(target_arch = "wasm32")]
fn save_bytes(bytes: &[u8], name: &str, mime: &str, _pdf: bool) -> Result<Option<String>, String> {
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
