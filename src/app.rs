//! Main window: menus, toolbar, parts palette, drawing sheet, properties.

use crate::circuit::{categories, t, Circuit, Comp, Kind, Wire, EN, LAMP_COLORS, P};
use crate::draw::{self, DrawCtx, Xf, DEAD, INK, SELECT};
use crate::examples;
use crate::sim::{self, Sim};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, CursorIcon, FontId, Key, Layout, Margin, Pos2, Rect, RichText,
    Sense, Shape, Stroke, StrokeKind, Vec2,
};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::Ordering;

const SHEET_W: f32 = 100.0;
const SHEET_H: f32 = 70.0;

const ACCENT: Color32 = Color32::from_rgb(0, 110, 230);
const FRAME_GREEN: Color32 = Color32::from_rgb(46, 150, 80);
const PANEL_BG: Color32 = Color32::from_rgb(246, 247, 250);
const DESK_BG: Color32 = Color32::from_rgb(214, 219, 226);

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Select,
    Wire,
    Place(Kind),
}

enum Drag {
    None,
    Move { last: P, pushed: bool },
    Rubber { start: Pos2 },
    Pan,
}

/// Bottom sheets used by the phone layout.
#[derive(Clone, Copy, PartialEq)]
enum Sheet {
    None,
    Parts,
    Props,
    Menu,
}

struct Shot {
    path: PathBuf,
    requested: bool,
}

