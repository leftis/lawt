//! Framework-agnostic core of the RawVault HTTP API.
//!
//! The core takes a method and a path and returns a status, headers and a JSON
//! body. It never touches the network itself: data comes from a [`Library`], so
//! the same code runs in the Lambda, in the local dev server and in unit tests.
//!
//! Routes (all `GET`):
//!
//! | Path | Response |
//! |---|---|
//! | `/health` | `{"ok":true}` |
//! | `/timeline` | [`rawvault_common::TimelineIndex`] |
//! | `/timeline/months/{YYYY-MM}` | [`rawvault_common::MonthPage`] |
//!
//! Authentication is enforced in front of this code (API Gateway's Cognito
//! authorizer), so the core assumes an authenticated caller.

// Rust guideline compliant 2026-07-03

mod library;
mod router;

#[doc(inline)]
pub use library::{Library, ObjectLibrary, ObjectReader, StoreError};
#[doc(inline)]
pub use router::{ApiResponse, allowed_origin, handle};
