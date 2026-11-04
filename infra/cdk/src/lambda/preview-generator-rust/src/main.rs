use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use image::DynamicImage;
use image::GenericImageView;
use image::ImageEncoder;
use image::ImageFormat;
use image::RgbaImage;
use lambda_runtime::{run, service_fn, LambdaEvent};
type LambdaError = lambda_runtime::Error;
use rawler::decoders::RawDecodeParams;
use rawler::rawsource::RawSource;
use rawler::get_decoder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Bucket name comes from the `STORAGE_BUCKET` environment variable that CDK sets on the function.
fn storage_bucket() -> String {
    std::env::var("STORAGE_BUCKET").unwrap_or_default()
}

// Events that can trigger this Lambda
#[derive(Deserialize)]
#[serde(untagged)]
enum TriggerEvent {
    S3(S3Event),
    Manual(ManualEvent),
}

#[derive(Deserialize)]
struct S3Event {
    records: Vec<S3Record>,
}

#[derive(Deserialize)]
struct S3Record {
    s3: S3Object,
}

#[derive(Deserialize)]
struct S3Object {
    bucket: S3Bucket,
    object: S3Item,
}

#[derive(Deserialize)]
struct S3Bucket {
    name: String,
}

#[derive(Deserialize)]
struct S3Item {
    key: String,
}

#[derive(Deserialize)]
struct ManualEvent {
    key: String,
}

#[derive(Serialize)]
struct HandlerResponse {
    status: String,
    message: String,
    preview_key: Option<String>,
    preview_type: String,
}

type AwsResult<T> = Result<T, aws_sdk_s3::Error>;

/// Check if a file extension is a RAW image format
fn is_raw_extension(key: &str) -> bool {
    let ext = key.split('.').last().unwrap_or("").to_lowercase();
    matches!(ext.as_str(),
        "raf" | "nef" | "cr2" | "cr3" | "arw" | "dng" | "orf" | "sr2" | "3fr"
        | "fff" | "mos" | "ia" | "pef" | "rw2" | "rwl" | "rwz" | "x3f" | "srw"
        | "k25" | "kdc"
    )
}

/// Check if a file extension is a TIFF image format
fn is_tiff_extension(key: &str) -> bool {
    let ext = key.split('.').last().unwrap_or("").to_lowercase();
    ext == "tiff" || ext == "tif"
}

/// Check if a key is already a generated preview (skip re-processing)
fn is_preview_key(key: &str) -> bool {
    key.ends_with(".thumbnail.jpeg")
        || key.ends_with(".medium.jpeg")
        || key.ends_with(".full.jpeg")
        || key.ends_with(".preview.png")
        || key.ends_with(".preview.jpg")
}

/// Try to extract an embedded JPEG preview from RAW file using rawler.
fn extract_preview_with_rawler(data: &[u8]) -> Option<Vec<u8>> {
    let raw_source = RawSource::new_from_slice(data);
    match get_decoder(&raw_source) {
        Ok(decoder) => {
            if let Ok(Some(img)) = decoder.full_image(&raw_source, &RawDecodeParams::default()) {
                let mut buf = Vec::new();
                if img.to_rgba8().write_to(
                    &mut std::io::Cursor::new(&mut buf),
                    ImageFormat::Png,
                ).is_ok() {
                    if buf.len() > 100 {
                        return Some(buf);
                    }
                }
            }
            None
        }
        Err(_) => None,
    }
}

/// Try to extract an embedded JPEG preview from RAW file bytes.
fn extract_embedded_jpeg(data: &[u8]) -> Option<Vec<u8>> {
    let mut pos = 0;
    while pos < data.len() - 1 {
        if data[pos] == 0xFF && data[pos + 1] == 0xD8 {
            let mut end = pos + 2;
            while end < data.len() - 1 {
                if data[end] == 0xFF && data[end + 1] == 0xD9 {
                    end += 2;
                    let size = end - pos;
                    if size > 100 && size <= 4 * 1024 * 1024 {
                        return Some(data[pos..end].to_vec());
                    }
                    break;
                }
                end += 1;
            }
        }
        pos += 1;
    }
    None
}

