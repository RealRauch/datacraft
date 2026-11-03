//! The backstage sidebar (step 38.2, item M41): a full-height accent column
//! in the Excel grammar — the back-to-workspace row, icon+label navigation,
//! and the pinned Options entry at the bottom. Rows are painted like the
//! datasheet cells and exposed as AccessKit labels so the headless kittest
//! drives them like every other control.
//!
//! Palette contract: the sidebar surface is `accent_fill` (the deeper brand
//! teal) with `on_accent` rows; the selected row inverts the app-icon grammar
//! (`accent_fill` on `on_accent`) with a 3 px accent bar — both pairs are
//! contrast-pinned in `theme.rs`.

use eframe::egui;
use eframe::egui::{Color32, Rect, Sense};

use crate::app::App;
use crate::backstage::BackstagePage;
use crate::i18n::t;
use crate::icons;
use crate::theme::{fonts, tokens};

/// Row geometry of the sidebar entries.
const ROW_HEIGHT: f32 = 30.0;
const ROW_INSET: f32 = 14.0;
const ACTIVE_BAR: f32 = 3.0;

/// The sidebar column: back to the workspace, the main pages, the pinned
/// Options entry at the bottom.
pub(super) fn nav_ui(app: &mut App, ui: &mut egui::Ui) {
    ui.add_space(10.0);
    nav_row(app, ui, None);
    ui.add_space(14.0);
    for page in BackstagePage::ALL {
        if page.pinned() {
            continue;
        }
        nav_row(app, ui, Some(page));
    }
    let pinned = ui.available_rect_before_wrap();
    ui.scope_builder(egui::UiBuilder::new().max_rect(pinned), |ui| {
        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.add_space(10.0);
            nav_row(app, ui, Some(BackstagePage::Options));
        });
    });
}

/// One painted sidebar row; `page == None` is the back-to-workspace row.
fn nav_row(app: &mut App, ui: &mut egui::Ui, page: Option<BackstagePage>) {
    let (label, glyph, slug, selected) = match page {
        Some(page) => (
            t(page.label()),
            page.glyph(),
            page.slug(),
            app.backstage_page == page,
        ),
        None => (t("Back"), "back", "back", false),
    };
    let tokens = tokens(ui.visuals());
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), ROW_HEIGHT),
        egui::Sense::hover(),
    );
    let id = ui.scope_id().with(("backstage.nav", slug));
    let response = ui.interact(rect, id, Sense::click());
    let clicked = response.clicked();
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label.to_owned()));
    if clicked {
        match page {
            Some(page) => app.backstage_page = page,
            None => app.active_ribbon_tab = "home",
        }
    }
    let text_color = if selected {
        tokens.accent_fill
    } else {
        tokens.on_accent
    };
    let fill = if selected {
        tokens.on_accent
    } else if response.hovered() {
        hover_wash(tokens.on_accent)
    } else {
        Color32::TRANSPARENT
    };
    if fill != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, 2.0, fill);
    }
    if selected {
        ui.painter().rect_filled(
            Rect::from_min_size(rect.min, egui::vec2(ACTIVE_BAR, rect.height())),
            1.0,
            tokens.accent,
        );
    }
    let icon_rect = Rect::from_center_size(
        egui::pos2(rect.left() + ROW_INSET + 8.0, rect.center().y),
        egui::vec2(icons::ICON_SIZE, icons::ICON_SIZE),
    );
    icons::paint(ui.painter(), icon_rect, glyph, text_color, text_color);
    ui.painter().text(
        egui::pos2(rect.left() + ROW_INSET + 30.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(fonts::BODY_SIZE),
        text_color,
    );
}

/// A translucent wash of `base` for hovered rows on the accent field.
fn hover_wash(base: Color32) -> Color32 {
    Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), 50)
}
