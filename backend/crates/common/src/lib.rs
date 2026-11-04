//! Pure domain logic shared by all lawt backend Lambdas.
//!
//! This crate performs no I/O and talks to no AWS service, so everything in
//! it is unit-testable offline. It is the single source of truth for:
//!
//! - which file formats exist and how they rank ([`format`]),
//! - what a valid original object key looks like ([`key`]),
//! - how several files of one shot become one photo ([`asset`]),
//! - camera and exposure metadata captured once per photo ([`camera`]),
//! - where each generated rendition lives ([`rendition`]),
//! - how photos are laid out on the timeline ([`timeline`]),
//! - how the served read model is built from photo records ([`readmodel`]).
//!
//! Originals live in a bucket that this code must never write to. Nothing here
//! builds a key for the originals bucket other than by parsing an existing one.

// Rust guideline compliant 2026-07-03

pub mod asset;
pub mod camera;
pub mod error;
pub mod format;
pub mod key;
pub mod readmodel;
pub mod rendition;
pub mod timeline;

#[doc(inline)]
pub use asset::{Asset, AssetId, Grouping, IdSalt, group_assets};
#[doc(inline)]
pub use camera::CameraExif;
#[doc(inline)]
pub use error::KeyError;
#[doc(inline)]
pub use format::FileKind;
#[doc(inline)]
pub use key::OriginalKey;
#[doc(inline)]
pub use rendition::Rendition;
#[doc(inline)]
pub use timeline::{
    MonthKey, MonthPage, Photo, TakenAtSource, TimelineDay, TimelineIndex, build_index,
    build_timeline, group_by_month, resolve_taken_at,
};