/// Generate a filename-seeded colorful gradient placeholder
fn generate_placeholder_png() -> Vec<u8> {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in b"RAW".iter() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    let hue0 = (h & 0xFFFF) as f32 / 65535.0;
    let hue1 = ((h >> 16) & 0xFFFF) as f32 / 65535.0;
    let c0 = hsl_to_rgb(hue0, 0.7, 0.55);
    let c1 = hsl_to_rgb(hue1, 0.6, 0.35);

    let w = 640u32;
    let h = 480u32;
    let mut img = image::RgbImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let t = ((x as f32 / w as f32) + (y as f32 / h as f32)) * 0.5;
            let r = (c0[0] as f32 * (1.0 - t) + c1[0] as f32 * t) as u8;
            let g = (c0[1] as f32 * (1.0 - t) + c1[1] as f32 * t) as u8;
            let b = (c0[2] as f32 * (1.0 - t) + c1[2] as f32 * t) as u8;
            img.put_pixel(x, y, image::Rgb([r, g, b]));
        }
    }
    let mut buffer = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut buffer),
        image::ImageFormat::Png,
    )
    .unwrap();
    buffer
}

/// HSL → RGB conversion
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> [u8; 3] {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match (h * 6.0) as u32 % 6 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    ]
}

/// Thumbnail key: small JPEG for gallery cards (~400px, ~20-50 KB)
fn thumbnail_key(raw_key: &str) -> String {
    format!("{}.thumbnail.jpeg", raw_key)
}

/// Medium key: large JPEG for the fullscreen viewer (~1920px, ~200-400 KB)
fn medium_preview_key(raw_key: &str) -> String {
    format!("{}.medium.jpeg", raw_key)
}

/// Full-size JPEG key: full resolution JPEG extracted from RAW (RAW-only output)
fn full_jpeg_key(raw_key: &str) -> String {
    format!("{}.full.jpeg", raw_key)
}

/// Derive S3 content-type from key extension
fn content_type_for(key: &str) -> &'static str {
    if key.ends_with(".jpeg") || key.ends_with(".jpg") { "image/jpeg" }
    else if key.ends_with(".png") { "image/png" }
    else { "application/octet-stream" }
}

/// Convert JPEG bytes to PNG bytes
fn convert_to_png(jpeg_data: &[u8]) -> Option<Vec<u8>> {
    let img = image::load_from_memory(jpeg_data).ok()?;
    let mut buf = Vec::new();
    img.to_rgba8().write_to(
        &mut std::io::Cursor::new(&mut buf),
        ImageFormat::Png,
    ).ok()?;
    if buf.len() > 100 {
        Some(buf)
    } else {
        None
    }
}

/// Decode TIFF file to PNG bytes
fn decode_tiff_to_png(tiff_data: &[u8]) -> Option<Vec<u8>> {
    let img = image::load_from_memory(tiff_data).ok()?;
    let mut buf = Vec::new();
    img.to_rgba8().write_to(
        &mut std::io::Cursor::new(&mut buf),
        ImageFormat::Png,
    ).ok()?;
    if buf.len() > 100 {
        Some(buf)
    } else {
        None
    }
}

/// Write JPEG bytes from an RGBA image buffer with the given quality.
fn write_jpeg_from_rgba(rgba: &RgbaImage, quality: u8) -> Vec<u8> {
    let (w, h) = rgba.dimensions();
    // Convert RGBA to RGB by dropping the alpha channel
    let rgb_buffer: Vec<u8> = rgba
        .chunks(4)
        .flat_map(|ch| [ch[0], ch[1], ch[2]])
        .collect();
    let mut buf = Vec::new();
    image::codecs::jpeg::JpegEncoder::new(std::io::Cursor::new(&mut buf))
        .write_image(
            &rgb_buffer,
            w,
            h,
            image::ColorType::Rgb8.into(),
        )
        .ok();
    buf
}

/// Resize an image so the long edge is at most `max_long_edge`, return as PNG bytes.
fn resize_image_to_png(img: &DynamicImage, max_long_edge: u32) -> Vec<u8> {
    let (w, h) = img.dimensions();
    let (nw, nh) = resize_dimensions(w, h, max_long_edge);
    let resized = image::imageops::resize(img, nw, nh, image::imageops::FilterType::Lanczos3);
    let mut buf = Vec::new();
    // Write the RGBA buffer as PNG (RgbaImage already is RGBA)
    image::codecs::png::PngEncoder::new(&mut buf)
        .write_image(
            resized.as_raw(),
            resized.width(),
            resized.height(),
            image::ColorType::Rgba8.into(),
        )
        .ok();
    buf
}

