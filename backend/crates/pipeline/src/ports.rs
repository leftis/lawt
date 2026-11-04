//! Storage ports. Real S3 adapters and in-memory fakes both implement these.

// Rust guideline compliant 2026-07-03

use std::backtrace::Backtrace;
use std::fmt::{self, Display, Formatter};

use chrono::NaiveDateTime;
use rawvault_common::OriginalKey;

/// Size and modification time of one listed object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectInfo {
    /// Full object key.
    pub key: String,
    /// Size in bytes.
    pub size: u64,
    /// Last modified time (UTC).
    pub modified: NaiveDateTime,
}

/// Storage failure reported by a port.
#[derive(Debug)]
pub struct PortError {
    detail: String,
    backtrace: Backtrace,
}

impl PortError {
    /// Wraps a description of what failed.
    #[must_use]
    pub fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
            backtrace: Backtrace::capture(),
        }
    }
}

impl Display for PortError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "storage error: {}", self.detail)?;
        if self.backtrace.status() == std::backtrace::BacktraceStatus::Captured {
            write!(f, "\n{}", self.backtrace)?;
        }
        Ok(())
    }
}

impl std::error::Error for PortError {}

/// Read-only view of the originals bucket.
///
/// There is deliberately no way to write, copy or delete through this trait.
pub trait Originals: Send + Sync {
    /// Lists every object under `prefix`.
    ///
    /// # Errors
    /// Returns [`PortError`] if the listing fails.
    fn list(&self, prefix: &str)
    -> impl Future<Output = Result<Vec<ObjectInfo>, PortError>> + Send;

    /// Downloads one original.
    ///
    /// # Errors
    /// Returns [`PortError`] if the object cannot be read.
    fn get(&self, key: &OriginalKey) -> impl Future<Output = Result<Vec<u8>, PortError>> + Send;
}

/// Read-write access to the derived bucket (renditions, per-photo records, read model).
pub trait Derived: Send + Sync {
    /// Returns the object body, or `None` if absent.
    ///
    /// # Errors
    /// Returns [`PortError`] on failures other than "not found".
    fn get(&self, key: &str) -> impl Future<Output = Result<Option<Vec<u8>>, PortError>> + Send;

    /// Stores an object with the given content type and cache policy.
    ///
    /// # Errors
    /// Returns [`PortError`] if the write fails.
    fn put(
        &self,
        key: &str,
        body: Vec<u8>,
        content_type: &str,
        cache_control: &str,
    ) -> impl Future<Output = Result<(), PortError>> + Send;

    /// Lists all keys under `prefix`.
    ///
    /// # Errors
    /// Returns [`PortError`] if the listing fails.
    fn list(&self, prefix: &str) -> impl Future<Output = Result<Vec<String>, PortError>> + Send;

    /// Deletes the given keys.
    ///
    /// # Errors
    /// Returns [`PortError`] if a delete fails.
    fn delete(&self, keys: &[String]) -> impl Future<Output = Result<(), PortError>> + Send;
}
