//! Lambda that plans, renders and publishes the timeline.
//!
//! It is never triggered automatically: the owner invokes it, first with `plan` (which only lists
//! keys, downloading no photo), then with `run` on a small folder, then on everything.
//!
//! Event (JSON):
//!
//! ```json
//! { "action": "plan" | "run" | "publish", "folder": "100RICOH", "limit": 25,
//!   "force": false, "concurrency": 2 }
//! ```
//!
//! Environment: `PHOTO_BUCKET` (read-only), `DERIVED_BUCKET` (read-write), `ID_SALT` (secret, 16+ characters).

// Rust guideline compliant 2026-07-03

mod s3;

use aws_sdk_s3::Client;
use lambda_runtime::{Error, LambdaEvent, run, service_fn};
use lawt_common::IdSalt;
use lawt_pipeline::{Options, plan, process, publish};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::s3::{S3Derived, S3Originals};

/// Photos rendered per invocation unless the event says otherwise; small enough to finish well
/// inside the 15-minute limit even for large RAW files.
const DEFAULT_LIMIT: usize = 25;

/// Photos rendered at once. Each holds a decoded frame (up to ~360 MB for a 16-bit TIFF).
const DEFAULT_CONCURRENCY: usize = 2;

/// Sample keys returned by `plan` so the owner can eyeball what would be processed.
const SAMPLE: usize = 10;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    Plan,
    Run,
    Publish,
}

#[derive(Debug, Deserialize)]
struct Request {
    action: Action,
    folder: Option<String>,
    limit: Option<usize>,
    #[serde(default)]
    force: bool,
    concurrency: Option<usize>,
}

async fn handler(
    originals: &S3Originals,
    derived: &S3Derived,
    salt: &IdSalt,
    event: LambdaEvent<Request>,
) -> Result<Value, Error> {
    let req = event.payload;
    let options = Options {
        salt: salt.clone(),
        force: req.force,
        folder: req.folder.clone(),
        limit: Some(req.limit.unwrap_or(DEFAULT_LIMIT)),
    };

    match req.action {
        Action::Plan => {
            let p = plan(
                originals,
                derived,
                &Options {
                    limit: None,
                    ..options
                },
            )
            .await?;
            let mut folders: std::collections::BTreeMap<&str, usize> =
                std::collections::BTreeMap::new();
            for t in &p.todo {
                *folders.entry(t.asset.gallery.as_str()).or_default() += 1;
            }
            Ok(json!({
                "action": "plan",
                "to_render_by_folder": folders,
                "photos_in_scope": p.assets,
                "to_render": p.todo.len(),
                "unsupported_files": p.unsupported_files,
                "legacy_derivatives_ignored": p.legacy_derivatives,
                "invalid_keys": p.invalid_keys,
                "sample": p.todo.iter().take(SAMPLE).map(|t| t.asset.primary.to_string()).collect::<Vec<_>>(),
                "note": "plan only lists keys; no photo was downloaded",
            }))
        }
        Action::Run => {
            let p = plan(originals, derived, &options).await?;
            let batch = p.todo.len();
            let report = process(
                originals,
                derived,
                p.todo,
                req.concurrency.unwrap_or(DEFAULT_CONCURRENCY),
            )
            .await;
            let published = publish(derived).await?;
            Ok(json!({
                "action": "run",
                "batch": batch,
                "remaining_after_batch": p.remaining,
                "rendered": report.rendered,
                "unsupported": report.unsupported,
                "failed": report.failed,
                "storage_errors": report.errors,
                "timeline_photos": published.photos,
            }))
        }
        Action::Publish => {
            let published = publish(derived).await?;
            Ok(json!({
                "action": "publish",
                "photos": published.photos,
                "written": published.written,
                "deleted": published.deleted,
                "corrupt_records": published.corrupt_records,
            }))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let photo_bucket = std::env::var("PHOTO_BUCKET")?;
    let derived_bucket = std::env::var("DERIVED_BUCKET")?;
    let salt =
        IdSalt::new(&std::env::var("ID_SALT")?).ok_or("ID_SALT must be at least 16 characters")?;
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let client = Client::new(&config);

    let originals = S3Originals {
        client: client.clone(),
        bucket: photo_bucket,
    };
    let derived = S3Derived {
        client,
        bucket: derived_bucket,
    };
    let (originals, derived, salt) = (&originals, &derived, &salt);

    run(service_fn(move |event| {
        handler(originals, derived, salt, event)
    }))
    .await
}
