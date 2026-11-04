//! Step 1: work out what needs rendering.

// Rust guideline compliant 2026-07-03

use std::collections::{BTreeSet, HashMap};

use rawvault_common::key::{ROOT_PREFIX, is_legacy_derivative};
use rawvault_common::{Asset, IdSalt, OriginalKey, group_assets};

use crate::ports::{Derived, ObjectInfo, Originals, PortError};
use crate::record::{RECORD_PREFIX, record_key};

/// Which photos to consider.
#[derive(Clone, Debug)]
pub struct Options {
    /// Secret that makes photo ids (and so rendition URLs) unguessable.
    pub salt: IdSalt,
    /// Re-render photos that already have a record.
    pub force: bool,
    /// Only photos whose folder path starts with this (e.g. `100RICOH`); everything when `None`.
    pub folder: Option<String>,
    /// Stop after this many photos to render; unlimited when `None`.
    pub limit: Option<usize>,
}

impl Options {
    /// Options that consider every photo, with no limit, using `salt` for ids.
    #[must_use]
    pub fn new(salt: IdSalt) -> Self {
        Self {
            salt,
            force: false,
            folder: None,
            limit: None,
        }
    }
}

/// A photo that needs rendering, with the listing entry of its primary original.
#[derive(Clone, Debug)]
pub struct Todo {
    /// The photo and its files.
    pub asset: Asset,
    /// Listing entry of the primary original.
    pub info: ObjectInfo,
}

/// What [`plan`] found.
#[derive(Debug, Default)]
pub struct Plan {
    /// Photos in scope (after the folder filter).
    pub assets: usize,
    /// Photos to render now (already truncated by `limit`).
    pub todo: Vec<Todo>,
    /// Photos still to render after this batch because of `limit`.
    pub remaining: usize,
    /// Files skipped because they are not supported images.
    pub unsupported_files: usize,
    /// Files skipped because they are derivatives of the legacy pipeline.
    pub legacy_derivatives: usize,
    /// Keys skipped because they failed validation.
    pub invalid_keys: usize,
}

/// Lists the originals and decides which photos need rendering.
///
/// Only reads: it lists the originals bucket and the derived records.
///
/// # Errors
/// Returns [`PortError`] if either listing fails.
pub async fn plan(
    originals: &impl Originals,
    derived: &impl Derived,
    options: &Options,
) -> Result<Plan, PortError> {
    let listing = originals.list(ROOT_PREFIX).await?;
    let mut plan = Plan::default();
    let mut info_by_key: HashMap<String, ObjectInfo> = HashMap::new();
    let mut keys = Vec::new();

    for info in listing {
        if is_legacy_derivative(&info.key) {
            plan.legacy_derivatives += 1;
            continue;
        }
        match OriginalKey::parse(&info.key) {
            Ok(key) => {
                keys.push(key);
                info_by_key.insert(info.key.clone(), info);
            }
            Err(_) => plan.invalid_keys += 1,
        }
    }

    let grouping = group_assets(keys, &options.salt);
    plan.unsupported_files = grouping.unsupported.len();

    let recorded: BTreeSet<String> = derived.list(RECORD_PREFIX).await?.into_iter().collect();

    let mut pending = Vec::new();
    for asset in grouping.assets {
        if let Some(folder) = &options.folder
            && !asset.dir.starts_with(folder.as_str())
        {
            continue;
        }
        plan.assets += 1;
        if !options.force && recorded.contains(&record_key(&asset.id)) {
            continue;
        }
        if let Some(info) = info_by_key.remove(asset.primary.as_str()) {
            pending.push(Todo { asset, info });
        }
    }

    plan.remaining = options.limit.map_or(0, |n| pending.len().saturating_sub(n));
    if let Some(n) = options.limit {
        pending.truncate(n);
    }
    plan.todo = pending;
    Ok(plan)
}
