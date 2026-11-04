//! Timeline model: when a photo was taken and how days are laid out.
//!
//! The UI loads a small day index first, then one file per month, so these
//! types are the API contract between the indexer and the frontend.

// Rust guideline compliant 2026-07-03

use std::collections::BTreeMap;

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

/// Earliest year accepted as a real EXIF capture date.
///
/// Cameras with a dead clock battery write 1970 or 2000-01-01; those photos fall
/// back to the upload time so they do not pile up at the bottom of the timeline.
const MIN_PLAUSIBLE_YEAR: i32 = 1990;

/// How far past the upload time an EXIF date may be before it is distrusted.
///
/// Absorbs timezone offsets (EXIF has no zone) while rejecting cameras set to the future.
const FUTURE_TOLERANCE_DAYS: i64 = 2;

/// Where a photo's `taken_at` came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum TakenAtSource {
    /// EXIF `DateTimeOriginal`, in the camera's wall-clock time.
    Exif,
    /// Object upload time; used when EXIF is missing or implausible.
    Uploaded,
}

/// One photo as the timeline shows it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Photo {
    /// Stable id, also the folder of its renditions in the derived bucket.
    pub id: String,
    /// Object key of the primary original (used for "download original").
    pub original_key: String,
    /// First folder of the original's path.
    pub gallery: String,
    /// File name without extension.
    pub stem: String,
    /// Lowercase extension of the primary original.
    pub ext: String,
    /// `true` when the primary original is camera RAW.
    pub is_raw: bool,
    /// Width of the rendered (EXIF-rotated) image in pixels.
    pub width: u32,
    /// Height of the rendered (EXIF-rotated) image in pixels.
    pub height: u32,
    /// Capture time in camera wall-clock time (no timezone).
    pub taken_at: NaiveDateTime,
    /// Origin of `taken_at`.
    pub taken_at_source: TakenAtSource,
    /// Size of the primary original in bytes (JSON number; safe below 2^53).
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub size_bytes: u64,
    /// `true` once the thumbnail rendition exists.
    pub has_thumbnail: bool,
    /// `true` when a derived full-size JPEG exists. JPEG originals serve as their own full size, so it is `false` for them.
    pub has_full: bool,
}

/// One row of the day index: a calendar day and how many photos it holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct TimelineDay {
    /// Calendar day in camera wall-clock time.
    pub day: NaiveDate,
    /// Number of photos taken that day.
    pub n: u32,
}

/// Response of `GET /timeline`: everything needed to lay out the whole scroll bar.
///
/// It is small (a few KB for ten years) and holds no photos; month pages are
/// fetched on demand with [`MonthPage`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct TimelineIndex {
    /// Days that contain photos, newest first.
    pub days: Vec<TimelineDay>,
    /// Gallery (camera folder) names, sorted, for the filter.
    pub galleries: Vec<String>,
    /// Total number of photos in the library.
    pub total: u32,
}

/// Response of `GET /timeline/months/{YYYY-MM}`: all photos of one month, newest first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct MonthPage {
    /// Month key, `YYYY-MM`.
    pub month: String,
    /// Photos of the month, newest first.
    pub photos: Vec<Photo>,
}

/// A validated `YYYY-MM` month key.
///
/// Used in URLs and object keys, so it is checked strictly: four-digit year
/// 1990–2100 and month 01–12.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MonthKey(String);

impl MonthKey {
    /// Parses and validates a month key.
    ///
    /// # Examples
    /// ```
    /// use lawt_common::timeline::MonthKey;
    /// assert_eq!(MonthKey::parse("2024-06").map(|m| m.to_string()), Some("2024-06".to_owned()));
    /// assert!(MonthKey::parse("2024-13").is_none());
    /// assert!(MonthKey::parse("../etc").is_none());
    /// ```
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let (y, m) = s.split_once('-')?;
        let well_formed = y.len() == 4
            && m.len() == 2
            && y.bytes().all(|b| b.is_ascii_digit())
            && m.bytes().all(|b| b.is_ascii_digit());
        if !well_formed {
            return None;
        }
        let (year, month): (i32, u32) = (y.parse().ok()?, m.parse().ok()?);
        ((MIN_PLAUSIBLE_YEAR..=2100).contains(&year) && (1..=12).contains(&month))
            .then(|| Self(s.to_owned()))
    }

    /// Returns the key as `YYYY-MM`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for MonthKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Builds the [`TimelineIndex`] for a set of photos.
#[must_use]
pub fn build_index(photos: &[Photo]) -> TimelineIndex {
    let galleries: std::collections::BTreeSet<&str> =
        photos.iter().map(|p| p.gallery.as_str()).collect();
    TimelineIndex {
        days: build_timeline(photos),
        galleries: galleries.into_iter().map(str::to_owned).collect(),
        total: u32::try_from(photos.len()).unwrap_or(u32::MAX),
    }
}

/// Chooses a photo's capture time from EXIF data and the upload time.
///
/// EXIF wins unless it is before 1990 or more than two days after `uploaded`.
///
/// # Examples
/// ```
/// use chrono::NaiveDate;
/// use lawt_common::{resolve_taken_at, TakenAtSource};
/// let up = NaiveDate::from_ymd_opt(2024, 7, 1).unwrap().and_hms_opt(9, 0, 0).unwrap();
/// let shot = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap().and_hms_opt(10, 23, 0).unwrap();
/// assert_eq!(resolve_taken_at(Some(shot), up), (shot, TakenAtSource::Exif));
/// assert_eq!(resolve_taken_at(None, up), (up, TakenAtSource::Uploaded));
/// ```
#[must_use]
pub fn resolve_taken_at(
    exif: Option<NaiveDateTime>,
    uploaded: NaiveDateTime,
) -> (NaiveDateTime, TakenAtSource) {
    match exif {
        Some(t)
            if t.year() >= MIN_PLAUSIBLE_YEAR
                && t <= uploaded + Duration::days(FUTURE_TOLERANCE_DAYS) =>
        {
            (t, TakenAtSource::Exif)
        }
        _ => (uploaded, TakenAtSource::Uploaded),
    }
}

