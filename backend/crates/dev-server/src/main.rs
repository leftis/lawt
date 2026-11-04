//! Local development server: the real API core over a synthetic photo library.
//!
//! Run with `cargo run -p lawt-dev-server`, then start the frontend with
//! `VITE_UI=timeline VITE_API_BASE_URL=http://localhost:3002/api npm run dev`.
//! Set `LAWT_DEV_PORT` to change the port.
//! It never talks to AWS.

// Rust guideline compliant 2026-07-03

mod fake;
mod render;

use std::sync::Arc;

use axum::Router;
use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use lawt_api::{ApiResponse, handle};
use lawt_common::Rendition;
use tower_http::cors::{Any, CorsLayer};

use crate::fake::FakeLibrary;

/// Shooting days generated for the synthetic library (~2,500 photos).
const DEV_DAYS: u32 = 110;

/// Default port; 3001 is taken by the legacy Docker mock API, so this one differs.
const DEFAULT_PORT: u16 = 3002;

type Shared = Arc<FakeLibrary>;

fn into_response(res: ApiResponse) -> Response {
    let mut out = (res.status, res.body).into_response();
    let headers = out.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(res.cache_control),
    );
    out
}

async fn api(State(lib): State<Shared>, Path(rest): Path<String>) -> Response {
    into_response(handle(&*lib, &http::Method::GET, &format!("/{rest}")).await)
}

async fn derived(State(lib): State<Shared>, Path((id, file)): Path<(String, String)>) -> Response {
    let Some(rendition) = Rendition::ALL.into_iter().find(|r| r.file_name() == file) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some(photo) = lib.photo(&id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match render::placeholder(photo, rendition) {
        Some(bytes) => (
            [
                (header::CONTENT_TYPE, "image/jpeg"),
                (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
            ],
            bytes,
        )
            .into_response(),
        None => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let lib: Shared = Arc::new(FakeLibrary::generate(DEV_DAYS));
    println!("synthetic library: {} photos", lib.count());

    let app = Router::new()
        .route("/api/derived/{id}/{file}", get(derived))
        .route("/api/{*rest}", get(api))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers(Any)
                .allow_methods(Any),
        )
        .with_state(lib);

    let port = std::env::var("LAWT_DEV_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    println!("dev server on http://localhost:{port}/api");
    axum::serve(listener, app).await?;
    Ok(())
}
