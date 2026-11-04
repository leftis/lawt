//! Camera and exposure metadata captured once, while each original is read.
//!
//! Stored in the per-photo record so a future search (camera, lens, ISO, place) never
//! has to read the originals again.

// Rust guideline compliant 2026-07-03

use serde::{Deserialize, Serialize};

/// Camera, lens, exposure and location facts about one photo. Every field may be absent.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CameraExif {
    /// Manufacturer, e.g. `RICOH IMAGING COMPANY, LTD.`.
    pub make: Option<String>,
    /// Camera model, e.g. `GR IIIx`.
    pub model: Option<String>,
    /// Lens model.
    pub lens: Option<String>,
    /// ISO sensitivity.
    pub iso: Option<u32>,
    /// F-number, e.g. `2.8`.
    pub aperture: Option<f64>,
    /// Shutter speed as shown on a camera: `1/250` or `2` (seconds).
    pub shutter: Option<String>,
    /// Focal length in millimetres.
    pub focal_length: Option<f64>,
    /// 35 mm equivalent focal length in millimetres.
    pub focal_length_35mm: Option<u32>,
    /// GPS latitude in decimal degrees (north positive).
    pub gps_lat: Option<f64>,
    /// GPS longitude in decimal degrees (east positive).
    pub gps_lon: Option<f64>,
}

impl CameraExif {
    /// Fills every field that is empty here from `other`, keeping what is already set.
    #[must_use]
    pub fn or(self, other: Self) -> Self {
        Self {
            make: self.make.or(other.make),
            model: self.model.or(other.model),
            lens: self.lens.or(other.lens),
            iso: self.iso.or(other.iso),
            aperture: self.aperture.or(other.aperture),
            shutter: self.shutter.or(other.shutter),
            focal_length: self.focal_length.or(other.focal_length),
            focal_length_35mm: self.focal_length_35mm.or(other.focal_length_35mm),
            gps_lat: self.gps_lat.or(other.gps_lat),
            gps_lon: self.gps_lon.or(other.gps_lon),
        }
    }
}

/// Formats an exposure time of `num / den` seconds the way cameras display it.
///
/// Returns `None` for a zero denominator or a zero exposure.
///
/// # Examples
/// ```
/// use rawvault_common::camera::shutter_string;
/// assert_eq!(shutter_string(1, 250).as_deref(), Some("1/250"));
/// assert_eq!(shutter_string(10, 2500).as_deref(), Some("1/250"));
/// assert_eq!(shutter_string(2, 1).as_deref(), Some("2"));
/// assert_eq!(shutter_string(3, 2).as_deref(), Some("1.5"));
/// assert_eq!(shutter_string(1, 0), None);
/// ```
#[must_use]
pub fn shutter_string(num: u32, den: u32) -> Option<String> {
    if den == 0 || num == 0 {
        return None;
    }
    let seconds = f64::from(num) / f64::from(den);
    if seconds >= 1.0 {
        return Some(if (seconds - seconds.round()).abs() < 0.05 {
            format!("{}", seconds.round())
        } else {
            format!("{seconds:.1}")
        });
    }
    // Sub-second exposures read as 1/N; N is rounded to a whole number like the camera shows.
    Some(format!("1/{}", (1.0 / seconds).round()))
}

/// Converts degrees, minutes and seconds plus a hemisphere reference (`N`, `S`, `E`, `W`) to decimal degrees.
///
/// Returns `None` if the reference is not one of the four, or the value is out of range.
///
/// # Examples
/// ```
/// use rawvault_common::camera::gps_decimal;
/// let lat = gps_decimal(37.0, 58.0, 30.0, "N").unwrap();
/// assert!((lat - 37.975).abs() < 1e-9);
/// assert!(gps_decimal(23.0, 43.0, 0.0, "W").unwrap() < 0.0);
/// assert_eq!(gps_decimal(10.0, 0.0, 0.0, "X"), None);
/// ```
#[must_use]
pub fn gps_decimal(degrees: f64, minutes: f64, seconds: f64, reference: &str) -> Option<f64> {
    let value = degrees + minutes / 60.0 + seconds / 3600.0;
    if !value.is_finite() || value > 180.0 {
        return None;
    }
    match reference.trim().to_ascii_uppercase().as_str() {
        "N" | "E" => Some(value),
        "S" | "W" => Some(-value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn or_keeps_existing_values_and_fills_gaps() {
        let a = CameraExif {
            make: Some("A".into()),
            iso: Some(100),
            ..CameraExif::default()
        };
        let b = CameraExif {
            make: Some("B".into()),
            iso: Some(800),
            lens: Some("L".into()),
            ..CameraExif::default()
        };
        let merged = a.or(b);
        assert_eq!(merged.make.as_deref(), Some("A"));
        assert_eq!(merged.iso, Some(100));
        assert_eq!(merged.lens.as_deref(), Some("L"));
    }

    #[test]
    fn shutter_formats_common_speeds() {
        assert_eq!(shutter_string(1, 4000).as_deref(), Some("1/4000"));
        assert_eq!(shutter_string(1, 60).as_deref(), Some("1/60"));
        assert_eq!(shutter_string(30, 1).as_deref(), Some("30"));
        assert_eq!(shutter_string(0, 5), None);
    }

    #[test]
    fn gps_signs_follow_hemisphere() {
        assert!(gps_decimal(10.0, 30.0, 0.0, "S").unwrap() < 0.0);
        assert!((gps_decimal(10.0, 30.0, 0.0, "e").unwrap() - 10.5).abs() < 1e-9);
        assert_eq!(gps_decimal(200.0, 0.0, 0.0, "N"), None);
    }
}
