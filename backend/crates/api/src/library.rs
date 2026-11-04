//! Read access to the timeline read model.

// Rust guideline compliant 2026-07-03

use std::backtrace::Backtrace;
use std::fmt::{self, Display, Formatter};

use lawt_common::{MonthKey, MonthPage, TimelineIndex};

use lawt_common::readmodel::{INDEX_KEY, MONTHS_PREFIX};

#[derive(Debug)]
enum ErrorKind {
    Backend,
    Corrupt,
}

/// Describes why the read model could not be loaded.
#[derive(Debug)]
pub struct StoreError {
    kind: ErrorKind,
    detail: String,
    backtrace: Backtrace,
}

impl StoreError {
    /// Creates an error for a failing storage backend (network, permissions, throttling).
    #[must_use]
    pub fn backend(detail: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Backend,
            detail: detail.into(),
            backtrace: Backtrace::capture(),
        }
    }

    /// Creates an error for stored data that does not parse.
    #[must_use]
    pub fn corrupt(detail: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Corrupt,
            detail: detail.into(),
            backtrace: Backtrace::capture(),
        }
    }

    /// Returns `true` if the stored data was present but malformed.
    #[must_use]
    pub fn is_corrupt(&self) -> bool {
        matches!(self.kind, ErrorKind::Corrupt)
    }
}

impl Display for StoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let what = match self.kind {
            ErrorKind::Backend => "storage backend failed",
            ErrorKind::Corrupt => "stored read model is corrupt",
        };
        write!(f, "{what}: {}", self.detail)?;
        if self.backtrace.status() == std::backtrace::BacktraceStatus::Captured {
            write!(f, "\n{}", self.backtrace)?;
        }
        Ok(())
    }
}

impl std::error::Error for StoreError {}

/// Source of the timeline read model.
pub trait Library: Send + Sync {
    /// Loads the day index. An empty library yields an empty index, not an error.
    ///
    /// # Errors
    /// Returns [`StoreError`] if the backend fails or the data is corrupt.
    fn index(&self) -> impl Future<Output = Result<TimelineIndex, StoreError>> + Send;

    /// Loads one month, or `None` if that month has no photos.
    ///
    /// # Errors
    /// Returns [`StoreError`] if the backend fails or the data is corrupt.
    fn month(
        &self,
        month: &MonthKey,
    ) -> impl Future<Output = Result<Option<MonthPage>, StoreError>> + Send;
}

/// Reads raw bytes by object key; the seam that keeps S3 out of unit tests.
pub trait ObjectReader: Send + Sync {
    /// Returns the object body, or `None` if the key does not exist.
    ///
    /// # Errors
    /// Returns [`StoreError`] on any failure other than "not found".
    fn get(&self, key: &str) -> impl Future<Output = Result<Option<Vec<u8>>, StoreError>> + Send;
}

/// [`Library`] backed by JSON objects: `index/timeline.json` and `index/months/{month}.json`.
#[derive(Debug, Clone)]
pub struct ObjectLibrary<R> {
    reader: R,
}

impl<R: ObjectReader> ObjectLibrary<R> {
    /// Wraps an object reader.
    #[must_use]
    pub fn new(reader: R) -> Self {
        Self { reader }
    }
}

impl<R: ObjectReader> Library for ObjectLibrary<R> {
    async fn index(&self) -> Result<TimelineIndex, StoreError> {
        match self.reader.get(INDEX_KEY).await? {
            Some(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| StoreError::corrupt(format!("{INDEX_KEY}: {e}"))),
            None => Ok(TimelineIndex {
                days: Vec::new(),
                galleries: Vec::new(),
                total: 0,
            }),
        }
    }

    async fn month(&self, month: &MonthKey) -> Result<Option<MonthPage>, StoreError> {
        let key = format!("{MONTHS_PREFIX}{month}.json");
        match self.reader.get(&key).await? {
            Some(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|e| StoreError::corrupt(format!("{key}: {e}"))),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    struct Mem(HashMap<String, Vec<u8>>);

    impl ObjectReader for Mem {
        async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StoreError> {
            Ok(self.0.get(key).cloned())
        }
    }

    #[tokio::test]
    async fn missing_index_is_an_empty_library() {
        let lib = ObjectLibrary::new(Mem(HashMap::new()));
        let idx = lib.index().await.unwrap();
        assert_eq!(idx.total, 0);
        assert!(idx.days.is_empty());
    }

    #[tokio::test]
    async fn corrupt_index_is_reported() {
        let lib = ObjectLibrary::new(Mem(HashMap::from([(
            INDEX_KEY.to_owned(),
            b"{nope".to_vec(),
        )])));
        assert!(lib.index().await.unwrap_err().is_corrupt());
    }

    #[tokio::test]
    async fn missing_month_is_none_and_present_month_parses() {
        let page = br#"{"month":"2024-06","photos":[]}"#.to_vec();
        let lib = ObjectLibrary::new(Mem(HashMap::from([(
            format!("{MONTHS_PREFIX}2024-06.json"),
            page,
        )])));
        let june = MonthKey::parse("2024-06").unwrap();
        let july = MonthKey::parse("2024-07").unwrap();
        assert_eq!(lib.month(&june).await.unwrap().unwrap().month, "2024-06");
        assert!(lib.month(&july).await.unwrap().is_none());
    }
}
