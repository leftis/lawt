//! File formats `lawt` understands and how they rank as a photo's original.

// Rust guideline compliant 2026-07-03

/// Broad class of an uploaded image file.
///
/// Variant order does not matter; use [`FileKind::rank`] for preference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FileKind {
    /// Camera RAW (RAF, NEF, ARW, DNG, ...).
    Raw,
    /// TIFF, often a 16-bit export or scan.
    Tiff,
    /// JPEG.
    Jpeg,
    /// PNG.
    Png,
    /// HEIC or HEIF.
    Heic,
    /// WebP.
    WebP,
    /// GIF.
    Gif,
    /// BMP.
    Bmp,
}

/// Lowercase RAW extensions. Single source of truth for the whole backend.
const RAW_EXTENSIONS: &[&str] = &[
    "raf", "nef", "cr2", "cr3", "arw", "dng", "orf", "sr2", "3fr", "fff", "mos", "ia", "pef",
    "rw2", "rwl", "rwz", "x3f", "srw", "k25", "kdc",
];

impl FileKind {
    /// Classifies a file extension, ignoring case and a leading dot.
    ///
    /// Returns `None` for anything that is not a supported image type.
    ///
    /// # Examples
    /// ```
    /// use lawt_common::FileKind;
    /// assert_eq!(FileKind::from_extension("RAF"), Some(FileKind::Raw));
    /// assert_eq!(FileKind::from_extension(".jpeg"), Some(FileKind::Jpeg));
    /// assert_eq!(FileKind::from_extension("txt"), None);
    /// ```
    #[must_use]
    pub fn from_extension(ext: &str) -> Option<Self> {
        let ext = ext.trim_start_matches('.').to_ascii_lowercase();
        if RAW_EXTENSIONS.contains(&ext.as_str()) {
            return Some(Self::Raw);
        }
        match ext.as_str() {
            "tif" | "tiff" => Some(Self::Tiff),
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "png" => Some(Self::Png),
            "heic" | "heif" => Some(Self::Heic),
            "webp" => Some(Self::WebP),
            "gif" => Some(Self::Gif),
            "bmp" => Some(Self::Bmp),
            _ => None,
        }
    }

    /// Classifies the file name at the end of an object key.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        let name = key.rsplit('/').next()?;
        let (_, ext) = name.rsplit_once('.')?;
        Self::from_extension(ext)
    }

    /// Returns `true` for camera RAW files.
    #[must_use]
    pub fn is_raw(self) -> bool {
        self == Self::Raw
    }

    /// Preference when several files share one photo; lower wins.
    ///
    /// RAW beats TIFF beats everything else, so the richest file is the
    /// primary original and the rest become companions.
    #[must_use]
    pub fn rank(self) -> u8 {
        match self {
            Self::Raw => 0,
            Self::Tiff => 1,
            _ => 2,
        }
    }

    /// Returns `true` if browsers can display the format directly.
    #[must_use]
    pub fn browser_renderable(self) -> bool {
        matches!(
            self,
            Self::Jpeg | Self::Png | Self::WebP | Self::Gif | Self::Bmp
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_every_raw_extension() {
        for ext in RAW_EXTENSIONS {
            assert_eq!(FileKind::from_extension(ext), Some(FileKind::Raw), "{ext}");
        }
    }

    #[test]
    fn classifies_by_key_and_ignores_case() {
        assert_eq!(
            FileKind::from_key("all images/100RICOH/R01.RAF"),
            Some(FileKind::Raw)
        );
        assert_eq!(FileKind::from_key("a/b.TiFf"), Some(FileKind::Tiff));
        assert_eq!(FileKind::from_key("a/b"), None);
        assert_eq!(FileKind::from_key("a.dir/b"), None);
    }

    #[test]
    fn raw_outranks_tiff_outranks_jpeg() {
        assert!(FileKind::Raw.rank() < FileKind::Tiff.rank());
        assert!(FileKind::Tiff.rank() < FileKind::Jpeg.rank());
    }

    #[test]
    fn raw_tiff_and_heic_need_conversion() {
        assert!(!FileKind::Raw.browser_renderable());
        assert!(!FileKind::Tiff.browser_renderable());
        assert!(!FileKind::Heic.browser_renderable());
        assert!(FileKind::Jpeg.browser_renderable());
    }
}
