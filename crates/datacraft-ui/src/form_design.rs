//! Form design canvas (steps 8.1/11.6, items H28/H29).
//!
//! Renders the form frame, the snap grid and the placed controls from the
//! snapshot, beside the designer palette (record source, field list and
//! toolbox). Pure paint; drags and palette clicks dispatch one command.

use eframe::egui;

use datacraft_core::ObjectKind;
use datacraft_core::canvas::Rect;
use datacraft_core::command::Command;
use datacraft_core::form::FormDef;
use datacraft_core::snapshot::TabSnapshot;

use crate::app::App;
use crate::i18n::{plural, t, tr};

/// Form canvas width in points.
pub(crate) const FORM_WIDTH: f32 = 640.0;
/// Form canvas height in points.
pub(crate) const FORM_HEIGHT: f32 = 400.0;

/// Renders the design view of one form tab: canvas plus palette.
pub fn form_design_ui(app: &mut App, tab: &TabSnapshot, ui: &mut egui::Ui) {
    let Some(form) = app
        .snapshot()
        .database
        .as_ref()
        .and_then(|db| db.forms.iter().find(|f| f.name == tab.object_name))
        .cloned()
    else {
        ui.label(t("Form no longer exists"));
        return;
    };

    ui.horizontal_top(|ui| {
        // Cap the canvas column at the canvas width: egui separators span
        // the full available width, and an uncapped column would push
        // the palette column past the viewport (F10, found in step 41.4).
        ui.vertical(|ui| {
            ui.set_max_width(FORM_WIDTH);
            // The canvas column scrolls both axes (step 91.10, item M122):
            // the fixed 640-wide canvas stays reachable at narrow widths.
            egui::ScrollArea::both()
                .id_salt(format!("form.design.{}", tab.ui_test_id))
                .show(ui, |ui| {
                    let canvas = canvas_ui(app, tab, &form, ui);
                    crate::layout_probe::record("form.design.canvas", canvas);
                });
            // The viewport rect rides the layout probe: narrower than the
            // canvas at the pinned minimum — the scroll is the
            // reachability.
            crate::layout_probe::record("form.design.viewport", ui.max_rect());
        });
        ui.separator();
        ui.vertical(|ui| {
            crate::designer_palette::designer_palette_ui(app, ObjectKind::Form, &form.name, ui);
            // The selected control's content (step 63.2, item H56).
            let mut apply = |app: &mut App, control: &str, value: &str| {
                app.dispatch(Command::SetFormControlContent {
                    form: form.name.clone(),
                    control: control.to_owned(),
                    value: value.to_owned(),
                });
            };
            crate::control_content::control_content_ui(
                app,
                tab.id,
                app.form_selection(tab.id),
                &form.controls,
                false,
                &mut apply,
                ui,
            );
            // The selected control's kind-specific data (phase 64).
            if let Some(selected) = app.form_selection(tab.id)
                && let Some(control) = form.controls.iter().find(|c| c.name == selected)
            {
                crate::combo_source::combo_source_ui(app, tab.id, &form.name, control, ui);
                crate::button_action::button_action_ui(app, tab.id, &form.name, control, ui);
            }
            // The full property sheet (phase 102, item M133).
            let selection = app.form_selection(tab.id);
            crate::control_properties::control_properties_ui(
                app,
                tab.id,
                ObjectKind::Form,
                &form.name,
                &form.controls,
                selection,
                ui,
            );
        });
    });
}

