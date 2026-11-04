//! Request routing and response building.

// Rust guideline compliant 2026-07-03

use http::{Method, StatusCode};
use rawvault_common::MonthKey;
use serde::Serialize;

use crate::library::Library;

/// Index changes only when the indexer runs; a minute of staleness is invisible.
const INDEX_CACHE: &str = "private, max-age=60, stale-while-revalidate=300";

/// Month pages are rebuilt rarely; five minutes keeps scrolling back and forth free.
const MONTH_CACHE: &str = "private, max-age=300, stale-while-revalidate=600";

/// Returns the origin to echo in `Access-Control-Allow-Origin`, or `None`.
///
/// Only an exact match with `allowed` is echoed; there is no wildcard, so credentialed
/// requests from other sites are never authorized.
///
/// # Examples
/// ```
/// use rawvault_api::allowed_origin;
/// assert_eq!(allowed_origin(Some("https://a.example"), "https://a.example"), Some("https://a.example"));
/// assert_eq!(allowed_origin(Some("https://evil.example"), "https://a.example"), None);
/// assert_eq!(allowed_origin(None, "https://a.example"), None);
/// ```
#[must_use]
pub fn allowed_origin<'a>(request_origin: Option<&'a str>, allowed: &str) -> Option<&'a str> {
    request_origin.filter(|o| !allowed.is_empty() && *o == allowed)
}

/// A response the caller turns into its framework's response type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiResponse {
    /// HTTP status.
    pub status: StatusCode,
    /// Value for the `Cache-Control` header.
    pub cache_control: &'static str,
    /// JSON body.
    pub body: Vec<u8>,
}

impl ApiResponse {
    fn json<T: Serialize>(status: StatusCode, cache_control: &'static str, value: &T) -> Self {
        match serde_json::to_vec(value) {
            Ok(body) => Self {
                status,
                cache_control,
                body,
            },
            Err(_) => Self::error(StatusCode::INTERNAL_SERVER_ERROR, "Serialization failed"),
        }
    }

    fn error(status: StatusCode, message: &str) -> Self {
        let body = serde_json::json!({ "error": message })
            .to_string()
            .into_bytes();
        Self {
            status,
            cache_control: "no-store",
            body,
        }
    }
}

/// Routes one request against `library`.
///
/// Unknown paths give 404, non-`GET` methods give 405, and storage failures give 500
/// with a generic message (details are the caller's to log, never the client's to see).
///
/// # Examples
/// ```
/// use http::{Method, StatusCode};
/// # async fn demo(lib: &impl rawvault_api::Library) {
/// let res = rawvault_api::handle(lib, &Method::GET, "/health").await;
/// assert_eq!(res.status, StatusCode::OK);
/// # }
/// ```
pub async fn handle<L: Library>(library: &L, method: &Method, path: &str) -> ApiResponse {
    if method != Method::GET {
        return ApiResponse::error(StatusCode::METHOD_NOT_ALLOWED, "Method not allowed");
    }
    let path = path.trim_end_matches('/');
    match path {
        "/health" => ApiResponse::json(
            StatusCode::OK,
            "no-store",
            &serde_json::json!({ "ok": true }),
        ),
        "/timeline" => match library.index().await {
            Ok(index) => ApiResponse::json(StatusCode::OK, INDEX_CACHE, &index),
            Err(_) => storage_failure(),
        },
        _ => match path.strip_prefix("/timeline/months/") {
            Some(raw) => month(library, raw).await,
            None => ApiResponse::error(StatusCode::NOT_FOUND, "Not found"),
        },
    }
}

async fn month<L: Library>(library: &L, raw: &str) -> ApiResponse {
    let Some(key) = MonthKey::parse(raw) else {
        return ApiResponse::error(StatusCode::BAD_REQUEST, "Month must look like 2024-06");
    };
    match library.month(&key).await {
        Ok(Some(page)) => ApiResponse::json(StatusCode::OK, MONTH_CACHE, &page),
        Ok(None) => ApiResponse::error(StatusCode::NOT_FOUND, "No photos in that month"),
        Err(_) => storage_failure(),
    }
}

fn storage_failure() -> ApiResponse {
    ApiResponse::error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to load the library",
    )
}

#[cfg(test)]
mod tests {
    use rawvault_common::{MonthPage, TimelineIndex};

    use super::*;
    use crate::library::StoreError;

    struct Fake {
        fail: bool,
    }

    impl Library for Fake {
        async fn index(&self) -> Result<TimelineIndex, StoreError> {
            if self.fail {
                return Err(StoreError::backend("boom secret-detail"));
            }
            Ok(TimelineIndex {
                days: vec![],
                galleries: vec!["100RICOH".into()],
                total: 0,
            })
        }

        async fn month(&self, month: &MonthKey) -> Result<Option<MonthPage>, StoreError> {
            Ok((month.as_str() == "2024-06").then(|| MonthPage {
                month: month.to_string(),
                photos: vec![],
            }))
        }
    }

    async fn get(path: &str) -> ApiResponse {
        handle(&Fake { fail: false }, &Method::GET, path).await
    }

    #[tokio::test]
    async fn routes_known_paths() {
        assert_eq!(get("/health").await.status, StatusCode::OK);
        assert_eq!(get("/timeline/").await.status, StatusCode::OK);
        assert_eq!(get("/timeline/months/2024-06").await.status, StatusCode::OK);
    }

    #[tokio::test]
    async fn rejects_bad_months_and_unknown_paths() {
        assert_eq!(
            get("/timeline/months/2024-13").await.status,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            get("/timeline/months/..%2Fx").await.status,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            get("/timeline/months/2024-07").await.status,
            StatusCode::NOT_FOUND
        );
        assert_eq!(get("/nope").await.status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn only_get_is_allowed() {
        let res = handle(&Fake { fail: false }, &Method::POST, "/timeline").await;
        assert_eq!(res.status, StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn storage_errors_do_not_leak_details() {
        let res = handle(&Fake { fail: true }, &Method::GET, "/timeline").await;
        assert_eq!(res.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!String::from_utf8_lossy(&res.body).contains("secret-detail"));
        assert_eq!(res.cache_control, "no-store");
    }

    #[tokio::test]
    async fn successful_reads_are_cacheable() {
        assert_eq!(get("/timeline").await.cache_control, INDEX_CACHE);
        assert_eq!(
            get("/timeline/months/2024-06").await.cache_control,
            MONTH_CACHE
        );
    }
}
