//! Window geometry persistence (step 22.2, item U13; ADR-5).
//!
//! `ui.json` stores the inner size, the outer position and the maximized
//! state. Values are sanitized on load (implausible geometry is dropped) and
//! a saved position that would land off-screen is rescued onto the visible
//! monitor — the model's `WindowGeometry::sanitized`/`rescue_position`
//! pattern (`crates/ui-egui/src/window_geometry.rs` @ `fd7f662`).
//!
//! Limitation: only the current monitor's size is known at rescue time, so a
//! window saved on a since-removed secondary monitor is pulled onto the
//! primary monitor rather than restored to another secondary one.

use eframe::egui;
use serde::{Deserialize, Serialize};

/// Minimum inner window width (also the plausibility floor).
pub const MIN_WIDTH: f32 = 960.0;
/// Minimum inner window height (also the plausibility floor).
pub const MIN_HEIGHT: f32 = 600.0;
/// Default inner width when no geometry is stored.
pub const DEFAULT_WIDTH: f32 = 1280.0;
/// Default inner height when no geometry is stored.
pub const DEFAULT_HEIGHT: f32 = 800.0;
/// Plausibility bound for a stored outer position.
const POSITION_LIMIT: f32 = 32_768.0;
/// Plausibility bound for a stored inner size.
const SIZE_LIMIT: f32 = 16_384.0;

/// Persisted window geometry.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct WindowGeometry {
    /// Inner width in points.
    pub width: f32,
    /// Inner height in points.
    pub height: f32,
    /// Outer position x in points (`None` in older files).
    #[serde(default)]
    pub x: Option<f32>,
    /// Outer position y in points (`None` in older files).
    #[serde(default)]
    pub y: Option<f32>,
    /// Whether the window was maximized.
    #[serde(default)]
    pub maximized: bool,
}

impl WindowGeometry {
    /// Drops implausible geometry so a corrupt file never yields an unusable
    /// window: an unusable size drops the whole record, a partial or absurd
    /// position is dropped while the size survives.
    pub fn sane(self) -> Option<Self> {
        let size_ok = self.width.is_finite()
            && self.height.is_finite()
            && (MIN_WIDTH..=SIZE_LIMIT).contains(&self.width)
            && (MIN_HEIGHT..=SIZE_LIMIT).contains(&self.height);
        if !size_ok {
            return None;
        }
        let position = match (self.x, self.y) {
            (Some(x), Some(y))
                if x.is_finite()
                    && y.is_finite()
                    && (-POSITION_LIMIT..=POSITION_LIMIT).contains(&x)
                    && (-POSITION_LIMIT..=POSITION_LIMIT).contains(&y) =>
            {
                (Some(x), Some(y))
            }
            _ => (None, None),
        };
        Some(Self {
            x: position.0,
            y: position.1,
            ..self
        })
    }
}

/// Clamps a saved outer position into the visible monitor, so a window that
/// was saved off-screen (monitor change, unplugged display) stays reachable.
pub fn rescued_position(saved: egui::Pos2, size: egui::Vec2, monitor: egui::Vec2) -> egui::Pos2 {
    let max_x = (monitor.x - size.x).max(0.0);
    let max_y = (monitor.y - size.y).max(0.0);
    egui::pos2(saved.x.clamp(0.0, max_x), saved.y.clamp(0.0, max_y))
}

/// The window builder for a persisted geometry (defaults when absent).
pub fn viewport(window: Option<WindowGeometry>) -> egui::ViewportBuilder {
    let mut builder = egui::ViewportBuilder::default().with_min_inner_size([MIN_WIDTH, MIN_HEIGHT]);
    match window {
        Some(window) => {
            builder = builder
                .with_inner_size([window.width, window.height])
                .with_maximized(window.maximized);
            if let (Some(x), Some(y)) = (window.x, window.y) {
                builder = builder.with_position([x, y]);
            }
        }
        None => builder = builder.with_inner_size([DEFAULT_WIDTH, DEFAULT_HEIGHT]),
    }
    builder
}

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
        let builder = viewport(Some(geometry));
        assert_eq!(builder.inner_size, Some(egui::vec2(1200.0, 800.0)));
        assert_eq!(builder.position, Some(egui::pos2(120.0, 80.0)));
        assert_eq!(builder.maximized, Some(true));
        assert_eq!(
            builder.min_inner_size,
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
