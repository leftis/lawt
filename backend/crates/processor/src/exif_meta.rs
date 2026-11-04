//! Capture time and orientation from EXIF.

// Rust guideline compliant 2026-07-03

use std::io::Cursor;

use chrono::NaiveDateTime;
use exif::{In, Reader, Tag};
use rawvault_common::CameraExif;
use rawvault_common::camera::{gps_decimal, shutter_string};

/// EXIF facts about a photo.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExifMeta {
    /// `DateTimeOriginal` in camera wall-clock time.
    pub taken: Option<NaiveDateTime>,
    /// EXIF orientation code 1–8, if present.
    pub orientation: Option<u16>,
    /// Camera, lens, exposure and location.
    pub camera: CameraExif,
}

/// Reads EXIF from a JPEG, TIFF-based RAW/TIFF, PNG or WebP container. Missing or malformed EXIF yields defaults.
#[must_use]
pub fn read(bytes: &[u8]) -> ExifMeta {
    let Ok(exif) = Reader::new().read_from_container(&mut Cursor::new(bytes)) else {
        return ExifMeta::default();
    };
    let taken = [Tag::DateTimeOriginal, Tag::DateTime]
        .into_iter()
        .find_map(|tag| {
            let field = exif.get_field(tag, In::PRIMARY)?;
            parse_datetime(&field.display_value().to_string())
        });
    let orientation = uint(&exif, Tag::Orientation)
        .and_then(|v| u16::try_from(v).ok())
        .filter(|v| (1..=8).contains(v));
    let shutter =
        rational(&exif, Tag::ExposureTime, 0).and_then(|r| shutter_string(r.num, r.denom));
    let camera = CameraExif {
        make: ascii(&exif, Tag::Make),
        model: ascii(&exif, Tag::Model),
        lens: ascii(&exif, Tag::LensModel),
        iso: uint(&exif, Tag::PhotographicSensitivity),
        aperture: rational(&exif, Tag::FNumber, 0).map(|r| r.to_f64()),
        shutter,
        focal_length: rational(&exif, Tag::FocalLength, 0).map(|r| r.to_f64()),
        focal_length_35mm: uint(&exif, Tag::FocalLengthIn35mmFilm),
        gps_lat: gps(&exif, Tag::GPSLatitude, Tag::GPSLatitudeRef),
        gps_lon: gps(&exif, Tag::GPSLongitude, Tag::GPSLongitudeRef),
    };
    ExifMeta {
        taken,
        orientation,
        camera,
    }
}

fn ascii(exif: &exif::Exif, tag: Tag) -> Option<String> {
    let field = exif.get_field(tag, In::PRIMARY)?;
    let exif::Value::Ascii(parts) = &field.value else {
        return None;
    };
    let text = String::from_utf8_lossy(parts.first()?)
        .trim_matches(['\0', ' '])
        .to_owned();
    (!text.is_empty()).then_some(text)
}

fn uint(exif: &exif::Exif, tag: Tag) -> Option<u32> {
    exif.get_field(tag, In::PRIMARY)?.value.get_uint(0)
}

fn rational(exif: &exif::Exif, tag: Tag, index: usize) -> Option<exif::Rational> {
    match &exif.get_field(tag, In::PRIMARY)?.value {
        exif::Value::Rational(v) => v.get(index).copied().filter(|r| r.denom != 0),
        _ => None,
    }
}

fn gps(exif: &exif::Exif, value: Tag, reference: Tag) -> Option<f64> {
    let (d, m, s) = (
        rational(exif, value, 0)?,
        rational(exif, value, 1)?,
        rational(exif, value, 2)?,
    );
    gps_decimal(d.to_f64(), m.to_f64(), s.to_f64(), &ascii(exif, reference)?)
}

