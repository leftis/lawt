//! AWS Lambda entrypoint for the lawt API.
//!
//! Serves the timeline read model that the indexer wrote into the derived bucket.
//! Reads only; it has no permission to write anywhere and none to touch the photo bucket.
//!
//! Environment:
//! - `DERIVED_BUCKET`: bucket holding `index/timeline.json` and `index/months/*.json`.
//! - `ALLOWED_ORIGIN`: the single frontend origin allowed by CORS.
//! - `API_STAGE`: API Gateway stage name to strip from paths, e.g. `prod` (optional).

// Rust guideline compliant 2026-07-03

use aws_sdk_s3::Client;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::get_object::GetObjectError;
use lambda_http::http::{HeaderValue, header};
use lambda_http::{Body, Error, Request, Response, run, service_fn};
use lawt_api::{ObjectLibrary, ObjectReader, StoreError, allowed_origin, handle};

/// Reads objects from the derived bucket.
#[derive(Debug, Clone)]
struct S3Reader {
    client: Client,
    bucket: String,
}

impl ObjectReader for S3Reader {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StoreError> {
        let result = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await;
        match result {
            Ok(out) => {
                let bytes = out
                    .body
                    .collect()
                    .await
                    .map_err(|e| StoreError::backend(format!("reading {key}: {e}")))?;
                Ok(Some(bytes.into_bytes().to_vec()))
            }
            Err(SdkError::ServiceError(e)) if matches!(e.err(), GetObjectError::NoSuchKey(_)) => {
                Ok(None)
            }
            Err(e) => Err(StoreError::backend(format!("get {key}: {e}"))),
        }
    }
}

/// Removes a leading `/{stage}` segment so routing sees the resource path only.
fn strip_stage(path: &str, stage: &str) -> String {
    if !stage.is_empty()
        && let Some(rest) = path.strip_prefix('/').and_then(|p| p.strip_prefix(stage))
        && (rest.is_empty() || rest.starts_with('/'))
    {
        return if rest.is_empty() {
            "/".to_owned()
        } else {
            rest.to_owned()
        };
    }
    path.to_owned()
}

async fn serve(
    library: &ObjectLibrary<S3Reader>,
    allowed: &str,
    stage: &str,
    req: Request,
) -> Result<Response<Body>, Error> {
    let origin = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    // Depending on how the API is fronted, the URI may carry the stage (`/prod/timeline`).
    let path = strip_stage(req.uri().path(), stage);

    let api = handle(library, req.method(), &path).await;

    let mut builder = Response::builder()
        .status(api.status)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CACHE_CONTROL, api.cache_control)
        .header(header::VARY, "Origin");
    if let Some(o) =
        allowed_origin(origin.as_deref(), allowed).and_then(|o| HeaderValue::from_str(o).ok())
    {
        builder = builder
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, o)
            .header(header::ACCESS_CONTROL_ALLOW_CREDENTIALS, "true");
    }
    Ok(builder.body(Body::from(api.body))?)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    lambda_http::tracing::init_default_subscriber();

    let bucket = std::env::var("DERIVED_BUCKET")?;
    let allowed = std::env::var("ALLOWED_ORIGIN").unwrap_or_default();
    let stage = std::env::var("API_STAGE").unwrap_or_default();
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let library = ObjectLibrary::new(S3Reader {
        client: Client::new(&config),
        bucket,
    });

    let library = &library;
    let (allowed, stage) = (allowed.as_str(), stage.as_str());
    run(service_fn(move |req| serve(library, allowed, stage, req))).await
}

#[cfg(test)]
mod tests {
    use super::strip_stage;

    #[test]
    fn strips_only_a_whole_leading_stage_segment() {
        assert_eq!(strip_stage("/prod/timeline", "prod"), "/timeline");
        assert_eq!(strip_stage("/timeline", "prod"), "/timeline");
        assert_eq!(strip_stage("/production/x", "prod"), "/production/x");
        assert_eq!(strip_stage("/prod", "prod"), "/");
        assert_eq!(strip_stage("/prod/timeline", ""), "/prod/timeline");
    }
}
