//! The recent-databases list (step 28.2, item U15; Excel-style rows with a
//! "Modified" stamp, step 38.2, item M41). One `OpenDatabase` per click;
//! missing paths were pruned on load with a startup notice.

use eframe::egui;
use eframe::egui::Sense;

use datacraft_core::command::Command;

use crate::app::App;
use crate::i18n::t;
use crate::icons;
use crate::theme::{fonts, tokens};

/// The bounded recent list: one `OpenDatabase` per click; each row carries a
/// localized "Modified" timestamp right-aligned (step 38.2, item M41).
pub(super) fn recent_databases_ui(app: &App, ui: &mut egui::Ui, intents: &mut Vec<Command>) {
    let entries = app.recent_databases();
    if entries.is_empty() {
        return;
    }
    ui.add_space(10.0);
    ui.heading(t("Recent Databases"));
    for entry in entries {
        let name = entry.path.file_name().map_or_else(
            || entry.path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let stamp = entry
            .modified
            .map(|secs| format!("{} {}", t("Modified"), iso_date_from_unix(secs)));
        let access = match &stamp {
            Some(stamp) => format!("{name} — {} — {stamp}", entry.path.display()),
            None => format!("{name} — {}", entry.path.display()),
        };
        let path = entry.path.clone();
        let clicked = ui
            .push_id(format!("backstage.recent.{}", entry.path.display()), |ui| {
                recent_row(
                    ui,
                    &name,
                    &entry.path.display().to_string(),
                    stamp.as_deref(),
                    &access,
                )
            })
            .inner;
        if clicked {
            intents.push(Command::OpenDatabase {
                path,
                password: None,
            });
        }
    }
}

/// One painted recent row: cylinder glyph, name over path, the stamp
/// right-aligned; hover uses the shared token so both palettes stay neutral.
fn recent_row(
    ui: &mut egui::Ui,
    name: &str,
    path: &str,
    stamp: Option<&str>,
    access: &str,
) -> bool {
    let tokens = tokens(ui.visuals());
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 36.0), Sense::hover());
    let response = ui.interact(rect, ui.scope_id().with(("recent", name)), Sense::click());
    let clicked = response.clicked();
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, access.to_owned()));
    if response.hovered() {
        ui.painter().rect_filled(rect, 2.0, tokens.hover);
    }
    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 20.0, rect.center().y),
        egui::vec2(icons::ICON_SIZE, icons::ICON_SIZE),
    );
    icons::paint(
        ui.painter(),
        icon_rect,
        "database",
        tokens.text_muted,
        tokens.accent,
    );
    ui.painter().text(
        egui::pos2(rect.left() + 38.0, rect.min.y + 11.0),
        egui::Align2::LEFT_CENTER,
        name,
        egui::FontId::proportional(fonts::BODY_SIZE),
        tokens.text,
    );
    ui.painter().text(
        egui::pos2(rect.left() + 38.0, rect.max.y - 9.0),
        egui::Align2::LEFT_CENTER,
        path,
        egui::FontId::proportional(fonts::SMALL_SIZE),
        tokens.text_muted,
    );
    if let Some(stamp) = stamp {
        ui.painter().text(
            egui::pos2(rect.right() - 8.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            stamp,
            egui::FontId::proportional(fonts::SMALL_SIZE),
            tokens.text_muted,
        );
    }
    clicked
}

/// Renders unix seconds as `YYYY-MM-DD` (proleptic Gregorian, UTC) without a
/// date dependency — the civil-from-days conversion (Hinnant 2012), pinned by
/// tests in this module and in `settings`.
pub(crate) fn iso_date_from_unix(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::iso_date_from_unix;

    /// The stamp is a deterministic ISO date, epoch inclusive.
    #[test]
    fn unix_seconds_render_as_an_iso_date() {
        assert_eq!(iso_date_from_unix(0), "1970-01-01");
        assert_eq!(iso_date_from_unix(951_782_400), "2000-02-29");
        assert_eq!(iso_date_from_unix(1_793_664_000), "2026-11-03");
    }
}