/// Parses EXIF's `YYYY:MM:DD HH:MM:SS` (and the `YYYY-MM-DD` display form). Zero dates are rejected.
pub(crate) fn parse_datetime(s: &str) -> Option<NaiveDateTime> {
    let s = s.trim().trim_matches('"');
    ["%Y-%m-%d %H:%M:%S", "%Y:%m:%d %H:%M:%S"]
        .iter()
        .find_map(|fmt| NaiveDateTime::parse_from_str(s, fmt).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_exif_date_forms() {
        let want = chrono::NaiveDate::from_ymd_opt(2024, 6, 15)
            .unwrap()
            .and_hms_opt(10, 23, 5)
            .unwrap();
        assert_eq!(parse_datetime("2024:06:15 10:23:05"), Some(want));
        assert_eq!(parse_datetime("2024-06-15 10:23:05"), Some(want));
        assert_eq!(parse_datetime("\"2024-06-15 10:23:05\""), Some(want));
    }

    #[test]
    fn rejects_zero_and_garbage_dates() {
        assert_eq!(parse_datetime("0000:00:00 00:00:00"), None);
        assert_eq!(parse_datetime(""), None);
        assert_eq!(parse_datetime("not a date"), None);
    }

    #[test]
    fn reads_camera_exposure_and_gps_from_a_real_exif_block() {
        use exif::experimental::Writer;
        use exif::{Field, Rational, Value};

        let f = |tag, value| Field {
            tag,
            ifd_num: In::PRIMARY,
            value,
        };
        let fields = [
            f(Tag::Make, Value::Ascii(vec![b"RICOH IMAGING".to_vec()])),
            f(Tag::Model, Value::Ascii(vec![b"GR IIIx".to_vec()])),
            f(Tag::LensModel, Value::Ascii(vec![b"26.1mm F2.8".to_vec()])),
            f(Tag::PhotographicSensitivity, Value::Short(vec![400])),
            f(
                Tag::FNumber,
                Value::Rational(vec![Rational { num: 28, denom: 10 }]),
            ),
            f(
                Tag::ExposureTime,
                Value::Rational(vec![Rational { num: 1, denom: 250 }]),
            ),
            f(
                Tag::FocalLength,
                Value::Rational(vec![Rational {
                    num: 261,
                    denom: 10,
                }]),
            ),
            f(Tag::FocalLengthIn35mmFilm, Value::Short(vec![40])),
            f(Tag::Orientation, Value::Short(vec![6])),
            f(
                Tag::DateTimeOriginal,
                Value::Ascii(vec![b"2024:06:15 10:23:05".to_vec()]),
            ),
            f(Tag::GPSLatitudeRef, Value::Ascii(vec![b"N".to_vec()])),
            f(
                Tag::GPSLatitude,
                Value::Rational(vec![
                    Rational { num: 37, denom: 1 },
                    Rational { num: 58, denom: 1 },
                    Rational { num: 30, denom: 1 },
                ]),
            ),
            f(Tag::GPSLongitudeRef, Value::Ascii(vec![b"W".to_vec()])),
            f(
                Tag::GPSLongitude,
                Value::Rational(vec![
                    Rational { num: 23, denom: 1 },
                    Rational { num: 43, denom: 1 },
                    Rational { num: 0, denom: 1 },
                ]),
            ),
        ];
        let mut writer = Writer::new();
        for field in &fields {
            writer.push_field(field);
        }
        let mut buf = Cursor::new(Vec::new());
        writer.write(&mut buf, false).unwrap();

        let meta = read(buf.get_ref());
        let cam = &meta.camera;
        assert_eq!(cam.make.as_deref(), Some("RICOH IMAGING"));
        assert_eq!(cam.model.as_deref(), Some("GR IIIx"));
        assert_eq!(cam.lens.as_deref(), Some("26.1mm F2.8"));
        assert_eq!(cam.iso, Some(400));
        assert_eq!(cam.aperture, Some(2.8));
        assert_eq!(cam.shutter.as_deref(), Some("1/250"));
        assert_eq!(cam.focal_length, Some(26.1));
        assert_eq!(cam.focal_length_35mm, Some(40));
        assert!((cam.gps_lat.unwrap() - 37.975).abs() < 1e-9);
        assert!(cam.gps_lon.unwrap() < 0.0, "west is negative");
        assert_eq!(meta.orientation, Some(6));
        assert_eq!(meta.taken, parse_datetime("2024:06:15 10:23:05"));
    }

    #[test]
    fn no_exif_gives_defaults() {
        assert_eq!(read(b"definitely not an image"), ExifMeta::default());
    }
}
