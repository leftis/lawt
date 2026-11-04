//! Error type for image processing.

// Rust guideline compliant 2026-07-03

use std::backtrace::Backtrace;
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
enum ErrorKind {
    Unsupported,
    Decode,
    Encode,
}

/// Describes why an original could not be turned into renditions.
#[derive(Debug)]
pub struct ProcessError {
    kind: ErrorKind,
    detail: String,
    backtrace: Backtrace,
}

impl ProcessError {
    fn new(kind: ErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
            backtrace: Backtrace::capture(),
        }
    }

    pub(crate) fn unsupported(detail: impl Into<String>) -> Self {
        Self::new(ErrorKind::Unsupported, detail)
    }

    pub(crate) fn decode(detail: impl Into<String>) -> Self {
        Self::new(ErrorKind::Decode, detail)
    }

    pub(crate) fn encode(detail: impl Into<String>) -> Self {
        Self::new(ErrorKind::Encode, detail)
    }

    /// Wraps a panic or task failure that happened while rendering; treated as a decode failure.
    #[must_use]
    pub fn from_panic(detail: &str) -> Self {
        Self::decode(format!("renderer aborted: {detail}"))
    }

    /// Returns `true` if the format is known but not processed (e.g. HEIC).
    #[must_use]
    pub fn is_unsupported(&self) -> bool {
        matches!(self.kind, ErrorKind::Unsupported)
    }

    /// Returns `true` if the file could not be decoded (corrupt or unrecognised content).
    #[must_use]
    pub fn is_decode(&self) -> bool {
        matches!(self.kind, ErrorKind::Decode)
    }
}

impl Display for ProcessError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let what = match self.kind {
            ErrorKind::Unsupported => "unsupported format",
            ErrorKind::Decode => "could not decode image",
            ErrorKind::Encode => "could not encode rendition",
        };
        write!(f, "{what}: {}", self.detail)?;
        if self.backtrace.status() == std::backtrace::BacktraceStatus::Captured {
            write!(f, "\n{}", self.backtrace)?;
        }
        Ok(())
    }
}

impl std::error::Error for ProcessError {}