/// Builds the day index, newest day first.
#[must_use]
pub fn build_timeline(photos: &[Photo]) -> Vec<TimelineDay> {
    let mut counts: BTreeMap<NaiveDate, u32> = BTreeMap::new();
    for p in photos {
        *counts.entry(p.taken_at.date()).or_default() += 1;
    }
    counts
        .into_iter()
        .rev()
        .map(|(day, n)| TimelineDay { day, n })
        .collect()
}

/// Splits photos into months keyed `YYYY-MM`; each month is sorted newest first.
///
/// Ties on `taken_at` break on `id` so the output is deterministic and the
/// generated JSON only changes when the data does.
#[must_use]
pub fn group_by_month(photos: &[Photo]) -> BTreeMap<String, Vec<Photo>> {
    let mut months: BTreeMap<String, Vec<Photo>> = BTreeMap::new();
    for p in photos {
        months
            .entry(month_key(p.taken_at))
            .or_default()
            .push(p.clone());
    }
    for list in months.values_mut() {
        list.sort_by(|a, b| b.taken_at.cmp(&a.taken_at).then_with(|| a.id.cmp(&b.id)));
    }
    months
}

/// Returns the `YYYY-MM` key of the month containing `t`.
#[must_use]
pub fn month_key(t: NaiveDateTime) -> String {
    format!("{:04}-{:02}", t.year(), t.month())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(y: i32, m: u32, d: u32, h: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, 0, 0)
            .unwrap()
    }

    fn photo(id: &str, taken: NaiveDateTime) -> Photo {
        Photo {
            id: id.into(),
            original_key: format!("all images/g/{id}.raf"),
            gallery: "g".into(),
            stem: id.into(),
            ext: "raf".into(),
            is_raw: true,
            width: 6000,
            height: 4000,
            taken_at: taken,
            taken_at_source: TakenAtSource::Exif,
            size_bytes: 1,
            has_thumbnail: true,
            has_full: true,
        }
    }

    #[test]
    fn distrusts_dead_battery_and_future_dates() {
        let up = dt(2024, 7, 1, 9);
        assert_eq!(
            resolve_taken_at(Some(dt(1970, 1, 1, 0)), up).1,
            TakenAtSource::Uploaded
        );
        assert_eq!(
            resolve_taken_at(Some(dt(2030, 1, 1, 0)), up).1,
            TakenAtSource::Uploaded
        );
        // Within tolerance (camera zone ahead of upload zone) is still trusted.
        assert_eq!(
            resolve_taken_at(Some(dt(2024, 7, 2, 0)), up).1,
            TakenAtSource::Exif
        );
    }

    #[test]
    fn timeline_counts_days_newest_first() {
        let photos = [
            photo("a", dt(2024, 6, 15, 10)),
            photo("b", dt(2024, 6, 15, 18)),
            photo("c", dt(2024, 6, 16, 8)),
        ];
        let days = build_timeline(&photos);
        assert_eq!(days.len(), 2);
        assert_eq!((days[0].day.day(), days[0].n), (16, 1));
        assert_eq!((days[1].day.day(), days[1].n), (15, 2));
    }

    #[test]
    fn months_are_keyed_and_sorted_deterministically() {
        let photos = [
            photo("b", dt(2024, 6, 15, 10)),
            photo("a", dt(2024, 6, 15, 10)),
            photo("c", dt(2024, 7, 1, 1)),
        ];
        let months = group_by_month(&photos);
        assert_eq!(months.keys().collect::<Vec<_>>(), ["2024-06", "2024-07"]);
        let june: Vec<_> = months["2024-06"].iter().map(|p| p.id.as_str()).collect();
        assert_eq!(june, ["a", "b"]);
    }

    #[test]
    fn month_keys_are_strict() {
        for ok in ["2024-06", "1990-01", "2100-12"] {
            assert!(MonthKey::parse(ok).is_some(), "{ok}");
        }
        for bad in [
            "",
            "2024-6",
            "24-06",
            "2024-00",
            "2024-13",
            "1989-12",
            "2101-01",
            "2024/06",
            "2024-06-01",
            "+024-06",
            "../../x",
        ] {
            assert!(MonthKey::parse(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn index_lists_galleries_and_total() {
        let mut a = photo("a", dt(2024, 6, 15, 10));
        a.gallery = "200RICOH".into();
        let idx = build_index(&[a, photo("b", dt(2024, 6, 16, 10))]);
        assert_eq!(
            (idx.total, idx.galleries.as_slice()),
            (2, ["200RICOH".to_owned(), "g".to_owned()].as_slice())
        );
        assert_eq!(idx.days.len(), 2);
    }

    #[test]
    fn photo_json_round_trips() {
        let p = photo("a", dt(2024, 6, 15, 10));
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"taken_at_source\":\"exif\""));
        assert_eq!(serde_json::from_str::<Photo>(&json).unwrap(), p);
    }
}
