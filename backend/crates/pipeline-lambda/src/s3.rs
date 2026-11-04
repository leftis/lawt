//! S3 adapters for the pipeline ports.
//!
//! [`S3Originals`] only ever calls `ListObjectsV2` and `GetObject`. It has no code path that
//! writes, copies or deletes, and the Lambda's role grants nothing more on the photo bucket.

// Rust guideline compliant 2026-07-03

use aws_sdk_s3::Client;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::get_object::GetObjectError;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{Delete, ObjectIdentifier};
use chrono::DateTime;
use lawt_common::OriginalKey;
use lawt_pipeline::{Derived, ObjectInfo, Originals, PortError};

/// S3 allows at most this many keys per `DeleteObjects` call.
const DELETE_BATCH: usize = 1000;

/// Read-only adapter for the photo bucket.
#[derive(Debug, Clone)]
pub struct S3Originals {
    pub client: Client,
    pub bucket: String,
}

impl Originals for S3Originals {
    async fn list(&self, prefix: &str) -> Result<Vec<ObjectInfo>, PortError> {
        let mut out = Vec::new();
        let mut pages = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix)
            .into_paginator()
            .send();
        while let Some(page) = pages.next().await {
            let page = page.map_err(|e| PortError::new(format!("listing {prefix}: {e}")))?;
            for obj in page.contents() {
                let (Some(key), Some(modified)) = (obj.key(), obj.last_modified()) else {
                    continue;
                };
                let Some(modified) = DateTime::from_timestamp(modified.secs(), 0) else {
                    continue;
                };
                out.push(ObjectInfo {
                    key: key.to_owned(),
                    size: u64::try_from(obj.size().unwrap_or(0)).unwrap_or(0),
                    modified: modified.naive_utc(),
                });
            }
        }
        Ok(out)
    }

    async fn get(&self, key: &OriginalKey) -> Result<Vec<u8>, PortError> {
        let out = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key.as_str())
            .send()
            .await
            .map_err(|e| PortError::new(format!("get {key}: {e}")))?;
        let bytes = out
            .body
            .collect()
            .await
            .map_err(|e| PortError::new(format!("reading {key}: {e}")))?;
        Ok(bytes.into_bytes().to_vec())
    }
}

/// Read-write adapter for the derived bucket.
#[derive(Debug, Clone)]
pub struct S3Derived {
    pub client: Client,
    pub bucket: String,
}

impl Derived for S3Derived {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, PortError> {
        match self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(out) => {
                let bytes = out
                    .body
                    .collect()
                    .await
                    .map_err(|e| PortError::new(format!("reading {key}: {e}")))?;
                Ok(Some(bytes.into_bytes().to_vec()))
            }
            Err(SdkError::ServiceError(e)) if matches!(e.err(), GetObjectError::NoSuchKey(_)) => {
                Ok(None)
            }
            Err(e) => Err(PortError::new(format!("get {key}: {e}"))),
        }
    }

    async fn put(
        &self,
        key: &str,
        body: Vec<u8>,
        content_type: &str,
        cache_control: &str,
    ) -> Result<(), PortError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(body))
            .content_type(content_type)
            .cache_control(cache_control)
            .send()
            .await
            .map_err(|e| PortError::new(format!("put {key}: {e}")))?;
        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, PortError> {
        let mut out = Vec::new();
        let mut pages = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix)
            .into_paginator()
            .send();
        while let Some(page) = pages.next().await {
            let page = page.map_err(|e| PortError::new(format!("listing {prefix}: {e}")))?;
            out.extend(
                page.contents()
                    .iter()
                    .filter_map(|o| o.key().map(str::to_owned)),
            );
        }
        Ok(out)
    }

    async fn delete(&self, keys: &[String]) -> Result<(), PortError> {
        for chunk in keys.chunks(DELETE_BATCH) {
            let objects = chunk
                .iter()
                .map(|k| ObjectIdentifier::builder().key(k).build())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| PortError::new(format!("building delete request: {e}")))?;
            let delete = Delete::builder()
                .set_objects(Some(objects))
                .build()
                .map_err(|e| PortError::new(format!("building delete request: {e}")))?;
            self.client
                .delete_objects()
                .bucket(&self.bucket)
                .delete(delete)
                .send()
                .await
                .map_err(|e| PortError::new(format!("deleting keys: {e}")))?;
        }
        Ok(())
    }
}
