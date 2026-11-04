//! Validated object keys for files in the originals bucket.

// Rust guideline compliant 2026-07-03

use std::fmt::{self, Display, Formatter};

use crate::error::KeyError;

/// Prefix under which every original lives in the photo bucket.
pub const ROOT_PREFIX: &str = "all images/";

/// Gallery name for files that sit directly under [`ROOT_PREFIX`].
pub const ROOT_GALLERY: &str = "all images";

/// S3 limits keys to 1024 bytes; we reject anything longer up front.
const MAX_KEY_BYTES: usize = 1024;

/// Suffixes of derivatives the legacy pipeline wrote next to the originals.
///
/// The indexer must skip these; they are not photos.
const LEGACY_DERIVATIVE_SUFFIXES: &[&str] = &[
    ".thumbnail.jpeg",
    ".medium.jpeg",
    ".full.jpeg",
    ".preview.png",
    ".preview.jpg",
];

/// An object key proven to lie under [`ROOT_PREFIX`] and to be free of traversal.
///
/// Construct it with [`OriginalKey::parse`]. Holding one means the key is safe to
/// pass to a read-only S3 call; it says nothing about whether the object exists.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OriginalKey(String);

impl OriginalKey {
    /// Validates `key` and wraps it.
    ///
    /// # Errors
    /// Returns [`KeyError`] if the key is empty, longer than 1024 bytes, not under
    /// `all images/`, has an empty or `..` path segment, or contains control characters.
    ///
    /// # Examples
    /// ```
    /// use rawvault_common::OriginalKey;
    /// let key = OriginalKey::parse("all images/100RICOH/R0001234.RAF")?;
    /// assert_eq!(key.gallery(), "100RICOH");
    /// assert_eq!(key.stem(), "R0001234");
    /// assert!(OriginalKey::parse("all images/../secret").is_err());
    /// # Ok::<(), rawvault_common::KeyError>(())
    /// ```
    pub fn parse(key: &str) -> Result<Self, KeyError> {
        if key.is_empty() {
            return Err(KeyError::empty());
        }
        if key.len() > MAX_KEY_BYTES {
            return Err(KeyError::too_long());
        }
        if key.chars().any(char::is_control) {
            return Err(KeyError::control_char());
        }
        let Some(rest) = key.strip_prefix(ROOT_PREFIX) else {
            return Err(KeyError::outside_root());
        };
        if rest.split('/').any(|seg| seg.is_empty() || seg == "..") {
            return Err(KeyError::traversal());
        }
        if rest.rsplit('/').next().is_none_or(str::is_empty) {
            return Err(KeyError::no_file_name());
        }
        Ok(Self(key.to_owned()))
    }

    /// Returns the full key.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the path below the prefix, e.g. `100RICOH/R0001234.RAF`.
    #[must_use]
    pub fn relative(&self) -> &str {
        &self.0[ROOT_PREFIX.len()..]
    }

    /// Returns the folder path below the prefix, or `""` for root-level files.
    #[must_use]
    pub fn dir(&self) -> &str {
        self.relative().rsplit_once('/').map_or("", |(dir, _)| dir)
    }

    /// Returns the first folder, or [`ROOT_GALLERY`] for root-level files.
    #[must_use]
    pub fn gallery(&self) -> &str {
        match self.relative().split_once('/') {
            Some((first, _)) => first,
            None => ROOT_GALLERY,
        }
    }

    /// Returns the file name including extension.
    #[must_use]
    pub fn file_name(&self) -> &str {
        self.relative().rsplit('/').next().unwrap_or_default()
    }

    /// Returns the file name without its final extension.
    #[must_use]
    pub fn stem(&self) -> &str {
        let name = self.file_name();
        name.rsplit_once('.').map_or(name, |(stem, _)| stem)
    }

    /// Returns the lowercase extension, or `""` if there is none.
    #[must_use]
    pub fn extension(&self) -> String {
        self.file_name()
            .rsplit_once('.')
            .map(|(_, ext)| ext.to_ascii_lowercase())
            .unwrap_or_default()
    }
}

impl Display for OriginalKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Returns `true` if `key` is a derivative written by the legacy pipeline.
#[must_use]
pub fn is_legacy_derivative(key: &str) -> bool {
    LEGACY_DERIVATIVE_SUFFIXES
        .iter()
        .any(|suffix| key.ends_with(suffix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_and_root_files() {
        let k = OriginalKey::parse("all images/100RICOH/R0001.RAF").unwrap();
        assert_eq!(
            (k.gallery(), k.dir(), k.stem(), k.extension().as_str()),
            ("100RICOH", "100RICOH", "R0001", "raf")
        );

        let root = OriginalKey::parse("all images/loose.JPG").unwrap();
        assert_eq!(
            (root.gallery(), root.dir(), root.extension().as_str()),
            (ROOT_GALLERY, "", "jpg")
        );

        let nested = OriginalKey::parse("all images/2024/100RICOH/a.b.tif").unwrap();
        assert_eq!(
            (nested.gallery(), nested.dir(), nested.stem()),
            ("2024", "2024/100RICOH", "a.b")
        );
    }

    #[test]
    fn rejects_hostile_and_malformed_keys() {
        for bad in [
            "",
            "other/x.jpg",
            "all images/",
            "all images//x.jpg",
            "all images/../x.jpg",
            "all images/a/../../x.jpg",
            "all images/a/",
            "all images/a\nb.jpg",
        ] {
            assert!(
                OriginalKey::parse(bad).is_err(),
                "{bad:?} should be rejected"
            );
        }
        let _ = OriginalKey::parse(&format!("all images/{}", "a".repeat(1100))).unwrap_err();
        assert!(OriginalKey::parse("x").unwrap_err().is_outside_root());
        assert!(
            OriginalKey::parse("all images/../x")
                .unwrap_err()
                .is_traversal()
        );
    }

    #[test]
    fn detects_legacy_derivatives() {
        assert!(is_legacy_derivative("all images/a/R1.RAF.thumbnail.jpeg"));
        assert!(is_legacy_derivative("all images/a/R1.RAF.preview.png"));
        assert!(!is_legacy_derivative("all images/a/R1.RAF"));
    }
}
