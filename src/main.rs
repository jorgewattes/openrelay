#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod circuit;
mod draw;
mod examples;
mod sim;

/// Desktop entry point.
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    use eframe::egui;
    use std::path::PathBuf;

    let args: Vec<String> = std::env::args().collect();
    let mut shot: Option<PathBuf> = None;
    let mut mobile = false;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--mobile" {
            mobile = true;
        }
        if args[i] == "--screenshot" {
            if let Some(p) = args.get(i + 1) {
                shot = Some(PathBuf::from(p));
                i += 1;
            }
        }
        i += 1;
    }
    // `--mobile` opens a phone-sized window with the phone layout.
    let size = if mobile { [412.0, 860.0] } else { [1600.0, 940.0] };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(size)
            .with_min_inner_size([320.0, 480.0])
            .with_title("SimuEletro"),
        ..Default::default()
    };
    eframe::run_native(
        "SimuEletro",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, shot, mobile)))),
    )
}

/// Web entry point (GitHub Pages): draws into the page's canvas.
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    let web_options = eframe::WebOptions::default();
    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window().expect("no window").document().expect("no document");
        let canvas = document
            .get_element_by_id("simueletro_canvas")
            .expect("canvas not found")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("not a canvas");
        let result = eframe::WebRunner::new()
            .start(canvas, web_options, Box::new(|cc| Ok(Box::new(app::App::new(cc, None, false)))))
            .await;
        if let Some(loading) = document.get_element_by_id("loading") {
            match result {
                Ok(_) => loading.remove(),
                Err(e) => {
                    loading.set_inner_html("<p>Não foi possível iniciar o SimuEletro neste navegador.</p>");
                    let _ = e;
                }
            }
        }
    });
}
