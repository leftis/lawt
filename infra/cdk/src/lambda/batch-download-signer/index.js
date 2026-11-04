/**
 * Batch Download Signer Lambda
 * 
 * Generates presigned URLs for multiple files in a single invocation.
 * Replaces N individual download-signer calls with 1 batch call.
 */

const { S3Client, GetObjectCommand, HeadObjectCommand } = require('@aws-sdk/client-s3');
const { getSignedUrl } = require('@aws-sdk/s3-request-presigner');

const s3 = new S3Client({ region: 'eu-west-1' });
const STORAGE_BUCKET = process.env.STORAGE_BUCKET;

const TYPE_MAP = {
  raw:       [null],
  thumbnail: ['.thumbnail.jpeg'],                                   // gallery cards — no PNG fallback, show placeholder if missing
  preview:   ['.thumbnail.jpeg', '.preview.png', '.preview.jpg'],  // legacy alias with fallback
  medium:    ['.medium.jpeg'],                                      // fullscreen viewer
  full:      ['.full.jpeg'],                                        // full-size JPEG (RAW only)
};

exports.handler = async function (event) {
  const origin = event.headers?.Origin || event.headers?.origin || '*';
  const corsHeaders = {
    'Access-Control-Allow-Origin': origin,
    'Access-Control-Allow-Credentials': true,
  };

  // Support both POST body (API Gateway) and direct Lambda invocation
  let requests = [];
  if (event.body) {
    try {
      const parsed = typeof event.body === 'string' ? JSON.parse(event.body) : event.body;
      requests = parsed.requests || parsed;
    } catch {
      return {
        statusCode: 400,
        headers: { 'Content-Type': 'application/json', ...corsHeaders },
        body: JSON.stringify({ error: 'Invalid JSON body' }),
      };
    }
  } else if (event.requests) {
    requests = event.requests;
  } else if (Array.isArray(event)) {
    requests = event;
  }

  if (!Array.isArray(requests) || requests.length === 0) {
    return {
      statusCode: 400,
      headers: { 'Content-Type': 'application/json', ...corsHeaders },
      body: JSON.stringify({ error: "Need 'requests' array" }),
    };
  }

  // Process in parallel with concurrency limit
  const MAX_CONCURRENT = 10;
  const results = [];

  for (let i = 0; i < requests.length; i += MAX_CONCURRENT) {
    const batch = requests.slice(i, i + MAX_CONCURRENT);
    const batchResults = await Promise.all(
      batch.map(async (req) => {
        const key = typeof req === 'string' ? req : req.key;
        const type = typeof req === 'string' ? 'raw' : (req.type || 'raw');
        const fileName = typeof req === 'string' ? key.split('/').pop() : req.file_name || key.split('/').pop();

        try {
          let objectKey = key;
          let previewExists = false;

          const extensions = TYPE_MAP[type] || TYPE_MAP.raw;
          for (const ext of extensions) {
            if (ext === null) {
              // raw type: always sign the original file
              objectKey = key;
              previewExists = true;
              break;
            }
            try {
              await s3.send(
                new HeadObjectCommand({ Bucket: STORAGE_BUCKET, Key: key + ext })
              );
              objectKey = key + ext;
              previewExists = true;
              break;
            } catch (err) {
              if (err.name !== 'NotFound' && err.code !== 'NotFound' && err.code !== '404') {
                throw err;
              }
            }
          }

          // No preview found — return null so frontend shows placeholder
          // (avoids signing a URL to the raw DNG which browsers can't render)
          if (!previewExists) {
            return {
              key,
              download_url: null,
              preview_exists: false,
              file_name: fileName,
              type,
            };
          }

          const command = new GetObjectCommand({
            Bucket: STORAGE_BUCKET,
            Key: objectKey,
          });

          const downloadUrl = await getSignedUrl(s3, command, { expiresIn: 900 });

          return {
            key,
            download_url: downloadUrl,
            preview_exists: previewExists,
            file_name: fileName,
            type,
          };
        } catch (err) {
          console.error(`Batch signer error for ${key}:`, err.message);
          return {
            key,
            download_url: null,
            preview_exists: false,
            file_name: fileName,
            type,
            error: err.message,
          };
        }
      })
    );
    results.push(...batchResults);
  }

  return {
    statusCode: 200,
    headers: { 'Content-Type': 'application/json', ...corsHeaders },
    body: JSON.stringify({ urls: results }),
  };
};
