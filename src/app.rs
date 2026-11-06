//! Main window: menus, toolbar, parts palette, drawing sheet, properties.

use crate::circuit::{categories, t, Circuit, Comp, Kind, Wire, EN, LAMP_COLORS, P};
use crate::draw::{self, DrawCtx, Xf, DEAD};
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
    /// Dragging a new wire out of a terminal.
    WireFrom { start: P },
}

mod mobile;

/// Bottom sheets used by the phone layout.
#[derive(Clone, Copy, PartialEq)]
enum Sheet {
    None,
    Props,
    Menu,
    More,
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
    /// Library category on phones: 0 = all, n = CATEGORY_DEFS[n-1].
    parts_cat: usize,
    /// Phone parts library open.
    lib_open: bool,
    /// Phone library showing its category list.
    cat_list_open: bool,
    /// Share dialog with the generated link.
    share_link: Option<String>,
    /// "Open from link" dialog text.
    link_input: Option<String>,
    link_msg: String,
    /// Visual theme currently applied to the widgets (dark = phone).
    dark_applied: Option<bool>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, shot: Option<PathBuf>, force_mobile: bool) -> Self {
        setup_style(&cc.egui_ctx, force_mobile);
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
            lib_open: false,
            cat_list_open: false,
            share_link: None,
            link_input: None,
            link_msg: String::new(),
            dark_applied: None,
        };
        // Preview helper for pictures: SIMU_SHEET=library|categories|selected.
        let preview = std::env::var("SIMU_SHEET").unwrap_or_default();
        match preview.as_str() {
            "library" => app.lib_open = true,
            "share" => app.open_share(),
            "volts" => {
                app.circ = examples::voltages();
                app.sim.start(&mut app.circ);
                if let Some(s) = app.circ.comps.iter_mut().find(|c| c.kind == Kind::Switch) {
                    s.st.on = true;
                }
            }
            "props" => {
                app.circ = examples::voltages();
                if let Some(c) = app.circ.comps.iter().find(|c| c.kind == Kind::Motor6) {
                    app.sel.insert(c.id);
                }
                app.sheet = Sheet::Props;
            }
            "categories" => {
                app.lib_open = true;
                app.cat_list_open = true;
            }
            "selected" => {
                if let Some(c) = app.circ.comps.iter().find(|c| c.kind == Kind::Motor3) {
                    app.sel.insert(c.id);
                }
            }
            "sim" => {
                if app.shot.is_some() {
                    app.sim.start(&mut app.circ);
                }
            }
            "none" => {}
            _ => {
                if app.shot.is_some() {
                    if force_mobile {
                        // Same view as the reference pictures: library open.
                        app.lib_open = true;
                    } else {
                        app.sim.start(&mut app.circ);
                    }
                }
            }
        }
        // `SIMU_LINK=<link>` opens a shared circuit (desktop check of share links).
        if let Some(c) = std::env::var("SIMU_LINK").ok().and_then(|l| crate::share::decode_from_url(&l)) {
            app.open_shared(c);
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
                        if ui.button(t("🔗 Compartilhar link…", "🔗 Share link…")).clicked() {
                            self.open_share();
                        }
                        if ui.button(t("📥 Abrir circuito de um link…", "📥 Open circuit from a link…")).clicked() {
                            self.link_input = Some(String::new());
                            self.link_msg.clear();
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

    /// Terminal (component pin) under the pointer, if any.
    fn pin_near(&self, w: (f32, f32)) -> Option<P> {
        let tol = if self.mobile { 0.9 } else { 0.45 };
        let mut best: Option<(f32, P)> = None;
        for c in &self.circ.comps {
            for p in c.pins() {
                let d = ((p.x as f32 - w.0).powi(2) + (p.y as f32 - w.1).powi(2)).sqrt();
                if d <= tol && best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, p));
                }
            }
        }
        best.map(|(_, p)| p)
    }

    /// Copy the selected parts (and wires) a little down-right.
    fn duplicate_selected(&mut self) {
        if self.sel.is_empty() && self.sel_w.is_empty() {
            return;
        }
        self.push_undo();
        let d = P::new(4, 4);
        let picked: Vec<Comp> = self.circ.comps.iter().filter(|c| self.sel.contains(&c.id)).cloned().collect();
        let wires: Vec<Wire> = self
            .sel_w
            .iter()
            .filter_map(|&i| self.circ.wires.get(i).cloned())
            .collect();
        self.clear_sel();
        for c in picked {
            // Devices get a fresh tag; contacts keep following the same device.
            let tag = if c.kind.is_linked() || c.kind.is_terminal() { c.tag.clone() } else { self.circ.next_tag(c.kind) };
            let id = self.circ.add(c.kind, c.pos + d, c.rot, &tag);
            if let Some(n) = self.circ.comp_mut(id) {
                n.preset = c.preset;
                n.color = c.color;
            }
            self.sel.insert(id);
        }
        for w in wires {
            self.circ.wires.push(Wire { a: w.a + d, b: w.b + d });
            self.sel_w.insert(self.circ.wires.len() - 1);
        }
    }

    /// Address used for share links: the page itself on the web, the public site elsewhere.
    fn share_base() -> String {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(h) = web_sys::window().and_then(|w| w.location().href().ok()) {
                return h;
            }
        }
        crate::share::PUBLIC_URL.to_string()
    }

    fn open_share(&mut self) {
        self.share_link = Some(crate::share::link(&self.circ, &Self::share_base()));
        self.link_msg.clear();
    }

    /// Load a circuit received through a share link (a copy: the link never changes).
    pub fn open_shared(&mut self, c: Circuit) {
        self.load_circuit(c, t("Circuito compartilhado", "Shared circuit"));
        self.status = t(
            "Circuito aberto a partir de um link. Suas alterações não mudam o link original.",
            "Circuit opened from a link. Your changes don't alter the original link.",
        )
        .into();
    }

    fn share_windows(&mut self, ctx: &egui::Context) {
        let width = (ctx.screen_rect().width() - 32.0).clamp(260.0, 460.0);
        if let Some(link) = self.share_link.clone() {
            let mut open = true;
            egui::Window::new(format!("🔗 {}", t("Compartilhar circuito", "Share circuit")))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .fixed_size(Vec2::new(width, 0.0))
                .show(ctx, |ui| {
                    ui.label(t(
                        "O circuito inteiro vai dentro deste link. Quem abrir recebe uma cópia para simular e alterar; o link original continua igual.",
                        "The whole circuit travels inside this link. Whoever opens it gets a copy to simulate and edit; the original link never changes.",
                    ));
                    ui.add_space(4.0);
                    let mut shown = link.clone();
                    ui.add(
                        egui::TextEdit::multiline(&mut shown)
                            .desired_rows(4)
                            .desired_width(f32::INFINITY)
                            .font(egui::TextStyle::Monospace),
                    );
                    ui.label(
                        RichText::new(format!("{} {}", link.len(), t("caracteres", "characters")))
                            .small()
                            .color(subtle(ui)),
                    );
                    ui.horizontal(|ui| {
                        if ui.button(format!("📋 {}", t("Copiar link", "Copy link"))).clicked() {
                            ui.ctx().copy_text(link.clone());
                            self.link_msg = t("Link copiado! Cole no WhatsApp, e-mail…", "Link copied! Paste it anywhere.").into();
                        }
                        if ui.button(format!("🌐 {}", t("Abrir", "Open"))).clicked() {
                            ui.ctx().open_url(egui::OpenUrl::new_tab(link.clone()));
                        }
                    });
                    if !self.link_msg.is_empty() {
                        ui.label(RichText::new(&self.link_msg).color(Color32::from_rgb(25, 170, 90)));
                    }
                });
            if !open {
                self.share_link = None;
                self.link_msg.clear();
            }
        }
        if let Some(mut input) = self.link_input.take() {
            let mut open = true;
            let mut loaded = None;
            egui::Window::new(format!("📥 {}", t("Abrir circuito de um link", "Open circuit from a link")))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .fixed_size(Vec2::new(width, 0.0))
                .show(ctx, |ui| {
                    ui.label(t("Cole aqui o link recebido:", "Paste the link you received:"));
                    ui.add(
                        egui::TextEdit::multiline(&mut input)
                            .desired_rows(3)
                            .desired_width(f32::INFINITY)
                            .hint_text("https://…/simueletro/?c=…"),
                    );
                    if ui.button(t("Abrir circuito", "Open circuit")).clicked() {
                        match crate::share::decode_from_url(input.trim()) {
                            Some(c) => loaded = Some(c),
                            None => {
                                self.link_msg =
                                    t("Link inválido ou incompleto.", "Invalid or incomplete link.").into()
                            }
                        }
                    }
                    if !self.link_msg.is_empty() {
                        ui.label(RichText::new(&self.link_msg).color(Color32::from_rgb(220, 70, 60)));
                    }
                });
            if let Some(c) = loaded {
                self.link_msg.clear();
                self.open_shared(c);
            } else if open {
                self.link_input = Some(input);
            } else {
                self.link_msg.clear();
            }
        }
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
                    if tb(ui, "🔗", t("Compartilhar link do circuito", "Share circuit link"), true) {
                        self.open_share();
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
                    // Voltage: only the tabulated values, picked from a list.
                    let choices = crate::circuit::volt_choices(c.kind);
                    if !choices.is_empty() {
                        ui.label(crate::circuit::volt_label(c.kind));
                        let cur = choices.get(c.volt as usize).cloned().unwrap_or_default();
                        egui::ComboBox::from_id_salt("volt")
                            .selected_text(cur)
                            .width(ui.available_width().min(220.0))
                            .show_ui(ui, |ui| {
                                for (i, ch) in choices.iter().enumerate() {
                                    if ui.selectable_label(c.volt as usize == i, ch).clicked() {
                                        c.volt = i as u8;
                                        changed = true;
                                    }
                                }
                            });
                        if c.kind == Kind::Motor6 {
                            ui.label(
                                RichText::new(t(
                                    "Ligue U2-V2-W2 juntos para estrela (Y) ou U1-W2, V1-U2, W1-V2 para triângulo (Δ).",
                                    "Tie U2-V2-W2 together for star (Y), or U1-W2, V1-U2, W1-V2 for delta (Δ).",
                                ))
                                .small()
                                .color(subtle(ui)),
                            );
                        }
                        ui.add_space(2.0);
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
                    .color(subtle(ui)),
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
                .color(subtle(ui)),
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
                    let (state, col) = match c.st.vstat {
                        -1 if c.st.volt > 0.0 => (
                            format!("{} {:.0} V", t("subtensão – não atraca", "under-voltage – no pull-in"), c.st.volt),
                            Color32::from_rgb(214, 130, 0),
                        ),
                        1 => (format!("{} {:.0} V", t("ligado, sobretensão", "on, over-voltage"), c.st.volt), Color32::from_rgb(220, 53, 69)),
                        2 => (t("CA/CC errado", "wrong AC/DC").to_string(), Color32::from_rgb(130, 60, 190)),
                        _ if on => (format!("{} {:.0} V", t("ligado", "on"), c.st.volt), Color32::from_rgb(25, 160, 85)),
                        _ => (t("desligado", "off").to_string(), Color32::GRAY),
                    };
                    rows.push((format!("{}  {}", c.tag, t("contator", "contactor")), state, col, None));
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
                Kind::Motor1 | Kind::Motor3 | Kind::Motor6 => {
                    let (s, mut col) = match c.st.motor {
                        1 => (t("girando ↻", "running ↻"), Color32::from_rgb(0, 110, 230)),
                        -1 => (t("girando ↺", "running ↺"), Color32::from_rgb(0, 110, 230)),
                        _ if c.st.hum => (t("falta de fase!", "missing phase!"), Color32::from_rgb(220, 140, 0)),
                        _ => (t("parado", "stopped"), Color32::GRAY),
                    };
                    let mut s = s.to_string();
                    if c.st.motor != 0 {
                        if c.kind == Kind::Motor6 {
                            s.push_str(match c.st.conn {
                                1 => " Y",
                                2 => " Δ",
                                _ => "",
                            });
                        }
                        s.push_str(&format!(" {:.0} V", c.st.volt));
                        match c.st.vstat {
                            -1 => {
                                s.push_str(t(" – subtensão", " – under-voltage"));
                                col = Color32::from_rgb(214, 130, 0);
                            }
                            1 => {
                                s.push_str(t(" – sobretensão", " – over-voltage"));
                                col = Color32::from_rgb(220, 53, 69);
                            }
                            2 => {
                                s.push_str(t(" – CA/CC errado", " – wrong AC/DC"));
                                col = Color32::from_rgb(130, 60, 190);
                            }
                            _ => {}
                        }
                    }
                    rows.push((c.tag.clone(), s, col, None));
                }
                Kind::Lamp | Kind::Buzzer => {
                    let on = c.st.energized;
                    let (state, col) = if c.st.burnt {
                        (format!("{} ({:.0} V)", t("queimada", "burnt"), c.st.volt), Color32::from_rgb(220, 53, 69))
                    } else {
                        match c.st.vstat {
                            -1 if on => (
                                format!("{} {:.0} V", t("fraca – subtensão", "dim – under-voltage"), c.st.volt),
                                Color32::from_rgb(214, 130, 0),
                            ),
                            2 => (t("CA/CC errado", "wrong AC/DC").to_string(), Color32::from_rgb(130, 60, 190)),
                            _ if on => (format!("{} {:.0} V", t("aceso", "on"), c.st.volt), Color32::from_rgb(25, 160, 85)),
                            _ => (t("apagado", "off").to_string(), Color32::GRAY),
                        }
                    };
                    rows.push((c.tag.clone(), state, col, None));
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
                let pin = if pressed && hovered && !shift { self.pin_near(self.to_world(pos)) } else { None };
                if let Some(p) = pin {
                    // Drag out of a terminal draws a wire (handy on touch screens).
                    self.drag = Drag::WireFrom { start: p };
                } else if pressed && hovered {
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
                    Drag::WireFrom { start } if !down || released => {
                        let end = self.snap(pos);
                        if end != start {
                            self.push_undo();
                            for (a, b) in Self::route(start, end) {
                                self.circ.wires.push(Wire { a, b });
                            }
                            self.status = t("Fio ligado.", "Wire added.").into();
                        } else {
                            // A plain tap on a terminal selects its component.
                            let w = self.to_world(pos);
                            if let Some(id) = self.hit_comp(w) {
                                self.clear_sel();
                                self.sel.insert(id);
                            }
                        }
                        self.drag = Drag::None;
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

    /// Dark endless grid used on phones (SimuRelay style).
    fn paint_dark_grid(&self, painter: &egui::Painter, th: &draw::Theme) {
        let r = self.canvas;
        painter.rect_filled(r, 0.0, th.canvas);
        if !self.grid {
            return;
        }
        let z = self.zoom;
        let step = if z >= 8.0 { 1 } else if z >= 4.0 { 2 } else { 5 };
        let (ax, ay) = self.to_world(r.min);
        let (bx, by) = self.to_world(r.max);
        let line = |painter: &egui::Painter, a: Pos2, b: Pos2, major: bool| {
            let col = if major { th.grid_major } else { th.grid };
            painter.line_segment([a, b], Stroke::new(1.0_f32, col));
        };
        let mut x = (ax.floor() as i32).div_euclid(step) * step;
        while (x as f32) <= bx {
            let sx = self.to_screen(x as f32, 0.0).x;
            line(painter, Pos2::new(sx, r.top()), Pos2::new(sx, r.bottom()), x % 10 == 0);
            x += step;
        }
        let mut y = (ay.floor() as i32).div_euclid(step) * step;
        while (y as f32) <= by {
            let sy = self.to_screen(0.0, y as f32).y;
            line(painter, Pos2::new(r.left(), sy), Pos2::new(r.right(), sy), y % 10 == 0);
            y += step;
        }
    }

    fn paint(&mut self, painter: &egui::Painter) {
        let th = draw::Theme::get(self.mobile);
        if th.dark {
            self.paint_dark_grid(painter, &th);
        } else {
            self.paint_sheet(painter);
        }
        self.paint_circuit(painter, &th);
    }

    /// Light drawing sheet with frame, columns and rows (computer layout).
    fn paint_sheet(&self, painter: &egui::Painter) {
        let r = self.canvas;
        let z = self.zoom;
        painter.rect_filled(r, 0.0, DESK_BG);
        // Sheet with shadow.
        let tl = self.to_screen(0.0, 0.0);
        let br = self.to_screen(SHEET_W, SHEET_H);
        let sheet = Rect::from_min_max(tl, br);
        painter.rect_filled(sheet.translate(Vec2::new(3.0, 4.0)), 4.0, Color32::from_black_alpha(30));
        painter.rect_filled(sheet, 2.0, Color32::WHITE);
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
    }

    fn paint_circuit(&mut self, painter: &egui::Painter, th: &draw::Theme) {
        let r = self.canvas;
        let z = self.zoom;
        let (th_ink, th_dead, th_sel) = (th.ink, th.dead, th.select);
        let sim_on = self.sim.running;
        let time = self.sim.time;
        let flash = ((self.frame / 15) % 2) == 0;
        let act = sim::device_active(&self.circ);

        // Wires.
        let ww = (z * 0.16).clamp(1.0, 3.0);
        for (i, w) in self.circ.wires.iter().enumerate() {
            let a = self.to_screen(w.a.x as f32, w.a.y as f32);
            let b = self.to_screen(w.b.x as f32, w.b.y as f32);
            let mut col = th_ink;
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
                        None => col = th_dead,
                    }
                }
            } else if self.sel_w.contains(&i) {
                col = th_sel;
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
                    self.sim.nets.pot_at(*p).map(draw::pot_color).unwrap_or(th_dead)
                } else {
                    th_ink
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
                    painter.rect_stroke(rr, 4.0, Stroke::new(1.0_f32, th_sel), StrokeKind::Middle);
                }
            }
            let pin_col: Vec<Option<Color32>> = if sim_on {
                c.pins()
                    .iter()
                    .map(|p| {
                        if self.sim.nets.shorted(*p) {
                            Some(Color32::from_rgb(255, 120, 0))
                        } else {
                            Some(self.sim.nets.pot_at(*p).map(draw::pot_color).unwrap_or(th_dead))
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
                ink: if selected { th_sel } else { th_ink },
                act: act_c,
                linked: if c.kind.is_linked() { self.circ.device_kind(&c.tag) } else { None },
                pin_col,
                sim: sim_on,
                time,
                numbers: self.numbers,
                tag: true,
                paper: th.paper,
                num: if selected { th_sel } else { th.num },
                tag_col: if selected { th_sel } else { th.tag },
                dark: th.dark,
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
        // Phone theme: every terminal is a small grey square (SimuRelay style).
        if th.dark && !sim_on {
            let k = (z * 0.28).clamp(2.0, 5.0);
            for c in &self.circ.comps {
                for p in c.pins() {
                    let open = !count.contains_key(&p);
                    let col = if open { Color32::from_rgb(230, 140, 60) } else { Color32::from_rgb(140, 140, 140) };
                    painter.rect_filled(
                        Rect::from_center_size(self.to_screen(p.x as f32, p.y as f32), Vec2::splat(k * 2.0)),
                        1.0,
                        col,
                    );
                }
            }
        }
        // Open pins (edit mode): small rings so the user sees where to connect.
        if !sim_on && !th.dark {
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
                        volt: 0,
                        st: Default::default(),
                    };
                    let mut cx = DrawCtx::plain(Color32::from_rgba_unmultiplied(0, 110, 230, 150), th.dark);
                    cx.tag = true;
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
        if let Drag::WireFrom { start } = self.drag {
            if let Some(mw) = self.mouse_world {
                let p = P::new(mw.0.round() as i32, mw.1.round() as i32);
                for (a, b) in Self::route(start, p) {
                    painter.line_segment(
                        [self.to_screen(a.x as f32, a.y as f32), self.to_screen(b.x as f32, b.y as f32)],
                        Stroke::new(ww * 1.3, Color32::from_rgba_unmultiplied(0, 140, 255, 200)),
                    );
                }
                let ring = if self.is_connection(p) { Color32::from_rgb(25, 190, 100) } else { ACCENT };
                painter.circle_stroke(self.to_screen(p.x as f32, p.y as f32), z * 0.5, Stroke::new(2.0_f32, ring));
            }
        }
        if let Drag::Rubber { start } = self.drag {
            if let Some(mw) = self.mouse_world {
                let rr = Rect::from_two_pos(start, self.to_screen(mw.0, mw.1));
                painter.rect_filled(rr, 0.0, Color32::from_rgba_unmultiplied(0, 122, 255, 18));
                painter.rect_stroke(rr, 0.0, Stroke::new(1.0_f32, th_sel), StrokeKind::Middle);
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
        if self.dark_applied != Some(self.mobile) {
            setup_style(ctx, self.mobile);
            self.dark_applied = Some(self.mobile);
        }
        if self.mobile {
            self.mobile_bars(ctx);
        } else {
            self.sheet = Sheet::None;
            self.lib_open = false;
            self.menu_bar(ctx);
            self.toolbar(ctx);
            self.status_bar(ctx);
            self.palette(ctx);
            self.properties(ctx);
        }
        let bg = draw::Theme::get(self.mobile).canvas;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(bg))
            .show(ctx, |ui| self.canvas_ui(ui));
        if self.mobile {
            self.mobile_overlays(ctx);
        }
        self.share_windows(ctx);
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

// ---------- small widgets ----------

/// Light widgets on the computer, dark ones on phones (SimuRelay style).
fn setup_style(ctx: &egui::Context, dark: bool) {
    ctx.set_visuals(if dark { egui::Visuals::dark() } else { egui::Visuals::light() });
    ctx.style_mut(|s| {
        s.spacing.item_spacing = Vec2::new(8.0, 6.0);
        s.spacing.button_padding = Vec2::new(8.0, 4.0);
        s.visuals.window_corner_radius = CornerRadius::same(10);
        s.visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
        s.visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
        s.visuals.widgets.active.corner_radius = CornerRadius::same(6);
        if dark {
            s.visuals.panel_fill = Color32::from_rgb(28, 28, 28);
            s.visuals.window_fill = Color32::from_rgb(28, 28, 28);
            s.visuals.extreme_bg_color = Color32::from_rgb(16, 16, 16);
            s.visuals.selection.bg_fill = Color32::from_rgb(30, 158, 87);
            s.visuals.selection.stroke = Stroke::new(1.0_f32, Color32::from_rgb(45, 212, 160));
            s.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(40, 40, 40);
            s.visuals.widgets.inactive.bg_fill = Color32::from_rgb(40, 40, 40);
            s.text_styles.insert(egui::TextStyle::Body, FontId::proportional(15.0));
            s.text_styles.insert(egui::TextStyle::Button, FontId::proportional(15.0));
        } else {
            s.visuals.panel_fill = PANEL_BG;
            s.visuals.selection.bg_fill = Color32::from_rgb(210, 228, 255);
            s.visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT);
            s.text_styles.insert(egui::TextStyle::Body, FontId::proportional(13.5));
            s.text_styles.insert(egui::TextStyle::Button, FontId::proportional(13.5));
        }
    });
}

/// Secondary text colour that reads well on both themes.
fn subtle(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(170, 170, 170)
    } else {
        Color32::from_rgb(90, 96, 110)
    }
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
    let dark = ui.visuals().dark_mode;
    egui::Frame::new()
        .fill(if dark { Color32::from_rgb(36, 36, 36) } else { Color32::WHITE })
        .stroke(Stroke::new(
            1.0_f32,
            if dark { Color32::from_rgb(58, 58, 58) } else { Color32::from_rgb(226, 230, 236) },
        ))
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
    symbol_preview(&p, icon, kind, 5.5, false);
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
fn symbol_preview(p: &egui::Painter, icon: Rect, kind: Kind, max_scale: f32, dark: bool) {
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
        volt: 0,
        st: Default::default(),
    };
    let cx = DrawCtx::plain(draw::Theme::get(dark).ink, dark);
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
