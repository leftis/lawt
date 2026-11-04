//! Decoding and rendition encoding.

// Rust guideline compliant 2026-07-03

use std::borrow::Cow;

use chrono::NaiveDateTime;
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ExtendedColorType, RgbImage};
use lawt_common::rendition::{MEDIUM_EDGE, THUMBNAIL_EDGE};
use lawt_common::{CameraExif, FileKind};

use crate::error::ProcessError;
use crate::exif_meta;

/// JPEG quality of the full-size rendition; high because it is what gets exported.
const FULL_QUALITY: u8 = 92;

/// JPEG quality of the viewer rendition; visually clean at 1920 px, about 300 KB.
const MEDIUM_QUALITY: u8 = 82;

/// JPEG quality of the grid thumbnail; grid tiles are small, so more compression is invisible.
const THUMBNAIL_QUALITY: u8 = 76;

/// Embedded-JPEG candidates decoded when a RAW has no decoder support; bounds worst-case work.
#[cfg(feature = "raw")]
const MAX_EMBEDDED_CANDIDATES: usize = 6;

/// Renditions and facts extracted from one original.
#[derive(Debug)]
pub struct Processed {
    /// Width of the upright (EXIF-rotated) image in pixels.
    pub width: u32,
    /// Height of the upright (EXIF-rotated) image in pixels.
    pub height: u32,
    /// Capture time from EXIF, in camera wall-clock time.
    pub taken: Option<NaiveDateTime>,
    /// Camera, lens, exposure and location from EXIF.
    pub camera: CameraExif,
    /// Grid thumbnail JPEG.
    pub thumbnail: Vec<u8>,
    /// Viewer JPEG.
    pub medium: Vec<u8>,
    /// Full-size JPEG, or `None` when the original is already a JPEG and serves as "full".
    pub full: Option<Vec<u8>>,
}

/// Produces the renditions of one original.
///
/// # Errors
/// Returns [`ProcessError`] if the format is unsupported (HEIC/HEIF), the content cannot be
/// decoded, or encoding fails.
pub fn process(kind: FileKind, bytes: &[u8]) -> Result<Processed, ProcessError> {
    if kind == FileKind::Heic {
        return Err(ProcessError::unsupported(
            "HEIC/HEIF decoding is not available yet",
        ));
    }

    let (image, taken, camera) = if kind == FileKind::Raw {
        decode_raw(bytes)?
    } else {
        let meta = exif_meta::read(bytes);
        let mut image =
            image::load_from_memory(bytes).map_err(|e| ProcessError::decode(e.to_string()))?;
        if let Some(code) = meta.orientation {
            image = orient(image, code);
        }
        (image, meta.taken, meta.camera)
    };

    // 16-bit and alpha sources become 8-bit RGB here; the JPEG renditions cannot carry more.
    let rgb = image.into_rgb8();
    let (width, height) = rgb.dimensions();

    let medium_img = scaled(&rgb, MEDIUM_EDGE, FilterType::Lanczos3);
    // Derive the thumbnail from the medium image: far less to resample than the original.
    let thumbnail = encode(
        &scaled(&medium_img, THUMBNAIL_EDGE, FilterType::Triangle),
        THUMBNAIL_QUALITY,
    )?;
    let medium = encode(&medium_img, MEDIUM_QUALITY)?;
    let full = (kind != FileKind::Jpeg)
        .then(|| encode(&rgb, FULL_QUALITY))
        .transpose()?;

    Ok(Processed {
        width,
        height,
        taken,
        camera,
        thumbnail,
        medium,
        full,
    })
}

/// Applies an EXIF orientation code (1–8) so the image is upright.
fn orient(image: DynamicImage, code: u16) -> DynamicImage {
    match code {
        2 => image.fliph(),
        3 => image.rotate180(),
        4 => image.flipv(),
        5 => image.rotate90().fliph(),
        6 => image.rotate90(),
        7 => image.rotate270().fliph(),
        8 => image.rotate270(),
        _ => image,
    }
}

/// Scales so the long edge is at most `edge`; never upscales.
fn scaled(image: &RgbImage, edge: u32, filter: FilterType) -> Cow<'_, RgbImage> {
    let (w, h) = image.dimensions();
    let long = w.max(h);
    if long <= edge {
        return Cow::Borrowed(image);
    }
    // Rounded (not truncated) so the aspect ratio is preserved as closely as whole pixels allow.
    let shrink = |v: u32| {
        let scaled = (u64::from(v) * u64::from(edge) + u64::from(long) / 2) / u64::from(long);
        u32::try_from(scaled).unwrap_or(1).max(1)
    };
    Cow::Owned(image::imageops::resize(image, shrink(w), shrink(h), filter))
}

fn encode(image: &RgbImage, quality: u8) -> Result<Vec<u8>, ProcessError> {
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, quality)
        .encode(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgb8,
        )
        .map_err(|e| ProcessError::encode(e.to_string()))?;
    Ok(out)
}