/// Resize an image so the long edge is at most `max_long_edge`, return as JPEG bytes.
fn resize_image_to_jpeg(img: &DynamicImage, max_long_edge: u32, quality: u8) -> Vec<u8> {
    let (w, h) = img.dimensions();
    let (nw, nh) = resize_dimensions(w, h, max_long_edge);
    let resized = image::imageops::resize(img, nw, nh, image::imageops::FilterType::Lanczos3);
    write_jpeg_from_rgba(&resized, quality)
}

/// Calculate new dimensions maintaining aspect ratio for a given max long edge.
fn resize_dimensions(w: u32, h: u32, max_long_edge: u32) -> (u32, u32) {
    if w >= h {
        let scale = max_long_edge as f64 / w as f64;
        ((max_long_edge as f64).round() as u32, (h as f64 * scale).round() as u32)
    } else {
        let scale = max_long_edge as f64 / h as f64;
        ((w as f64 * scale).round() as u32, (max_long_edge as f64).round() as u32)
    }
}

/// Generate thumbnail (400px JPEG) and medium (1920px JPEG) previews.
/// PNG is intentionally avoided — JPEG is 5-10x smaller for photos.
fn generate_previews(img: &DynamicImage) -> (Vec<u8>, Vec<u8>) {
    let thumbnail = resize_image_to_jpeg(img, 400, 65);
    let medium    = resize_image_to_jpeg(img, 1920, 85);
    (thumbnail, medium)
}

/// Upload preview and return success response
async fn upload_and_return(
    client: &Client,
    bucket: &str,
    key: &str,
    data: &[u8],
    method: &str,
) -> Result<HandlerResponse, LambdaError> {
    match upload_file(client, bucket, key, data).await {
        Ok(()) => {
            println!("  Preview saved: {} ({} bytes, method={})", key, data.len(), method);
            Ok(HandlerResponse {
                status: "success".to_string(),
                message: format!("Preview generated: {}", key),
                preview_key: Some(key.to_string()),
                preview_type: method.to_string(),
            })
        }
        Err(e) => {
            Ok(HandlerResponse {
                status: "error".to_string(),
                message: format!("Failed to upload preview: {}", e),
                preview_key: Some(key.to_string()),
                preview_type: "error".to_string(),
            })
        }
    }
}

async fn handle_request(
    client: &Client,
    event: TriggerEvent,
) -> Result<HandlerResponse, LambdaError> {
    match event {
        TriggerEvent::S3(s3_event) => {
            let mut results = Vec::new();
            for record in s3_event.records {
                let key = decode_url(&record.s3.object.key);
                let bucket = &record.s3.bucket.name;
                let result = process_file(client, bucket, &key).await;
                results.push(result);
            }
            Ok(HandlerResponse {
                status: "completed".to_string(),
                message: format!("Processed {} file(s)", results.len()),
                preview_key: None,
                preview_type: "batch".to_string(),
            })
        }
        TriggerEvent::Manual(manual_event) => {
            let key = manual_event.key;
            process_file(client, &storage_bucket(), &key).await
        }
    }
}

