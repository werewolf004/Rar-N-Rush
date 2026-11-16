//! Application icon, drawn in code so no image file or decoder is needed.
//!
//! An amber rounded square holding a dark archive box with a lightning bolt:
//! archives, handled in a rush. Used for the window, the notification area,
//! the Windows notifications, the installer and the executable itself (see
//! `build.rs`).

mod art;

pub use art::{ICO_SIZES, ico, rgba};

/// Icon for the window title bar and taskbar.
pub fn window_icon() -> egui::IconData {
    egui::IconData {
        rgba: rgba(64),
        width: 64,
        height: 64,
    }
}

#[cfg(test)]
mod tests {
    use super::art::{AMBER, BOLT, BOX};
    use super::*;

    #[test]
    fn icon_has_the_right_size_and_transparent_corners() {
        for size in ICO_SIZES {
            let px = rgba(size);
            assert_eq!(px.len(), (size * size * 4) as usize);
            assert_eq!(px[3], 0, "top-left corner is transparent");
            let centre = ((size / 2 * size + size / 2) * 4) as usize;
            assert_eq!(px[centre + 3], 255, "centre is opaque");
        }
    }

    #[test]
    fn icon_shows_its_three_colours() {
        let px = rgba(64);
        let opaque: Vec<&[u8]> = px.chunks(4).filter(|p| p[3] == 255).collect();
        for colour in [AMBER, BOX, BOLT] {
            assert!(opaque.iter().any(|p| p[..3] == colour), "{colour:?} missing");
        }
    }

    #[test]
    fn small_sizes_keep_the_bolt_visible() {
        // At 16 px the bolt must still stand out from the box around it.
        let px = rgba(16);
        let bright = px
            .chunks(4)
            .filter(|p| p[3] == 255 && p[0] > 0xC0 && p[1] > 0x90 && p[2] > 0x50 && p[2] < 0xA0)
            .count();
        assert!(bright >= 3, "only {bright} bolt-coloured pixels at 16 px");
    }

    #[test]
    fn ico_file_holds_every_size() {
        let data = ico(&ICO_SIZES);
        let u16_at = |i: usize| u16::from_le_bytes([data[i], data[i + 1]]);
        let u32_at = |i: usize| u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
        assert_eq!((u16_at(0), u16_at(2)), (0, 1), "icon header");
        assert_eq!(usize::from(u16_at(4)), ICO_SIZES.len());
        let mut end = 6 + 16 * ICO_SIZES.len();
        for (i, size) in ICO_SIZES.into_iter().enumerate() {
            let entry = 6 + 16 * i;
            let side = if size == 256 { 0 } else { size };
            assert_eq!((u32::from(data[entry]), u32::from(data[entry + 1])), (side, side));
            assert_eq!(u16_at(entry + 6), 32, "32 bits per pixel");
            let (len, offset) = (u32_at(entry + 8) as usize, u32_at(entry + 12) as usize);
            assert_eq!(offset, end, "images follow each other");
            assert_eq!(u32_at(offset), 40, "bitmap header");
            assert_eq!(u32_at(offset + 4), size);
            assert_eq!(u32_at(offset + 8), size * 2, "colour image plus mask");
            // Pixels are stored bottom-up in BGRA: compare with the RGBA image.
            let px = rgba(size);
            let first = offset + 40;
            let row_bytes = size as usize * 4;
            for (stored_row, row) in px.chunks(row_bytes).rev().enumerate() {
                for (x, p) in row.chunks(4).enumerate() {
                    let at = first + stored_row * row_bytes + x * 4;
                    assert_eq!(&data[at..at + 4], &[p[2], p[1], p[0], p[3]]);
                }
            }
            end = offset + len;
        }
        assert_eq!(data.len(), end, "nothing after the last image");
    }
}
