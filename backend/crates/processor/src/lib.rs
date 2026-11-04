//! Turns an original photo of any supported format into browser-friendly renditions.
//!
//! [`process`] takes the bytes of one original and returns a thumbnail, a medium
//! image, optionally a full-size JPEG, the oriented pixel size and the capture time.
//! It performs no I/O, so it is unit-tested on synthetic images and can run anywhere.
//!
//! How each format is read:
//!
//! - RAW: the largest JPEG preview embedded by the camera (fast, and it carries the
//!   camera's own look), found with `rawler`, else by scanning for an embedded JPEG.
//! - TIFF (8- or 16-bit), PNG, WebP, GIF, BMP: decoded and converted to 8-bit RGB.
//! - JPEG: decoded for the small renditions; the original itself serves as "full".
//! - HEIC/HEIF: recognised but reported as unsupported.
//!
//! Known limitation: embedded ICC profiles are not applied, so wide-gamut TIFF
//! exports are treated as sRGB. Verify on real files before relying on colour accuracy.

// Rust guideline compliant 2026-07-03

mod error;
mod exif_meta;
mod render;

#[doc(inline)]
pub use error::ProcessError;
#[doc(inline)]
pub use render::{Processed, process};
