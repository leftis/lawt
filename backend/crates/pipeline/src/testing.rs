//! In-memory fakes and image fixtures for tests.

// Rust guideline compliant 2026-07-03

use std::collections::BTreeMap;
use std::io::Cursor;
use std::sync::Mutex;

use chrono::{NaiveDate, NaiveDateTime};
use image::{DynamicImage, ImageFormat, Rgb, RgbImage};
use lawt_common::OriginalKey;

use crate::ports::{Derived, ObjectInfo, Originals, PortError};

pub fn mtime() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2024, 7, 1)
        .unwrap()
        .and_hms_opt(9, 0, 0)
        .unwrap()
}

pub fn image_bytes(w: u32, h: u32, format: ImageFormat) -> Vec<u8> {
    let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(w, h, Rgb([200, 100, 50])));
    let mut out = Vec::new();
    img.write_to(&mut Cursor::new(&mut out), format).unwrap();
    out
}

/// Read-only fake of the photo bucket. Counts reads so tests can assert on access.
#[derive(Default)]
pub struct MemOriginals {
    objects: BTreeMap<String, Vec<u8>>,
    pub reads: Mutex<Vec<String>>,
}

impl MemOriginals {
    pub fn with(mut self, key: &str, body: Vec<u8>) -> Self {
        self.objects.insert(key.to_owned(), body);
        self
    }
}

impl Originals for MemOriginals {
    async fn list(&self, prefix: &str) -> Result<Vec<ObjectInfo>, PortError> {
        Ok(self
            .objects
            .iter()
            .filter(|(k, _)| k.starts_with(prefix))
            .map(|(k, v)| ObjectInfo {
                key: k.clone(),
                size: v.len() as u64,
                modified: mtime(),
            })
            .collect())
    }

    async fn get(&self, key: &OriginalKey) -> Result<Vec<u8>, PortError> {
        self.reads.lock().unwrap().push(key.to_string());
        self.objects
            .get(key.as_str())
            .cloned()
            .ok_or_else(|| PortError::new("missing"))
    }
}

#[derive(Default)]
pub struct MemDerived {
    pub store: Mutex<BTreeMap<String, Vec<u8>>>,
}

impl MemDerived {
    pub fn keys(&self) -> Vec<String> {
        self.store.lock().unwrap().keys().cloned().collect()
    }
}

impl Derived for MemDerived {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, PortError> {
        Ok(self.store.lock().unwrap().get(key).cloned())
    }

    async fn put(&self, key: &str, body: Vec<u8>, _ct: &str, _cc: &str) -> Result<(), PortError> {
        self.store.lock().unwrap().insert(key.to_owned(), body);
        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, PortError> {
        Ok(self
            .store
            .lock()
            .unwrap()
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect())
    }

    async fn delete(&self, keys: &[String]) -> Result<(), PortError> {
        let mut store = self.store.lock().unwrap();
        for k in keys {
            store.remove(k);
        }
        Ok(())
    }
}
