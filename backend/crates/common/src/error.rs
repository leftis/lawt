//! Error type for key validation.

// Rust guideline compliant 2026-07-03

use std::backtrace::Backtrace;
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
enum ErrorKind {
    Empty,
    TooLong,
    OutsideRoot,
    Traversal,
    ControlChar,
    NoFileName,
}

/// Describes why an object key was rejected as an original key.
///
/// Use the `is_*` helpers to react to a specific cause.
#[derive(Debug)]
pub struct KeyError {
    kind: ErrorKind,
    backtrace: Backtrace,
}

impl KeyError {
    fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            backtrace: Backtrace::capture(),
        }
    }

    pub(crate) fn empty() -> Self {
        Self::new(ErrorKind::Empty)
    }
    pub(crate) fn too_long() -> Self {
        Self::new(ErrorKind::TooLong)
    }
    pub(crate) fn outside_root() -> Self {
        Self::new(ErrorKind::OutsideRoot)
    }
    pub(crate) fn traversal() -> Self {
        Self::new(ErrorKind::Traversal)
    }
    pub(crate) fn control_char() -> Self {
        Self::new(ErrorKind::ControlChar)
    }
    pub(crate) fn no_file_name() -> Self {
        Self::new(ErrorKind::NoFileName)
    }

    /// Returns `true` if the key was empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        matches!(self.kind, ErrorKind::Empty)
    }

    /// Returns `true` if the key does not start with the originals prefix.
    #[must_use]
    pub fn is_outside_root(&self) -> bool {
        matches!(self.kind, ErrorKind::OutsideRoot)
    }

    /// Returns `true` if the key contains a `..` or empty path segment.
    #[must_use]
    pub fn is_traversal(&self) -> bool {
        matches!(self.kind, ErrorKind::Traversal)
    }
}

impl Display for KeyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let what = match self.kind {
            ErrorKind::Empty => "key is empty",
            ErrorKind::TooLong => "key exceeds the maximum length",
            ErrorKind::OutsideRoot => "key is not under the originals prefix",
            ErrorKind::Traversal => "key contains an empty or `..` path segment",
            ErrorKind::ControlChar => "key contains a control character",
            ErrorKind::NoFileName => "key has no file name",
        };
        write!(f, "invalid original key: {what}")?;
        if self.backtrace.status() == std::backtrace::BacktraceStatus::Captured {
            write!(f, "\n{}", self.backtrace)?;
        }
        Ok(())
    }
}

impl std::error::Error for KeyError {}