pub struct App {
    circ: Circuit,
    sim: Sim,
    tool: Tool,
    place_rot: u8,
    sel: HashSet<u32>,
    sel_w: HashSet<usize>,
    off: Vec2,
    zoom: f32,
    fitted: bool,
    wire_start: Option<P>,
    drag: Drag,
    undo: Vec<Circuit>,
    redo: Vec<Circuit>,
    held: Option<u32>,
    status: String,
    file: Option<PathBuf>,
    frame: u64,
    mouse_world: Option<(f32, f32)>,
    shot: Option<Shot>,
    numbers: bool,
    grid: bool,
    canvas: Rect,
    help: bool,
    title: String,
    /// Phone layout forced from the command line (`--mobile`).
    force_mobile: bool,
    /// Phone layout active this frame (narrow screen or forced).
    mobile: bool,
    sheet: Sheet,
    parts_cat: usize,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, shot: Option<PathBuf>, force_mobile: bool) -> Self {
        setup_style(&cc.egui_ctx);
        let mut app = App {
            circ: examples::direct_start(),
            sim: Sim::default(),
            tool: Tool::Select,
            place_rot: 0,
            sel: HashSet::new(),
            sel_w: HashSet::new(),
            off: Vec2::ZERO,
            zoom: 10.0,
            fitted: false,
            wire_start: None,
            drag: Drag::None,
            undo: vec![],
            redo: vec![],
            held: None,
            status: String::new(),
            file: None,
            frame: 0,
            mouse_world: None,
            shot: shot.map(|path| Shot { path, requested: false }),
            numbers: true,
            grid: true,
            canvas: Rect::NOTHING,
            help: false,
            title: examples::list()[0].0.to_string(),
            force_mobile,
            mobile: force_mobile,
            sheet: Sheet::None,
            parts_cat: 0,
        };
        // Preview helper for pictures: SIMU_SHEET=parts opens the parts sheet.
        let preview_sheet = std::env::var("SIMU_SHEET").unwrap_or_default();
        if preview_sheet == "parts" {
            app.sheet = Sheet::Parts;
            app.parts_cat = 3;
        } else if app.shot.is_some() {
            app.sim.start(&mut app.circ);
        }
        app
    }

    // ---------- coordinates ----------

    fn to_screen(&self, x: f32, y: f32) -> Pos2 {
        self.canvas.min + self.off + Vec2::new(x, y) * self.zoom
    }

    fn to_world(&self, p: Pos2) -> (f32, f32) {
        let v = (p - self.canvas.min - self.off) / self.zoom;
        (v.x, v.y)
    }

    fn snap(&self, p: Pos2) -> P {
        let (x, y) = self.to_world(p);
        P::new(x.round() as i32, y.round() as i32)
    }

    fn fit(&mut self) {
        let r = self.canvas;
        if r.width() < 10.0 {
            return;
        }
        // On phones fit the drawn circuit (the whole sheet would be tiny).
        let (x0, y0, x1, y1) = if self.mobile {
            self.content_bounds().unwrap_or((0.0, 0.0, SHEET_W, SHEET_H))
        } else {
            (0.0, 0.0, SHEET_W, SHEET_H)
        };
        let (w, h) = ((x1 - x0).max(10.0), (y1 - y0).max(10.0));
        let margin = if self.mobile { 16.0 } else { 40.0 };
        let z = ((r.width() - margin) / w).min((r.height() - margin) / h);
        self.zoom = z.clamp(2.0, 40.0);
        self.off = Vec2::new(
            (r.width() - w * self.zoom) / 2.0 - x0 * self.zoom,
            (r.height() - h * self.zoom) / 2.0 - y0 * self.zoom,
        );
    }

    /// Area covered by components and wires, with a small margin.
    fn content_bounds(&self) -> Option<(f32, f32, f32, f32)> {
        let mut b: Option<(f32, f32, f32, f32)> = None;
        let mut grow = |x0: f32, y0: f32, x1: f32, y1: f32| {
            b = Some(match b {
                None => (x0, y0, x1, y1),
                Some((a, bb, c, d)) => (a.min(x0), bb.min(y0), c.max(x1), d.max(y1)),
            });
        };
        for c in &self.circ.comps {
            let (x0, y0, x1, y1) = c.world_bounds();
            grow(x0 - 4.0, y0, x1, y1);
        }
        for w in &self.circ.wires {
            let (ax, ay, bx, by) = (w.a.x as f32, w.a.y as f32, w.b.x as f32, w.b.y as f32);
            grow(ax.min(bx), ay.min(by), ax.max(bx), ay.max(by));
        }
        b.map(|(a, bb, c, d)| (a - 2.0, bb - 3.0, c + 2.0, d + 2.0))
    }

    fn zoom_at(&mut self, factor: f32, at: Pos2) {
        let (wx, wy) = self.to_world(at);
        self.zoom = (self.zoom * factor).clamp(2.0, 48.0);
        self.off = at - self.canvas.min - Vec2::new(wx, wy) * self.zoom;
    }

    // ---------- editing helpers ----------

    fn push_undo(&mut self) {
        self.undo.push(self.circ.clone());
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn do_undo(&mut self) {
        if self.sim.running {
            return;
        }
        if let Some(c) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.circ, c));
            self.clear_sel();
        }
    }

    fn do_redo(&mut self) {
        if self.sim.running {
            return;
        }
        if let Some(c) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.circ, c));
            self.clear_sel();
        }
    }

    fn clear_sel(&mut self) {
        self.sel.clear();
        self.sel_w.clear();
    }

    fn hit_comp(&self, w: (f32, f32)) -> Option<u32> {
        self.circ.comps.iter().rev().find_map(|c| {
            let (x0, y0, x1, y1) = c.world_bounds();
            let m = if self.mobile { 0.9 } else { 0.3 };
            (w.0 >= x0 - m && w.0 <= x1 + m && w.1 >= y0 - m && w.1 <= y1 + m).then_some(c.id)
        })
    }

    fn hit_wire(&self, p: Pos2) -> Option<usize> {
        let mut best = None;
        let mut bd = if self.mobile { 14.0f32 } else { 6.0f32 };
        for (i, w) in self.circ.wires.iter().enumerate() {
            let a = self.to_screen(w.a.x as f32, w.a.y as f32);
            let b = self.to_screen(w.b.x as f32, w.b.y as f32);
            let d = seg_dist(p, a, b);
            if d < bd {
                bd = d;
                best = Some(i);
            }
        }
        best
    }

    fn delete_selected(&mut self) {
        if self.sel.is_empty() && self.sel_w.is_empty() {
            return;
        }
        self.push_undo();
        let sel = std::mem::take(&mut self.sel);
        self.circ.comps.retain(|c| !sel.contains(&c.id));
        let sw = std::mem::take(&mut self.sel_w);
        let mut i = 0;
        self.circ.wires.retain(|_| {
            let keep = !sw.contains(&i);
            i += 1;
            keep
        });
    }

    fn rotate_selected(&mut self) {
        if self.sel.is_empty() {
            return;
        }
        self.push_undo();
        for c in self.circ.comps.iter_mut() {
            if self.sel.contains(&c.id) {
                c.rot = (c.rot + 1) % 4;
            }
        }
    }

    fn move_selection(&mut self, d: P) {
        let mut old_pins: HashSet<P> = HashSet::new();
        for c in &self.circ.comps {
            if self.sel.contains(&c.id) {
                old_pins.extend(c.pins());
            }
        }
        for c in self.circ.comps.iter_mut() {
            if self.sel.contains(&c.id) {
                c.pos = c.pos + d;
            }
        }
        for (i, w) in self.circ.wires.iter_mut().enumerate() {
            if self.sel_w.contains(&i) {
                w.a = w.a + d;
                w.b = w.b + d;
            } else {
                if old_pins.contains(&w.a) {
                    w.a = w.a + d;
                }
                if old_pins.contains(&w.b) {
                    w.b = w.b + d;
                }
            }
        }
    }

    fn cleanup_wires(&mut self) {
        if self.circ.wires.iter().any(|w| w.a == w.b) {
            self.circ.wires.retain(|w| w.a != w.b);
            self.sel_w.clear();
        }
    }

    /// True when a point already connects to something (pin, wire end or wire).
    fn is_connection(&self, p: P) -> bool {
        self.circ.comps.iter().any(|c| c.pins().contains(&p))
            || self.circ.wires.iter().any(|w| w.a == p || w.b == p || w.interior(p))
    }

    fn route(s: P, p: P) -> Vec<(P, P)> {
        if s.x == p.x || s.y == p.y {
            return vec![(s, p)];
        }
        let corner = if (p.y - s.y).abs() >= (p.x - s.x).abs() {
            P::new(s.x, p.y)
        } else {
            P::new(p.x, s.y)
        };
        vec![(s, corner), (corner, p)]
    }

    fn set_tool(&mut self, t: Tool) {
        self.tool = t;
        self.wire_start = None;
        if !matches!(t, Tool::Place(_)) {
            self.place_rot = 0;
        }
    }

    fn toggle_sim(&mut self) {
        if self.sim.running {
            self.sim.stop(&mut self.circ);
            self.status = t("Simulação parada.", "Simulation stopped.").into();
        } else {
            self.set_tool(Tool::Select);
            self.clear_sel();
            self.sim.start(&mut self.circ);
            self.status = t(
                "Simulação em curso: clique nos botões, chaves e disjuntores.",
                "Simulating: click push buttons, switches and breakers.",
            )
            .into();
        }
    }

    fn load_circuit(&mut self, c: Circuit, title: &str) {
        if self.sim.running {
            self.sim.stop(&mut self.circ);
        }
        self.push_undo();
        self.circ = c;
        self.clear_sel();
        self.set_tool(Tool::Select);
        self.title = title.to_string();
        self.fitted = false;
    }

    fn new_file(&mut self) {
        self.load_circuit(Circuit::default(), t("Sem título", "Untitled"));
        self.file = None;
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_file(&mut self) {
        if self.shot.is_some() {
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .add_filter(t("Circuito", "Circuit"), &["json", "sime"])
            .pick_file()
        else {
            return;
        };
        match std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|s| serde_json::from_str::<Circuit>(&s).map_err(|e| e.to_string()))
        {
            Ok(c) => {
                let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                self.load_circuit(c, &name);
                self.file = Some(path);
                self.status = t("Arquivo aberto.", "File opened.").into();
            }
            Err(e) => self.status = format!("{} {e}", t("Não foi possível abrir:", "Could not open:")),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn open_file(&mut self) {}

    #[cfg(not(target_arch = "wasm32"))]
    fn save_file(&mut self, ask: bool) {
        if self.shot.is_some() {
            return;
        }
        let path = match (&self.file, ask) {
            (Some(p), false) => p.clone(),
            _ => match rfd::FileDialog::new()
                .add_filter(t("Circuito", "Circuit"), &["json"])
                .set_file_name("circuito.json")
                .save_file()
            {
                Some(p) => p,
                None => return,
            },
        };
        let json = serde_json::to_string_pretty(&self.circ).unwrap_or_default();
        match std::fs::write(&path, json) {
            Ok(()) => {
                self.title = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                self.file = Some(path);
                self.status = t("Circuito salvo.", "Circuit saved.").into();
            }
            Err(e) => self.status = format!("{} {e}", t("Não foi possível salvar:", "Could not save:")),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn save_file(&mut self, _ask: bool) {}

    // ---------- panels ----------

    fn menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu")
            .frame(egui::Frame::new().fill(Color32::WHITE).inner_margin(Margin::symmetric(8, 3)))
            .show(ctx, |ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.menu_button(t("Arquivo", "File"), |ui| {
                        if ui.button(t("Novo            Ctrl+N", "New            Ctrl+N")).clicked() {
                            self.new_file();
                        }
                        if ui.button(t("Abrir…          Ctrl+O", "Open…          Ctrl+O")).clicked() {
                            self.open_file();
                        }
                        if ui.button(t("Salvar          Ctrl+S", "Save           Ctrl+S")).clicked() {
                            self.save_file(false);
                        }
                        if ui.button(t("Salvar como…", "Save as…")).clicked() {
                            self.save_file(true);
                        }
                        ui.separator();
                        ui.menu_button(t("Exemplos", "Examples"), |ui| {
                            for (name, f) in examples::list() {
                                if ui.button(name).clicked() {
                                    self.load_circuit(f(), name);
                                }
                            }
                        });
                        ui.separator();
                        if ui.button(t("Sair", "Quit")).clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.menu_button(t("Editar", "Edit"), |ui| {
                        if ui.button(t("Desfazer   Ctrl+Z", "Undo   Ctrl+Z")).clicked() {
                            self.do_undo();
                        }
                        if ui.button(t("Refazer    Ctrl+Y", "Redo   Ctrl+Y")).clicked() {
                            self.do_redo();
                        }
                        ui.separator();
                        if ui.button(t("Girar      R", "Rotate   R")).clicked() {
                            self.rotate_selected();
                        }
                        if ui.button(t("Excluir    Del", "Delete   Del")).clicked() {
                            self.delete_selected();
                        }
                        if ui.button(t("Selecionar tudo  Ctrl+A", "Select all  Ctrl+A")).clicked() {
                            self.select_all();
                        }
                    });
                    ui.menu_button(t("Desenhar", "Draw"), |ui| {
                        if ui.button(t("Selecionar   V", "Select   V")).clicked() {
                            self.set_tool(Tool::Select);
                        }
                        if ui.button(t("Fio          W", "Wire     W")).clicked() {
                            self.set_tool(Tool::Wire);
                        }
                    });
                    ui.menu_button(t("Simulação", "Simulation"), |ui| {
                        let label = if self.sim.running { t("Parar   F5", "Stop   F5") } else { t("Iniciar   F5", "Start   F5") };
                        if ui.button(label).clicked() {
                            self.toggle_sim();
                        }
                        if ui
                            .add_enabled(self.sim.running, egui::Button::new(t("Pausar / continuar", "Pause / resume")))
                            .clicked()
                        {
                            self.sim.paused = !self.sim.paused;
                        }
                    });
                    ui.menu_button(t("Ver", "View"), |ui| {
                        if ui.button(t("Ajustar à folha", "Fit sheet")).clicked() {
                            self.fit();
                        }
                        ui.checkbox(&mut self.numbers, t("Numeração dos bornes", "Terminal numbers"));
                        ui.checkbox(&mut self.grid, t("Grade", "Grid"));
                        ui.separator();
                        let mut en = EN.load(Ordering::Relaxed);
                        ui.label(t("Idioma", "Language"));
                        let a = ui.radio_value(&mut en, false, "Português").clicked();
                        let b = ui.radio_value(&mut en, true, "English").clicked();
                        if a || b {
                            EN.store(en, Ordering::Relaxed);
                        }
                    });
                    ui.menu_button(t("Ajuda", "Help"), |ui| {
                        if ui.button(t("Como usar", "How to use")).clicked() {
                            self.help = true;
                        }
                    });
                });
            });
    }

    fn select_all(&mut self) {
        if self.sim.running {
            return;
        }
        self.sel = self.circ.comps.iter().map(|c| c.id).collect();
        self.sel_w = (0..self.circ.wires.len()).collect();
    }

    fn toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar")
            .frame(
                egui::Frame::new()
                    .fill(Color32::WHITE)
                    .inner_margin(Margin::symmetric(8, 6))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(225, 228, 234))),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let edit = !self.sim.running;
                    if tb(ui, "📄", t("Novo", "New"), edit) {
                        self.new_file();
                    }
                    if tb(ui, "📂", t("Abrir", "Open"), edit) {
                        self.open_file();
                    }
                    if tb(ui, "💾", t("Salvar", "Save"), true) {
                        self.save_file(false);
                    }
                    ui.separator();
                    if tb(ui, "⟲", t("Desfazer", "Undo"), edit && !self.undo.is_empty()) {
                        self.do_undo();
                    }
                    if tb(ui, "⟳", t("Refazer", "Redo"), edit && !self.redo.is_empty()) {
                        self.do_redo();
                    }
                    ui.separator();
                    if tool_btn(ui, "⬉", t("Selecionar", "Select"), self.tool == Tool::Select, edit) {
                        self.set_tool(Tool::Select);
                    }
                    if tool_btn(ui, "✏", t("Fio", "Wire"), self.tool == Tool::Wire, edit) {
                        self.set_tool(Tool::Wire);
                    }
                    if tb(ui, "↻", t("Girar", "Rotate"), edit) {
                        if let Tool::Place(_) = self.tool {
                            self.place_rot = (self.place_rot + 1) % 4;
                        } else {
                            self.rotate_selected();
                        }
                    }
                    if tb(ui, "🗑", t("Excluir", "Delete"), edit) {
                        self.delete_selected();
                    }
                    ui.separator();
                    let (txt, fill) = if self.sim.running {
                        (format!("⏹  {}", t("Parar", "Stop")), Color32::from_rgb(220, 53, 69))
                    } else {
                        (format!("▶  {}", t("Simular", "Simulate")), Color32::from_rgb(25, 160, 85))
                    };
                    let b = egui::Button::new(RichText::new(txt).color(Color32::WHITE).strong())
                        .fill(fill)
                        .corner_radius(CornerRadius::same(6))
                        .min_size(Vec2::new(110.0, 28.0));
                    if ui.add(b).on_hover_text("F5").clicked() {
                        self.toggle_sim();
                    }
                    let pause_txt = if self.sim.paused { "▶" } else { "⏸" };
                    if tb(ui, pause_txt, t("Pausar / continuar", "Pause / resume"), self.sim.running) {
                        self.sim.paused = !self.sim.paused;
                    }
                    ui.separator();
                    if tb(ui, "➖", t("Afastar", "Zoom out"), true) {
                        let c = self.canvas.center();
                        self.zoom_at(1.0 / 1.2, c);
                    }
                    ui.label(RichText::new(format!("{:.0}%", self.zoom * 10.0)).monospace().color(Color32::GRAY));
                    if tb(ui, "➕", t("Aproximar", "Zoom in"), true) {
                        let c = self.canvas.center();
                        self.zoom_at(1.2, c);
                    }
                    if tb(ui, "⛶", t("Ajustar à folha", "Fit sheet"), true) {
                        self.fit();
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if tb(ui, "❓", t("Como usar", "How to use"), true) {
                            self.help = !self.help;
                        }
                        ui.label(RichText::new(&self.title).color(Color32::from_rgb(90, 96, 110)));
                    });
                });
            });
    }

    fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(Color32::WHITE)
                    .inner_margin(Margin::symmetric(10, 4))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(225, 228, 234))),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let hint = if !self.status.is_empty() {
                        self.status.clone()
                    } else {
                        match self.tool {
                            Tool::Select => t(
                                "Clique para selecionar, arraste para mover. Roda do mouse: zoom; botão do meio/direito: deslocar.",
                                "Click to select, drag to move. Mouse wheel: zoom; middle/right drag: pan.",
                            )
                            .to_string(),
                            Tool::Wire => t(
                                "Clique no ponto inicial e nos cantos do fio. Clique direito ou Esc termina.",
                                "Click the start point and each corner. Right click or Esc ends the wire.",
                            )
                            .to_string(),
                            Tool::Place(k) => format!(
                                "{}: {}",
                                k.name(),
                                t("clique na folha para colocar. R gira, Esc cancela.", "click the sheet to place. R rotates, Esc cancels.")
                            ),
                        }
                    };
                    ui.label(RichText::new(hint).color(Color32::from_rgb(70, 76, 90)));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (run, col) = if !self.sim.running {
                            ("Stop", Color32::from_rgb(120, 126, 138))
                        } else if self.sim.paused {
                            (t("Pausa", "Paused"), Color32::from_rgb(220, 140, 0))
                        } else {
                            ("Run", Color32::from_rgb(25, 160, 85))
                        };
                        pill(ui, run, col);
                        let mode = if self.sim.running { t("Simulação", "Simulation") } else { t("Edição", "Editing") };
                        pill(ui, mode, ACCENT);
                        if let Some((x, y)) = self.mouse_world {
                            ui.label(
                                RichText::new(format!("X={:.0}  Y={:.0}", x.round(), y.round()))
                                    .monospace()
                                    .color(Color32::GRAY),
                            );
                        }
                    });
                });
            });
    }

    fn palette(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("palette")
            .resizable(false)
            .exact_width(232.0)
            .frame(egui::Frame::new().fill(PANEL_BG).inner_margin(Margin::symmetric(10, 10)))
            .show(ctx, |ui| {
                ui.label(RichText::new(t("Componentes", "Components")).strong().size(15.0));
                ui.add_space(4.0);
                let edit = !self.sim.running;
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.add_enabled_ui(edit, |ui| {
                        for (cat, kinds) in categories() {
                            egui::CollapsingHeader::new(RichText::new(cat).strong())
                                .default_open(true)
                                .show(ui, |ui| {
                                    for k in kinds {
                                        let active = self.tool == Tool::Place(k);
                                        if palette_item(ui, k, active) {
                                            self.set_tool(Tool::Place(k));
                                        }
                                    }
                                });
                        }
                    });
                });
            });
    }

    fn properties(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("props")
            .resizable(false)
            .exact_width(250.0)
            .frame(egui::Frame::new().fill(PANEL_BG).inner_margin(Margin::symmetric(12, 10)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    if self.sim.running {
                        self.sim_panel(ui);
                    } else {
                        self.edit_panel(ui);
                    }
                });
            });
    }

    fn edit_panel(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new(t("Propriedades", "Properties")).strong().size(15.0));
        ui.add_space(6.0);
        if self.sel.len() == 1 && self.sel_w.is_empty() {
            let id = *self.sel.iter().next().unwrap();
            let tags = self.circ.device_tags();
            let before = self.circ.clone();
            let mut changed = false;
            if let Some(c) = self.circ.comp_mut(id) {
                card(ui, |ui| {
                    ui.label(RichText::new(c.kind.name()).strong());
                    ui.add_space(4.0);
                    if !c.kind.is_terminal() {
                        ui.horizontal(|ui| {
                            ui.label(t("Identificação", "Tag"));
                            changed |= ui
                                .add(egui::TextEdit::singleline(&mut c.tag).desired_width(90.0))
                                .changed();
                        });
                    }
                    if c.kind.is_linked() {
                        ui.horizontal(|ui| {
                            ui.label(t("Segue", "Follows"));
                            egui::ComboBox::from_id_salt("link")
                                .selected_text(c.tag.clone())
                                .show_ui(ui, |ui| {
                                    for (tg, k) in &tags {
                                        if c.kind == Kind::Main3 && *k != Kind::Coil {
                                            continue;
                                        }
                                        let label = format!("{tg}  ({})", k.name());
                                        if ui.selectable_label(c.tag == *tg, label).clicked() {
                                            c.tag = tg.clone();
                                            changed = true;
                                        }
                                    }
                                });
                        });
                        if matches!(c.kind, Kind::ContactNo | Kind::ContactNc) {
                            ui.horizontal(|ui| {
                                ui.label(t("Contato nº", "Contact no."));
                                let mut n = c.num as i32 + 1;
                                if ui.add(egui::DragValue::new(&mut n).range(1..=9)).changed() {
                                    c.num = (n - 1) as u8;
                                    changed = true;
                                }
                            });
                        }
                    }
                    if matches!(c.kind, Kind::TimerOn | Kind::TimerOff) {
                        ui.horizontal(|ui| {
                            ui.label(t("Tempo", "Time"));
                            changed |= ui
                                .add(egui::DragValue::new(&mut c.preset).range(0.1..=600.0).speed(0.1).suffix(" s"))
                                .changed();
                        });
                    }
                    if c.kind == Kind::Lamp {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(t("Cor", "Colour"));
                            for (i, (pt, en, rgb)) in LAMP_COLORS.iter().enumerate() {
                                let col = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                                let (r, resp) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::click());
                                ui.painter().circle_filled(r.center(), 8.0, col);
                                let st = if c.color as usize == i { Stroke::new(2.0_f32, ACCENT) } else { Stroke::new(1.0_f32, Color32::GRAY) };
                                ui.painter().circle_stroke(r.center(), 9.0, st);
                                if resp.on_hover_text(t(pt, en)).clicked() {
                                    c.color = i as u8;
                                    changed = true;
                                }
                            }
                        });
                    }
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("X={}  Y={}  {}°", c.pos.x, c.pos.y, c.rot as u32 * 90))
                            .small()
                            .color(Color32::GRAY),
                    );
                });
            }
            if changed {
                self.undo.push(before);
                self.redo.clear();
            }
            ui.horizontal(|ui| {
                if ui.button(format!("↻ {}", t("Girar", "Rotate"))).clicked() {
                    self.rotate_selected();
                }
                if ui.button(format!("🗑 {}", t("Excluir", "Delete"))).clicked() {
                    self.delete_selected();
                }
            });
        } else if !self.sel.is_empty() || !self.sel_w.is_empty() {
            let (n, m) = (self.sel.len(), self.sel_w.len());
            card(ui, |ui| {
                ui.label(format!(
                    "{} {}, {} {}",
                    n,
                    t("componentes", "components"),
                    m,
                    t("fios", "wires")
                ));
            });
            ui.horizontal(|ui| {
                if ui.button(format!("↻ {}", t("Girar", "Rotate"))).clicked() {
                    self.rotate_selected();
                }
                if ui.button(format!("🗑 {}", t("Excluir", "Delete"))).clicked() {
                    self.delete_selected();
                }
            });
        } else {
            card(ui, |ui| {
                ui.label(
                    RichText::new(t(
                        "Nada selecionado. Escolha um componente à esquerda e clique na folha, ou use a ferramenta Fio para ligar os bornes.",
                        "Nothing selected. Pick a component on the left and click the sheet, or use the Wire tool to connect terminals.",
                    ))
                    .color(Color32::from_rgb(90, 96, 110)),
                );
            });
        }
        ui.add_space(10.0);
        ui.label(RichText::new(t("Circuito", "Circuit")).strong().size(15.0));
        ui.add_space(4.0);
        let (nc, nw) = (self.circ.comps.len(), self.circ.wires.len());
        let devs = self.circ.device_tags();
        card(ui, |ui| {
            ui.label(format!("{}: {}", t("Componentes", "Components"), nc));
            ui.label(format!("{}: {}", t("Fios", "Wires"), nw));
            if !devs.is_empty() {
                let s: Vec<String> = devs.iter().map(|(t, _)| t.clone()).collect();
                ui.label(format!("{}: {}", t("Dispositivos", "Devices"), s.join(", ")));
            }
        });
        ui.add_space(10.0);
        let b = egui::Button::new(
            RichText::new(format!("▶  {}", t("Simular circuito", "Simulate circuit"))).color(Color32::WHITE).strong(),
        )
        .fill(Color32::from_rgb(25, 160, 85))
        .min_size(Vec2::new(ui.available_width(), 32.0));
        if ui.add(b).clicked() {
            self.toggle_sim();
        }
    }

    fn sim_panel(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new(t("Simulação", "Simulation")).strong().size(15.0));
        ui.add_space(6.0);
        if let Some(f) = &self.sim.fault {
            egui::Frame::new()
                .fill(Color32::from_rgb(255, 235, 232))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(230, 80, 60)))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(8))
                .show(ui, |ui| {
                    ui.label(RichText::new(format!("⚠ {f}")).color(Color32::from_rgb(180, 30, 20)));
                });
            ui.add_space(6.0);
        }
        let time = self.sim.time;
        card(ui, |ui| {
            ui.label(format!("{} {:.1} s", t("Tempo:", "Time:"), time));
            ui.label(
                RichText::new(t(
                    "Clique nos botões (segure), chaves, fins de curso, disjuntores e relés térmicos para operá-los.",
                    "Click push buttons (hold), switches, limit switches, breakers and overload relays to operate them.",
                ))
                .small()
                .color(Color32::from_rgb(90, 96, 110)),
            );
        });
        ui.add_space(6.0);
        let mut rows: Vec<(String, String, Color32, Option<f32>)> = vec![];
        let mut tags_seen = HashSet::new();
        for c in &self.circ.comps {
            match c.kind {
                Kind::Coil => {
                    if !tags_seen.insert(c.tag.clone()) {
                        continue;
                    }
                    let on = c.st.energized;
                    rows.push((
                        format!("{}  {}", c.tag, t("contator", "contactor")),
                        if on { t("ligado", "on") } else { t("desligado", "off") }.into(),
                        if on { Color32::from_rgb(25, 160, 85) } else { Color32::GRAY },
                        None,
                    ));
                }
                Kind::TimerOn | Kind::TimerOff => {
                    let frac = if c.kind == Kind::TimerOn {
                        c.st.timer / c.preset.max(0.01)
                    } else if c.st.out && !c.st.energized {
                        1.0 - c.st.timer / c.preset.max(0.01)
                    } else if c.st.out {
                        1.0
                    } else {
                        0.0
                    };
                    let state = if c.st.out { t("comutado", "switched") } else { t("repouso", "rest") };
                    rows.push((
                        format!("{}  {:.1}/{:.1} s", c.tag, c.st.timer, c.preset),
                        state.into(),
                        if c.st.out { Color32::from_rgb(25, 160, 85) } else { Color32::GRAY },
                        Some(frac),
                    ));
                }
                Kind::Breaker1 | Kind::Breaker3 | Kind::Thermal3 => {
                    let tripped = c.st.on;
                    rows.push((
                        c.tag.clone(),
                        if tripped {
                            if c.kind == Kind::Thermal3 { t("disparado", "tripped") } else { t("aberto", "open") }
                        } else if c.kind == Kind::Thermal3 {
                            "OK"
                        } else {
                            t("fechado", "closed")
                        }
                        .into(),
                        if tripped { Color32::from_rgb(220, 53, 69) } else { Color32::from_rgb(25, 160, 85) },
                        None,
                    ));
                }
                Kind::Motor1 | Kind::Motor3 => {
                    let (s, col) = match c.st.motor {
                        1 => (t("girando ↻", "running ↻"), Color32::from_rgb(0, 110, 230)),
                        -1 => (t("girando ↺", "running ↺"), Color32::from_rgb(0, 110, 230)),
                        _ if c.st.hum => (t("falta de fase!", "missing phase!"), Color32::from_rgb(220, 140, 0)),
                        _ => (t("parado", "stopped"), Color32::GRAY),
                    };
                    rows.push((c.tag.clone(), s.into(), col, None));
                }
                Kind::Lamp | Kind::Buzzer => {
                    let on = c.st.energized;
                    rows.push((
                        c.tag.clone(),
                        if on { t("aceso", "on") } else { t("apagado", "off") }.into(),
                        if on { Color32::from_rgb(25, 160, 85) } else { Color32::GRAY },
                        None,
                    ));
                }
                _ => {}
            }
        }
        card(ui, |ui| {
            if rows.is_empty() {
                ui.label(t("Sem dispositivos.", "No devices."));
            }
            for (name, state, col, frac) in rows {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
                    ui.painter().circle_filled(r.center(), 5.0, col);
                    ui.label(RichText::new(name).strong());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(state).color(col));
                    });
                });
                if let Some(f) = frac {
                    ui.add(egui::ProgressBar::new(f.clamp(0.0, 1.0)).desired_height(6.0));
                }
            }
        });
        ui.add_space(10.0);
        ui.label(RichText::new(t("Legenda", "Legend")).strong());
        card(ui, |ui| {
            legend(ui, draw::pot_color(crate::circuit::Pot::L1), t("Fase / +24 V", "Phase / +24 V"));
            legend(ui, draw::pot_color(crate::circuit::Pot::N), t("Neutro / 0 V", "Neutral / 0 V"));
            legend(ui, draw::pot_color(crate::circuit::Pot::PE), t("Terra (PE)", "Earth (PE)"));
            legend(ui, DEAD, t("Sem tensão", "No voltage"));
        });
    }

    // ---------- canvas ----------

    fn canvas_ui(&mut self, ui: &mut egui::Ui) {
        let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        self.canvas = resp.rect;
        if !self.fitted {
            self.fit();
            self.fitted = true;
        }
        self.handle_input(ui, &resp);
        self.paint(&painter);
        if self.mobile {
            self.mobile_hint(&painter);
        }
    }

    fn handle_input(&mut self, ui: &egui::Ui, resp: &egui::Response) {
        let (pos, pressed, released, down, sec_pressed, sec_down, mid_down, delta, scroll, zoom_d, shift, dbl) =
            ui.input(|i| {
                (
                    i.pointer.interact_pos(),
                    i.pointer.primary_pressed(),
                    i.pointer.primary_released(),
                    i.pointer.primary_down(),
                    i.pointer.secondary_pressed(),
                    i.pointer.secondary_down(),
                    i.pointer.middle_down(),
                    i.pointer.delta(),
                    i.raw_scroll_delta.y,
                    i.zoom_delta(),
                    i.modifiers.shift,
                    i.pointer.button_double_clicked(egui::PointerButton::Primary),
                )
            });
        let hovered = resp.hovered();
        self.mouse_world = if hovered { pos.map(|p| self.to_world(p)) } else { None };

        // Pan & zoom.
        if (mid_down || sec_down) && (hovered || resp.dragged()) {
            self.off += delta;
        }
        if hovered {
            if let Some(p) = pos {
                if scroll != 0.0 {
                    self.zoom_at((scroll * 0.0018).exp(), p);
                } else if zoom_d != 1.0 {
                    self.zoom_at(zoom_d, p);
                }
            }
        }

        // One-finger drag on empty space pans (phone layout).
        if let Drag::Pan = self.drag {
            if down {
                self.off += delta;
            } else {
                self.drag = Drag::None;
            }
            return;
        }

        let Some(pos) = pos else { return };

        if self.sim.running {
            if hovered {
                let w = self.to_world(pos);
                if pressed && self.mobile && self.hit_comp(w).is_none() {
                    self.drag = Drag::Pan;
                }
                if let Some(id) = self.hit_comp(w) {
                    let k = self.circ.comp(id).map(|c| c.kind);
                    if matches!(
                        k,
                        Some(
                            Kind::PushNo
                                | Kind::PushNc
                                | Kind::Switch
                                | Kind::EStop
                                | Kind::LimitNo
                                | Kind::LimitNc
                                | Kind::Breaker1
                                | Kind::Breaker3
                                | Kind::Thermal3
                        )
                    ) {
                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                    }
                }
                if pressed {
                    if let Some(id) = self.hit_comp(w) {
                        if let Some(c) = self.circ.comp_mut(id) {
                            match c.kind {
                                Kind::PushNo | Kind::PushNc => {
                                    c.st.pressed = true;
                                    self.held = Some(id);
                                }
                                Kind::Switch
                                | Kind::EStop
                                | Kind::LimitNo
                                | Kind::LimitNc
                                | Kind::Breaker1
                                | Kind::Breaker3
                                | Kind::Thermal3 => c.st.on = !c.st.on,
                                _ => {}
                            }
                        }
                    }
                }
            }
            if released || !down {
                if let Some(id) = self.held.take() {
                    if let Some(c) = self.circ.comp_mut(id) {
                        c.st.pressed = false;
                    }
                }
            }
            return;
        }

        if sec_pressed && hovered {
            if self.wire_start.is_some() {
                self.wire_start = None;
            } else if matches!(self.tool, Tool::Place(_)) {
                self.set_tool(Tool::Select);
            }
        }

        match self.tool {
            Tool::Place(kind) => {
                if pressed && hovered {
                    let origin = self.place_origin(kind, pos);
                    self.push_undo();
                    let tag = self.circ.next_tag(kind);
                    let id = self.circ.add(kind, origin, self.place_rot, &tag);
                    self.clear_sel();
                    self.sel.insert(id);
                    self.status = format!("{} {}", kind.name(), t("colocado.", "placed."));
                    if self.mobile {
                        // On a phone go back to selecting so the new part can be dragged.
                        self.set_tool(Tool::Select);
                    }
                }
            }
            Tool::Wire => {
                if pressed && hovered {
                    let p = self.snap(pos);
                    match self.wire_start {
                        None => self.wire_start = Some(p),
                        Some(s) => {
                            let ends = self.is_connection(p);
                            if s != p {
                                self.push_undo();
                                for (a, b) in Self::route(s, p) {
                                    self.circ.wires.push(Wire { a, b });
                                }
                            }
                            self.wire_start = if (ends && s != p) || (dbl && s == p) { None } else { Some(p) };
                        }
                    }
                }
            }
            Tool::Select => {
                if pressed && hovered {
                    let w = self.to_world(pos);
                    if let Some(id) = self.hit_comp(w) {
                        if shift {
                            if !self.sel.remove(&id) {
                                self.sel.insert(id);
                            }
                        } else if !self.sel.contains(&id) {
                            self.clear_sel();
                            self.sel.insert(id);
                        }
                        self.drag = Drag::Move { last: self.snap(pos), pushed: false };
                    } else if let Some(wi) = self.hit_wire(pos) {
                        if shift {
                            if !self.sel_w.remove(&wi) {
                                self.sel_w.insert(wi);
                            }
                        } else if !self.sel_w.contains(&wi) {
                            self.clear_sel();
                            self.sel_w.insert(wi);
                        }
                        self.drag = Drag::Move { last: self.snap(pos), pushed: false };
                    } else if self.mobile {
                        self.clear_sel();
                        self.drag = Drag::Pan;
                    } else {
                        if !shift {
                            self.clear_sel();
                        }
                        self.drag = Drag::Rubber { start: pos };
                    }
                }
                match self.drag {
                    Drag::Move { last, pushed } if down => {
                        let cur = self.snap(pos);
                        let d = cur - last;
                        if d != P::new(0, 0) {
                            if !pushed {
                                self.push_undo();
                            }
                            self.move_selection(d);
                            self.drag = Drag::Move { last: cur, pushed: true };
                        }
                    }
                    Drag::Rubber { start } if !down || released => {
                        let r = Rect::from_two_pos(start, pos);
                        if r.width() > 3.0 || r.height() > 3.0 {
                            let a = self.to_world(r.min);
                            let b = self.to_world(r.max);
                            for c in &self.circ.comps {
                                let (x0, y0, x1, y1) = c.world_bounds();
                                if x0 >= a.0 && y0 >= a.1 && x1 <= b.0 && y1 <= b.1 {
                                    self.sel.insert(c.id);
                                }
                            }
                            for (i, w) in self.circ.wires.iter().enumerate() {
                                let inside = |p: P| {
                                    p.x as f32 >= a.0 && p.x as f32 <= b.0 && p.y as f32 >= a.1 && p.y as f32 <= b.1
                                };
                                if inside(w.a) && inside(w.b) {
                                    self.sel_w.insert(i);
                                }
                            }
                        }
                        self.drag = Drag::None;
                    }
                    Drag::Move { .. } if !down => {
                        self.drag = Drag::None;
                        self.cleanup_wires();
                    }
                    _ => {}
                }
            }
        }
    }

    fn place_origin(&self, kind: Kind, pos: Pos2) -> P {
        let (x0, y0, x1, y1) = kind.bounds();
        let (cx, cy) = crate::circuit::rot_f((x0 + x1) / 2.0, (y0 + y1) / 2.0, self.place_rot);
        let (wx, wy) = self.to_world(pos);
        P::new((wx - cx).round() as i32, (wy - cy).round() as i32)
    }

    fn paint(&mut self, painter: &egui::Painter) {
        let r = self.canvas;
        painter.rect_filled(r, 0.0, DESK_BG);
        // Sheet with shadow.
        let tl = self.to_screen(0.0, 0.0);
        let br = self.to_screen(SHEET_W, SHEET_H);
        let sheet = Rect::from_min_max(tl, br);
        painter.rect_filled(sheet.translate(Vec2::new(3.0, 4.0)), 4.0, Color32::from_black_alpha(30));
        painter.rect_filled(sheet, 2.0, Color32::WHITE);
        let z = self.zoom;
        // Dot grid.
        if self.grid {
            let step = if z >= 7.0 { 1 } else { 2 };
            let dot = Color32::from_rgb(200, 205, 214);
            let rad = (z * 0.07).clamp(0.6, 1.3);
            let vis = r.intersect(sheet);
            let (ax, ay) = self.to_world(vis.min);
            let (bx, by) = self.to_world(vis.max);
            let (ax, ay) = ((ax.ceil() as i32).max(0), (ay.ceil() as i32).max(0));
            let (bx, by) = ((bx.floor() as i32).min(SHEET_W as i32), (by.floor() as i32).min(SHEET_H as i32));
            let mut y = ay;
            while y <= by {
                if y % step == 0 {
                    let mut x = ax;
                    while x <= bx {
                        if x % step == 0 {
                            painter.circle_filled(self.to_screen(x as f32, y as f32), rad, dot);
                        }
                        x += 1;
                    }
                }
                y += 1;
            }
        }
        // Drawing frame with column letters and row numbers.
        let fs = Stroke::new(1.2_f32, FRAME_GREEN);
        let (fx0, fy0, fx1, fy1) = (2.0, 2.0, SHEET_W - 2.0, SHEET_H - 2.0);
        painter.rect_stroke(
            Rect::from_min_max(self.to_screen(fx0, fy0), self.to_screen(fx1, fy1)),
            0.0,
            fs,
            StrokeKind::Middle,
        );
        painter.line_segment([self.to_screen(fx0, 4.0), self.to_screen(fx1, 4.0)], fs);
        painter.line_segment([self.to_screen(4.0, 4.0), self.to_screen(4.0, fy1)], fs);
        let cols = 8;
        let cw = (fx1 - 4.0) / cols as f32;
        for i in 0..cols {
            let x = 4.0 + cw * i as f32;
            if i > 0 {
                painter.line_segment([self.to_screen(x, fy0), self.to_screen(x, 4.0)], fs);
            }
            painter.text(
                self.to_screen(x + cw / 2.0, 3.0),
                Align2::CENTER_CENTER,
                ((b'A' + i as u8) as char).to_string(),
                FontId::proportional(0.95 * z),
                FRAME_GREEN,
            );
        }
        let rows = 5;
        let rh = (fy1 - 4.0) / rows as f32;
        for i in 0..rows {
            let y = 4.0 + rh * i as f32;
            if i > 0 {
                painter.line_segment([self.to_screen(fx0, y), self.to_screen(4.0, y)], fs);
            }
            painter.text(
                self.to_screen(3.0, y + rh / 2.0),
                Align2::CENTER_CENTER,
                (i + 1).to_string(),
                FontId::proportional(0.95 * z),
                FRAME_GREEN,
            );
        }

        let sim_on = self.sim.running;
        let time = self.sim.time;
        let flash = ((self.frame / 15) % 2) == 0;
        let act = sim::device_active(&self.circ);

        // Wires.
        let ww = (z * 0.16).clamp(1.0, 3.0);
        for (i, w) in self.circ.wires.iter().enumerate() {
            let a = self.to_screen(w.a.x as f32, w.a.y as f32);
            let b = self.to_screen(w.b.x as f32, w.b.y as f32);
            let mut col = INK;
            let mut dashed = false;
            let mut width = ww;
            if sim_on {
                if self.sim.nets.shorted(w.a) {
                    col = if flash { Color32::from_rgb(255, 120, 0) } else { Color32::from_rgb(222, 40, 40) };
                    width = ww * 1.8;
                } else {
                    match self.sim.nets.pot_at(w.a) {
                        Some(p) => {
                            col = draw::pot_color(p);
                            dashed = p == crate::circuit::Pot::PE;
                            width = ww * 1.25;
                        }
                        None => col = DEAD,
                    }
                }
            } else if self.sel_w.contains(&i) {
                col = SELECT;
                width = ww * 1.6;
            }
            if dashed {
                painter.extend(Shape::dashed_line(&[a, b], Stroke::new(width, col), z * 0.6, z * 0.4));
            } else {
                painter.line_segment([a, b], Stroke::new(width, col));
            }
        }
        // Junction dots.
        let mut count: HashMap<P, i32> = HashMap::new();
        for w in &self.circ.wires {
            *count.entry(w.a).or_default() += 1;
            *count.entry(w.b).or_default() += 1;
        }
        for c in &self.circ.comps {
            for p in c.pins() {
                if count.contains_key(&p) {
                    *count.entry(p).or_default() += 1;
                }
            }
        }
        let ends: Vec<P> = count.keys().copied().collect();
        for e in ends {
            if self.circ.wires.iter().any(|w| w.interior(e)) {
                *count.entry(e).or_default() += 2;
            }
        }
        for (p, n) in &count {
            if *n >= 3 {
                let col = if sim_on {
                    self.sim.nets.pot_at(*p).map(draw::pot_color).unwrap_or(DEAD)
                } else {
                    INK
                };
                painter.circle_filled(self.to_screen(p.x as f32, p.y as f32), (z * 0.32).max(2.0), col);
            }
        }

        // Components.
        let hover_id = if !sim_on && self.tool == Tool::Select && matches!(self.drag, Drag::None) {
            self.mouse_world.and_then(|w| self.hit_comp(w))
        } else {
            None
        };
        for c in &self.circ.comps {
            let selected = self.sel.contains(&c.id);
            if selected || hover_id == Some(c.id) {
                let (x0, y0, x1, y1) = c.world_bounds();
                let rr = Rect::from_min_max(self.to_screen(x0 - 0.4, y0 - 0.4), self.to_screen(x1 + 0.4, y1 + 0.4));
                let fill = if selected {
                    Color32::from_rgba_unmultiplied(0, 122, 255, 22)
                } else {
                    Color32::from_rgba_unmultiplied(0, 122, 255, 10)
                };
                painter.rect_filled(rr, 4.0, fill);
                if selected {
                    painter.rect_stroke(rr, 4.0, Stroke::new(1.0_f32, SELECT), StrokeKind::Middle);
                }
            }
            let pin_col: Vec<Option<Color32>> = if sim_on {
                c.pins()
                    .iter()
                    .map(|p| {
                        if self.sim.nets.shorted(*p) {
                            Some(Color32::from_rgb(255, 120, 0))
                        } else {
                            Some(self.sim.nets.pot_at(*p).map(draw::pot_color).unwrap_or(DEAD))
                        }
                    })
                    .collect()
            } else {
                vec![]
            };
            let act_c = match c.kind {
                Kind::Coil | Kind::TimerOn | Kind::TimerOff | Kind::Lamp | Kind::Buzzer => c.st.energized,
                _ => sim::actuated(c, &act),
            };
            let cx = DrawCtx {
                ink: if selected { SELECT } else { INK },
                act: act_c,
                linked: if c.kind.is_linked() { self.circ.device_kind(&c.tag) } else { None },
                pin_col,
                sim: sim_on,
                time,
                numbers: self.numbers,
                tag: true,
            };
            let xf = Xf { o: self.to_screen(c.pos.x as f32, c.pos.y as f32), s: z, r: c.rot };
            draw::draw_comp(painter, xf, c, &cx);
            // Timer progress ring.
            if sim_on && matches!(c.kind, Kind::TimerOn | Kind::TimerOff) && c.st.timer > 0.0 {
                let f = (c.st.timer / c.preset.max(0.01)).clamp(0.0, 1.0);
                let center = xf.p(2.8, 3.0);
                let n = 32;
                let pts: Vec<Pos2> = (0..=n)
                    .map(|i| {
                        let a = -std::f32::consts::FRAC_PI_2 + f * std::f32::consts::TAU * i as f32 / n as f32;
                        center + Vec2::new(a.cos(), a.sin()) * z * 0.9
                    })
                    .collect();
                painter.circle_stroke(center, z * 0.9, Stroke::new(2.0_f32, Color32::from_rgb(220, 225, 232)));
                painter.add(Shape::line(pts, Stroke::new(2.5_f32, Color32::from_rgb(230, 140, 0))));
            }
        }
        // Open pins (edit mode): small rings so the user sees where to connect.
        if !sim_on {
            for c in &self.circ.comps {
                for p in c.pins() {
                    if !count.contains_key(&p) {
                        painter.circle_stroke(
                            self.to_screen(p.x as f32, p.y as f32),
                            (z * 0.22).max(1.5),
                            Stroke::new(1.0_f32, Color32::from_rgb(230, 120, 40)),
                        );
                    }
                }
            }
        }

        // Tool previews.
        if let Some(mw) = self.mouse_world {
            let mp = self.to_screen(mw.0, mw.1);
            match self.tool {
                Tool::Place(kind) if !sim_on => {
                    let origin = self.place_origin(kind, mp);
                    let tmp = Comp {
                        id: u32::MAX,
                        kind,
                        pos: origin,
                        rot: self.place_rot,
                        tag: self.circ.next_tag(kind),
                        preset: 3.0,
                        color: 0,
                        num: 0,
                        st: Default::default(),
                    };
                    let cx = DrawCtx {
                        ink: Color32::from_rgba_unmultiplied(0, 110, 230, 150),
                        act: false,
                        linked: None,
                        pin_col: vec![],
                        sim: false,
                        time: 0.0,
                        numbers: false,
                        tag: true,
                    };
                    let xf = Xf { o: self.to_screen(origin.x as f32, origin.y as f32), s: z, r: self.place_rot };
                    draw::draw_comp(painter, xf, &tmp, &cx);
                }
                Tool::Wire if !sim_on => {
                    let p = P::new(mw.0.round() as i32, mw.1.round() as i32);
                    let sp = self.to_screen(p.x as f32, p.y as f32);
                    let good = self.is_connection(p);
                    let ring = if good { Color32::from_rgb(25, 160, 85) } else { ACCENT };
                    painter.circle_stroke(sp, z * 0.45, Stroke::new(1.5_f32, ring));
                    if let Some(s) = self.wire_start {
                        for (a, b) in Self::route(s, p) {
                            painter.line_segment(
                                [self.to_screen(a.x as f32, a.y as f32), self.to_screen(b.x as f32, b.y as f32)],
                                Stroke::new(ww, Color32::from_rgba_unmultiplied(0, 110, 230, 180)),
                            );
                        }
                    }
                }
                _ => {}
            }
        }
        if let Drag::Rubber { start } = self.drag {
            if let Some(mw) = self.mouse_world {
                let rr = Rect::from_two_pos(start, self.to_screen(mw.0, mw.1));
                painter.rect_filled(rr, 0.0, Color32::from_rgba_unmultiplied(0, 122, 255, 18));
                painter.rect_stroke(rr, 0.0, Stroke::new(1.0_f32, SELECT), StrokeKind::Middle);
            }
        }

        // Overlays.
        if sim_on {
            let txt = if self.sim.paused { t("SIMULAÇÃO EM PAUSA", "SIMULATION PAUSED") } else { t("SIMULAÇÃO", "SIMULATING") };
            let pos = r.left_top() + Vec2::new(14.0, 12.0);
            let galley = painter.layout_no_wrap(txt.to_string(), FontId::proportional(13.0), Color32::WHITE);
            let br = Rect::from_min_size(pos, galley.size() + Vec2::new(26.0, 10.0));
            let col = if self.sim.paused { Color32::from_rgb(220, 140, 0) } else { Color32::from_rgb(25, 160, 85) };
            painter.rect_filled(br, 12.0, col);
            painter.circle_filled(br.left_center() + Vec2::new(11.0, 0.0), 4.0, if flash { Color32::WHITE } else { col });
            painter.galley(pos + Vec2::new(19.0, 5.0), galley, Color32::WHITE);
            if let Some(f) = &self.sim.fault {
                let g = painter.layout(
                    format!("⚠  {f}"),
                    FontId::proportional(14.0),
                    Color32::WHITE,
                    (r.width() - 80.0).max(100.0),
                );
                let bw = g.size() + Vec2::new(24.0, 14.0);
                let bpos = Pos2::new(r.center().x - bw.x / 2.0, r.top() + 12.0);
                painter.rect_filled(Rect::from_min_size(bpos, bw), 8.0, Color32::from_rgb(200, 40, 30));
                painter.galley(bpos + Vec2::new(12.0, 7.0), g, Color32::WHITE);
            }
        }
    }

    fn keyboard(&mut self, ctx: &egui::Context) {
        if ctx.wants_keyboard_input() {
            return;
        }
        let (del, r, esc, z, y, s, o, n, a, f5, w, v) = ctx.input(|i| {
            let cmd = i.modifiers.command;
            (
                i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace),
                i.key_pressed(Key::R) && !cmd,
                i.key_pressed(Key::Escape),
                i.key_pressed(Key::Z) && cmd,
                i.key_pressed(Key::Y) && cmd,
                i.key_pressed(Key::S) && cmd,
                i.key_pressed(Key::O) && cmd,
                i.key_pressed(Key::N) && cmd,
                i.key_pressed(Key::A) && cmd,
                i.key_pressed(Key::F5),
                i.key_pressed(Key::W) && !cmd,
                i.key_pressed(Key::V) && !cmd,
            )
        });
        if f5 {
            self.toggle_sim();
        }
        if s {
            self.save_file(false);
        }
        if self.sim.running {
            if esc {
                self.toggle_sim();
            }
            return;
        }
        if del {
            self.delete_selected();
        }
        if r {
            if let Tool::Place(_) = self.tool {
                self.place_rot = (self.place_rot + 1) % 4;
            } else {
                self.rotate_selected();
            }
        }
        if esc {
            if self.wire_start.is_some() {
                self.wire_start = None;
            } else {
                self.set_tool(Tool::Select);
                self.clear_sel();
            }
        }
        if z {
            self.do_undo();
        }
        if y {
            self.do_redo();
        }
        if o {
            self.open_file();
        }
        if n {
            self.new_file();
        }
        if a {
            self.select_all();
        }
        if w {
            self.set_tool(Tool::Wire);
        }
        if v {
            self.set_tool(Tool::Select);
        }
    }

    fn help_window(&mut self, ctx: &egui::Context) {
        let mut open = self.help;
        egui::Window::new(t("Como usar", "How to use"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                let items: [(&str, &str); 7] = [
                    ("1. Escolha um componente na lista à esquerda e clique na folha para colocá-lo (R gira).",
                     "1. Pick a component in the list on the left and click the sheet to place it (R rotates)."),
                    ("2. Use a ferramenta Fio (W): clique num borne, depois em outro. Fios fazem cantos automaticamente.",
                     "2. Use the Wire tool (W): click a terminal, then another. Wires make corners automatically."),
                    ("3. Contatos seguem a bobina, temporizador ou disjuntor com a mesma identificação (ex.: KM1).",
                     "3. Contacts follow the coil, timer or breaker with the same tag (e.g. KM1)."),
                    ("4. Pressione Simular (F5). Fios com tensão ficam vermelhos (fase) ou azuis (neutro).",
                     "4. Press Simulate (F5). Live wires turn red (phase) or blue (neutral)."),
                    ("5. Na simulação, segure os botões com o mouse; clique em chaves, disjuntores e relés térmicos.",
                     "5. While simulating, hold push buttons with the mouse; click switches, breakers and overload relays."),
                    ("6. Roda do mouse aproxima; arraste com o botão do meio ou direito para deslocar.",
                     "6. Mouse wheel zooms; drag with the middle or right button to pan."),
                    ("7. Arquivo → Exemplos tem circuitos prontos para testar.",
                     "7. File → Examples has ready-made circuits to try."),
                ];
                for (pt, en) in items {
                    ui.label(t(pt, en));
                    ui.add_space(4.0);
                }
            });
        self.help = open;
    }

    fn screenshot_logic(&mut self, ctx: &egui::Context) {
        let Some(shot) = &mut self.shot else { return };
        // Press the start button for a moment so the picture shows a running circuit.
        if self.frame == 20 || self.frame == 45 {
            let press = self.frame == 20;
            for c in self.circ.comps.iter_mut() {
                if c.kind == Kind::PushNo && c.tag == "S2" {
                    c.st.pressed = press;
                }
            }
        }
        if self.frame >= 120 && !shot.requested {
            shot.requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let events = ctx.input(|i| i.raw.events.clone());
        for e in events {
            if let egui::Event::Screenshot { image, .. } = e {
                let [w, h] = image.size;
                let mut bytes = Vec::with_capacity(w * h * 4);
                for p in &image.pixels {
                    bytes.extend_from_slice(&p.to_array());
                }
                if let Some(img) = image::RgbaImage::from_raw(w as u32, h as u32, bytes) {
                    if let Some(dir) = shot.path.parent() {
                        let _ = std::fs::create_dir_all(dir);
                    }
                    match img.save(&shot.path) {
                        Ok(()) => println!("saved screenshot to {}", shot.path.display()),
                        Err(e) => eprintln!("could not save screenshot: {e}"),
                    }
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if self.frame > 600 {
            eprintln!("screenshot timed out");
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame += 1;
        let dt = ctx.input(|i| i.stable_dt).min(0.1);
        self.screenshot_logic(ctx);
        if self.sim.running {
            self.sim.step(&mut self.circ, dt);
        }
        self.keyboard(ctx);
        let was_mobile = self.mobile;
        self.mobile = self.force_mobile || ctx.screen_rect().width() < 760.0;
        if was_mobile != self.mobile {
            self.fitted = false;
        }
        if self.mobile {
            self.mobile_bars(ctx);
        } else {
            self.sheet = Sheet::None;
            self.menu_bar(ctx);
            self.toolbar(ctx);
            self.status_bar(ctx);
            self.palette(ctx);
            self.properties(ctx);
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(DESK_BG))
            .show(ctx, |ui| self.canvas_ui(ui));
        self.help_window(ctx);
        if self.frame % 30 == 1 {
            let title = format!("SimuEletro – {}", self.title);
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
        }
        if self.sim.running || self.shot.is_some() {
            ctx.request_repaint();
        }
    }
}

// ---------- phone layout ----------

impl App {
    /// Top app bar, bottom tool bar and the slide-up sheets used on phones.
    fn mobile_bars(&mut self, ctx: &egui::Context) {
        let edit = !self.sim.running;
        let screen = ctx.screen_rect();

        // Top bar: menu, title, undo/redo and the big simulate button.
        egui::TopBottomPanel::top("m_top")
            .frame(
                egui::Frame::new()
                    .fill(Color32::WHITE)
                    .inner_margin(Margin::symmetric(8, 6))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(225, 228, 234))),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    if round_btn(ui, "☰", self.sheet == Sheet::Menu) {
                        self.toggle_sheet(Sheet::Menu);
                    }
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(RichText::new("SimuEletro").strong().size(16.0));
                        ui.label(RichText::new(&self.title).small().color(Color32::from_rgb(110, 116, 128)));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (txt, fill) = if self.sim.running {
                            ("⏹", Color32::from_rgb(220, 53, 69))
                        } else {
                            ("▶", Color32::from_rgb(25, 160, 85))
                        };
                        let b = egui::Button::new(RichText::new(txt).size(20.0).color(Color32::WHITE))
                            .fill(fill)
                            .corner_radius(CornerRadius::same(22))
                            .min_size(Vec2::splat(44.0));
                        if ui.add(b).clicked() {
                            self.toggle_sim();
                            if self.sim.running && self.sheet != Sheet::Props {
                                self.sheet = Sheet::None;
                            }
                        }
                        if edit {
                            if round_btn_enabled(ui, "⟳", false, !self.redo.is_empty()) {
                                self.do_redo();
                            }
                            if round_btn_enabled(ui, "⟲", false, !self.undo.is_empty()) {
                                self.do_undo();
                            }
                        } else if round_btn(ui, if self.sim.paused { "▶" } else { "⏸" }, self.sim.paused) {
                            self.sim.paused = !self.sim.paused;
                        }
                    });
                });
            });

        // Bottom tool bar (thumb reach).
        egui::TopBottomPanel::bottom("m_bottom")
            .frame(
                egui::Frame::new()
                    .fill(Color32::WHITE)
                    .inner_margin(Margin::symmetric(4, 4))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(225, 228, 234))),
            )
            .show(ctx, |ui| {
                let items: Vec<(&str, &str, bool, u8)> = if edit {
                    vec![
                        ("⬉", t("Selecionar", "Select"), self.tool == Tool::Select, 0),
                        ("✏", t("Fio", "Wire"), self.tool == Tool::Wire, 1),
                        ("➕", t("Componentes", "Parts"), self.sheet == Sheet::Parts || matches!(self.tool, Tool::Place(_)), 2),
                        ("↻", t("Girar", "Rotate"), false, 3),
                        ("🗑", t("Excluir", "Delete"), false, 4),
                        ("ℹ", t("Propriedades", "Properties"), self.sheet == Sheet::Props, 5),
                    ]
                } else {
                    vec![
                        ("⛶", t("Ajustar", "Fit"), false, 6),
                        ("ℹ", t("Estado", "Status"), self.sheet == Sheet::Props, 5),
                        ("⏹", t("Parar", "Stop"), false, 7),
                    ]
                };
                let n = items.len() as f32;
                let w = (ui.available_width() / n).floor();
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for (icon, label, active, id) in items {
                        if nav_item(ui, w, icon, label, active) {
                            match id {
                                0 => self.set_tool(Tool::Select),
                                1 => self.set_tool(Tool::Wire),
                                2 => self.toggle_sheet(Sheet::Parts),
                                3 => {
                                    if let Tool::Place(_) = self.tool {
                                        self.place_rot = (self.place_rot + 1) % 4;
                                    } else {
                                        self.rotate_selected();
                                    }
                                }
                                4 => self.delete_selected(),
                                5 => self.toggle_sheet(Sheet::Props),
                                6 => self.fit(),
                                _ => self.toggle_sim(),
                            }
                        }
                    }
                });
            });

        // Slide-up sheet above the tool bar.
        if self.sheet != Sheet::None {
            let h = (screen.height() * 0.46).clamp(220.0, 520.0);
            egui::TopBottomPanel::bottom("m_sheet")
                .exact_height(h)
                .resizable(false)
                .frame(
                    egui::Frame::new()
                        .fill(PANEL_BG)
                        .inner_margin(Margin { left: 12, right: 12, top: 6, bottom: 8 })
                        .corner_radius(CornerRadius { nw: 16, ne: 16, sw: 0, se: 0 })
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(215, 220, 228))),
                )
                .show(ctx, |ui| {
                    // Grab handle + title + close.
                    let (hr, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 8.0), Sense::hover());
                    ui.painter().rect_filled(
                        Rect::from_center_size(hr.center(), Vec2::new(40.0, 4.0)),
                        2.0,
                        Color32::from_rgb(200, 205, 214),
                    );
                    let title = match self.sheet {
                        Sheet::Parts => t("Componentes", "Components"),
                        Sheet::Props if self.sim.running => t("Estado da simulação", "Simulation status"),
                        Sheet::Props => t("Propriedades", "Properties"),
                        _ => "Menu",
                    };
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(title).strong().size(16.0));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if round_btn(ui, "✖", false) {
                                self.sheet = Sheet::None;
                            }
                        });
                    });
                    match self.sheet {
                        Sheet::Parts => self.parts_sheet(ui),
                        Sheet::Props => {
                            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                                if self.sim.running {
                                    self.sim_panel(ui);
                                } else {
                                    self.edit_panel(ui);
                                }
                            });
                        }
                        _ => self.menu_sheet(ui),
                    }
                });
        }
    }

    fn toggle_sheet(&mut self, s: Sheet) {
        self.sheet = if self.sheet == s { Sheet::None } else { s };
    }

    /// Parts grouped by category: a row of category tabs and a grid of tiles.
    fn parts_sheet(&mut self, ui: &mut egui::Ui) {
        let cats = categories();
        self.parts_cat = self.parts_cat.min(cats.len() - 1);
        egui::ScrollArea::horizontal().id_salt("cats").show(ui, |ui| {
            ui.horizontal(|ui| {
                for (i, (name, _)) in cats.iter().enumerate() {
                    if chip(ui, name, i == self.parts_cat) {
                        self.parts_cat = i;
                    }
                }
            });
        });
        ui.add_space(6.0);
        let kinds = cats[self.parts_cat].1.clone();
        let cols = ((ui.available_width() / 104.0).floor() as usize).clamp(3, 6);
        let gap = 8.0;
        let tw = (ui.available_width() - gap * (cols as f32 - 1.0)) / cols as f32;
        egui::ScrollArea::vertical().id_salt("parts").auto_shrink([false, false]).show(ui, |ui| {
            for row in kinds.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for &k in row {
                        let active = self.tool == Tool::Place(k);
                        if part_tile(ui, k, tw, active) {
                            self.set_tool(Tool::Place(k));
                            self.sheet = Sheet::None;
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

    fn menu_sheet(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let full = Vec2::new(ui.available_width(), 40.0);
            let item = |ui: &mut egui::Ui, text: String| ui.add(egui::Button::new(text).min_size(full)).clicked();
            if item(ui, format!("📄  {}", t("Novo circuito", "New circuit"))) {
                self.new_file();
                self.sheet = Sheet::None;
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                if item(ui, format!("📂  {}", t("Abrir…", "Open…"))) {
                    self.open_file();
                }
                if item(ui, format!("💾  {}", t("Salvar", "Save"))) {
                    self.save_file(false);
                }
            }
            ui.add_space(4.0);
            ui.label(RichText::new(t("Exemplos", "Examples")).strong());
            for (name, f) in examples::list() {
                if item(ui, format!("⚡  {name}")) {
                    self.load_circuit(f(), name);
                    self.sheet = Sheet::None;
                }
            }
            ui.add_space(4.0);
            ui.label(RichText::new(t("Ver", "View")).strong());
            if item(ui, format!("⛶  {}", t("Ajustar ao circuito", "Fit circuit"))) {
                self.fit();
            }
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
            if item(ui, format!("❓  {}", t("Como usar", "How to use"))) {
                self.help = true;
                self.sheet = Sheet::None;
            }
        });
    }

    /// Short hint chip shown over the sheet on phones (there is no status bar).
    fn mobile_hint(&self, painter: &egui::Painter) {
        let text = if !self.status.is_empty() {
            self.status.clone()
        } else {
            match self.tool {
                Tool::Select if self.sim.running => {
                    t("Toque nos botões e chaves para operá-los.", "Tap buttons and switches to operate them.").into()
                }
                Tool::Select => t(
                    "Toque para selecionar · arraste para mover · pinça para zoom",
                    "Tap to select · drag to move · pinch to zoom",
                )
                .into(),
                Tool::Wire => t("Toque num borne e depois no outro.", "Tap a terminal, then another.").into(),
                Tool::Place(k) => format!("{}: {}", k.name(), t("toque na folha.", "tap the sheet.")),
            }
        };
        let r = self.canvas;
        let g = painter.layout(text, FontId::proportional(12.5), Color32::WHITE, r.width() - 48.0);
        let size = g.size() + Vec2::new(20.0, 10.0);
        let pos = Pos2::new(r.center().x - size.x / 2.0, r.bottom() - size.y - 10.0);
        painter.rect_filled(Rect::from_min_size(pos, size), 12.0, Color32::from_rgba_unmultiplied(30, 36, 46, 210));
        painter.galley(pos + Vec2::new(10.0, 5.0), g, Color32::WHITE);
    }
}

fn round_btn(ui: &mut egui::Ui, icon: &str, active: bool) -> bool {
    round_btn_enabled(ui, icon, active, true)
}

fn round_btn_enabled(ui: &mut egui::Ui, icon: &str, active: bool, enabled: bool) -> bool {
    let txt = RichText::new(icon).size(18.0);
    let mut b = egui::Button::new(if active { txt.color(Color32::WHITE) } else { txt })
        .corner_radius(CornerRadius::same(20))
        .min_size(Vec2::splat(40.0));
    if active {
        b = b.fill(ACCENT);
    }
    ui.add_enabled(enabled, b).clicked()
}

/// Bottom navigation item: big icon with a small label under it.
fn nav_item(ui: &mut egui::Ui, w: f32, icon: &str, label: &str, active: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 54.0), Sense::click());
    let p = ui.painter();
    let col = if active { ACCENT } else { Color32::from_rgb(70, 76, 90) };
    if active {
        p.rect_filled(
            Rect::from_center_size(rect.center() - Vec2::new(0.0, 8.0), Vec2::new(52.0, 28.0)),
            14.0,
            Color32::from_rgb(214, 230, 255),
        );
    } else if resp.is_pointer_button_down_on() {
        p.rect_filled(rect.shrink(2.0), 8.0, Color32::from_rgb(236, 239, 244));
    }
    p.text(rect.center() - Vec2::new(0.0, 8.0), Align2::CENTER_CENTER, icon, FontId::proportional(19.0), col);
    p.text(rect.center() + Vec2::new(0.0, 15.0), Align2::CENTER_CENTER, label, FontId::proportional(11.0), col);
    resp.clicked()
}

