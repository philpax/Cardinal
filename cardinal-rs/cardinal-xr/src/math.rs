// cardinal-xr/src/math.rs
//! Math utilities for 3D coordinate conversion and curve generation.

use glam::Vec3;

use crate::constants;

/// Generate points along a cubic Bezier cable curve that sags downward.
///
/// Uses `constants::CABLE_SEGMENT_COUNT` for point count and
/// `constants::CABLE_SAG_FACTOR` for sag amount.
pub fn cable_bezier_points(start: Vec3, end: Vec3) -> Vec<Vec3> {
    let sag = constants::CABLE_SAG_FACTOR * (end - start).length();

    // Midpoint between start and end, drooped downward
    let mid = (start + end) * 0.5 - Vec3::new(0.0, sag, 0.0);

    // Control points: pull toward the midpoint with extra droop
    let ctrl1 = start + (mid - start) * 0.5 - Vec3::new(0.0, sag * 0.5, 0.0);
    let ctrl2 = end + (mid - end) * 0.5 - Vec3::new(0.0, sag * 0.5, 0.0);

    let n = constants::CABLE_SEGMENT_COUNT;
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            // Cubic Bezier: B(t) = u³·P0 + 3u²t·P1 + 3ut²·P2 + t³·P3
            start * (u * u * u)
                + ctrl1 * (3.0 * u * u * t)
                + ctrl2 * (3.0 * u * t * t)
                + end * (t * t * t)
        })
        .collect()
}

/// Convert a widget pixel position (origin top-left) to a 3D offset from the
/// panel center. X increases rightward, Y increases upward, Z = 0.
pub fn pixel_to_panel_offset(
    pixel_x: f32,
    pixel_y: f32,
    module_width_px: f32,
    module_height_px: f32,
) -> Vec3 {
    let x = (pixel_x - module_width_px * 0.5) / constants::PIXELS_PER_METER;
    // Flip Y: pixel Y increases downward, panel Y increases upward
    let y = -(pixel_y - module_height_px * 0.5) / constants::PIXELS_PER_METER;
    Vec3::new(x, y, 0.0)
}

/// Convert a 3D panel offset from center back to pixel coordinates (origin
/// top-left). Inverse of `pixel_to_panel_offset`.
pub fn panel_offset_to_pixel(
    offset: Vec3,
    module_width_px: f32,
    module_height_px: f32,
) -> (f32, f32) {
    let pixel_x = offset.x * constants::PIXELS_PER_METER + module_width_px * 0.5;
    let pixel_y = -offset.y * constants::PIXELS_PER_METER + module_height_px * 0.5;
    (pixel_x, pixel_y)
}

/// Exponential smoothing: move `current` toward `target` by `factor` each
/// frame via linear interpolation.
pub fn smooth(current: Vec3, target: Vec3, factor: f32) -> Vec3 {
    current.lerp(target, factor)
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: f32 = 600.0;
    const H: f32 = 400.0;

    #[test]
    fn test_cable_bezier_correct_endpoints() {
        let start = Vec3::new(0.0, 0.0, 0.0);
        let end = Vec3::new(1.0, 0.0, 0.0);
        let pts = cable_bezier_points(start, end);

        assert!((pts[0] - start).length() < 1e-5, "first point should equal start");
        assert!(
            (pts[pts.len() - 1] - end).length() < 1e-5,
            "last point should equal end"
        );
    }

    #[test]
    fn test_cable_bezier_sags_below_endpoints() {
        let start = Vec3::new(0.0, 0.0, 0.0);
        let end = Vec3::new(1.0, 0.0, 0.0);
        let pts = cable_bezier_points(start, end);

        let mid_idx = pts.len() / 2;
        let mid_y = pts[mid_idx].y;
        let endpoint_y = start.y.min(end.y);

        assert!(
            mid_y < endpoint_y,
            "midpoint Y ({mid_y}) should be below endpoint Y ({endpoint_y})"
        );
    }

    #[test]
    fn test_pixel_to_panel_offset_center() {
        let offset = pixel_to_panel_offset(W * 0.5, H * 0.5, W, H);
        assert!(offset.x.abs() < 1e-5, "center X should be 0, got {}", offset.x);
        assert!(offset.y.abs() < 1e-5, "center Y should be 0, got {}", offset.y);
        assert_eq!(offset.z, 0.0);
    }

    #[test]
    fn test_pixel_to_panel_roundtrip() {
        let px = 123.4_f32;
        let py = 78.9_f32;
        let offset = pixel_to_panel_offset(px, py, W, H);
        let (rx, ry) = panel_offset_to_pixel(offset, W, H);
        assert!((rx - px).abs() < 1e-3, "roundtrip X: expected {px}, got {rx}");
        assert!((ry - py).abs() < 1e-3, "roundtrip Y: expected {py}, got {ry}");
    }

    #[test]
    fn test_pixel_to_panel_top_left_is_positive_y() {
        // Top-left pixel: (0, 0)
        // X should be negative (left of center), Y should be positive (above center)
        let offset = pixel_to_panel_offset(0.0, 0.0, W, H);
        assert!(offset.x < 0.0, "top-left X should be negative, got {}", offset.x);
        assert!(offset.y > 0.0, "top-left Y should be positive, got {}", offset.y);
    }
}
