/**
 * Download Signer Lambda
 * 
 * Generates presigned URLs for downloading RAW files.
 * This Lambda will be replaced by a Rust Lambda for better performance.
 */

const { S3Client, GetObjectCommand, HeadObjectCommand } = require('@aws-sdk/client-s3');
const { getSignedUrl } = require('@aws-sdk/s3-request-presigner');

// Create S3 client - presigned URLs work for all file types including TIFF
const s3 = new S3Client({ region: 'eu-west-1' });
const STORAGE_BUCKET = process.env.STORAGE_BUCKET;

exports.handler = async function(event) {
  const origin = event.headers?.Origin || '*';
  const corsHeaders = {
    'Access-Control-Allow-Origin': origin,
    'Access-Control-Allow-Credentials': true,
  };

  const queryStringParameters = event.queryStringParameters || {};
  const key = queryStringParameters.key;
  const type = queryStringParameters.type || 'raw';

  if (!key) {
    return {
      statusCode: 400,
      headers: {
        'Content-Type': 'application/json',
        ...corsHeaders,
      },
      body: JSON.stringify({ error: "Missing 'key' query parameter" }),
    };
  }

  // Resolve preview key if type is "preview" or "medium"
  let objectKey = key;
  let previewExists = false;
  if (type === 'preview') {
    // Try .preview.png first (new format), fall back to .preview.jpg (legacy)
    const candidateKeys = [key + '.preview.png', key + '.preview.jpg'];
    for (const candidate of candidateKeys) {
      try {
        await s3.send(new HeadObjectCommand({
          Bucket: STORAGE_BUCKET,
          Key: candidate,
        }));
        objectKey = candidate;
        previewExists = true;
        break;
      } catch (err) {
        // S3 SDK v3 uses err.name for error types (e.g., 'NotFound')
        if (err.name !== 'NotFound' && err.code !== 'NotFound' && err.code !== '404') {
          throw err;
        }
      }
    }
  } else if (type === 'medium') {
    // Medium: .medium.jpeg only — no fallback (viewer shows spinner if missing)
    try {
      await s3.send(new HeadObjectCommand({ Bucket: STORAGE_BUCKET, Key: key + '.medium.jpeg' }));
      objectKey = key + '.medium.jpeg';
      previewExists = true;
    } catch (err) {
      if (err.name !== 'NotFound' && err.code !== 'NotFound' && err.code !== '404') throw err;
    }
  } else if (type === 'full') {
    // Full-size JPEG — no fallback to original. Either it exists or it doesn't.
    try {
      await s3.send(new HeadObjectCommand({ Bucket: STORAGE_BUCKET, Key: key + '.full.jpeg' }));
      objectKey = key + '.full.jpeg';
      previewExists = true;
    } catch (err) {
      if (err.name !== 'NotFound' && err.code !== 'NotFound' && err.code !== '404') throw err;
    }
    // If full.jpeg doesn't exist, return immediately — don't generate a URL for the original
    if (!previewExists) {
      return {
        statusCode: 200,
        headers: { 'Content-Type': 'application/json', ...corsHeaders },
        body: JSON.stringify({
          download_url: null,
          preview_exists: false,
          file_name: objectKey.split('/').pop(),
          type: type,
        }),
      };
    }
  } else if (type === 'thumbnail' || type === 'preview') {
    // Thumbnail: new .thumbnail.jpeg first, fall back to legacy .preview.png/.jpg
    const candidates = [key + '.thumbnail.jpeg', key + '.preview.png', key + '.preview.jpg'];
    for (const candidate of candidates) {
      try {
        await s3.send(new HeadObjectCommand({ Bucket: STORAGE_BUCKET, Key: candidate }));
        objectKey = candidate;
        previewExists = true;
        break;
      } catch (err) {
        if (err.name !== 'NotFound' && err.code !== 'NotFound' && err.code !== '404') throw err;
      }
    }
  }

  try {
    const command = new GetObjectCommand({
      Bucket: STORAGE_BUCKET,
      Key: objectKey,
    });

    const downloadUrl = await getSignedUrl(s3, command, { expiresIn: 900 });

    return {
      statusCode: 200,
      headers: {
        'Content-Type': 'application/json',
        ...corsHeaders,
      },
      body: JSON.stringify({
        download_url: downloadUrl || null,
        preview_exists: previewExists,
        expires_in: 900,
        file_name: objectKey.split('/').pop(),
        type: type,
      }),
    };
  } catch (err) {
    console.error('Download Signer Error:', err);
    return {
      statusCode: 500,
      headers: {
        'Content-Type': 'application/json',
        ...corsHeaders,
      },
      body: JSON.stringify({ error: 'Failed to generate download URL' }),
    };
  }
};