async fn process_file(
    client: &Client,
    bucket: &str,
    key: &str,
) -> Result<HandlerResponse, LambdaError> {
    if is_preview_key(key) {
        return Ok(HandlerResponse {
            status: "skipped".to_string(),
            message: format!("Skipping preview file: {}", key),
            preview_key: None,
            preview_type: "none".to_string(),
        });
    }

    // Handle TIFF files: decode → thumbnail + medium + full.jpeg (browser can't render TIFF natively)
    if is_tiff_extension(key) {
        println!("Processing TIFF file: {}", key);
        let tiff_data = match download_file(client, bucket, key).await {
            Ok(data) => data,
            Err(e) => {
                return Ok(HandlerResponse {
                    status: "error".to_string(),
                    message: format!("Failed to download {}: {}", key, e),
                    preview_key: None,
                    preview_type: "none".to_string(),
                });
            }
        };

        if let Ok(img) = image::load_from_memory(&tiff_data) {
            let (thumbnail, medium) = generate_previews(&img);
            // full.jpeg: encode the full-resolution image at high quality
            let full = write_jpeg_from_rgba(&img.to_rgba8(), 92);
            println!("  TIFF -> thumbnail ({} bytes), medium ({} bytes), full ({} bytes)",
                thumbnail.len(), medium.len(), full.len());
            upload_file(client, bucket, &thumbnail_key(&key), &thumbnail).await.ok();
            upload_file(client, bucket, &medium_preview_key(&key), &medium).await.ok();
            upload_file(client, bucket, &full_jpeg_key(&key), &full).await.ok();
            return Ok(HandlerResponse {
                status: "success".to_string(),
                message: format!("TIFF previews: {} + {} + {}",
                    thumbnail_key(&key), medium_preview_key(&key), full_jpeg_key(&key)),
                preview_key: Some(thumbnail_key(&key)),
                preview_type: "tiff_resized".to_string(),
            });
        }

        let placeholder = generate_placeholder_png();
        println!("  No TIFF decode available, generating placeholder ({} bytes)", placeholder.len());
        upload_file(client, bucket, &thumbnail_key(&key), &placeholder).await.ok();
        upload_file(client, bucket, &medium_preview_key(&key), &placeholder).await.ok();
        upload_file(client, bucket, &full_jpeg_key(&key), &placeholder).await.ok();
        return Ok(HandlerResponse {
            status: "success".to_string(),
            message: format!("TIFF placeholder: {}", thumbnail_key(&key)),
            preview_key: Some(thumbnail_key(&key)),
            preview_type: "placeholder".to_string(),
        });
    }

    // Handle JPEG and PNG files: resize for thumbnail + medium, copy original as full.jpeg.
    // Browsers render these natively — no re-encode, no quality loss for the full version.
    let ext = key.split('.').last().unwrap_or("").to_lowercase();
    if ext == "jpg" || ext == "jpeg" || ext == "png" {
        println!("Processing {} file: {}", ext.to_uppercase(), key);
        let file_data = match download_file(client, bucket, key).await {
            Ok(data) => data,
            Err(e) => {
                return Ok(HandlerResponse {
                    status: "error".to_string(),
                    message: format!("Failed to download {}: {}", key, e),
                    preview_key: None,
                    preview_type: "none".to_string(),
                });
            }
        };

        // full.jpeg = S3 server-side copy of the original (no re-encode, zero quality loss)
        match copy_s3_object(client, bucket, key, &full_jpeg_key(key)).await {
            Ok(()) => println!("  {} -> full.jpeg (server-side copy)", ext.to_uppercase()),
            Err(e) => println!("  WARN: full.jpeg copy failed: {}", e),
        }

        if let Ok(img) = image::load_from_memory(&file_data) {
            let (thumbnail, medium) = generate_previews(&img);
            println!("  {} -> thumbnail ({} bytes), medium ({} bytes)", ext.to_uppercase(), thumbnail.len(), medium.len());
            upload_file(client, bucket, &thumbnail_key(&key), &thumbnail).await.ok();
            upload_file(client, bucket, &medium_preview_key(&key), &medium).await.ok();
            return Ok(HandlerResponse {
                status: "success".to_string(),
                message: format!("{} previews: {} + {} + {}",
                    ext.to_uppercase(), thumbnail_key(&key), medium_preview_key(&key), full_jpeg_key(&key)),
                preview_key: Some(thumbnail_key(&key)),
                preview_type: format!("{}_resized", ext),
            });
        }

        let placeholder = generate_placeholder_png();
        println!("  Could not decode {} for resize, generating placeholder", ext.to_uppercase());
        upload_file(client, bucket, &thumbnail_key(&key), &placeholder).await.ok();
        upload_file(client, bucket, &medium_preview_key(&key), &placeholder).await.ok();
        return Ok(HandlerResponse {
            status: "success".to_string(),
            message: format!("{} placeholder: {}", ext.to_uppercase(), thumbnail_key(&key)),
            preview_key: Some(thumbnail_key(&key)),
            preview_type: "placeholder".to_string(),
        });
    }

    // RAW file processing
    let thumbnail_key = thumbnail_key(key);
    let medium_key = medium_preview_key(key);
    let full_key = full_jpeg_key(key);
    println!("Processing RAW file: {}", key);

    let raw_data = match download_file(client, bucket, key).await {
        Ok(data) => data,
        Err(e) => {
            return Ok(HandlerResponse {
                status: "error".to_string(),
                message: format!("Failed to download {}: {}", key, e),
                preview_key: None,
                preview_type: "none".to_string(),
            });
        }
    };

    // Decode the RAW file: rawler (full demosaic) preferred, embedded JPEG as fallback.
    // This decoded image is used for ALL three outputs: thumbnail, medium, and full.jpeg.
    let embedded_jpeg = extract_embedded_jpeg(&raw_data);

    let preview_img: Option<image::DynamicImage> = {
        let from_rawler = extract_preview_with_rawler(&raw_data)
            .and_then(|data| if data.len() > 100 {
                println!("  rawler full decode: {} bytes", data.len());
                image::load_from_memory(&data).ok()
            } else { None });

        if from_rawler.is_some() {
            from_rawler
        } else if let Some(ref jpeg) = embedded_jpeg {
            println!("  rawler unavailable — using embedded JPEG as decode source");
            image::load_from_memory(jpeg).ok()
        } else {
            None
        }
    };

    if let Some(img) = preview_img {
        let (thumbnail, medium) = generate_previews(&img);
        // full.jpeg: full-resolution encode from the decoded RAW (rawler or embedded JPEG)
        let full = write_jpeg_from_rgba(&img.to_rgba8(), 92);
        println!("  RAW -> thumbnail {} B  medium {} B  full {} B",
            thumbnail.len(), medium.len(), full.len());
        upload_file(client, bucket, &thumbnail_key, &thumbnail).await.ok();
        upload_file(client, bucket, &medium_key, &medium).await.ok();
        upload_file(client, bucket, &full_key, &full).await.ok();
        return Ok(HandlerResponse {
            status: "success".to_string(),
            message: format!("RAW previews: {} + {} + {}", thumbnail_key, medium_key, full_key),
            preview_key: Some(thumbnail_key),
            preview_type: "raw_resized".to_string(),
        });
    }

    // No preview could be decoded — generate placeholder for all three outputs
    let placeholder = generate_placeholder_png();
    println!("  No preview found, generating placeholder ({} bytes)", placeholder.len());
    upload_file(client, bucket, &thumbnail_key, &placeholder).await.ok();
    upload_file(client, bucket, &medium_key, &placeholder).await.ok();
    upload_file(client, bucket, &full_key, &placeholder).await.ok();

    Ok(HandlerResponse {
        status: "success".to_string(),
        message: format!("RAW placeholder: {}", thumbnail_key),
        preview_key: Some(thumbnail_key),
        preview_type: "placeholder".to_string(),
    })
}