/// The design canvas itself (frame, grid, controls, footer).
fn canvas_ui(app: &mut App, tab: &TabSnapshot, form: &FormDef, ui: &mut egui::Ui) -> egui::Rect {
    ui.heading(egui::RichText::new(tr(
        "Form: {name}",
        &[("name", &form.name)],
    )));
    ui.label(
        egui::RichText::new(tr(
            "Snap grid: {grid} px",
            &[("grid", &form.grid.to_string())],
        ))
        .small(),
    );
    ui.separator();

    let (canvas_rect, _) =
        ui.allocate_exact_size(egui::vec2(FORM_WIDTH, FORM_HEIGHT), egui::Sense::hover());
    let painter = ui.painter_at(canvas_rect);

    // Form surface.
    painter.rect_filled(canvas_rect, 0.0, ui.visuals().extreme_bg_color);
    painter.rect_stroke(
        canvas_rect,
        0.0,
        ui.visuals().widgets.noninteractive.fg_stroke,
        egui::StrokeKind::Inside,
    );

    // Snap grid dots.
    let grid = form.grid;
    let mut x = grid;
    while x < FORM_WIDTH {
        let mut y = grid;
        while y < FORM_HEIGHT {
            painter.circle_filled(
                canvas_rect.min + egui::vec2(x, y),
                1.0,
                ui.visuals().weak_text_color(),
            );
            y += grid;
        }
        x += grid;
    }

    // Controls paint in tab order (later = on top); drag accumulates the
    // delta and dispatches one snapped move per gesture.
    let mut selected: Option<String> = None;
    let mut moved: Option<(String, Rect)> = None;
    for control in &form.controls {
        let rect = egui::Rect::from_min_size(
            canvas_rect.min + egui::vec2(control.rect.x, control.rect.y),
            egui::vec2(control.rect.width, control.rect.height),
        );
        let stroke = ui.visuals().widgets.noninteractive.fg_stroke;
        let is_selected = app.form_selection(tab.id).as_deref() == Some(control.name.as_str());
        ui.push_id(
            format!(
                "form.{}.control.{}",
                form.name.to_lowercase(),
                control.name.to_lowercase()
            ),
            |ui| {
                let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());
                paint_control(ui, &painter, rect, control, stroke, is_selected);
                if response.clicked() {
                    selected = Some(control.name.clone());
                }
                if response.dragged() {
                    let delta = response.drag_delta();
                    app.accumulate_drag(tab.id, &control.name, delta.x, delta.y);
                }
                if response.drag_stopped()
                    && let Some((dx, dy)) = app.finish_drag(tab.id, &control.name)
                {
                    moved = Some((
                        control.name.clone(),
                        Rect {
                            x: control.rect.x + dx,
                            y: control.rect.y + dy,
                            width: control.rect.width,
                            height: control.rect.height,
                        },
                    ));
                }
            },
        );
    }
    if let Some(name) = selected {
        app.set_form_selection(tab.id, name);
    }
    if let Some((name, rect)) = moved {
        app.dispatch(Command::SetFormControlRect {
            form: form.name.clone(),
            control: name,
            rect,
        });
    }
    // The selected control's resize handles (phase 102, step 102.2): the
    // gesture yields one snapped, bounds-clamped rect command.
    let mut resized: Option<(String, Rect)> = None;
    if let Some(name) = app.form_selection(tab.id)
        && let Some(control) = form.controls.iter().find(|control| control.name == name)
        && let Some(rect) = crate::design_handles::resize_handles_ui(
            app,
            tab.id,
            control,
            canvas_rect,
            form.grid,
            (FORM_WIDTH, FORM_HEIGHT),
            &painter,
            ui,
        )
    {
        resized = Some((control.name.clone(), rect));
    }
    if let Some((name, rect)) = resized {
        app.dispatch(Command::SetFormControlRect {
            form: form.name.clone(),
            control: name,
            rect,
        });
    }

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(plural(
                form.controls.len(),
                "{count} control",
                "{count} controls",
            ))
            .small(),
        );
        if let Some(name) = app.form_selection(tab.id)
            && ui
                .small_button(tr("Delete {name}", &[("name", &name)]))
                .clicked()
        {
            app.dispatch(Command::DeleteFormControl {
                form: form.name.clone(),
                control: name,
            });
        }
    });

    canvas_rect
}

