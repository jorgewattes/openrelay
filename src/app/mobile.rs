//! Phone layout (SimuRelay style): dark theme, a top bar of icon + caption
//! buttons (MENU / PEÇAS / SIMULAR …), a floating parts library with
//! coloured categories, zoom buttons in the corner and slide-up sheets.

use super::*;
use crate::circuit::{category_color, CATEGORY_DEFS};

const BAR_BG: Color32 = Color32::from_rgb(21, 21, 21);
const PANEL: Color32 = Color32::from_rgb(28, 28, 28);
const PANEL_LINE: Color32 = Color32::from_rgb(58, 58, 58);
const TEXT: Color32 = Color32::from_rgb(235, 235, 235);
const MUTED: Color32 = Color32::from_rgb(110, 110, 110);
const GREEN: Color32 = Color32::from_rgb(30, 158, 87);
const RED: Color32 = Color32::from_rgb(235, 70, 60);

#[derive(Clone, Copy, PartialEq)]
enum BtnState {
    Normal,
    Active,
    Disabled,
    Danger,
    Go,
}

impl App {
    /// Top bar and slide-up sheets (drawn before the canvas).
    pub(super) fn mobile_bars(&mut self, ctx: &egui::Context) {
        let screen = ctx.screen_rect();
        egui::TopBottomPanel::top("m_top")
            .exact_height(60.0)
            .frame(
                egui::Frame::new()
                    .fill(BAR_BG)
                    .inner_margin(Margin::symmetric(2, 4))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(52, 52, 52))),
            )
            .show(ctx, |ui| self.top_bar(ui));

        if self.sheet != Sheet::None {
            let h = (screen.height() * 0.48).clamp(240.0, 560.0);
            egui::TopBottomPanel::bottom("m_sheet")
                .exact_height(h)
                .resizable(false)
                .frame(
                    egui::Frame::new()
                        .fill(PANEL)
                        .inner_margin(Margin { left: 12, right: 12, top: 6, bottom: 10 })
                        .corner_radius(CornerRadius { nw: 14, ne: 14, sw: 0, se: 0 })
                        .stroke(Stroke::new(1.0_f32, PANEL_LINE)),
                )
                .show(ctx, |ui| self.sheet_ui(ui));
        }
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let edit = !self.sim.running;
        let has_sel = edit && (!self.sel.is_empty() || !self.sel_w.is_empty());
        ui.spacing_mut().item_spacing.x = 0.0;
        // (icon, caption, state, action id); `None` marks the flexible gap.
        let mut items: Vec<Option<(&str, &str, BtnState, u8)>> = Vec::new();
        let undo = if self.undo.is_empty() { BtnState::Disabled } else { BtnState::Normal };
        let redo = if self.redo.is_empty() { BtnState::Disabled } else { BtnState::Normal };
        if !edit {
            items.push(Some(("☰", "MENU", self.sheet_state(Sheet::Menu), 0)));
            items.push(Some(("■", t("PARAR", "STOP"), BtnState::Danger, 3)));
            let p = if self.sim.paused { ("▶", t("SEGUIR", "RESUME")) } else { ("⏸", t("PAUSAR", "PAUSE")) };
            items.push(Some((p.0, p.1, if self.sim.paused { BtnState::Active } else { BtnState::Normal }, 4)));
            items.push(Some(("ℹ", t("ESTADO", "STATUS"), self.sheet_state(Sheet::Props), 5)));
            items.push(None);
            items.push(Some(("•••", t("MAIS", "MORE"), self.sheet_state(Sheet::More), 6)));
        } else if has_sel {
            items.push(Some(("🔧", t("VALORES", "VALUES"), self.sheet_state(Sheet::Props), 5)));
            items.push(Some(("↻", t("GIRAR", "ROTATE"), BtnState::Normal, 10)));
            items.push(Some(("📋", t("COPIAR", "COPY"), BtnState::Normal, 11)));
            items.push(Some(("✂", t("RECORTAR", "CUT"), BtnState::Normal, 13)));
            items.push(Some(("🗑", t("EXCLUIR", "DELETE"), BtnState::Danger, 12)));
            items.push(None);
            items.push(Some(("⟲", t("DESFAZER", "UNDO"), undo, 7)));
            items.push(Some(("⟳", t("REFAZER", "REDO"), redo, 8)));
        } else {
            items.push(Some(("☰", "MENU", self.sheet_state(Sheet::Menu), 0)));
            let lib = if self.lib_open || matches!(self.tool, Tool::Place(_)) { BtnState::Active } else { BtnState::Normal };
            items.push(Some(("⊞", t("PEÇAS", "LIBRARY"), lib, 1)));
            items.push(Some(("▶", t("SIMULAR", "PLAY"), BtnState::Go, 3)));
            let wire = if self.tool == Tool::Wire { BtnState::Active } else { BtnState::Normal };
            items.push(Some(("✏", t("FIO", "WIRE"), wire, 2)));
            if self.clip.is_some() {
                items.push(Some(("📥", t("COLAR", "PASTE"), BtnState::Normal, 14)));
            }
            items.push(None);
            items.push(Some(("•••", t("MAIS", "MORE"), self.sheet_state(Sheet::More), 6)));
            items.push(Some(("⟲", t("DESFAZER", "UNDO"), undo, 7)));
            items.push(Some(("⟳", t("REFAZER", "REDO"), redo, 8)));
        }
        let n = items.iter().filter(|i| i.is_some()).count() as f32;
        let full = ui.available_width();
        let w = (full / n).clamp(44.0, 66.0);
        let gap = (full - w * n).max(0.0);
        ui.horizontal(|ui| {
            for item in items {
                match item {
                    None => {
                        let (r, _) = ui.allocate_exact_size(Vec2::new(gap, 52.0), Sense::hover());
                        if gap > 6.0 {
                            ui.painter().line_segment(
                                [Pos2::new(r.right() - 1.0, r.top() + 8.0), Pos2::new(r.right() - 1.0, r.bottom() - 8.0)],
                                Stroke::new(1.0_f32, Color32::from_rgb(60, 60, 60)),
                            );
                        }
                    }
                    Some((icon, label, st, id)) => {
                        if bar_btn(ui, w, icon, label, st) {
                            let ctx = ui.ctx().clone();
                            self.bar_action(id, &ctx);
                        }
                    }
                }
            }
        });
    }

    fn sheet_state(&self, s: Sheet) -> BtnState {
        if self.sheet == s {
            BtnState::Active
        } else {
            BtnState::Normal
        }
    }

    fn bar_action(&mut self, id: u8, ctx: &egui::Context) {
        match id {
            0 => {
                self.lib_open = false;
                self.toggle_sheet(Sheet::Menu);
            }
            1 => {
                self.lib_open = !self.lib_open;
                self.cat_list_open = false;
                self.sheet = Sheet::None;
                if !self.lib_open && matches!(self.tool, Tool::Place(_)) {
                    self.set_tool(Tool::Select);
                }
            }
            2 => {
                self.lib_open = false;
                if self.tool == Tool::Wire {
                    self.set_tool(Tool::Select);
                } else {
                    self.set_tool(Tool::Wire);
                }
            }
            3 => {
                self.lib_open = false;
                self.toggle_sim();
                self.sheet = Sheet::None;
            }
            4 => self.sim.paused = !self.sim.paused,
            5 => self.toggle_sheet(Sheet::Props),
            6 => {
                self.lib_open = false;
                self.toggle_sheet(Sheet::More);
            }
            7 => self.do_undo(),
            8 => self.do_redo(),
            10 => self.rotate_selected(),
            11 => {
                self.copy_selected(ctx);
            }
            13 => {
                self.cut_selected(ctx);
                self.sheet = Sheet::None;
            }
            14 => {
                self.lib_open = false;
                self.paste(None);
            }
            _ => {
                self.delete_selected();
                self.sheet = Sheet::None;
            }
        }
    }

    pub(super) fn toggle_sheet(&mut self, s: Sheet) {
        self.sheet = if self.sheet == s { Sheet::None } else { s };
    }

    fn sheet_ui(&mut self, ui: &mut egui::Ui) {
        let (hr, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 8.0), Sense::hover());
        ui.painter().rect_filled(
            Rect::from_center_size(hr.center(), Vec2::new(40.0, 4.0)),
            2.0,
            Color32::from_rgb(80, 80, 80),
        );
        let title = match self.sheet {
            Sheet::Props if self.sim.running => t("Estado da simulação", "Simulation status"),
            Sheet::Props => t("Valores", "Values"),
            Sheet::More => t("Mais opções", "More options"),
            _ => "Menu",
        };
        ui.horizontal(|ui| {
            ui.label(RichText::new(title).strong().size(16.0).color(TEXT));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.add(egui::Button::new(RichText::new("✖").size(16.0)).min_size(Vec2::splat(36.0))).clicked() {
                    self.sheet = Sheet::None;
                }
            });
        });
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match self.sheet {
            Sheet::Props => {
                if self.sim.running {
                    self.sim_panel(ui);
                } else {
                    self.edit_panel(ui);
                }
            }
            Sheet::More => self.more_sheet(ui),
            _ => self.menu_sheet(ui),
        });
    }

    fn menu_sheet(&mut self, ui: &mut egui::Ui) {
        let full = Vec2::new(ui.available_width(), 42.0);
        let item = |ui: &mut egui::Ui, text: String| ui.add(egui::Button::new(text).min_size(full)).clicked();
        if item(ui, format!("🔗  {}", t("Compartilhar link do circuito", "Share circuit link"))) {
            self.open_share();
            self.sheet = Sheet::None;
        }
        if item(ui, format!("📥  {}", t("Abrir circuito de um link", "Open circuit from a link"))) {
            self.link_input = Some(String::new());
            self.link_msg.clear();
            self.sheet = Sheet::None;
        }
        if item(ui, format!("📄  {}", t("Novo circuito", "New circuit"))) {
            self.new_file();
            self.sheet = Sheet::None;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if item(ui, format!("📂  {}", t("Abrir arquivo…", "Open file…"))) {
                self.open_file();
            }
            if item(ui, format!("💾  {}", t("Salvar arquivo", "Save file"))) {
                self.save_file(false);
            }
        }
        ui.add_space(6.0);
        ui.label(RichText::new(t("Exemplos", "Examples")).strong().color(TEXT));
        for (name, f) in examples::list() {
            if item(ui, format!("⚡  {name}")) {
                self.load_circuit(f(), name);
                self.sheet = Sheet::None;
            }
        }
        ui.add_space(6.0);
        if item(ui, format!("❓  {}", t("Como usar", "How to use"))) {
            self.help = true;
            self.sheet = Sheet::None;
        }
    }

    fn more_sheet(&mut self, ui: &mut egui::Ui) {
        let full = Vec2::new(ui.available_width(), 42.0);
        if ui
            .add(egui::Button::new(format!("⛶  {}", t("Ajustar o circuito à tela", "Fit circuit to screen"))).min_size(full))
            .clicked()
        {
            self.fit();
            self.sheet = Sheet::None;
        }
        if !self.sim.running
            && ui
                .add(egui::Button::new(format!("ℹ  {}", t("Valores / propriedades", "Values / properties"))).min_size(full))
                .clicked()
        {
            self.sheet = Sheet::Props;
        }
        ui.add_space(6.0);
        ui.checkbox(&mut self.numbers, t("Numeração dos bornes", "Terminal numbers"));
        ui.checkbox(&mut self.grid, t("Grade", "Grid"));
        let mut en = EN.load(Ordering::Relaxed);
        ui.horizontal(|ui| {
            ui.label(t("Idioma", "Language"));
            let a = ui.radio_value(&mut en, false, "Português").clicked();
            let b = ui.radio_value(&mut en, true, "English").clicked();
            if a || b {
                EN.store(en, Ordering::Relaxed);
            }
        });
        ui.add_space(8.0);
        ui.label(
            RichText::new(t(
                "Dica: no modo Selecionar, arraste a partir de um borne (quadradinho) até outro para ligar um fio.",
                "Tip: in Select mode, drag from a terminal (small square) to another to draw a wire.",
            ))
            .small()
            .color(Color32::from_rgb(170, 170, 170)),
        );
    }

    /// Floating library and zoom buttons (drawn after the canvas).
    pub(super) fn mobile_overlays(&mut self, ctx: &egui::Context) {
        let screen = ctx.screen_rect();
        let top = 68.0;
        if self.lib_open && !self.sim.running {
            let w = (screen.width() * 0.62).clamp(220.0, 300.0);
            let max_h = (screen.height() - top - 70.0).max(200.0);
            egui::Area::new(egui::Id::new("m_library"))
                .fixed_pos(Pos2::new(8.0, top))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(PANEL)
                        .stroke(Stroke::new(1.0_f32, PANEL_LINE))
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(Margin::same(10))
                        .shadow(egui::Shadow { offset: [0, 4], blur: 16, spread: 0, color: Color32::from_black_alpha(140) })
                        .show(ui, |ui| {
                            ui.set_width(w);
                            ui.set_max_height(max_h);
                            self.library_ui(ui, w);
                        });
                });
        }
        // Zoom box (bottom-right), like the reference app.
        egui::Area::new(egui::Id::new("m_zoom"))
            .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-10.0, -10.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(PANEL)
                    .stroke(Stroke::new(1.0_f32, PANEL_LINE))
                    .corner_radius(CornerRadius::same(4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            let c = self.canvas.center();
                            if zoom_btn(ui, "−") {
                                self.zoom_at(1.0 / 1.25, c);
                            }
                            let (r, _) = ui.allocate_exact_size(Vec2::new(1.0, 44.0), Sense::hover());
                            ui.painter().line_segment(
                                [Pos2::new(r.center().x, r.top() + 10.0), Pos2::new(r.center().x, r.bottom() - 10.0)],
                                Stroke::new(1.0_f32, PANEL_LINE),
                            );
                            if zoom_btn(ui, "+") {
                                self.zoom_at(1.25, c);
                            }
                        });
                    });
            });
    }

    fn library_ui(&mut self, ui: &mut egui::Ui, w: f32) {
        // Category selector: "▢ Todos ▾".
        let (name, col) = if self.parts_cat == 0 {
            (t("Todas", "All"), [150u8, 150, 150])
        } else {
            let d = &CATEGORY_DEFS[self.parts_cat - 1];
            (t(d.0, d.1), d.2)
        };
        let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 40.0), Sense::click());
        let p = ui.painter();
        cat_square(p, Pos2::new(r.left() + 14.0, r.center().y), col);
        p.text(Pos2::new(r.left() + 34.0, r.center().y), Align2::LEFT_CENTER, name, FontId::proportional(18.0), TEXT);
        // Drop-down arrow.
        let c = Pos2::new(r.right() - 14.0, r.center().y);
        let k = 6.0;
        let tri = if self.cat_list_open {
            vec![c + Vec2::new(-k, k * 0.5), c + Vec2::new(k, k * 0.5), c + Vec2::new(0.0, -k * 0.5)]
        } else {
            vec![c + Vec2::new(-k, -k * 0.5), c + Vec2::new(k, -k * 0.5), c + Vec2::new(0.0, k * 0.5)]
        };
        p.add(Shape::convex_polygon(tri, Color32::from_rgb(190, 190, 190), Stroke::NONE));
        p.line_segment(
            [Pos2::new(r.left(), r.bottom()), Pos2::new(r.right(), r.bottom())],
            Stroke::new(1.0_f32, Color32::from_rgb(80, 80, 80)),
        );
        if resp.clicked() {
            self.cat_list_open = !self.cat_list_open;
        }
        ui.add_space(6.0);

        if self.cat_list_open {
            let mut pick = None;
            egui::ScrollArea::vertical().id_salt("cats").show(ui, |ui| {
                let mut rows: Vec<(String, [u8; 3])> = vec![(t("Todas", "All").to_string(), [150, 150, 150])];
                rows.extend(CATEGORY_DEFS.iter().map(|d| (t(d.0, d.1).to_string(), d.2)));
                for (i, (n, c)) in rows.into_iter().enumerate() {
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 48.0), Sense::click());
                    if resp.hovered() || i == self.parts_cat {
                        ui.painter().rect_filled(r, 4.0, Color32::from_rgb(40, 40, 40));
                    }
                    cat_square(ui.painter(), Pos2::new(r.left() + 14.0, r.center().y), c);
                    ui.painter().text(
                        Pos2::new(r.left() + 40.0, r.center().y),
                        Align2::LEFT_CENTER,
                        n,
                        FontId::proportional(18.0),
                        TEXT,
                    );
                    if resp.clicked() {
                        pick = Some(i);
                    }
                }
            });
            if let Some(i) = pick {
                self.parts_cat = i;
                self.cat_list_open = false;
            }
            return;
        }

        let kinds: Vec<Kind> = if self.parts_cat == 0 {
            CATEGORY_DEFS.iter().flat_map(|d| d.3.iter().copied()).collect()
        } else {
            CATEGORY_DEFS[self.parts_cat - 1].3.to_vec()
        };
        let cols = 3;
        let gap = 6.0;
        let tw = ((w - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        egui::ScrollArea::vertical().id_salt("lib").auto_shrink([false, true]).show(ui, |ui| {
            for row in kinds.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for &k in row {
                        let active = self.tool == Tool::Place(k);
                        if lib_tile(ui, k, tw, active) {
                            self.set_tool(Tool::Place(k));
                            self.lib_open = false;
                            self.status = format!(
                                "{}: {}",
                                k.name(),
                                t("toque na folha para colocar.", "tap the sheet to place it.")
                            );
                        }
                    }
                });
                ui.add_space(gap);
            }
        });
    }

    /// Short hint chip at the bottom of the sheet (phones have no status bar).
    pub(super) fn mobile_hint(&self, painter: &egui::Painter) {
        let text = if !self.status.is_empty() {
            self.status.clone()
        } else {
            match self.tool {
                Tool::Select if self.sim.running => {
                    t("Toque nos botões e chaves para operá-los.", "Tap buttons and switches to operate them.").into()
                }
                Tool::Select => t(
                    "Toque para selecionar · arraste de um borne para ligar · pinça para zoom",
                    "Tap to select · drag from a terminal to wire · pinch to zoom",
                )
                .into(),
                Tool::Wire => t("Toque num borne e depois no outro.", "Tap a terminal, then another.").into(),
                Tool::Place(k) => format!("{}: {}", k.name(), t("toque na folha.", "tap the sheet.")),
            }
        };
        let r = self.canvas;
        let max_w = (r.width() - 140.0).max(120.0);
        let g = painter.layout(text, FontId::proportional(12.5), TEXT, max_w);
        let size = g.size() + Vec2::new(20.0, 10.0);
        let pos = Pos2::new(r.left() + 10.0, r.bottom() - size.y - 12.0);
        painter.rect_filled(Rect::from_min_size(pos, size), 4.0, Color32::from_rgba_unmultiplied(40, 40, 40, 230));
        painter.galley(pos + Vec2::new(10.0, 5.0), g, TEXT);
    }
}

