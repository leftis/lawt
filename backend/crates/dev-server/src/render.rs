//! Placeholder JPEG renditions for the synthetic library.

// Rust guideline compliant 2026-07-03

#![expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    reason = "pixel math on bounded image dimensions; dev-only placeholder art"
)]

use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, RgbImage};
use lawt_common::{Photo, Rendition};

/// JPEG quality for placeholders; small files, faithful enough for layout work.
const QUALITY: u8 = 72;

/// Longest edge used for the "full" placeholder; real full size would only slow dev.
const FULL_DEV_EDGE: u32 = 2400;

/// Renders a gradient JPEG with the photo's aspect ratio, tinted by its id and time of day.
///
/// Returns `None` if encoding fails.
#[must_use]
pub fn placeholder(photo: &Photo, rendition: Rendition) -> Option<Vec<u8>> {
    let edge = rendition.max_edge().unwrap_or(FULL_DEV_EDGE);
    let (w, h) = fit(photo.width, photo.height, edge);
    let seed = photo
        .id
        .bytes()
        .fold(0_u32, |a, b| a.wrapping_mul(31).wrapping_add(u32::from(b)));
    let hue = f32::from(u16::try_from(seed % 360).unwrap_or(0));
    let mut img = RgbImage::new(w, h);
    for (x, y, px) in img.enumerate_pixels_mut() {
        let fy = y as f32 / h as f32;
        let fx = x as f32 / w as f32;
        let light = 0.25 + 0.5 * (1.0 - fy) + 0.15 * fx;
        px.0 = hsl(hue + 40.0 * fx, 0.55, light.clamp(0.0, 1.0));
    }
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, QUALITY)
        .encode(img.as_raw(), w, h, ExtendedColorType::Rgb8)
        .ok()?;
    Some(out)
}

/// Scales `(w, h)` so the long edge is at most `edge`, never below 1 pixel.
fn fit(w: u32, h: u32, edge: u32) -> (u32, u32) {
    let long = w.max(h).max(1);
    if long <= edge {
        return (w.max(1), h.max(1));
    }
    let scale = |v: u32| {
        u32::try_from(u64::from(v) * u64::from(edge) / u64::from(long))
            .unwrap_or(1)
            .max(1)
    };
    (scale(w), scale(h))
}

fn hsl(h: f32, s: f32, l: f32) -> [u8; 3] {
    let h = h.rem_euclid(360.0);
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match h {
        h if h < 60.0 => (c, x, 0.0),
        h if h < 120.0 => (x, c, 0.0),
        h if h < 180.0 => (0.0, c, x),
        h if h < 240.0 => (0.0, x, c),
        h if h < 300.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let to_u8 = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    [to_u8(r), to_u8(g), to_u8(b)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeLibrary;

    #[test]
    fn keeps_aspect_and_caps_edge() {
        assert_eq!(fit(6000, 4000, 400), (400, 266));
        assert_eq!(fit(4000, 6000, 400), (266, 400));
        assert_eq!(fit(100, 50, 400), (100, 50));
    }

    #[test]
    fn placeholder_is_a_jpeg() {
        let lib = FakeLibrary::generate(3);
        let photo = lib.photo(&lib.index_first_id()).expect("photo exists");
        let bytes = placeholder(photo, Rendition::Thumbnail).expect("encodes");
        assert_eq!(&bytes[..2], &[0xFF, 0xD8]);
    }
}
