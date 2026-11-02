//! The `datacraft` binary: composition root of the desktop shell.
//!
//! Everything below the icon bootstrap and `main` lives in the
//! `datacraft_ui` library so the headless kittest suite can drive the same
//! widgets (step 23.2, item T5).

use eframe::egui;

use datacraft_core::port::StoragePort;
use datacraft_ui::app::App;

/// The composed driver set (ADR-7): the built-in Jet/ACE+SQLite probes
/// plus the out-of-tree CSV directory driver.
fn csv_driver() -> Box<dyn StoragePort> {
    Box::new(datacraft_csv::CsvPort::new())
}

/// Embedded 64 px brand render (assets/app-icon, Crafting Apps office-symbol
/// grammar — see `assets/app-icon/README.md`).
const ICON_PNG: &[u8] = include_bytes!("../../../assets/app-icon/datacraft-64.png");

/// Decodes the embedded brand icon into an egui window icon.
/// Returns `None` on decode failure — the window simply starts without an
/// icon instead of refusing to start.
fn window_icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(ICON_PNG).ok()?.to_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        width,
        height,
        rgba: img.into_raw(),
    })
}

fn main() -> eframe::Result {
    let settings_path = datacraft_ui::settings::default_path();
    let settings = settings_path
        .as_deref()
        .map(datacraft_ui::settings::Settings::load)
        .unwrap_or_default();

    let mut viewport = datacraft_ui::window::viewport(settings.window);
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "DataCraft",
        options,
        Box::new(move |cc| {
            datacraft_ui::theme::fonts::install(&cc.egui_ctx);
            let mut app = App::new(Box::new(
                datacraft_storage::detecting::DetectingPort::new().with_fallback(csv_driver),
            ));
            app.set_settings_path(settings_path);
            app.apply_settings(settings);
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_icon_decodes_to_square_rgba() {
        let icon = window_icon().expect("embedded brand png must decode");
        assert_eq!(icon.width, 64);
        assert_eq!(icon.height, 64);
        assert_eq!(icon.rgba.len(), 64 * 64 * 4);
    }
}