/// Top-bar button: icon above an upper-case caption.
fn bar_btn(ui: &mut egui::Ui, w: f32, icon: &str, label: &str, st: BtnState) -> bool {
    let sense = if st == BtnState::Disabled { Sense::hover() } else { Sense::click() };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 52.0), sense);
    let p = ui.painter();
    if st == BtnState::Active {
        p.rect_filled(rect, 0.0, GREEN);
    } else if resp.is_pointer_button_down_on() {
        p.rect_filled(rect, 0.0, Color32::from_rgb(40, 40, 40));
    }
    let (icol, tcol) = match st {
        BtnState::Disabled => (MUTED, MUTED),
        BtnState::Danger => (RED, TEXT),
        BtnState::Go => (Color32::from_rgb(40, 200, 100), TEXT),
        _ => (TEXT, TEXT),
    };
    p.text(rect.center() - Vec2::new(0.0, 9.0), Align2::CENTER_CENTER, icon, FontId::proportional(19.0), icol);
    p.text(
        rect.center() + Vec2::new(0.0, 14.0),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(if label.chars().count() > 7 { 9.5 } else { 11.0 }),
        tcol,
    );
    resp.clicked()
}

fn zoom_btn(ui: &mut egui::Ui, s: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(54.0, 44.0), Sense::click());
    if resp.is_pointer_button_down_on() {
        ui.painter().rect_filled(rect, 4.0, Color32::from_rgb(45, 45, 45));
    }
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, s, FontId::proportional(24.0), TEXT);
    resp.clicked()
}

