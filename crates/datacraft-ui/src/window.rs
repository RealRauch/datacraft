//! Window geometry persistence (step 22.2, item U13; ADR-5).
//!
//! `ui.json` stores the inner size, the outer position and the maximized
//! state. Values are sanitized on load (implausible geometry is dropped) and
//! a saved position that would land off-screen is rescued onto the visible
//! monitor — the model's `WindowGeometry::sanitized`/`rescue_position`
//! pattern (`crates/ui-egui/src/window_geometry.rs` @ `fd7f662`).

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui;

    #[test]
    fn geometry_round_trips_position_and_maximized() {
        let geometry = WindowGeometry {
            width: 1200.0,
            height: 800.0,
            x: Some(120.0),
            y: Some(80.0),
            maximized: true,
        };
        let json = serde_json::to_string(&geometry).expect("serialize");
        let loaded: WindowGeometry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(loaded, geometry);
    }

    /// Older files only carry the inner size; they load without position or
    /// maximized state instead of failing.
    #[test]
    fn legacy_files_load_without_position_or_maximized() {
        let loaded: WindowGeometry =
            serde_json::from_str(r#"{"width":1000.0,"height":700.0}"#).expect("deserialize");
        assert_eq!(loaded.x, None);
        assert_eq!(loaded.y, None);
        assert!(!loaded.maximized);
    }

    #[test]
    fn implausible_geometry_is_dropped() {
        for (width, height) in [
            (0.0, 700.0),
            (1280.0, -1.0),
            (f32::NAN, 700.0),
            (f32::INFINITY, 700.0),
        ] {
            assert!(
                WindowGeometry {
                    width,
                    height,
                    ..WindowGeometry::default()
                }
                .sane()
                .is_none(),
                "size {width}x{height} must be dropped"
            );
        }

        // A partial or absurd position is dropped; the size survives.
        for (x, y) in [(Some(10.0), None), (Some(50_000.0), Some(0.0))] {
            let sane = WindowGeometry {
                width: 1000.0,
                height: 700.0,
                x,
                y,
                maximized: false,
            }
            .sane()
            .expect("size survives");
            assert_eq!((sane.x, sane.y), (None, None), "position {x:?}/{y:?}");
        }
    }

    #[test]
    fn off_screen_positions_are_rescued_onto_the_monitor() {
        let size = egui::vec2(1200.0, 800.0);
        let monitor = egui::vec2(1920.0, 1080.0);
        assert_eq!(
            rescued_position(egui::pos2(100.0, 50.0), size, monitor),
            egui::pos2(100.0, 50.0),
            "a visible position stays unchanged"
        );
        assert_eq!(
            rescued_position(egui::pos2(-5000.0, -5000.0), size, monitor),
            egui::pos2(0.0, 0.0)
        );
        assert_eq!(
            rescued_position(egui::pos2(5000.0, 4000.0), size, monitor),
            egui::pos2(720.0, 280.0)
        );
        assert_eq!(
            rescued_position(
                egui::pos2(200.0, 100.0),
                egui::vec2(3000.0, 2000.0),
                monitor
            ),
            egui::pos2(0.0, 0.0),
            "a window larger than the monitor pins to the origin"
        );
    }

    #[test]
    fn viewport_applies_the_saved_geometry() {
        let geometry = WindowGeometry {
            width: 1200.0,
            height: 800.0,
            x: Some(120.0),
            y: Some(80.0),
            maximized: true,
        };
        let viewport = viewport(Some(geometry));
        assert_eq!(viewport.inner_size, Some(egui::vec2(1200.0, 800.0)));
        assert_eq!(viewport.position, Some(egui::pos2(120.0, 80.0)));
        assert_eq!(viewport.maximized, Some(true));
        assert_eq!(
            viewport.min_inner_size,
            Some(egui::vec2(MIN_WIDTH, MIN_HEIGHT))
        );

        let defaults = viewport(None);
        assert_eq!(
            defaults.inner_size,
            Some(egui::vec2(DEFAULT_WIDTH, DEFAULT_HEIGHT))
        );
        assert_eq!(defaults.position, None);
        assert_eq!(defaults.maximized, None);
    }
}