fn chip(ui: &mut egui::Ui, text: &str, active: bool) -> bool {
    let txt = RichText::new(text).size(13.0);
    let b = egui::Button::new(if active { txt.color(Color32::WHITE).strong() } else { txt })
        .fill(if active { ACCENT } else { Color32::WHITE })
        .stroke(Stroke::new(1.0_f32, if active { ACCENT } else { Color32::from_rgb(210, 215, 224) }))
        .corner_radius(CornerRadius::same(16))
        .min_size(Vec2::new(0.0, 32.0));
    ui.add(b).clicked()
}

/// Grid tile with the part's symbol and its name.
fn part_tile(ui: &mut egui::Ui, kind: Kind, w: f32, active: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 96.0), Sense::click());
    let p = ui.painter_at(rect);
    let fill = if active {
        Color32::from_rgb(214, 230, 255)
    } else if resp.is_pointer_button_down_on() {
        Color32::from_rgb(236, 239, 244)
    } else {
        Color32::WHITE
    };
    p.rect_filled(rect, 10.0, fill);
    p.rect_stroke(
        rect,
        10.0,
        Stroke::new(1.0_f32, if active { ACCENT } else { Color32::from_rgb(222, 226, 233) }),
        StrokeKind::Inside,
    );
    let icon = Rect::from_min_size(rect.min + Vec2::new(8.0, 6.0), Vec2::new(w - 16.0, 52.0));
    symbol_preview(&p, icon, kind, 7.0);
    let g = p.layout(kind.name().to_string(), FontId::proportional(11.5), Color32::from_rgb(40, 44, 52), w - 10.0);
    let gp = Pos2::new(rect.center().x - g.size().x / 2.0, rect.top() + 62.0);
    p.galley(gp, g, Color32::from_rgb(40, 44, 52));
    resp.clicked()
}

