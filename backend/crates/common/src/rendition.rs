//! Generated renditions of a photo and where they are stored.
//!
//! Renditions live in a separate derived bucket, never next to the originals.

// Rust guideline compliant 2026-07-03

use crate::asset::AssetId;
#[cfg(test)]
use crate::asset::IdSalt;

/// Prefix for all generated files in the derived bucket.
pub const DERIVED_PREFIX: &str = "derived/";

/// Long edge of the timeline thumbnail, in pixels.
///
/// Sized for a retina grid cell of ~200 CSS px; raising it costs bandwidth on
/// every timeline load, lowering it blurs the grid on high-DPI screens.
pub const THUMBNAIL_EDGE: u32 = 400;

/// Long edge of the viewer image, in pixels (fits a 1080p screen).
pub const MEDIUM_EDGE: u32 = 1920;

/// A generated JPEG derived from a photo's original.
///
/// The original itself is never a `Rendition`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Rendition {
    /// Small image for the timeline grid.
    Thumbnail,
    /// Larger image for the fullscreen viewer.
    Medium,
    /// Full-resolution, browser-renderable JPEG.
    Full,
}

impl Rendition {
    /// All renditions, smallest first.
    pub const ALL: [Self; 3] = [Self::Thumbnail, Self::Medium, Self::Full];

    /// Returns the file name used for this rendition.
    #[must_use]
    pub fn file_name(self) -> &'static str {
        match self {
            Self::Thumbnail => "thumbnail.jpeg",
            Self::Medium => "medium.jpeg",
            Self::Full => "full.jpeg",
        }
    }

    /// Returns the maximum long edge in pixels, or `None` for full size.
    #[must_use]
    pub fn max_edge(self) -> Option<u32> {
        match self {
            Self::Thumbnail => Some(THUMBNAIL_EDGE),
            Self::Medium => Some(MEDIUM_EDGE),
            Self::Full => None,
        }
    }

    /// Returns the object key of this rendition in the derived bucket.
    ///
    /// # Examples
    /// ```
    /// use lawt_common::{IdSalt, OriginalKey, Rendition, group_assets};
    /// let key = OriginalKey::parse("all images/100RICOH/R0001.RAF")?;
    /// let asset = group_assets([key], &IdSalt::insecure_dev()).assets.remove(0);
    /// let k = Rendition::Thumbnail.derived_key(&asset.id);
    /// assert!(k.starts_with("derived/") && k.ends_with("/thumbnail.jpeg"));
    /// # Ok::<(), lawt_common::KeyError>(())
    /// ```
    #[must_use]
    pub fn derived_key(self, id: &AssetId) -> String {
        format!("{DERIVED_PREFIX}{id}/{}", self.file_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_stable_and_distinct() {
        let id = AssetId::for_asset(&IdSalt::insecure_dev(), "100RICOH", "R0001");
        let keys: Vec<_> = Rendition::ALL.iter().map(|r| r.derived_key(&id)).collect();
        assert_eq!(keys.len(), 3);
        assert_ne!(keys[0], keys[1]);
        assert!(keys.iter().all(|k| k.starts_with("derived/")));
        assert_eq!(keys[0], Rendition::Thumbnail.derived_key(&id));
    }

    #[test]
    fn only_full_is_unbounded() {
        assert_eq!(Rendition::Thumbnail.max_edge(), Some(THUMBNAIL_EDGE));
        assert_eq!(Rendition::Full.max_edge(), None);
    }
}
