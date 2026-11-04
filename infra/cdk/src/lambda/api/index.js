/**
 * RawVault API Lambda
 * 
 * Routes requests to the appropriate handler.
 * This Lambda will be replaced by a Rust Lambda for better performance.
 */

import { listImagesHandler } from '../list-images/index.js';
import { downloadSignerHandler } from '../download-signer/index.js';

export async function handler(event) {
  const path = (event.path || '').replace(/^\/api\//, '');
  const method = event.httpMethod || event.requestContext?.httpMethod;

  // Simple routing - in production, this would use API Gateway route table
  if (path === 'images' && method === 'GET') {
    return await listImagesHandler(event);
  }

  if (path === 'images/download' && method === 'GET') {
    return await downloadSignerHandler(event);
  }

  return {
    statusCode: 404,
    body: JSON.stringify({ error: 'Not found' }),
  };
}
