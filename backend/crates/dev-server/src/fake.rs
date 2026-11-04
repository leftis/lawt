//! Deterministic synthetic photo library for local development.

// Rust guideline compliant 2026-07-03

use std::collections::HashMap;

use chrono::{Duration, NaiveDate, NaiveDateTime};
use lawt_api::{Library, StoreError};
use lawt_common::{
    AssetId, IdSalt, MonthKey, MonthPage, Photo, TakenAtSource, TimelineIndex, build_index,
    group_by_month,
};

/// Camera-card folders and the extensions their cameras produce.
const CAMERAS: &[(&str, &[&str])] = &[
    ("100RICOH", &["dng", "dng", "jpg"]),
    ("101RICOH", &["dng", "jpg"]),
    ("100_FUJI", &["raf", "raf", "jpg", "tif"]),
    ("100NIKON", &["nef", "nef", "jpg"]),
    ("SONY_A7", &["arw", "arw", "jpg", "png"]),
];

/// (width, height) shapes: mostly 3:2 landscape, some portrait, square and wide.
const SHAPES: &[(u32, u32)] = &[
    (6000, 4000),
    (6000, 4000),
    (6000, 4000),
    (4000, 6000),
    (4000, 6000),
    (5000, 5000),
    (6000, 3375),
];

/// Small xorshift generator so the library is identical on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// In-memory library of synthetic photos.
#[derive(Debug)]
pub struct FakeLibrary {
    photos: Vec<Photo>,
    index: TimelineIndex,
    by_month: std::collections::BTreeMap<String, Vec<Photo>>,
    by_id: HashMap<String, usize>,
}

impl FakeLibrary {
    /// Generates about `days` shooting days spread over the last three years.
    #[must_use]
    pub fn generate(days: u32) -> Self {
        let salt = IdSalt::insecure_dev();
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        let start = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap_or_default();
        let span = 1350_u64;
        let mut photos = Vec::new();
        let mut counter = 0_u32;

        for _ in 0..days {
            let day = start + Duration::days(i64::try_from(rng.below(span)).unwrap_or(0));
            let (gallery, exts) =
                CAMERAS[usize::try_from(rng.below(CAMERAS.len() as u64)).unwrap_or(0)];
            let shots = 1 + rng.below(40);
            let base_hour = 8 + rng.below(9);
            for s in 0..shots {
                counter += 1;
                let stem = format!("IMG_{counter:05}");
                let ext = exts[usize::try_from(rng.below(exts.len() as u64)).unwrap_or(0)];
                let (width, height) =
                    SHAPES[usize::try_from(rng.below(SHAPES.len() as u64)).unwrap_or(0)];
                let taken_at: NaiveDateTime = day
                    .and_hms_opt(u32::try_from(base_hour).unwrap_or(9), 0, 0)
                    .unwrap_or_default()
                    + Duration::seconds(i64::try_from(s * 47 + rng.below(40)).unwrap_or(0));
                photos.push(Photo {
                    id: AssetId::for_asset(&salt, gallery, &stem).to_string(),
                    original_key: format!("all images/{gallery}/{stem}.{ext}"),
                    gallery: gallery.to_owned(),
                    stem,
                    ext: ext.to_owned(),
                    is_raw: matches!(ext, "dng" | "raf" | "nef" | "arw"),
                    width,
                    height,
                    taken_at,
                    taken_at_source: TakenAtSource::Exif,
                    size_bytes: 20_000_000 + rng.below(30_000_000),
                    has_thumbnail: true,
                    has_full: !matches!(ext, "jpg"),
                });
            }
        }

        let index = build_index(&photos);
        let by_month = group_by_month(&photos);
        let by_id = photos
            .iter()
            .enumerate()
            .map(|(i, p)| (p.id.clone(), i))
            .collect();
        Self {
            photos,
            index,
            by_month,
            by_id,
        }
    }

    /// Looks up a photo by id.
    #[must_use]
    pub fn photo(&self, id: &str) -> Option<&Photo> {
        self.by_id.get(id).and_then(|&i| self.photos.get(i))
    }

    /// Number of generated photos.
    #[must_use]
    pub fn count(&self) -> usize {
        self.photos.len()
    }
}

impl Library for FakeLibrary {
    async fn index(&self) -> Result<TimelineIndex, StoreError> {
        Ok(self.index.clone())
    }

    async fn month(&self, month: &MonthKey) -> Result<Option<MonthPage>, StoreError> {
        Ok(self.by_month.get(month.as_str()).map(|photos| MonthPage {
            month: month.to_string(),
            photos: photos.clone(),
        }))
    }
}

#[cfg(test)]
impl FakeLibrary {
    /// Returns any photo id, for tests.
    pub(crate) fn index_first_id(&self) -> String {
        self.photos
            .first()
            .map(|p| p.id.clone())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_is_deterministic_and_nonempty() {
        let a = FakeLibrary::generate(50);
        let b = FakeLibrary::generate(50);
        assert!(a.count() > 100);
        assert_eq!(a.index, b.index);
    }

    #[test]
    fn ids_are_unique() {
        let lib = FakeLibrary::generate(120);
        assert_eq!(lib.by_id.len(), lib.count());
    }
}