#[cfg(feature = "raw")]
fn decode_raw(
    bytes: &[u8],
) -> Result<(DynamicImage, Option<NaiveDateTime>, CameraExif), ProcessError> {
    use rawler::decoders::RawDecodeParams;
    use rawler::get_decoder;
    use rawler::rawsource::RawSource;

    let source = RawSource::new_from_slice(bytes);
    let params = RawDecodeParams::default();
    let decoder = get_decoder(&source).ok();

    let metadata = decoder
        .as_ref()
        .and_then(|d| d.raw_metadata(&source, &params).ok());
    let orientation = metadata.as_ref().and_then(|m| m.exif.orientation);
    // Generic EXIF parsing works for TIFF-based RAWs; rawler covers formats it cannot (e.g. RAF).
    let generic = exif_meta::read(bytes);
    let taken = metadata
        .as_ref()
        .and_then(|m| m.exif.date_time_original.as_deref())
        .and_then(exif_meta::parse_datetime)
        .or(generic.taken);
    let camera = metadata
        .as_ref()
        .map_or_else(CameraExif::default, camera_from_rawler)
        .or(generic.camera);

    // Largest embedded preview the camera stored: fast, and it has the camera's own rendering.
    let preview = decoder
        .as_ref()
        .and_then(|d| d.full_image(&source, &params).ok().flatten())
        .or_else(|| embedded_jpeg(bytes));
    let Some(preview) = preview else {
        return Err(ProcessError::decode(
            "no embedded preview found in RAW file",
        ));
    };

    // Previews are sometimes stored unrotated. Rotate only when the preview is landscape but the
    // camera recorded a portrait orientation; a preview that is already portrait stays as is.
    let image = match orientation {
        Some(code @ 5..=8) if preview.width() > preview.height() => orient(preview, code),
        Some(code @ 2..=4) => orient(preview, code),
        _ => preview,
    };
    Ok((image, taken, camera))
}

/// Maps rawler's metadata to [`CameraExif`].
#[cfg(feature = "raw")]
fn camera_from_rawler(m: &rawler::decoders::RawMetadata) -> CameraExif {
    use lawt_common::camera::{gps_decimal, shutter_string};

    let ratio =
        |r: &rawler::formats::tiff::Rational| (r.d != 0).then(|| f64::from(r.n) / f64::from(r.d));
    let exif = &m.exif;
    let gps = exif.gps.as_ref().map(|g| {
        let axis = |dms: &Option<[rawler::formats::tiff::Rational; 3]>,
                    reference: &Option<String>| {
            let [d, mi, s] = dms.as_ref()?;
            gps_decimal(ratio(d)?, ratio(mi)?, ratio(s)?, reference.as_deref()?)
        };
        (
            axis(&g.gps_latitude, &g.gps_latitude_ref),
            axis(&g.gps_longitude, &g.gps_longitude_ref),
        )
    });
    let clean = |s: &str| {
        let t = s.trim_matches(['\0', ' ']);
        (!t.is_empty()).then(|| t.to_owned())
    };
    CameraExif {
        make: clean(&m.make),
        model: clean(&m.model),
        lens: exif.lens_model.as_deref().and_then(clean),
        iso: exif.iso_speed_ratings.map(u32::from).or(exif.iso_speed),
        aperture: exif.fnumber.as_ref().and_then(ratio),
        shutter: exif
            .exposure_time
            .as_ref()
            .and_then(|r| shutter_string(r.n, r.d)),
        focal_length: exif.focal_length.as_ref().and_then(ratio),
        focal_length_35mm: None,
        gps_lat: gps.and_then(|(lat, _)| lat),
        gps_lon: gps.and_then(|(_, lon)| lon),
    }
}

#[cfg(not(feature = "raw"))]
fn decode_raw(
    _bytes: &[u8],
) -> Result<(DynamicImage, Option<NaiveDateTime>, CameraExif), ProcessError> {
    Err(ProcessError::unsupported("built without RAW support"))
}

/// Finds the largest JPEG stream embedded in `bytes` that actually decodes.
///
/// Candidates are `FF D8 FF ... FF D9` spans. A nested EXIF thumbnail can end an outer stream
/// early, so candidates are tried largest first and only a successful decode counts.
#[cfg(feature = "raw")]
fn embedded_jpeg(bytes: &[u8]) -> Option<DynamicImage> {
    let mut spans: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i + 3 < bytes.len() {
        if bytes[i..i + 3] == [0xFF, 0xD8, 0xFF]
            && let Some(len) = bytes[i + 2..].windows(2).position(|w| w == [0xFF, 0xD9])
        {
            let end = i + 2 + len + 2;
            if end - i > 1024 {
                spans.push((i, end));
            }
            i = end;
            continue;
        }
        i += 1;
    }
    spans.sort_by_key(|&(s, e)| std::cmp::Reverse(e - s));
    spans
        .into_iter()
        .take(MAX_EMBEDDED_CANDIDATES)
        .find_map(|(s, e)| image::load_from_memory(&bytes[s..e]).ok())
}