async fn download_file(client: &Client, bucket: &str, key: &str) -> Result<Vec<u8>, LambdaError> {
    let response = client.get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| LambdaError::from(e.to_string()))?;

    let body = response.body.collect().await
        .map_err(|e| LambdaError::from(e.to_string()))?;
    Ok(body.to_vec())
}

/// Copy an S3 object to a new key within the same bucket (server-side, no data transfer).
async fn copy_s3_object(client: &Client, bucket: &str, src_key: &str, dst_key: &str) -> Result<(), LambdaError> {
    client.copy_object()
        .copy_source(format!("{}/{}", bucket, src_key))
        .bucket(bucket)
        .key(dst_key)
        .send()
        .await
        .map_err(|e| LambdaError::from(e.to_string()))?;
    Ok(())
}

async fn upload_file(client: &Client, bucket: &str, key: &str, data: &[u8]) -> Result<(), LambdaError> {
    let body = ByteStream::from(data.to_vec());
    let metadata: HashMap<String, String> = [
        ("source".to_string(), key.to_string()),
        ("generated".to_string(), chrono::Utc::now().to_rfc3339()),
        ("method".to_string(), "rawler".to_string()),
    ].into_iter().collect();
    client.put_object()
        .bucket(bucket)
        .key(key)
        .body(body)
        .content_type(content_type_for(key))
        .set_metadata(Some(metadata))
        .send()
        .await
        .map_err(|e| LambdaError::from(e.to_string()))?;
    Ok(())
}

fn decode_url(encoded: &str) -> String {
    urlencoding::decode(encoded).unwrap_or_else(|_| encoded.to_string().into()).to_string()
}

#[tokio::main]
async fn main() -> Result<(), LambdaError> {
    let cfg = aws_config::load_from_env().await;
    let client = Client::new(&cfg);
    let client = Arc::new(client);

    run(service_fn(|event: LambdaEvent<TriggerEvent>| {
        let client = Arc::clone(&client);
        async move {
            handle_request(&client, event.payload).await
        }
    })).await
}
