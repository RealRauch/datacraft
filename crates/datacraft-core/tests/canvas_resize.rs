//! Resize-handle geometry contract (phase 102, step 102.2; item M133).
//!
//! The eight handles and the resize math live in core `canvas` — the
//! designers only paint the centers and dispatch the finished rect. The
//! rules: the grabbed edges move, the opposite ones stay, everything
//! snaps to the grid, a control never shrinks below one grid step, and a
//! resize never leaves the canvas/page bounds.

use datacraft_core::canvas::{
    HANDLE_SIZE, ResizeHandle, Rect, handle_at, handle_centers, resize_rect,
};

fn rect() -> Rect {
    Rect {
        x: 40.0,
        y: 40.0,
        width: 160.0,
        height: 24.0,
    }
}

const BOUNDS: (f32, f32) = (640.0, 400.0);

#[test]
fn the_eight_handles_sit_on_the_edges_and_corners() {
    let centers = handle_centers(rect());
    assert_eq!(centers.len(), 8);
    for (handle, (x, y)) in centers {
        assert_eq!(
            handle_at(rect(), x, y),
            Some(handle),
            "the handle center hits its own handle"
        );
    }
    assert_eq!(
        handle_centers(rect()).map(|(handle, _)| handle),
        [
            ResizeHandle::NorthWest,
            ResizeHandle::North,
            ResizeHandle::NorthEast,
            ResizeHandle::West,
            ResizeHandle::East,
            ResizeHandle::SouthWest,
            ResizeHandle::South,
            ResizeHandle::SouthEast,
        ]
    );
}

#[test]
fn the_body_and_the_area_outside_the_tolerance_are_not_handles() {
    assert_eq!(handle_at(rect(), 120.0, 52.0), None, "the body center");
    assert_eq!(
        handle_at(rect(), 200.0 + HANDLE_SIZE, 52.0),
        None,
        "outside the east handle's reach"
    );
}

#[test]
fn a_corner_drag_moves_both_edges_and_snaps() {
    let resized = resize_rect(
        rect(),
        ResizeHandle::SouthEast,
        13.0,
        13.0,
        8.0,
        BOUNDS,
    );
    assert_eq!(
        resized,
        Rect {
            x: 40.0,
            y: 40.0,
            width: 176.0,
            height: 40.0,
        },
        "right 213 -> 216, bottom 77 -> 80"
    );
}

#[test]
fn a_north_west_drag_moves_the_origin_and_keeps_the_far_edges() {
    let resized = resize_rect(rect(), ResizeHandle::NorthWest, -9.0, -5.0, 8.0, BOUNDS);
    assert_eq!(
        resized,
        Rect {
            x: 32.0,
            y: 40.0,
            width: 168.0,
            height: 24.0,
        },
        "left 31 -> 32, the right edge stays at 200; top 35 -> 40"
    );
}

#[test]
fn an_edge_drag_keeps_the_opposite_edge() {
    let resized = resize_rect(rect(), ResizeHandle::West, 16.0, 0.0, 8.0, BOUNDS);
    assert_eq!(
        resized,
        Rect {
            x: 56.0,
            y: 40.0,
            width: 144.0,
            height: 24.0,
        }
    );
    let resized = resize_rect(rect(), ResizeHandle::East, -16.0, 0.0, 8.0, BOUNDS);
    assert_eq!(resized.x, 40.0, "the left edge stays");
    assert_eq!(resized.width, 144.0);
}

#[test]
fn a_resize_never_shrinks_below_one_grid_step() {
    let resized = resize_rect(rect(), ResizeHandle::East, -500.0, 0.0, 8.0, BOUNDS);
    assert_eq!(resized.x, 40.0, "the anchored edge stays");
    assert_eq!(resized.width, 8.0);

    let resized = resize_rect(rect(), ResizeHandle::North, 0.0, 500.0, 8.0, BOUNDS);
    assert_eq!(resized.y, 40.0);
    assert_eq!(resized.height, 8.0);
}

#[test]
fn dragging_west_past_the_right_edge_keeps_the_right_edge() {
    let resized = resize_rect(rect(), ResizeHandle::West, 10_000.0, 0.0, 8.0, BOUNDS);
    assert_eq!(resized.width, 8.0);
    assert_eq!(resized.x + resized.width, 200.0);
}

#[test]
fn a_resize_stays_inside_the_bounds() {
    let resized = resize_rect(
        rect(),
        ResizeHandle::SouthEast,
        10_000.0,
        10_000.0,
        8.0,
        BOUNDS,
    );
    assert_eq!(resized.x + resized.width, 640.0);
    assert_eq!(resized.y + resized.height, 400.0);
    assert_eq!(resized.x, 40.0, "the anchored edges stay");
    assert_eq!(resized.y, 40.0);

    let resized = resize_rect(rect(), ResizeHandle::NorthWest, -10_000.0, -10_000.0, 8.0, BOUNDS);
    assert_eq!(resized.x, 0.0);
    assert_eq!(resized.y, 0.0);
    assert_eq!(resized.x + resized.width, 200.0);
    assert_eq!(resized.y + resized.height, 64.0);
}

#[test]
fn a_boundary_that_is_not_on_the_grid_still_yields_a_valid_rect() {
    // A4 portrait is 595 pt wide — not a multiple of the 8 pt grid.
    let bounds = (595.0, 842.0);
    let resized = resize_rect(
        Rect {
            x: 560.0,
            y: 40.0,
            width: 32.0,
            height: 24.0,
        },
        ResizeHandle::East,
        900.0,
        0.0,
        8.0,
        bounds,
    );
    assert!(resized.x + resized.width <= 595.0, "{resized:?}");
    assert_eq!(resized.width % 8.0, 0.0, "still on the grid: {resized:?}");
    assert!(resized.width >= 8.0);
}
