//! Grouping several files of one shot into a single photo.

// Rust guideline compliant 2026-07-03

use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::format::FileKind;
use crate::key::OriginalKey;

/// Bytes of the digest kept in an id (128 bits: collision-free at any realistic library size).
const ID_BYTES: usize = 16;

/// Shortest accepted salt; anything shorter would make ids guessable by brute force.
const MIN_SALT_BYTES: usize = 16;

/// Secret mixed into every [`AssetId`] so the ids, and therefore rendition URLs, cannot be guessed.
///
/// Without it, ids would be a plain hash of a folder and file name, which anyone who knows a
/// camera's naming scheme could compute. Changing the salt changes every id, so renditions
/// would have to be regenerated; keep it stable once the library has been processed.
#[derive(Clone, PartialEq, Eq)]
pub struct IdSalt(Vec<u8>);

impl IdSalt {
    /// Wraps a secret of at least 16 bytes; `None` if it is too short.
    #[must_use]
    pub fn new(secret: &str) -> Option<Self> {
        (secret.len() >= MIN_SALT_BYTES).then(|| Self(secret.as_bytes().to_vec()))
    }

    /// A fixed, publicly known salt for local development and tests. Never use it in production.
    #[must_use]
    pub fn insecure_dev() -> Self {
        Self(b"lawt-dev-only-salt".to_vec())
    }
}

impl std::fmt::Debug for IdSalt {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("IdSalt(..)")
    }
}

/// Stable identifier of a photo, derived from a secret salt, its folder and its file stem.
///
/// The same salt, folder and stem always give the same id, so re-indexing is idempotent
/// and derived keys never need encoding.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AssetId(String);

impl AssetId {
    /// Derives the id for the photo at `dir` with file stem `stem`.
    ///
    /// # Panics
    /// Never in practice: HMAC-SHA256 accepts keys of any length, so key setup cannot fail.
    #[must_use]
    pub fn for_asset(salt: &IdSalt, dir: &str, stem: &str) -> Self {
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&salt.0)
            .expect("HMAC accepts keys of any length");
        mac.update(dir.as_bytes());
        // A byte that cannot occur in a path segment keeps ("a/b","c") != ("a","b/c").
        mac.update(&[0]);
        mac.update(stem.as_bytes());
        let digest = mac.finalize().into_bytes();
        let hex =
            digest[..ID_BYTES]
                .iter()
                .fold(String::with_capacity(ID_BYTES * 2), |mut acc, b| {
                    use std::fmt::Write as _;
                    let _ = write!(acc, "{b:02x}");
                    acc
                });
        Self(hex)
    }

    /// Returns the id as 32 lowercase hex characters.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for AssetId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One photo: a primary original plus any companion files of the same shot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asset {
    /// Stable identifier.
    pub id: AssetId,
    /// First folder below the originals prefix.
    pub gallery: String,
    /// Full folder path below the originals prefix.
    pub dir: String,
    /// File name without extension, shared by all files of the photo.
    pub stem: String,
    /// The richest file: RAW, else TIFF, else the first other file by key.
    pub primary: OriginalKey,
    /// Kind of the primary original.
    pub kind: FileKind,
    /// Other files of the same shot (e.g. the camera JPEG next to a RAW), sorted.
    pub companions: Vec<OriginalKey>,
}

/// Result of [`group_assets`].
#[derive(Debug, Default)]
pub struct Grouping {
    /// Photos, sorted by folder then stem.
    pub assets: Vec<Asset>,
    /// Valid keys that are not a supported image type.
    pub unsupported: Vec<OriginalKey>,
}

