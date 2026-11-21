//! Docked task-pane host (step 92.2, item M108).
//!
//! The tool surfaces were free-floating `egui::Window`s with no remembered
//! placement (M100's problem); ROADMAP §21 decided an own minimal dock
//! host — no new dependency, craft-libs' `panes`/`docking` stay
//! pattern-only. The host carries a dock position per surface (right /
//! bottom / float), resize within the host, an i18n'd toggle reachable
//! from the pane's title area and its context menu, and per-surface
//! geometry persistence in `ui.json` (ADR-5) with sane values.
//!
//! The SQL window migrates first as the pilot (`sql_window.rs`); the
//! remaining tool surfaces follow per-surface items once the model has
//! proven itself. The host renders the docked panel **before** the
//! central workspace so egui reserves the edge — the workspace can never
//! overlap the docked pane (`panes.sql_panel` probe pins both window
//! sizes).

use eframe::egui;

use crate::app::App;
use crate::i18n::t;
use crate::theme::tokens;

/// Dock position of one task-pane surface. `Float` is the default — the
/// pre-host behavior (a free `egui::Window`) stays unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DockMode {
    /// A free floating window (the default).
    #[default]
    Float,
    /// Docked on the right edge; `size` is the pane width.
    Right,
    /// Docked on the bottom edge above the status bar; `size` is the pane
    /// height.
    Bottom,
}

impl DockMode {
    /// Maps the persisted value; unknown or absent values float.
    pub fn from_value(value: Option<&str>) -> Self {
        match value {
            Some("right") => Self::Right,
            Some("bottom") => Self::Bottom,
            Some("float") => Self::Float,
            _ => Self::Float,
        }
    }

    /// The persisted value.
    pub fn as_value(self) -> &'static str {
        match self {
            Self::Float => "float",
            Self::Right => "right",
            Self::Bottom => "bottom",
        }
    }
}

/// Per-surface pane geometry: the dock mode plus the dock size (width on
/// the right edge, height on the bottom edge). Persisted in `ui.json`
/// under an opaque surface key.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaneGeometry {
    /// Where the pane lives.
    pub mode: DockMode,
    /// Width (right) or height (bottom) in points; unused while floating.
    pub size: f32,
}

impl PaneGeometry {
    /// Dock-width bounds on the right edge.
    pub const RIGHT_MIN: f32 = 160.0;
    /// Upper bound of the right-edge width.
    pub const RIGHT_MAX: f32 = 1200.0;
    /// Default right-edge width.
    pub const RIGHT_DEFAULT: f32 = 320.0;
    /// Dock-height bounds on the bottom edge.
    pub const BOTTOM_MIN: f32 = 96.0;
    /// Upper bound of the bottom-edge height.
    pub const BOTTOM_MAX: f32 = 800.0;
    /// Default bottom-edge height.
    pub const BOTTOM_DEFAULT: f32 = 240.0;

    /// The default geometry of a dock mode.
    pub fn default_for(mode: DockMode) -> Self {
        Self {
            mode,
            size: match mode {
                DockMode::Bottom => Self::BOTTOM_DEFAULT,
                _ => Self::RIGHT_DEFAULT,
            },
        }
    }

    /// Clamps the geometry into the mode's bounds; a broken size takes
    /// the mode default (the window-geometry rescue pattern, U13).
    pub fn sane(self) -> Self {
        let size = match self.mode {
            DockMode::Right => clamp(
                self.size,
                Self::RIGHT_MIN,
                Self::RIGHT_MAX,
                Self::RIGHT_DEFAULT,
            ),
            DockMode::Bottom => clamp(
                self.size,
                Self::BOTTOM_MIN,
                Self::BOTTOM_MAX,
                Self::BOTTOM_DEFAULT,
            ),
            // Unused while floating; the recorded value is irrelevant.
            DockMode::Float => self.size,
        };
        Self {
            mode: self.mode,
            size,
        }
    }
}

/// A finite size clamped into `[min, max]`; non-finite values take the
/// default.
fn clamp(size: f32, min: f32, max: f32, default: f32) -> f32 {
    if size.is_finite() {
        size.clamp(min, max)
    } else {
        default
    }
}

impl App {
    /// The stored geometry of one pane surface; a surface without a
    /// record floats.
    pub fn pane_geometry(&self, surface: &str) -> PaneGeometry {
        self.settings
            .panes
            .get(surface)
            .copied()
            .unwrap_or_else(|| PaneGeometry::default_for(DockMode::Float))
    }

    /// Switches a pane surface's dock mode; the geometry resets to the
    /// new mode's default (a previous dock size of another edge has no
    /// meaning).
    pub fn set_pane_mode(&mut self, surface: &str, mode: DockMode) {
        if self.pane_geometry(surface).mode == mode {
            return;
        }
        self.settings
            .panes
            .insert(surface.to_owned(), PaneGeometry::default_for(mode).sane());
    }