// ---------- small widgets ----------

fn setup_style(ctx: &egui::Context) {
    ctx.set_visuals(egui::Visuals::light());
    ctx.style_mut(|s| {
        s.spacing.item_spacing = Vec2::new(8.0, 6.0);
        s.spacing.button_padding = Vec2::new(8.0, 4.0);
        s.visuals.panel_fill = PANEL_BG;
        s.visuals.window_corner_radius = CornerRadius::same(10);
        s.visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
        s.visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
        s.visuals.widgets.active.corner_radius = CornerRadius::same(6);
        s.visuals.selection.bg_fill = Color32::from_rgb(210, 228, 255);
        s.visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT);
        s.text_styles.insert(egui::TextStyle::Body, FontId::proportional(13.5));
        s.text_styles.insert(egui::TextStyle::Button, FontId::proportional(13.5));
    });
}

fn tb(ui: &mut egui::Ui, icon: &str, tip: &str, enabled: bool) -> bool {
    let b = egui::Button::new(RichText::new(icon).size(16.0)).min_size(Vec2::new(30.0, 28.0));
    ui.add_enabled(enabled, b).on_hover_text(tip).clicked()
}

fn tool_btn(ui: &mut egui::Ui, icon: &str, label: &str, active: bool, enabled: bool) -> bool {
    let txt = RichText::new(format!("{icon} {label}"));
    let mut b = egui::Button::new(if active { txt.color(Color32::WHITE) } else { txt }).min_size(Vec2::new(0.0, 28.0));
    if active {
        b = b.fill(ACCENT);
    }
    ui.add_enabled(enabled, b).clicked()
}

