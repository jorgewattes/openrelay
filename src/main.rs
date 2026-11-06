#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod circuit;
mod draw;
mod examples;
mod share;
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

/// Shows a readable error on the web page (see `simuShowError` in index.html).
#[cfg(target_arch = "wasm32")]
fn show_web_error(msg: &str) {
    use eframe::wasm_bindgen::{JsCast as _, JsValue};
    let Some(win) = web_sys::window() else { return };
    if let Ok(f) = js_sys::Reflect::get(&win, &JsValue::from_str("simuShowError")) {
        if let Some(f) = f.dyn_ref::<js_sys::Function>() {
            let _ = f.call1(&JsValue::NULL, &JsValue::from_str(msg));
        }
    }
}

/// Web entry point (GitHub Pages): draws into the page's canvas.
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    // A crash shows its reason on the page instead of a frozen screen.
    std::panic::set_hook(Box::new(|info| {
        show_web_error(&format!("Erro interno: {info}"));
    }));

    let web_options = eframe::WebOptions::default();
    wasm_bindgen_futures::spawn_local(async {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let Some(canvas) = document
            .get_element_by_id("simueletro_canvas")
            .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
        else {
            show_web_error("Área de desenho não encontrada na página.");
            return;
        };
        // A circuit shared by link (…/?c=…) opens directly.
        let shared = web_sys::window()
            .and_then(|w| w.location().href().ok())
            .and_then(|href| share::decode_from_url(&href));
        let result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(move |cc| {
                    let mut a = app::App::new(cc, None, false);
                    if let Some(c) = shared {
                        a.open_shared(c);
                    }
                    Ok(Box::new(a))
                }),
            )
            .await;
        match result {
            Ok(_) => {
                if let Some(loading) = document.get_element_by_id("loading") {
                    loading.remove();
                }
            }
            Err(e) => show_web_error(&format!("Falha ao iniciar os gráficos (WebGL): {e:?}")),
        }
    });
}
