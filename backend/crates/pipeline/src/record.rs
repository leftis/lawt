//! The per-photo record stored next to its renditions.

// Rust guideline compliant 2026-07-03

use chrono::NaiveDateTime;
use rawvault_common::{AssetId, CameraExif, Photo};
use serde::{Deserialize, Serialize};

/// Object key prefix of records in the derived bucket.
pub const RECORD_PREFIX: &str = "records/";

/// Returns the object key of a photo's record.
#[must_use]
pub fn record_key(id: &AssetId) -> String {
    format!("{RECORD_PREFIX}{id}.json")
}

/// Outcome of rendering a photo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Renditions were written.
    Rendered,
    /// The format is recognised but not supported (e.g. HEIC); the photo is listed without a thumbnail.
    Unsupported,
    /// The file could not be decoded; the photo is listed without a thumbnail.
    Failed,
}

/// Everything the pipeline knows about one photo.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Record {
    /// The timeline entry.
    pub photo: Photo,
    /// How rendering went.
    pub status: Status,
    /// Why rendering failed or was skipped.
    pub reason: Option<String>,
    /// Size of the primary original when it was processed.
    pub source_size: u64,
    /// Modification time of the primary original when it was processed.
    pub source_modified: NaiveDateTime,
    /// Camera, lens, exposure and location, captured now so search never needs the originals again.
    #[serde(default)]
    pub camera: CameraExif,
    /// Keys of other files of the same shot (e.g. the camera JPEG next to a RAW).
    pub companions: Vec<String>,
}