fn pill(ui: &mut egui::Ui, text: &str, col: Color32) {
    egui::Frame::new()
        .fill(col.gamma_multiply(0.14))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(10, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(col).strong());
        });
}

fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(226, 230, 236)))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
    ui.add_space(6.0);
}

fn legend(ui: &mut egui::Ui, col: Color32, text: &str) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::new(26.0, 12.0), Sense::hover());
        ui.painter().line_segment([r.left_center(), r.right_center()], Stroke::new(3.0_f32, col));
        ui.label(text);
    });
}

fn palette_item(ui: &mut egui::Ui, kind: Kind, active: bool) -> bool {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 40.0), Sense::click());
    let p = ui.painter_at(rect);
    let bg = if active {
        Color32::from_rgb(214, 230, 255)
    } else if resp.hovered() {
        Color32::from_rgb(232, 236, 243)
    } else {
        Color32::TRANSPARENT
    };
    p.rect_filled(rect, 6.0, bg);
    if active {
        p.rect_stroke(rect, 6.0, Stroke::new(1.0_f32, ACCENT), StrokeKind::Inside);
    }
    // Symbol preview.
    let icon = Rect::from_min_size(rect.min + Vec2::new(4.0, 2.0), Vec2::new(44.0, 36.0));
    p.rect_filled(icon, 5.0, Color32::WHITE);
    symbol_preview(&p, icon, kind, 5.5);
    p.text(
        Pos2::new(icon.right() + 8.0, rect.center().y),
        Align2::LEFT_CENTER,
        kind.name(),
        FontId::proportional(13.0),
        if ui.is_enabled() { Color32::from_rgb(40, 44, 52) } else { Color32::GRAY },
    );
    resp.on_hover_text(kind.name()).clicked()
}

/// Draw a part's symbol centred in `icon`.
fn symbol_preview(p: &egui::Painter, icon: Rect, kind: Kind, max_scale: f32) {
    let (x0, y0, x1, y1) = kind.bounds();
    let s = ((icon.width() - 6.0) / (x1 - x0)).min((icon.height() - 6.0) / (y1 - y0)).min(max_scale);
    let center = icon.center();
    let o = center - Vec2::new((x0 + x1) / 2.0, (y0 + y1) / 2.0) * s;
    let c = Comp {
        id: 0,
        kind,
        pos: P::new(0, 0),
        rot: 0,
        tag: String::new(),
        preset: 3.0,
        color: 0,
        num: 0,
        st: Default::default(),
    };
    let cx = DrawCtx {
        ink: INK,
        act: false,
        linked: None,
        pin_col: vec![],
        sim: false,
        time: 0.0,
        numbers: false,
        tag: false,
    };
    draw::draw_comp(p, Xf { o, s, r: 0 }, &c, &cx);
}

fn seg_dist(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_sq();
    if len2 < 1e-6 {
        return (p - a).length();
    }
    let tt = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    (p - (a + ab * tt)).length()
}