#[cfg(test)]
#[expect(
    clippy::cast_possible_truncation,
    clippy::needless_pass_by_value,
    reason = "synthetic test pixels and small helpers"
)]
mod tests {
    use std::io::Cursor;

    use image::{ImageBuffer, ImageFormat, Rgb};

    use super::*;

    fn encoded(img: DynamicImage, format: ImageFormat) -> Vec<u8> {
        let mut out = Vec::new();
        img.write_to(&mut Cursor::new(&mut out), format).unwrap();
        out
    }

    fn gradient(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgb8(RgbImage::from_fn(w, h, |x, y| {
            Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        }))
    }

    fn dims(bytes: &[u8]) -> (u32, u32) {
        image::load_from_memory(bytes).unwrap().dimensions_pair()
    }

    trait Pair {
        fn dimensions_pair(&self) -> (u32, u32);
    }
    impl Pair for DynamicImage {
        fn dimensions_pair(&self) -> (u32, u32) {
            (self.width(), self.height())
        }
    }

    #[test]
    fn jpeg_original_is_its_own_full_rendition() {
        let out = process(
            FileKind::Jpeg,
            &encoded(gradient(3000, 2000), ImageFormat::Jpeg),
        )
        .unwrap();
        assert!(out.full.is_none());
        assert_eq!((out.width, out.height), (3000, 2000));
        assert_eq!(dims(&out.thumbnail), (400, 267));
        assert_eq!(dims(&out.medium), (1920, 1280));
        assert_eq!(&out.thumbnail[..2], &[0xFF, 0xD8]);
    }

    #[test]
    fn sixteen_bit_tiff_becomes_eight_bit_jpegs() {
        let img: ImageBuffer<Rgb<u16>, Vec<u16>> = ImageBuffer::from_fn(800, 600, |x, y| {
            Rgb([(x * 80) as u16, (y * 100) as u16, 40_000])
        });
        let tiff = encoded(DynamicImage::ImageRgb16(img), ImageFormat::Tiff);
        let out = process(FileKind::Tiff, &tiff).unwrap();
        assert_eq!((out.width, out.height), (800, 600));
        let full = out.full.expect("TIFF gets a full JPEG");
        assert_eq!(dims(&full), (800, 600));
        assert_eq!(
            dims(&out.medium),
            (800, 600),
            "smaller than the medium edge: no upscaling"
        );
        assert_eq!(dims(&out.thumbnail), (400, 300));
    }

    #[test]
    fn png_is_converted_to_a_full_jpeg() {
        let out = process(
            FileKind::Png,
            &encoded(gradient(500, 700), ImageFormat::Png),
        )
        .unwrap();
        assert_eq!(dims(&out.full.unwrap()), (500, 700));
        assert_eq!(dims(&out.thumbnail), (286, 400));
    }

    #[test]
    fn heic_is_reported_unsupported_not_faked() {
        assert!(
            process(FileKind::Heic, b"whatever")
                .unwrap_err()
                .is_unsupported()
        );
    }

    #[test]
    fn garbage_is_a_decode_error() {
        assert!(
            process(FileKind::Png, b"not an image at all")
                .unwrap_err()
                .is_decode()
        );
    }

    #[test]
    fn small_images_are_never_upscaled() {
        let out = process(FileKind::Png, &encoded(gradient(100, 50), ImageFormat::Png)).unwrap();
        assert_eq!(dims(&out.thumbnail), (100, 50));
    }

    #[test]
    fn orientation_codes_swap_dimensions_where_they_should() {
        for (code, swapped) in [
            (1, false),
            (2, false),
            (3, false),
            (4, false),
            (5, true),
            (6, true),
            (7, true),
            (8, true),
        ] {
            let out = orient(gradient(30, 20), code);
            assert_eq!(out.dimensions_pair() == (20, 30), swapped, "code {code}");
        }
    }

    #[test]
    fn orientation_6_rotates_clockwise() {
        // Mark the bottom-left pixel; a 90° clockwise turn moves it to the top-left.
        let mut img = RgbImage::from_pixel(30, 20, Rgb([0, 0, 0]));
        img.put_pixel(0, 19, Rgb([255, 0, 0]));
        let out = orient(DynamicImage::ImageRgb8(img), 6).into_rgb8();
        assert_eq!(out.get_pixel(0, 0).0, [255, 0, 0]);
    }

    #[cfg(feature = "raw")]
    #[test]
    fn finds_the_real_embedded_jpeg_even_after_a_nested_thumbnail() {
        let thumb = encoded(gradient(16, 16), ImageFormat::Jpeg);
        let big = encoded(gradient(640, 480), ImageFormat::Jpeg);
        // Junk, then a "container" whose header embeds a small JPEG, then the real preview.
        let mut file = vec![0_u8; 300];
        file.extend_from_slice(&thumb);
        file.extend_from_slice(&[0_u8; 200]);
        file.extend_from_slice(&big);
        let found = embedded_jpeg(&file).expect("finds a preview");
        assert_eq!(found.dimensions_pair(), (640, 480));
    }
}
