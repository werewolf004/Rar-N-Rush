//! The application icon, drawn in code: an amber rounded square holding a
//! dark archive box with a lightning bolt ("Rush").
//!
//! This file depends on nothing but `std`, so that the build script can
//! include it too (`#[path]`) and embed the same icon in `rar-n-rush.exe`.

/// Sizes stored in the `.ico` file: the ones Windows picks for the title bar,
/// taskbar, Explorer views and Start menu, at 100 % to 200 % display scaling.
pub const ICO_SIZES: [u32; 6] = [16, 24, 32, 48, 64, 256];

/// Background (amber).
pub const AMBER: [u8; 3] = [0xEF, 0x9F, 0x27];
/// Box body (dark brown).
pub const BOX: [u8; 3] = [0x41, 0x24, 0x02];
/// Box lid (brown).
pub const LID: [u8; 3] = [0x63, 0x38, 0x06];
/// Lightning bolt and tape (light amber).
pub const BOLT: [u8; 3] = [0xFA, 0xC7, 0x75];

/// Samples per pixel side: 4 × 4 samples give smooth edges at every size.
const SAMPLES: u32 = 4;

/// Lightning bolt, in a 64 × 64 grid.
const BOLT_SHAPE: [(f64, f64); 6] = [
    (37.0, 25.0),
    (23.5, 41.0),
    (31.0, 41.0),
    (27.0, 52.0),
    (41.5, 34.0),
    (34.0, 34.0),
];

/// Bolt drawn at 24 px and below: larger, so that it stays readable.
const SMALL_BOLT_SHAPE: [(f64, f64); 6] = [
    (41.0, 19.0),
    (17.0, 43.0),
    (30.0, 43.0),
    (23.0, 59.0),
    (48.0, 32.0),
    (35.0, 32.0),
];

/// Colour of the topmost shape at `(x, y)` (64 × 64 grid), or `None` outside
/// the icon. `small`: the simpler drawing used at 24 px and below.
fn paint(x: f64, y: f64, small: bool) -> Option<[u8; 3]> {
    if !in_rounded_rect(x, y, (2.0, 2.0, 62.0, 62.0), 13.0) {
        return None;
    }
    let bolt: &[(f64, f64)] = if small { &SMALL_BOLT_SHAPE } else { &BOLT_SHAPE };
    if in_polygon(x, y, bolt) {
        return Some(BOLT);
    }
    // Tape across the lid (too thin to read when small).
    if !small && in_rounded_rect(x, y, (29.0, 12.0, 35.0, 22.0), 0.0) {
        return Some(BOLT);
    }
    if in_rounded_rect(x, y, (10.0, 12.0, 54.0, 22.0), 2.5) {
        return Some(LID);
    }
    if in_rounded_rect(x, y, (12.5, 19.0, 51.5, 54.0), 3.5) {
        return Some(BOX);
    }
    Some(AMBER)
}

/// RGBA pixels (unpremultiplied) of a `size` × `size` icon.
pub fn rgba(size: u32) -> Vec<u8> {
    let size = size.max(1);
    let step = 64.0 / f64::from(size * SAMPLES);
    let small = size <= 24;
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    for py in 0..size {
        for px in 0..size {
            // Colour sums and number of samples inside the icon.
            let (mut rgb, mut covered) = ([0.0_f64; 3], 0.0_f64);
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let x = (f64::from(px * SAMPLES + sx) + 0.5) * step;
                    let y = (f64::from(py * SAMPLES + sy) + 0.5) * step;
                    if let Some(c) = paint(x, y, small) {
                        for (s, v) in rgb.iter_mut().zip(c) {
                            *s += f64::from(v);
                        }
                        covered += 1.0;
                    }
                }
            }
            for s in rgb {
                out.push(if covered > 0.0 {
                    to_byte(s / covered / 255.0)
                } else {
                    0
                });
            }
            out.push(to_byte(covered / f64::from(SAMPLES * SAMPLES)));
        }
    }
    out
}

/// The icon as a Windows `.ico` file holding one 32-bit image per size
/// (each at most 256). Used by the notifications, the installer and the
/// executable itself.
pub fn ico(sizes: &[u32]) -> Vec<u8> {
    let images: Vec<(u32, Vec<u8>)> = sizes
        .iter()
        .map(|&s| s.clamp(1, 256))
        .map(|s| (s, bmp(s)))
        .collect();
    let count = u16::try_from(images.len()).unwrap_or(u16::MAX);
    let mut out = Vec::new();
    // ICONDIR: reserved, type 1 (icon), image count.
    for v in [0u16, 1, count] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    // One 16-byte ICONDIRENTRY per image, then the images.
    let mut offset = 6 + 16 * u32::from(count);
    for (size, data) in &images {
        // 256 is written as 0.
        let side = u8::try_from(*size).unwrap_or(0);
        out.extend_from_slice(&[side, side, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes()); // planes
        out.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        let len = u32::try_from(data.len()).unwrap_or(u32::MAX);
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset = offset.saturating_add(len);
    }
    for (_, data) in images {
        out.extend_from_slice(&data);
    }
    out
}

/// One icon image: BITMAPINFOHEADER, BGRA pixels bottom-up, empty AND mask.
fn bmp(size: u32) -> Vec<u8> {
    let px = rgba(size);
    // Rows of the AND mask are padded to 4 bytes; it stays empty (alpha is used).
    let mask_row = size.div_ceil(32) * 4;
    let mut out = Vec::with_capacity((40 + size * size * 4 + mask_row * size) as usize);
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&size.to_le_bytes());
    // The height counts the colour image and the mask.
    out.extend_from_slice(&(size * 2).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&[0; 24]); // no compression; sizes and palette unused
    for row in px.chunks_exact(size as usize * 4).rev() {
        for &[r, g, b, a] in row.as_chunks::<4>().0 {
            out.extend_from_slice(&[b, g, r, a]);
        }
    }
    out.resize(out.len() + (mask_row * size) as usize, 0);
    out
}

/// `true` inside the rectangle `(x0, y0, x1, y1)` with corners rounded by `r`.
fn in_rounded_rect(x: f64, y: f64, (x0, y0, x1, y1): (f64, f64, f64, f64), r: f64) -> bool {
    if x < x0 || x > x1 || y < y0 || y > y1 {
        return false;
    }
    let cx = x.clamp(x0 + r, x1 - r);
    let cy = y.clamp(y0 + r, y1 - r);
    (x - cx).hypot(y - cy) <= r
}

/// Even-odd point-in-polygon test.
fn in_polygon(x: f64, y: f64, points: &[(f64, f64)]) -> bool {
    // Each point with the one before it (the last one before the first).
    let previous = points.iter().cycle().skip(points.len().saturating_sub(1));
    let mut inside = false;
    for (&(xi, yi), &(xj, yj)) in points.iter().zip(previous) {
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
    }
    inside
}

/// Converts 0–1 to 0–255, saturating.
fn to_byte(x: f64) -> u8 {
    // The value is clamped to 0..=255 first, so the conversion cannot truncate.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let byte = (x * 255.0).round().clamp(0.0, 255.0) as u8;
    byte
}
