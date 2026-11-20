//! Docked task-pane host (step 92.2, item M108).
//!
//! The tool surfaces were free-floating `egui::Window`s with no remembered
//! placement (M100's problem); ROADMAP §21 decided an own minimal dock
//! host — no new dependency, craft-libs' `panes`/`docking` stay
//! pattern-only. The host carries a dock position per surface (right /
//! bottom / float), resize within the host, an i18n'd toggle reachable
//! from the pane's title area and its context menu, and per-surface
//! geometry persistence in `ui.json` (ADR-5) with sane/rescued values.
//!
//! The SQL window migrates first as the pilot (`sql_window.rs`); the
//! remaining tool surfaces follow per-surface items once the model has
//! proven itself. The host renders the docked panel **before** the
//! central workspace so egui reserves the edge — the workspace can never
//! overlap the docked pane (`panes.sql_panel` probe pins both window
//! sizes).

#[cfg(test)]
mod tests {
    use super::{DockMode, PaneGeometry};

    /// The persisted mode strings round-trip; unknown or absent values
    /// fall back to the floating default (step 92.2, item M108).
    #[test]
    fn dock_mode_values_round_trip_and_unknowns_float() {
        assert_eq!(DockMode::from_value(Some("right")), DockMode::Right);
        assert_eq!(DockMode::from_value(Some("bottom")), DockMode::Bottom);
        assert_eq!(DockMode::from_value(Some("float")), DockMode::Float);
        assert_eq!(DockMode::from_value(None), DockMode::Float);
        assert_eq!(DockMode::from_value(Some("sideways")), DockMode::Float);
        for mode in [DockMode::Float, DockMode::Right, DockMode::Bottom] {
            assert_eq!(DockMode::from_value(Some(mode.as_value())), mode);
        }
    }

    /// Sane geometry clamps the dock size into the mode's bounds; broken
    /// values fall back to the mode default (the window-geometry rescue
    /// pattern, step 92.2, item M108).
    #[test]
    fn pane_geometry_sane_clamps_dock_sizes() {
        let narrow = PaneGeometry {
            mode: DockMode::Right,
            size: 40.0,
        }
        .sane();
        assert!(narrow.size >= PaneGeometry::RIGHT_MIN, "{narrow:?}");
        let wide = PaneGeometry {
            mode: DockMode::Right,
            size: 5000.0,
        }
        .sane();
        assert!(wide.size <= PaneGeometry::RIGHT_MAX, "{wide:?}");
        let flat = PaneGeometry {
            mode: DockMode::Bottom,
            size: 10.0,
        }
        .sane();
        assert!(flat.size >= PaneGeometry::BOTTOM_MIN, "{flat:?}");
        let tall = PaneGeometry {
            mode: DockMode::Bottom,
            size: 2000.0,
        }
        .sane();
        assert!(tall.size <= PaneGeometry::BOTTOM_MAX, "{tall:?}");
        let broken = PaneGeometry {
            mode: DockMode::Bottom,
            size: f32::NAN,
        }
        .sane();
        assert_eq!(broken.size, PaneGeometry::BOTTOM_DEFAULT, "{broken:?}");
        let float = PaneGeometry {
            mode: DockMode::Float,
            size: 99.0,
        }
        .sane();
        assert_eq!(float.mode, DockMode::Float, "{float:?}");
    }
}
