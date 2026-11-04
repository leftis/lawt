//! Builds the timeline read model files from photo records.
//!
//! The read model is what the API serves: one day index plus one file per month.
//! It is derived data and can be rebuilt from the per-photo records at any time.

// Rust guideline compliant 2026-07-03

use std::collections::BTreeSet;

use crate::timeline::{Photo, build_index, group_by_month};

/// Object key of the day index in the derived bucket.
pub const INDEX_KEY: &str = "index/timeline.json";

/// Object key prefix of month pages in the derived bucket.
pub const MONTHS_PREFIX: &str = "index/months/";

/// The complete set of read model objects for a library.
#[derive(Debug, PartialEq, Eq)]
pub struct ReadModel {
    /// `(object key, JSON body)` for the index and every non-empty month.
    pub objects: Vec<(String, Vec<u8>)>,
}

impl ReadModel {
    /// Returns the keys of all objects in this model.
    #[must_use]
    pub fn keys(&self) -> BTreeSet<&str> {
        self.objects.iter().map(|(k, _)| k.as_str()).collect()
    }

    /// Returns which of `existing` keys under the read model prefix are no longer part of it.
    ///
    /// Callers delete these so a month that lost all its photos disappears instead of lingering.
    #[must_use]
    pub fn stale<'a>(&self, existing: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
        let keep = self.keys();
        existing
            .into_iter()
            .filter(|k| (*k == INDEX_KEY || k.starts_with(MONTHS_PREFIX)) && !keep.contains(k))
            .collect()
    }
}

/// Builds the read model for `photos`.
///
/// Output is deterministic (sorted, stable field order), so unchanged data produces
/// byte-identical objects and re-publishing is a no-op for caches.
///
/// # Errors
/// Returns the serializer's error if a record cannot be encoded (not expected for these types).
pub fn build(photos: &[Photo]) -> Result<ReadModel, serde_json::Error> {
    let mut objects = Vec::new();
    objects.push((
        INDEX_KEY.to_owned(),
        serde_json::to_vec(&build_index(photos))?,
    ));
    for (month, list) in group_by_month(photos) {
        let page = crate::timeline::MonthPage {
            month: month.clone(),
            photos: list,
        };
        objects.push((
            format!("{MONTHS_PREFIX}{month}.json"),
            serde_json::to_vec(&page)?,
        ));
    }
    Ok(ReadModel { objects })
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::timeline::TakenAtSource;

    fn photo(id: &str, y: i32, m: u32, d: u32) -> Photo {
        Photo {
            id: id.into(),
            original_key: format!("all images/g/{id}.jpg"),
            gallery: "g".into(),
            stem: id.into(),
            ext: "jpg".into(),
            is_raw: false,
            width: 3,
            height: 2,
            taken_at: NaiveDate::from_ymd_opt(y, m, d)
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap(),
            taken_at_source: TakenAtSource::Exif,
            size_bytes: 1,
            has_thumbnail: true,
            has_full: false,
        }
    }

    #[test]
    fn one_index_plus_one_file_per_month() {
        let model = build(&[
            photo("a", 2024, 6, 1),
            photo("b", 2024, 6, 2),
            photo("c", 2024, 7, 1),
        ])
        .unwrap();
        let keys: Vec<_> = model.keys().into_iter().collect();
        assert_eq!(
            keys,
            [
                "index/months/2024-06.json",
                "index/months/2024-07.json",
                "index/timeline.json"
            ]
        );
    }

    #[test]
    fn output_is_deterministic_regardless_of_input_order() {
        let a = build(&[photo("a", 2024, 6, 1), photo("b", 2024, 7, 2)]).unwrap();
        let b = build(&[photo("b", 2024, 7, 2), photo("a", 2024, 6, 1)]).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn empty_library_still_has_an_index() {
        let model = build(&[]).unwrap();
        assert_eq!(model.objects.len(), 1);
        assert!(String::from_utf8_lossy(&model.objects[0].1).contains("\"total\":0"));
    }

    #[test]
    fn stale_lists_only_read_model_objects_that_dropped_out() {
        let model = build(&[photo("a", 2024, 7, 1)]).unwrap();
        let existing = [
            "index/timeline.json",
            "index/months/2024-06.json",
            "index/months/2024-07.json",
            "derived/x/thumbnail.jpeg",
        ];
        assert_eq!(model.stale(existing), ["index/months/2024-06.json"]);
    }
}