/// Kind-specific painting (pure paint; no logic). Shared with the
/// report design view (same palette, step 10.4). The phase-102 control
/// properties drive the result: a hidden control paints a selectable
/// placeholder, a disabled one weak text, bold and alignment apply to
/// every text the control renders.
pub(crate) fn paint_control(
    ui: &egui::Ui,
    painter: &egui::Painter,
    rect: egui::Rect,
    control: &datacraft_core::form::ControlDef,
    stroke: egui::Stroke,
    selected: bool,
) {
    let properties = control.properties;
    if !properties.visible {
        paint_hidden(ui, painter, rect, control);
        paint_selection(ui, painter, rect, selected);
        return;
    }
    let weak = ui.visuals().weak_text_color();
    let color = if properties.enabled {
        stroke.color
    } else {
        weak
    };
    let font = std::rc::Rc::new(control_font(ui, properties.bold));
    let align = properties.align;
    match &control.kind {
        datacraft_core::form::ControlKind::Label { text } => {
            let (pos, anchor) = text_anchor(rect, align, false);
            painter.text(pos, anchor, text, (*font).clone(), color);
        }
        datacraft_core::form::ControlKind::TextBox { bound_field } => {
            painter.rect_filled(rect, 2.0, ui.visuals().extreme_bg_color);
            painter.rect_stroke(rect, 2.0, stroke, egui::StrokeKind::Inside);
            if let Some(field) = bound_field {
                let (pos, anchor) = text_anchor(rect, align, true);
                painter.text(pos, anchor, format!("{{{field}}}"), (*font).clone(), weak);
            }
        }
        datacraft_core::form::ControlKind::CheckBox { bound_field } => {
            let box_rect = egui::Rect::from_center_size(
                rect.left_center() + egui::vec2(8.0, 0.0),
                egui::vec2(12.0, 12.0),
            );
            painter.rect_stroke(box_rect, 2.0, stroke, egui::StrokeKind::Inside);
            if let Some(field) = bound_field {
                painter.text(
                    box_rect.right_center() + egui::vec2(4.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    field,
                    (*font).clone(),
                    color,
                );
            }
        }
        datacraft_core::form::ControlKind::ComboBox { bound_field, .. } => {
            painter.rect_filled(rect, 2.0, ui.visuals().extreme_bg_color);
            painter.rect_stroke(rect, 2.0, stroke, egui::StrokeKind::Inside);
            // A painted caret, never a text glyph: the bundled fonts have
            // no U+25BE (tofu panics under the kittest font policy).
            let caret = rect.right_center() - egui::vec2(10.0, 0.0);
            painter.add(egui::Shape::convex_polygon(
                vec![
                    caret + egui::vec2(-4.0, -2.0),
                    caret + egui::vec2(4.0, -2.0),
                    caret + egui::vec2(0.0, 3.0),
                ],
                stroke.color,
                egui::Stroke::NONE,
            ));
            if let Some(field) = bound_field {
                let (pos, anchor) = text_anchor(rect, align, true);
                painter.text(pos, anchor, format!("{{{field}}}"), (*font).clone(), weak);
            }
        }
        datacraft_core::form::ControlKind::Button { caption, .. } => {
            painter.rect_filled(rect, 4.0, ui.visuals().widgets.inactive.bg_fill);
            painter.rect_stroke(rect, 4.0, stroke, egui::StrokeKind::Inside);
            let (pos, anchor) = text_anchor(rect, align, true);
            painter.text(pos, anchor, caption, (*font).clone(), color);
        }
        datacraft_core::form::ControlKind::SubForm { form } => {
            painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);
            let (pos, anchor) = text_anchor(rect, align, false);
            painter.text(
                pos,
                anchor,
                tr("subform: {form}", &[("form", form)]),
                (*font).clone(),
                weak,
            );
        }
    }
    paint_selection(ui, painter, rect, selected);
}

/// A hidden control stays selectable in the designer: a faint outline and
/// the control name, nothing a runtime or print would show.
fn paint_hidden(
    ui: &egui::Ui,
    painter: &egui::Painter,
    rect: egui::Rect,
    control: &datacraft_core::form::ControlDef,
) {
    let weak = egui::Stroke::new(1.0, ui.visuals().weak_text_color());
    painter.rect_stroke(rect, 2.0, weak, egui::StrokeKind::Inside);
    let font = egui::TextStyle::Small.resolve(ui.style());
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        &control.name,
        font,
        ui.visuals().weak_text_color(),
    );
}

/// The selection ring paints the accent family (step 91.10, item M120):
/// the selection wash is nearly invisible on the dark form surface; the
/// accent keeps the ring legible in both palettes.
fn paint_selection(ui: &egui::Ui, painter: &egui::Painter, rect: egui::Rect, selected: bool) {
    if selected {
        painter.rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(2.0, crate::theme::tokens(ui.visuals()).accent),
            egui::StrokeKind::Outside,
        );
    }
}

/// The body rung in the requested weight (phase 102): bold uses the
/// bundled SemiBold face, whose glyph chain falls back to Inter Regular.
fn control_font(ui: &egui::Ui, bold: bool) -> egui::FontId {
    let body = egui::TextStyle::Body.resolve(ui.style());
    if bold {
        egui::FontId::new(
            body.size,
            egui::FontFamily::Name(crate::theme::fonts::FAMILY_SEMIBOLD.into()),
        )
    } else {
        body
    }
}

/// Text anchor inside a control rect (phase 102): the alignment property
/// decides the horizontal edge, `center` the vertical one.
fn text_anchor(
    rect: egui::Rect,
    align: datacraft_core::form::TextAlign,
    center: bool,
) -> (egui::Pos2, egui::Align2) {
    use datacraft_core::form::TextAlign;
    let pad = egui::vec2(2.0, 2.0);
    match (align, center) {
        (TextAlign::Left, false) => (rect.left_top() + pad, egui::Align2::LEFT_TOP),
        (TextAlign::Left, true) => (
            rect.left_center() + egui::vec2(pad.x, 0.0),
            egui::Align2::LEFT_CENTER,
        ),
        (TextAlign::Center, false) => (
            rect.center_top() + egui::vec2(0.0, pad.y),
            egui::Align2::CENTER_TOP,
        ),
        (TextAlign::Center, true) => (rect.center(), egui::Align2::CENTER_CENTER),
        (TextAlign::Right, false) => (
            rect.right_top() + egui::vec2(-pad.x, pad.y),
            egui::Align2::RIGHT_TOP,
        ),
        (TextAlign::Right, true) => (
            rect.right_center() - egui::vec2(pad.x, 0.0),
            egui::Align2::RIGHT_CENTER,
        ),
    }
}