    /// Records the pane's live dock size (called every frame the panel
    /// renders; resize drags update the value continuously). Saves ride
    /// the settings persistence.
    pub fn set_pane_size(&mut self, surface: &str, size: f32) {
        if self.pane_geometry(surface).mode == DockMode::Float {
            return;
        }
        let mut geometry = self.pane_geometry(surface);
        geometry.size = size;
        self.settings
            .panes
            .insert(surface.to_owned(), geometry.sane());
    }

    /// The dock-mode entries shared by the header menu and the header's
    /// context menu: dock right, dock bottom, float — the current mode
    /// renders disabled.
    pub(crate) fn pane_dock_entries(&mut self, ui: &mut egui::Ui, surface: &str) {
        let mode = self.pane_geometry(surface).mode;
        for (candidate, label) in [
            (DockMode::Right, t("Dock Right")),
            (DockMode::Bottom, t("Dock Bottom")),
            (DockMode::Float, t("Float")),
        ] {
            if ui
                .add_enabled(mode != candidate, egui::Button::new(label))
                .clicked()
            {
                self.set_pane_mode(surface, candidate);
                ui.close();
            }
        }
    }
}

/// The static identity of one pane surface: the settings key, the pane
/// title and the layout-probe id of the docked panel.
#[derive(Debug, Clone, Copy)]
pub struct PaneSpec {
    /// The opaque surface id (the `ui.json` panes key, e.g. `"sql"`).
    pub surface: &'static str,
    /// The pane title, localized at render.
    pub title: &'static str,
    /// The layout-probe id the docked panel records its rect under.
    pub probe: &'static str,
}

/// Renders the docked panel of one pane surface while it is open and
/// docked; returns whether the panel claimed the surface (the floating
/// window skips then). Declared **before** the central workspace so egui
/// reserves the edge — the workspace cannot overlap the docked pane. The
/// surface's close intent travels through the bus via `close` (every
/// user action is one command).
pub fn pane_panel(
    app: &mut App,
    ui: &mut egui::Ui,
    spec: PaneSpec,
    open: bool,
    content: impl FnOnce(&mut App, &mut egui::Ui),
    close: impl FnOnce(&mut App),
) -> bool {
    if !open {
        return false;
    }
    let surface = spec.surface;
    let geometry = app.pane_geometry(surface);
    let chrome = tokens(ui.visuals());
    let id = egui::Id::unique(format!("datacraft.pane.{surface}"));
    let panel = match geometry.mode {
        DockMode::Float => return false,
        DockMode::Right => egui::Panel::right(id)
            .resizable(true)
            .size_range(PaneGeometry::RIGHT_MIN..=PaneGeometry::RIGHT_MAX)
            .default_size(geometry.size),
        DockMode::Bottom => egui::Panel::bottom(id)
            .resizable(true)
            .size_range(PaneGeometry::BOTTOM_MIN..=PaneGeometry::BOTTOM_MAX)
            .default_size(geometry.size),
    }
    .frame(egui::Frame::side_top_panel(ui.style()).fill(chrome.panel));
    let mut close_requested = false;
    // Reborrow: the closures borrow the app, the capture after the pass
    // needs it again.
    let response = panel.show(ui, |ui| {
        close_requested |= pane_header(&mut *app, ui, surface, spec.title);
        content(&mut *app, ui);
    });
    if close_requested {
        close(app);
    }
    let rect = response.response.rect;
    crate::layout_probe::record(spec.probe, rect);
    let size = match geometry.mode {
        DockMode::Bottom => rect.height(),
        _ => rect.width(),
    };
    app.set_pane_size(surface, size);
    true
}

/// The pane header row: title, the dock menu, close — the dock entries
/// also ride the header's context menu. Returns the close intent.
fn pane_header(app: &mut App, ui: &mut egui::Ui, surface: &str, title: &'static str) -> bool {
    let mut close = false;
    let row = ui
        .horizontal(|ui| {
            // The title is the English key; the catalog resolves it (the
            // pilot's "SQL" hid this — "Relationships" needs it, M125).
            ui.strong(t(title));
            dock_menu(&mut *app, ui, surface);
            if ui.small_button(t("Close")).clicked() {
                close = true;
            }
        })
        .response;
    row.context_menu(|ui| app.pane_dock_entries(ui, surface));
    close
}

/// The dock menu over the pane entries (the floating window hosts it in
/// its header row, the docked pane in the host header).
pub(crate) fn dock_menu(app: &mut App, ui: &mut egui::Ui, surface: &str) {
    ui.menu_button(t("Dock"), |ui| app.pane_dock_entries(ui, surface));
}
