/**
 * List Images Lambda
 * 
 * Lists all images in the storage bucket, optionally filtered by gallery.
 * This Lambda will be replaced by a Rust Lambda for better performance.
 */

const { S3Client, ListObjectsV2Command } = require('@aws-sdk/client-s3');
const STORAGE_PREFIX = 'all images/';
const ROOT_GALLERY = 'all images';

const s3 = new S3Client({ region: 'eu-west-1' });
const STORAGE_BUCKET = process.env.STORAGE_BUCKET;

const RAW_EXTENSIONS = new Set([
  'raf', 'nef', 'cr2', 'cr3', 'arw', 'dng', 'orf', 'sr2', '3fr', 'fff',
  'mos', 'ia', 'pef', 'rw2', 'rwl', 'rwz', 'x3f', 'srw', 'k25', 'kdc',
]);

const IMAGE_EXTENSIONS = new Set([
  ...RAW_EXTENSIONS,
  'jpg', 'jpeg', 'png', 'webp', 'tiff', 'tif', 'bmp', 'gif', 'heic', 'heif',
]);

exports.handler = async function(event) {
  try {
    // API Gateway may lowercase header keys
    const origin = event.headers?.Origin || event.headers?.origin || '*';
    const corsHeaders = {
      'Access-Control-Allow-Origin': origin,
      'Access-Control-Allow-Credentials': true,
    };

    const qs = event.queryStringParameters || {};

    // ?galleries=1 → only the gallery names (one cheap delimiter listing, no per-object scan)
    if (qs.galleries) {
      const names = new Set();
      let token;
      do {
        const res = await s3.send(new ListObjectsV2Command({
          Bucket: STORAGE_BUCKET,
          Prefix: STORAGE_PREFIX,
          Delimiter: '/',
          ContinuationToken: token,
        }));
        for (const p of res.CommonPrefixes || []) {
          names.add(p.Prefix.slice(STORAGE_PREFIX.length).replace(/\/$/, ''));
        }
        // Files sitting directly under "all images/" form the root gallery
        if ((res.Contents || []).some((f) => IMAGE_EXTENSIONS.has(f.Key.split('.').pop().toLowerCase()))) {
          names.add(ROOT_GALLERY);
        }
        token = res.NextContinuationToken;
      } while (token);
      return {
        statusCode: 200,
        headers: { 'Content-Type': 'application/json', ...corsHeaders },
        body: JSON.stringify([...names].sort()),
      };
    }

    // Gallery filter → list only that prefix. The root gallery ("all images") is the
    // set of files directly under STORAGE_PREFIX, so it needs a delimiter.
    const gallery = qs.gallery;
    if (gallery && (gallery.includes('..') || gallery.length > 255)) {
      return {
        statusCode: 400,
        headers: { 'Content-Type': 'application/json', ...corsHeaders },
        body: JSON.stringify({ error: 'Invalid gallery' }),
      };
    }
    const isRoot = gallery === ROOT_GALLERY;
    const prefix = gallery ? (isRoot ? STORAGE_PREFIX : `${STORAGE_PREFIX}${gallery}/`) : STORAGE_PREFIX;
    const delimiter = isRoot ? '/' : undefined;

    // List objects with pagination (handle >1000 objects)
    const images = [];
    let continuationToken = undefined;

    do {
      const command = new ListObjectsV2Command({
        Bucket: STORAGE_BUCKET,
        Prefix: prefix,
        Delimiter: delimiter,
        MaxKeys: 1000,
        ContinuationToken: continuationToken,
      });

      const response = await s3.send(command);
      const files = response.Contents || [];

      for (const file of files) {
        const key = file.Key;
        
        // Skip all generated derivative files — only list source images
        if (
          key.endsWith('.preview.png') ||
          key.endsWith('.preview.jpg') ||
          key.endsWith('.medium.jpeg') ||
          key.endsWith('.thumbnail.jpeg') ||
          key.endsWith('.full.jpeg')
        ) continue;
        
        // Extract extension and check if it's an image
        const ext = key.split('.').pop().toLowerCase();
        if (!IMAGE_EXTENSIONS.has(ext)) continue;

        // Extract gallery from path: "all images/100RICOH/file.RAF" -> "100RICOH"
        // Files directly in "all images/" get gallery "all images"
        const relativePath = key.startsWith(STORAGE_PREFIX) ? key.slice(STORAGE_PREFIX.length) : key;
        const parts = relativePath.split('/');
        const gallery = parts.length === 1 ? ROOT_GALLERY : parts[0];

        images.push({
          id: key,
          name: key.split('/').pop().replace(/\.\w+$/, ''),
          ext: ext,
          raw_path: key,
          preview_path: key + '.preview.png',
          medium_preview_path: key + '.medium.jpeg',
          size_bytes: file.Size,
          updated_at: file.LastModified?.toISOString() || new Date().toISOString(),
          gallery: gallery,
          path: key,
          is_raw: RAW_EXTENSIONS.has(ext),
        });
      }

      continuationToken = response.NextContinuationToken;
    } while (continuationToken);

    images.sort((a, b) => new Date(b.updated_at) - new Date(a.updated_at));

    return {
      statusCode: 200,
      headers: {
        'Content-Type': 'application/json',
        ...corsHeaders,
      },
      body: JSON.stringify(images),
    };
  } catch (err) {
    console.error('List Images Error:', err);
    return {
      statusCode: 500,
      headers: {
        'Content-Type': 'application/json',
        'Access-Control-Allow-Origin': '*',
        'Access-Control-Allow-Credentials': true,
      },
      body: JSON.stringify({ error: 'Failed to list images', message: err.message }),
    };
  }
};
