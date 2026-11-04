//! Step 3: rebuild the timeline read model from all records.

// Rust guideline compliant 2026-07-03

use futures::stream::{self, StreamExt};
use rawvault_common::readmodel::{self, MONTHS_PREFIX};

use crate::ports::{Derived, PortError};
use crate::record::{RECORD_PREFIX, Record};

/// Records fetched at once. Small JSON objects; enough parallelism to read 10k in seconds.
const FETCH_CONCURRENCY: usize = 32;

/// What [`publish`] did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Published {
    /// Photos in the timeline.
    pub photos: usize,
    /// Read model objects written.
    pub written: usize,
    /// Stale read model objects removed (months that lost all their photos).
    pub deleted: usize,
    /// Records that could not be parsed and were left out.
    pub corrupt_records: usize,
}

/// Rebuilds `index/timeline.json` and `index/months/*.json` from every record.
///
/// # Errors
/// Returns [`PortError`] if listing, reading or writing the derived bucket fails.
pub async fn publish(derived: &impl Derived) -> Result<Published, PortError> {
    let keys = derived.list(RECORD_PREFIX).await?;
    let fetched: Vec<_> = stream::iter(keys)
        .map(|key| async move { derived.get(&key).await })
        .buffer_unordered(FETCH_CONCURRENCY)
        .collect()
        .await;

    let mut out = Published::default();
    let mut photos = Vec::new();
    for body in fetched {
        let Some(body) = body? else { continue };
        match serde_json::from_slice::<Record>(&body) {
            Ok(record) => photos.push(record.photo),
            Err(_) => out.corrupt_records += 1,
        }
    }
    out.photos = photos.len();

    let model = readmodel::build(&photos)
        .map_err(|e| PortError::new(format!("building read model: {e}")))?;
    for (key, body) in &model.objects {
        derived
            .put(key, body.clone(), "application/json", "no-cache")
            .await?;
        out.written += 1;
    }

    let mut existing = derived.list(MONTHS_PREFIX).await?;
    existing.push(readmodel::INDEX_KEY.to_owned());
    let stale: Vec<String> = model
        .stale(existing.iter().map(String::as_str))
        .into_iter()
        .map(str::to_owned)
        .collect();
    if !stale.is_empty() {
        derived.delete(&stale).await?;
    }
    out.deleted = stale.len();
    Ok(out)
}
