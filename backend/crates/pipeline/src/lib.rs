//! Indexing pipeline: discovers photos, renders them, and publishes the timeline.
//!
//! Three steps, each safe to repeat:
//!
//! 1. [`plan`] lists the originals (read-only) and works out which photos still need rendering.
//! 2. [`process`] renders those photos into the derived bucket and stores one [`Record`] each.
//! 3. [`publish`] rebuilds the timeline read model from all records.
//!
//! Storage sits behind two ports. [`Originals`] has **no write or delete method**, so nothing in
//! this crate can modify the photo bucket, by construction. [`Derived`] is the only writable side.

// Rust guideline compliant 2026-07-03

mod plan;
mod ports;
mod process;
mod publish;
mod record;

#[cfg(test)]
mod testing;

#[doc(inline)]
pub use plan::{Options, Plan, Todo, plan};
#[doc(inline)]
pub use ports::{Derived, ObjectInfo, Originals, PortError};
#[doc(inline)]
pub use process::{Report, process};
#[doc(inline)]
pub use publish::{Published, publish};
#[doc(inline)]
pub use record::{Record, Status, record_key};

#[cfg(test)]
mod tests {
    use image::ImageFormat;
    use lawt_common::IdSalt;
    use lawt_common::readmodel::INDEX_KEY;

    use super::*;
    use crate::testing::{MemDerived, MemOriginals, image_bytes};

    fn library() -> MemOriginals {
        MemOriginals::default()
            .with(
                "all images/100RICOH/A1.jpg",
                image_bytes(600, 400, ImageFormat::Jpeg),
            )
            .with(
                "all images/100RICOH/A2.png",
                image_bytes(300, 500, ImageFormat::Png),
            )
            .with(
                "all images/101RICOH/B1.tif",
                image_bytes(800, 600, ImageFormat::Tiff),
            )
            .with("all images/101RICOH/B2.heic", b"heic bytes".to_vec())
            .with("all images/101RICOH/B3.png", b"corrupt".to_vec())
            .with("all images/101RICOH/notes.txt", b"hi".to_vec())
            .with(
                "all images/100RICOH/A1.jpg.thumbnail.jpeg",
                b"legacy".to_vec(),
            )
            .with("elsewhere/x.jpg", b"outside".to_vec())
    }

    #[tokio::test]
    async fn plan_classifies_everything_it_lists() {
        let p = plan(
            &library(),
            &MemDerived::default(),
            &Options::new(IdSalt::insecure_dev()),
        )
        .await
        .unwrap();
        assert_eq!(p.assets, 5);
        assert_eq!(p.todo.len(), 5);
        assert_eq!((p.unsupported_files, p.legacy_derivatives), (1, 1));
    }

    #[tokio::test]
    async fn full_run_renders_records_and_publishes() {
        let (originals, derived) = (library(), MemDerived::default());
        let p = plan(&originals, &derived, &Options::new(IdSalt::insecure_dev()))
            .await
            .unwrap();
        let report = process(&originals, &derived, p.todo, 2).await;
        assert_eq!(
            (report.rendered, report.unsupported, report.failed),
            (3, 1, 1)
        );
        assert!(report.errors.is_empty());

        // Renditions exist for rendered photos; the JPEG original gets no derived "full".
        let keys = derived.keys();
        let jpeg_id = lawt_common::AssetId::for_asset(&IdSalt::insecure_dev(), "100RICOH", "A1");
        assert!(keys.contains(&lawt_common::Rendition::Thumbnail.derived_key(&jpeg_id)));
        assert!(!keys.contains(&lawt_common::Rendition::Full.derived_key(&jpeg_id)));
        let tiff_id = lawt_common::AssetId::for_asset(&IdSalt::insecure_dev(), "101RICOH", "B1");
        assert!(keys.contains(&lawt_common::Rendition::Full.derived_key(&tiff_id)));

        let out = publish(&derived).await.unwrap();
        assert_eq!(
            out.photos, 5,
            "failed and unsupported photos are still listed"
        );
        assert!(derived.keys().iter().any(|k| k == INDEX_KEY));

        // Second plan finds nothing left to do.
        let again = plan(&originals, &derived, &Options::new(IdSalt::insecure_dev()))
            .await
            .unwrap();
        assert!(again.todo.is_empty());
    }

    #[tokio::test]
    async fn originals_are_only_ever_read() {
        let (originals, derived) = (library(), MemDerived::default());
        let p = plan(&originals, &derived, &Options::new(IdSalt::insecure_dev()))
            .await
            .unwrap();
        process(&originals, &derived, p.todo, 3).await;
        // Structural guarantee: `Originals` has no mutating method. Behavioural check: reads only touched originals.
        let reads = originals.reads.lock().unwrap();
        assert!(reads.iter().all(|k| k.starts_with("all images/")));
        assert!(
            !reads
                .iter()
                .any(|k| k.ends_with("notes.txt") || k.contains("thumbnail"))
        );
    }

    #[tokio::test]
    async fn limit_and_folder_scope_the_batch() {
        let (originals, derived) = (library(), MemDerived::default());
        let opts = Options {
            limit: Some(2),
            ..Options::new(IdSalt::insecure_dev())
        };
        let p = plan(&originals, &derived, &opts).await.unwrap();
        assert_eq!((p.todo.len(), p.remaining), (2, 3));

        let opts = Options {
            folder: Some("100RICOH".into()),
            ..Options::new(IdSalt::insecure_dev())
        };
        let p = plan(&originals, &derived, &opts).await.unwrap();
        assert_eq!((p.assets, p.todo.len()), (2, 2));
    }

    #[tokio::test]
    async fn broken_raw_falls_back_to_its_camera_jpeg() {
        let originals = MemOriginals::default()
            .with("all images/1/R1.RAF", b"not a real raf".to_vec())
            .with(
                "all images/1/R1.JPG",
                image_bytes(600, 400, ImageFormat::Jpeg),
            );
        let derived = MemDerived::default();
        let p = plan(&originals, &derived, &Options::new(IdSalt::insecure_dev()))
            .await
            .unwrap();
        let report = process(&originals, &derived, p.todo, 1).await;
        assert_eq!(report.rendered, 1, "{report:?}");
        let out = publish(&derived).await.unwrap();
        assert_eq!(out.photos, 1);
    }

    #[tokio::test]
    async fn publish_removes_months_that_lost_their_photos() {
        let derived = MemDerived::default();
        derived
            .put(
                "index/months/2001-01.json",
                b"{}".to_vec(),
                "application/json",
                "",
            )
            .await
            .unwrap();
        let out = publish(&derived).await.unwrap();
        assert_eq!(out.deleted, 1);
        assert!(!derived.keys().iter().any(|k| k.contains("2001-01")));
    }
}
