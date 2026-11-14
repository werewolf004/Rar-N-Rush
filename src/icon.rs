//! Application icon, drawn in code so no image file or decoder is needed.
//!
//! A blue rounded square with a white arrow pointing down into a tray: "files
//! arrive, get unpacked". Used for the window and the notification-area icon.

/// Background colour (`#2563C9`).
const BLUE: [u8; 3] = [0x25, 0x63, 0xC9];

/// RGBA pixels (unpremultiplied) of a `size` × `size` icon.
pub fn rgba(size: u32) -> Vec<u8> {
    let scale = f64::from(size.max(1));
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            // Pixel centre in unit coordinates.
            let (u, v) = ((f64::from(x) + 0.5) / scale, (f64::from(y) + 0.5) / scale);
            let background = rounded_square(u, v);
            let white = arrow(u, v);
            let alpha = background.max(white);
            let colour = if white > 0.0 {
                mix(BLUE, [255, 255, 255], white / alpha)
            } else {
                BLUE
            };
            out.extend_from_slice(&colour);
            out.push(to_byte(alpha));
        }
    }
    out
}

/// Icon for the window title bar and taskbar.
pub fn window_icon() -> egui::IconData {
    egui::IconData {
        rgba: rgba(64),
        width: 64,
        height: 64,
    }
}

/// Coverage (0–1) of the rounded square background.
fn rounded_square(u: f64, v: f64) -> f64 {
    let (half, radius) = (0.46, 0.2);
    let (dx, dy) = (
        (u - 0.5).abs() - (half - radius),
        (v - 0.5).abs() - (half - radius),
    );
    let outside = dx.max(0.0).hypot(dy.max(0.0)) + dx.max(dy).min(0.0) - radius;
    coverage(outside)
}

/// Coverage (0–1) of the white arrow and tray.
fn arrow(u: f64, v: f64) -> f64 {
    let stem = rect(u, v, 0.44, 0.2, 0.56, 0.52);
    // Head: triangle pointing down, apex at (0.5, 0.66).
    let head = {
        let t = (v - 0.4) / 0.26;
        let half = 0.17 * (1.0 - t);
        let inside = (0.0..=1.0).contains(&t) && (u - 0.5).abs() <= half;
        if inside { 1.0 } else { 0.0 }
    };
    let tray = rect(u, v, 0.24, 0.72, 0.76, 0.8)
        .max(rect(u, v, 0.24, 0.6, 0.32, 0.8))
        .max(rect(u, v, 0.68, 0.6, 0.76, 0.8));
    stem.max(head).max(tray)
}

fn rect(u: f64, v: f64, x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
    if (x0..=x1).contains(&u) && (y0..=y1).contains(&v) {
        1.0
    } else {
        0.0
    }
}

/// Anti-aliased coverage from a signed distance (in unit coordinates).
fn coverage(distance: f64) -> f64 {
    (0.5 - distance * 48.0).clamp(0.0, 1.0)
}

fn mix(a: [u8; 3], b: [u8; 3], t: f64) -> [u8; 3] {
    let channel = |x: u8, y: u8| to_byte((f64::from(x) * (1.0 - t) + f64::from(y) * t) / 255.0);
    [channel(a[0], b[0]), channel(a[1], b[1]), channel(a[2], b[2])]
}

/// Converts 0–1 to 0–255, saturating.
fn to_byte(x: f64) -> u8 {
    // The value is clamped to 0..=255 first, so the conversion cannot truncate.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let byte = (x * 255.0).round().clamp(0.0, 255.0) as u8;
    byte
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_has_the_right_size_and_transparent_corners() {
        for size in [16, 32, 64] {
            let px = rgba(size);
            assert_eq!(px.len(), (size * size * 4) as usize);
            assert_eq!(px[3], 0, "top-left corner is transparent");
            let centre = ((size / 2 * size + size / 2) * 4) as usize;
            assert_eq!(px[centre + 3], 255, "centre is opaque");
        }
    }

    #[test]
    fn icon_shows_white_and_blue() {
        let px = rgba(32);
        let colours: Vec<&[u8]> = px.chunks(4).filter(|p| p[3] == 255).collect();
        assert!(colours.iter().any(|p| p[..3] == [255, 255, 255]));
        assert!(colours.iter().any(|p| p[..3] == BLUE));
    }
}
