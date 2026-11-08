//! Phone layout (SimuRelay style): dark theme, a top bar of icon + caption
//! buttons (MENU / PEÇAS / SIMULAR …), a floating parts library with
//! coloured categories, zoom buttons in the corner and slide-up sheets.

use super::*;
use crate::circuit::{category_color, CATEGORY_DEFS};

// Phone colours, dark (SimuRelay style) or light, following the chosen theme.
fn dk() -> bool {
    UI_DARK.load(Ordering::Relaxed)
}
fn pick(d: (u8, u8, u8), l: (u8, u8, u8)) -> Color32 {
    let (r, g, b) = if dk() { d } else { l };
    Color32::from_rgb(r, g, b)
}
fn bar_bg() -> Color32 {
    pick((21, 21, 21), (255, 255, 255))
}
fn panel() -> Color32 {
    pick((28, 28, 28), (248, 249, 251))
}
fn panel_line() -> Color32 {
    pick((58, 58, 58), (214, 219, 226))
}
fn text_col() -> Color32 {
    pick((235, 235, 235), (32, 36, 44))
}
fn muted() -> Color32 {
    pick((110, 110, 110), (160, 166, 176))
}
fn hover() -> Color32 {
    pick((40, 40, 40), (232, 236, 242))
}
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
                    .fill(bar_bg())
                    .inner_margin(Margin::symmetric(2, 4))
                    .stroke(Stroke::new(1.0_f32, panel_line())),
            )
            .show(ctx, |ui| self.top_bar(ui));

        if self.sheet != Sheet::None {
            let h = (screen.height() * 0.48).clamp(240.0, 560.0);
            egui::TopBottomPanel::bottom("m_sheet")
                .exact_height(h)
                .resizable(false)
                .frame(
                    egui::Frame::new()
                        .fill(panel())
                        .inner_margin(Margin { left: 12, right: 12, top: 6, bottom: 10 })
                        .corner_radius(CornerRadius { nw: 14, ne: 14, sw: 0, se: 0 })
                        .stroke(Stroke::new(1.0_f32, panel_line())),
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
        } else if self.multi {
            let any = if has_sel { BtnState::Normal } else { BtnState::Disabled };
            items.push(Some(("↻", t("GIRAR", "ROTATE"), any, 10)));
            items.push(Some(("📋", t("COPIAR", "COPY"), any, 11)));
            items.push(Some(("✂", t("RECORTAR", "CUT"), any, 13)));
            items.push(Some(("🗑", t("EXCLUIR", "DELETE"), if has_sel { BtnState::Danger } else { BtnState::Disabled }, 12)));
            items.push(None);
            items.push(Some(("☑", t("TUDO", "ALL"), BtnState::Normal, 15)));
            items.push(Some(("✔", "OK", BtnState::Go, 16)));
        } else if has_sel {
            items.push(Some(("☑", t("VÁRIOS", "MULTI"), BtnState::Normal, 17)));
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
                                Stroke::new(1.0_f32, panel_line()),
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
            15 => {
                self.sel = self.circ.comps.iter().map(|c| c.id).collect();
                self.sel_w = (0..self.circ.wires.len()).collect();
                self.multi_status();
            }
            16 => {
                self.multi = false;
                self.clear_sel();
                self.status.clear();
            }
            17 => {
                self.multi = true;
                self.sheet = Sheet::None;
                self.multi_status();
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
            muted(),
        );
        let title = match self.sheet {
            Sheet::Props if self.sim.running => t("Estado da simulação", "Simulation status"),
            Sheet::Props => t("Valores", "Values"),
            Sheet::More => t("Mais opções", "More options"),
            _ => "Menu",
        };
        ui.horizontal(|ui| {
            ui.label(RichText::new(title).strong().size(16.0).color(text_col()));
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
        crate::brand::logo_title(ui, 26.0, text_col());
        ui.label(RichText::new(crate::brand::tagline()).small().color(muted()));
        ui.add_space(6.0);
        let full = Vec2::new(ui.available_width(), 42.0);
        let item = |ui: &mut egui::Ui, text: String| ui.add(egui::Button::new(text).min_size(full)).clicked();
        if item(ui, format!("🔗  {}", t("Compartilhar link do circuito", "Share circuit link"))) {
            self.open_share();
            self.sheet = Sheet::None;
        }
        if item(ui, format!("🖶  {}", t("Exportar / imprimir…", "Export / print…"))) {
            self.open_export_dialog(false);
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
        ui.label(RichText::new(t("Exemplos", "Examples")).strong().color(text_col()));
        for (group, items) in examples::groups() {
            ui.add_space(4.0);
            ui.label(RichText::new(group).small().color(muted()));
            for (name, f) in items {
                if item(ui, format!("⚡  {name}")) {
                    self.load_circuit(f(), name);
                    self.sheet = Sheet::None;
                }
            }
        }
        ui.add_space(6.0);
        ui.label(RichText::new(t("Desafios (encontre o erro)", "Challenges (find the fault)")).strong().color(text_col()));
        for (name, goal, f) in examples::corrections() {
            if item(ui, format!("🔎  {name}")) {
                self.open_correction(name, goal, f());
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
        if ui
            .add(egui::Button::new(format!("🔍  {}", t("Ajustar aos componentes", "Fit to components"))).min_size(full))
            .clicked()
        {
            self.fit_content();
            self.sheet = Sheet::None;
        }
        ui.add_space(6.0);
        self.page_menu(ui);
        ui.add_space(6.0);
        ui.checkbox(&mut self.numbers, t("Numeração dos bornes", "Terminal numbers"));
        ui.checkbox(&mut self.grid, t("Grade", "Grid"));
        ui.checkbox(&mut self.sound_on, t("Som da sirene", "Buzzer sound"));
        ui.horizontal_wrapped(|ui| {
            ui.label(t("Tema", "Theme"));
            ui.radio_value(&mut self.theme_mode, ThemeMode::Auto, t("Automático (do aparelho)", "Automatic (device)"));
            ui.radio_value(&mut self.theme_mode, ThemeMode::Light, t("Claro", "Light"));
            ui.radio_value(&mut self.theme_mode, ThemeMode::Dark, t("Escuro", "Dark"));
        });
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
            .color(subtle(ui)),
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
                        .fill(panel())
                        .stroke(Stroke::new(1.0_f32, panel_line()))
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
                    .fill(panel())
                    .stroke(Stroke::new(1.0_f32, panel_line()))
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
                                Stroke::new(1.0_f32, panel_line()),
                            );
                            if zoom_btn(ui, "+") {
                                self.zoom_at(1.25, c);
                            }
                            let (r, _) = ui.allocate_exact_size(Vec2::new(1.0, 44.0), Sense::hover());
                            ui.painter().line_segment(
                                [Pos2::new(r.center().x, r.top() + 10.0), Pos2::new(r.center().x, r.bottom() - 10.0)],
                                Stroke::new(1.0_f32, panel_line()),
                            );
                            // Fit the view to the components.
                            if zoom_btn(ui, "🔍") {
                                self.fit_content();
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
        p.text(Pos2::new(r.left() + 34.0, r.center().y), Align2::LEFT_CENTER, name, FontId::proportional(18.0), text_col());
        // Drop-down arrow.
        let c = Pos2::new(r.right() - 14.0, r.center().y);
        let k = 6.0;
        let tri = if self.cat_list_open {
            vec![c + Vec2::new(-k, k * 0.5), c + Vec2::new(k, k * 0.5), c + Vec2::new(0.0, -k * 0.5)]
        } else {
            vec![c + Vec2::new(-k, -k * 0.5), c + Vec2::new(k, -k * 0.5), c + Vec2::new(0.0, k * 0.5)]
        };
        p.add(Shape::convex_polygon(tri, muted(), Stroke::NONE));
        p.line_segment(
            [Pos2::new(r.left(), r.bottom()), Pos2::new(r.right(), r.bottom())],
            Stroke::new(1.0_f32, muted()),
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
                        ui.painter().rect_filled(r, 4.0, hover());
                    }
                    cat_square(ui.painter(), Pos2::new(r.left() + 14.0, r.center().y), c);
                    ui.painter().text(
                        Pos2::new(r.left() + 40.0, r.center().y),
                        Align2::LEFT_CENTER,
                        n,
                        FontId::proportional(18.0),
                        text_col(),
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

    /// Phone select-tool gestures: tap selects (or toggles in multi-selection),
    /// double tap opens the values, drag moves the part (or pans on empty space,
    /// or draws a wire from a terminal), long press enters multi-selection
    /// (on empty space it starts a selection box). Pinch is handled before.
    pub(super) fn phone_gesture(&mut self, ctx: &egui::Context, pos: Pos2, pressed: bool, down: bool, dbl: bool, now: f64) {
        const SLOP: f32 = 10.0;
        const HOLD: f64 = 0.45;
        if pressed {
            let w = self.to_world(pos);
            let comp = self.hit_comp(w);
            let target = match (comp, self.pin_near(w)) {
                // A terminal of a part that is not selected starts a wire.
                (Some(id), Some(p)) if !self.sel.contains(&id) && !self.multi => Hit::Pin(p, Some(id)),
                (Some(id), _) => Hit::Comp(id),
                (None, Some(p)) => Hit::Pin(p, None),
                (None, None) => match self.hit_wire(pos) {
                    Some(wi) => Hit::Wire(wi),
                    None => Hit::Empty,
                },
            };
            self.drag = Drag::Pending { start: pos, t0: now, target };
        }
        match self.drag {
            Drag::Pending { start, t0, target } => {
                if !down {
                    self.drag = Drag::None;
                    self.phone_tap(target, dbl);
                    return;
                }
                if (pos - start).length() > SLOP {
                    match target {
                        Hit::Comp(id) | Hit::Pin(_, Some(id)) if matches!(target, Hit::Comp(_)) || self.sel.contains(&id) => {
                            if !self.sel.contains(&id) {
                                if !self.multi {
                                    self.clear_sel();
                                }
                                self.sel.insert(id);
                            }
                            self.drag = Drag::Move { last: self.snap(start), pushed: false };
                        }
                        Hit::Pin(p, _) => self.drag = Drag::WireFrom { start: p },
                        Hit::Wire(wi) => {
                            if !self.sel_w.contains(&wi) {
                                if !self.multi {
                                    self.clear_sel();
                                }
                                self.sel_w.insert(wi);
                            }
                            self.drag = Drag::Move { last: self.snap(start), pushed: false };
                        }
                        Hit::Comp(_) | Hit::Empty => {
                            self.off += pos - start;
                            self.drag = Drag::Pan;
                        }
                    }
                } else if now - t0 > HOLD {
                    // Long press: multi-selection.
                    self.multi = true;
                    self.sheet = Sheet::None;
                    match target {
                        Hit::Comp(id) | Hit::Pin(_, Some(id)) => {
                            self.sel.insert(id);
                            self.drag = Drag::Hold { start };
                        }
                        Hit::Wire(wi) => {
                            self.sel_w.insert(wi);
                            self.drag = Drag::Hold { start };
                        }
                        Hit::Pin(_, None) | Hit::Empty => self.drag = Drag::Rubber { start },
                    }
                    self.multi_status();
                } else {
                    ctx.request_repaint();
                }
            }
            Drag::Hold { start } => {
                if !down {
                    self.drag = Drag::None;
                } else if (pos - start).length() > SLOP {
                    self.drag = Drag::Move { last: self.snap(start), pushed: false };
                }
            }
            _ => {}
        }
    }

    fn phone_tap(&mut self, target: Hit, dbl: bool) {
        let comp = match target {
            Hit::Comp(id) | Hit::Pin(_, Some(id)) => Some(id),
            _ => None,
        };
        if let Some(id) = comp {
            if self.multi {
                if !self.sel.remove(&id) {
                    self.sel.insert(id);
                }
                self.multi_status();
            } else {
                self.clear_sel();
                self.sel.insert(id);
                if dbl {
                    self.sheet = Sheet::Props;
                }
            }
        } else if let Hit::Wire(wi) = target {
            if self.multi {
                if !self.sel_w.remove(&wi) {
                    self.sel_w.insert(wi);
                }
                self.multi_status();
            } else {
                self.clear_sel();
                self.sel_w.insert(wi);
            }
        } else if !self.multi {
            self.clear_sel();
            self.sheet = Sheet::None;
            if dbl {
                self.fit_content();
            }
        }
    }

    pub(super) fn multi_status(&mut self) {
        let n = self.sel.len() + self.sel_w.len();
        self.status = format!(
            "{} {n} {} {}",
            t("Seleção múltipla:", "Multi-selection:"),
            t("item(ns).", "item(s)."),
            t("Toque para incluir/remover, arraste para mover, ✔ OK para sair.", "Tap to add/remove, drag to move, ✔ OK to finish.")
        );
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
                    "Toque: selecionar · arraste: mover · segure: seleção múltipla · arraste de um borne: fio · pinça: zoom",
                    "Tap: select · drag: move · hold: multi-select · drag from a terminal: wire · pinch: zoom",
                )
                .into(),
                Tool::Wire => t("Toque num borne e depois no outro.", "Tap a terminal, then another.").into(),
                Tool::Place(k) => format!("{}: {}", k.name(), t("toque na folha.", "tap the sheet.")),
            }
        };
        let r = self.canvas;
        let max_w = (r.width() - 230.0).max(120.0);
        let g = painter.layout(text, FontId::proportional(12.5), text_col(), max_w);
        let size = g.size() + Vec2::new(20.0, 10.0);
        let pos = Pos2::new(r.left() + 10.0, r.bottom() - size.y - 12.0);
        painter.rect_filled(Rect::from_min_size(pos, size), 4.0, if dk() { Color32::from_rgba_unmultiplied(40, 40, 40, 230) } else { Color32::from_rgba_unmultiplied(255, 255, 255, 235) });
        painter.galley(pos + Vec2::new(10.0, 5.0), g, text_col());
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
        p.rect_filled(rect, 0.0, hover());
    }
    let (icol, tcol) = match st {
        BtnState::Disabled => (muted(), muted()),
        BtnState::Active => (Color32::WHITE, Color32::WHITE),
        BtnState::Danger => (RED, text_col()),
        BtnState::Go => (GREEN, text_col()),
        _ => (text_col(), text_col()),
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
        ui.painter().rect_filled(rect, 4.0, hover());
    }
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, s, FontId::proportional(24.0), text_col());
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
        p.rect_filled(rect, 3.0, pick((30, 70, 50), (210, 240, 222)));
    } else if resp.is_pointer_button_down_on() || resp.hovered() {
        p.rect_filled(rect, 3.0, hover());
    }
    let icon = Rect::from_min_max(rect.min + Vec2::new(4.0, 4.0), Pos2::new(rect.right() - 4.0, rect.bottom() - 10.0));
    symbol_preview(&p, icon, kind, 6.5, dk());
    let [r, g, b] = category_color(kind);
    p.line_segment(
        [Pos2::new(rect.left() + 2.0, rect.bottom() - 3.0), Pos2::new(rect.right() - 2.0, rect.bottom() - 3.0)],
        Stroke::new(3.0_f32, Color32::from_rgb(r, g, b)),
    );
    resp.on_hover_text(kind.lib_name()).clicked()
}