/// Groups original keys into photos by folder and file stem.
///
/// Stems are compared exactly (case-sensitive) so two distinct files are never merged
/// by accident. Legacy derivative keys must be filtered out by the caller.
///
/// # Examples
/// ```
/// use lawt_common::{OriginalKey, IdSalt, group_assets, FileKind};
/// let keys = ["all images/1/R1.RAF", "all images/1/R1.JPG", "all images/1/R2.JPG"]
///     .map(|k| OriginalKey::parse(k).unwrap());
/// let g = group_assets(keys, &IdSalt::insecure_dev());
/// assert_eq!(g.assets.len(), 2);
/// assert_eq!(g.assets[0].kind, FileKind::Raw);
/// assert_eq!(g.assets[0].companions.len(), 1);
/// ```
#[must_use]
pub fn group_assets(keys: impl IntoIterator<Item = OriginalKey>, salt: &IdSalt) -> Grouping {
    let mut by_asset: BTreeMap<(String, String), Vec<(FileKind, OriginalKey)>> = BTreeMap::new();
    let mut unsupported = Vec::new();

    for key in keys {
        match FileKind::from_key(key.as_str()) {
            Some(kind) => by_asset
                .entry((key.dir().to_owned(), key.stem().to_owned()))
                .or_default()
                .push((kind, key)),
            None => unsupported.push(key),
        }
    }

    let assets = by_asset
        .into_iter()
        .filter_map(|((dir, stem), mut files)| {
            files.sort_by(|(ka, a), (kb, b)| ka.rank().cmp(&kb.rank()).then_with(|| a.cmp(b)));
            files.dedup_by(|(_, a), (_, b)| a == b);
            let mut files = files.into_iter();
            let (kind, primary) = files.next()?;
            Some(Asset {
                id: AssetId::for_asset(salt, &dir, &stem),
                gallery: primary.gallery().to_owned(),
                dir,
                stem,
                primary,
                kind,
                companions: files.map(|(_, k)| k).collect(),
            })
        })
        .collect();

    Grouping {
        assets,
        unsupported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn salt() -> IdSalt {
        IdSalt::insecure_dev()
    }

    fn keys(list: &[&str]) -> Vec<OriginalKey> {
        list.iter()
            .map(|k| OriginalKey::parse(k).unwrap())
            .collect()
    }

    #[test]
    fn raw_plus_jpeg_is_one_photo_with_raw_primary() {
        let g = group_assets(
            keys(&["all images/1/R1.JPG", "all images/1/R1.RAF"]),
            &IdSalt::insecure_dev(),
        );
        assert_eq!(g.assets.len(), 1);
        let a = &g.assets[0];
        assert_eq!(a.kind, FileKind::Raw);
        assert_eq!(a.primary.as_str(), "all images/1/R1.RAF");
        assert_eq!(a.companions[0].as_str(), "all images/1/R1.JPG");
    }

    #[test]
    fn tiff_beats_jpeg_but_not_raw() {
        let g = group_assets(
            keys(&["all images/1/a.jpg", "all images/1/a.tif"]),
            &IdSalt::insecure_dev(),
        );
        assert_eq!(g.assets[0].kind, FileKind::Tiff);
        let g = group_assets(
            keys(&[
                "all images/1/a.tif",
                "all images/1/a.nef",
                "all images/1/a.jpg",
            ]),
            &IdSalt::insecure_dev(),
        );
        assert_eq!(g.assets[0].kind, FileKind::Raw);
        assert_eq!(g.assets[0].companions.len(), 2);
    }

    #[test]
    fn same_stem_in_different_folders_stays_separate() {
        let g = group_assets(
            keys(&["all images/1/R1.RAF", "all images/2/R1.RAF"]),
            &IdSalt::insecure_dev(),
        );
        assert_eq!(g.assets.len(), 2);
        assert_ne!(g.assets[0].id, g.assets[1].id);
    }

    #[test]
    fn unsupported_files_are_reported_not_grouped() {
        let g = group_assets(
            keys(&["all images/1/notes.txt", "all images/1/a.jpg"]),
            &IdSalt::insecure_dev(),
        );
        assert_eq!(g.assets.len(), 1);
        assert_eq!(g.unsupported.len(), 1);
    }

    #[test]
    fn a_different_salt_gives_different_ids_and_short_salts_are_refused() {
        let other = IdSalt::new("another-secret-of-decent-length").unwrap();
        assert_ne!(
            AssetId::for_asset(&salt(), "a", "b"),
            AssetId::for_asset(&other, "a", "b")
        );
        assert!(IdSalt::new("short").is_none());
    }

    #[test]
    fn salt_debug_does_not_leak_the_secret() {
        let secret = "a-very-secret-salt-value";
        let shown = format!("{:?}", IdSalt::new(secret).unwrap());
        assert!(!shown.contains(secret));
    }

    #[test]
    fn ids_are_deterministic_and_boundary_safe() {
        assert_eq!(
            AssetId::for_asset(&salt(), "a", "b"),
            AssetId::for_asset(&salt(), "a", "b")
        );
        assert_ne!(
            AssetId::for_asset(&salt(), "a/b", "c"),
            AssetId::for_asset(&salt(), "a", "b/c")
        );
        assert_eq!(AssetId::for_asset(&salt(), "a", "b").as_str().len(), 32);
    }

    #[test]
    fn output_is_sorted_and_input_order_does_not_matter() {
        let a = group_assets(
            keys(&["all images/2/z.jpg", "all images/1/a.jpg"]),
            &IdSalt::insecure_dev(),
        );
        let b = group_assets(
            keys(&["all images/1/a.jpg", "all images/2/z.jpg"]),
            &IdSalt::insecure_dev(),
        );
        assert_eq!(a.assets, b.assets);
        assert_eq!(a.assets[0].dir, "1");
    }
}
