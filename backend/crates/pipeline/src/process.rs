//! Step 2: render photos into the derived bucket.

// Rust guideline compliant 2026-07-03

use futures::stream::{self, StreamExt};
use lawt_common::rendition::Rendition;
use lawt_common::{Asset, FileKind, OriginalKey, Photo, TakenAtSource, resolve_taken_at};
use lawt_processor::{ProcessError, Processed};

use crate::plan::Todo;
use crate::ports::{Derived, Originals, PortError};
use crate::record::{Record, Status, record_key};

/// Cache policy for renditions: their path contains the photo id, so they never change.
const IMMUTABLE: &str = "public, max-age=31536000, immutable";

/// Shape assumed for photos that could not be decoded, so their tiles still lay out (3:2).
const FALLBACK_SIZE: (u32, u32) = (3, 2);

/// Totals for a batch of [`process`] calls.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Photos rendered successfully.
    pub rendered: usize,
    /// Photos in a format that is recognised but unsupported.
    pub unsupported: usize,
    /// Photos that could not be decoded.
    pub failed: usize,
    /// Storage errors (transient); these photos get no record and are retried by the next plan.
    pub errors: Vec<String>,
}

/// Renders every photo in `todos`, at most `concurrency` at a time.
///
/// Each photo's original is downloaded once (read-only), rendered, and its renditions and
/// record are written to the derived bucket. A photo that cannot be decoded still gets a record
/// (without thumbnail) so it appears in the timeline and is not retried forever.
pub async fn process(
    originals: &impl Originals,
    derived: &impl Derived,
    todos: Vec<Todo>,
    concurrency: usize,
) -> Report {
    let results: Vec<_> = stream::iter(todos)
        .map(|todo| async move {
            let key = todo.asset.primary.to_string();
            (key, process_one(originals, derived, todo).await)
        })
        .buffer_unordered(concurrency.max(1))
        .collect()
        .await;

    let mut report = Report::default();
    for (key, result) in results {
        match result {
            Ok(Status::Rendered) => report.rendered += 1,
            Ok(Status::Unsupported) => report.unsupported += 1,
            Ok(Status::Failed) => report.failed += 1,
            Err(e) => report.errors.push(format!("{key}: {e}")),
        }
    }
    report
}

async fn process_one(
    originals: &impl Originals,
    derived: &impl Derived,
    todo: Todo,
) -> Result<Status, PortError> {
    let Todo { asset, info } = todo;

    // Try the primary original first; if it cannot be decoded, the camera JPEG next to it may.
    let mut first_error: Option<ProcessError> = None;
    let mut rendered: Option<Processed> = None;
    for key in std::iter::once(&asset.primary).chain(asset.companions.iter()) {
        let Some(kind) = FileKind::from_key(key.as_str()) else {
            continue;
        };
        match render(originals, key, kind).await? {
            Ok(processed) => {
                rendered = Some(processed);
                break;
            }
            Err(e) => first_error = first_error.or(Some(e)),
        }
    }

    let (status, reason) = match (&rendered, &first_error) {
        (Some(_), _) => (Status::Rendered, None),
        (None, Some(e)) if e.is_unsupported() => (Status::Unsupported, Some(e.to_string())),
        (None, Some(e)) => (Status::Failed, Some(e.to_string())),
        (None, None) => (Status::Failed, Some("no readable file".to_owned())),
    };

    let (taken_at, source) =
        resolve_taken_at(rendered.as_ref().and_then(|p| p.taken), info.modified);
    let (width, height) = rendered
        .as_ref()
        .map_or(FALLBACK_SIZE, |p| (p.width, p.height));
    let has_full = rendered.as_ref().is_some_and(|p| p.full.is_some());
    let camera = rendered
        .as_ref()
        .map(|p| p.camera.clone())
        .unwrap_or_default();

    if let Some(p) = rendered {
        put_jpeg(derived, &asset, Rendition::Thumbnail, p.thumbnail).await?;
        put_jpeg(derived, &asset, Rendition::Medium, p.medium).await?;
        if let Some(full) = p.full {
            put_jpeg(derived, &asset, Rendition::Full, full).await?;
        }
    }

    let record = Record {
        photo: photo(
            &asset,
            width,
            height,
            taken_at,
            source,
            info.size,
            status == Status::Rendered,
            has_full,
        ),
        status,
        reason,
        source_size: info.size,
        source_modified: info.modified,
        camera,
        companions: asset.companions.iter().map(ToString::to_string).collect(),
    };
    let body =
        serde_json::to_vec(&record).map_err(|e| PortError::new(format!("encoding record: {e}")))?;
    derived
        .put(&record_key(&asset.id), body, "application/json", "no-store")
        .await?;
    Ok(status)
}

/// Downloads `key` and renders it on a blocking thread (decoding is CPU-bound).
async fn render(
    originals: &impl Originals,
    key: &OriginalKey,
    kind: FileKind,
) -> Result<Result<Processed, ProcessError>, PortError> {
    let bytes = originals.get(key).await?;
    Ok(
        tokio::task::spawn_blocking(move || lawt_processor::process(kind, &bytes))
            .await
            .unwrap_or_else(|e| Err(ProcessError::from_panic(&e.to_string()))),
    )
}

async fn put_jpeg(
    derived: &impl Derived,
    asset: &Asset,
    rendition: Rendition,
    body: Vec<u8>,
) -> Result<(), PortError> {
    derived
        .put(
            &rendition.derived_key(&asset.id),
            body,
            "image/jpeg",
            IMMUTABLE,
        )
        .await
}

#[expect(
    clippy::too_many_arguments,
    reason = "flat constructor for a plain record"
)]
fn photo(
    asset: &Asset,
    width: u32,
    height: u32,
    taken_at: chrono::NaiveDateTime,
    source: TakenAtSource,
    size: u64,
    has_thumbnail: bool,
    has_full: bool,
) -> Photo {
    Photo {
        id: asset.id.to_string(),
        original_key: asset.primary.to_string(),
        gallery: asset.gallery.clone(),
        stem: asset.stem.clone(),
        ext: asset.primary.extension(),
        is_raw: asset.kind.is_raw(),
        width,
        height,
        taken_at,
        taken_at_source: source,
        size_bytes: size,
        has_thumbnail,
        has_full,
    }
}