fn cat_square(p: &egui::Painter, c: Pos2, rgb: [u8; 3]) {
    let col = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
    p.rect_stroke(Rect::from_center_size(c, Vec2::splat(20.0)), 2.0, Stroke::new(2.0_f32, col), StrokeKind::Middle);
}

/// Library tile: white symbol on dark, coloured category line underneath.
fn lib_tile(ui: &mut egui::Ui, kind: Kind, w: f32, active: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 70.0), Sense::click());
    let p = ui.painter_at(rect);
    if active {
        p.rect_filled(rect, 3.0, Color32::from_rgb(30, 70, 50));
    } else if resp.is_pointer_button_down_on() || resp.hovered() {
        p.rect_filled(rect, 3.0, Color32::from_rgb(38, 38, 38));
    }
    let icon = Rect::from_min_max(rect.min + Vec2::new(4.0, 4.0), Pos2::new(rect.right() - 4.0, rect.bottom() - 10.0));
    symbol_preview(&p, icon, kind, 6.5, true);
    let [r, g, b] = category_color(kind);
    p.line_segment(
        [Pos2::new(rect.left() + 2.0, rect.bottom() - 3.0), Pos2::new(rect.right() - 2.0, rect.bottom() - 3.0)],
        Stroke::new(3.0_f32, Color32::from_rgb(r, g, b)),
    );
    resp.on_hover_text(kind.name()).clicked()
}
