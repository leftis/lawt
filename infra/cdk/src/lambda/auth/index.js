const https = require('https');
const { CognitoIdentityProviderClient, DescribeUserPoolClientCommand } = require('@aws-sdk/client-cognito-identity-provider');

const userPoolRegion = (process.env.USER_POOL_ID || '').split('_')[0] || 'eu-west-1';
const cognito = new CognitoIdentityProviderClient({ region: userPoolRegion });

exports.handler = async (event) => {
  try {
    // API Gateway may lowercase header keys
    const headers = event.headers || {};
    const origin = headers.Origin || headers.origin || '*';
    const corsHeaders = {
      'Access-Control-Allow-Origin': origin,
      'Access-Control-Allow-Credentials': true,
    };

    // Parse body - API Gateway Lambda Proxy sends body as string, direct Lambda calls may send object directly
    const rawBody = event.body;
    let body;
    if (typeof rawBody === 'string') {
      body = JSON.parse(rawBody);
    } else if (rawBody && typeof rawBody === 'object') {
      // Direct Lambda call with object body (for testing)
      body = rawBody;
    } else {
      // Try event itself
      body = (event && typeof event === 'object') ? event : {};
    }
    const { code, redirect_uri } = body;

    if (!code || !redirect_uri) {
      return {
        statusCode: 400,
        headers: { 'Content-Type': 'application/json', ...corsHeaders },
        body: JSON.stringify({ error: 'Missing code or redirect_uri' }),
      };
    }

    const userPoolId = process.env.USER_POOL_ID;
    const clientId = process.env.CLIENT_ID;
    if (!userPoolId || !clientId) {
      return {
        statusCode: 500,
        headers: { 'Content-Type': 'application/json', ...corsHeaders },
        body: JSON.stringify({ error: 'Lambda environment not configured' }),
      };
    }
    const region = userPoolId.split('_')[0];

    // Fetch client secret from Cognito at runtime
    let clientSecret;
    try {
      const describeCommand = new DescribeUserPoolClientCommand({
        UserPoolId: userPoolId,
        ClientId: clientId,
      });
      const describeResponse = await cognito.send(describeCommand);
      clientSecret = describeResponse.UserPoolClient?.ClientSecret;
    } catch (err) {
      console.error('describeUserPoolClient error:', err.message);
      return {
        statusCode: 500,
        headers: { 'Content-Type': 'application/json', ...corsHeaders },
        body: JSON.stringify({ error: 'Failed to fetch client secret', details: err.message }),
      };
    }

    const cognitoDomain = process.env.COGNITO_DOMAIN || `${region}.amazoncognito.com`;
    const tokenUrl = `https://${cognitoDomain}/oauth2/token`;
    const params = new URLSearchParams({
      grant_type: 'authorization_code',
      client_id: clientId,
      code: code,
      redirect_uri: redirect_uri,
      client_secret: clientSecret,
    });

    const response = await new Promise((resolve, reject) => {
      const req = https.request(
        tokenUrl,
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/x-www-form-urlencoded',
            'Content-Length': Buffer.byteLength(params.toString()),
          },
        },
        (res) => {
          let data = '';
          res.on('data', (chunk) => (data += chunk));
          res.on('end', () => {
            try {
              const parsed = JSON.parse(data);
              // Cognito returns 400 for auth errors - treat as success with error fields
              if (res.statusCode >= 200 && res.statusCode < 300) {
                resolve(parsed);
              } else {
                // API error
                resolve({ error: parsed.code || parsed.error, error_description: parsed.message || parsed.error_description });
              }
            } catch (e) {
              reject(new Error('Failed to parse token response (' + res.statusCode + '): ' + data));
            }
          });
        }
      );
      req.on('error', (e) => reject(e));
      req.on('timeout', () => { req.destroy(); reject(new Error('HTTPS request timed out')); });
      req.setTimeout(10000);
      req.write(params.toString());
      req.end();
    });

    if (response.error) {
      return {
        statusCode: 400,
        headers: { 'Content-Type': 'application/json', ...corsHeaders },
        body: JSON.stringify({ error: response.error_description || 'Token exchange failed', code: response.error }),
      };
    }

    return {
      statusCode: 200,
      headers: { 'Content-Type': 'application/json', ...corsHeaders },
      body: JSON.stringify({
        access_token: response.access_token,
        id_token: response.id_token,
        refresh_token: response.refresh_token,
        token_type: response.token_type,
        expires_in: response.expires_in,
      }),
    };
  } catch (err) {
    console.error('Unhandled error:', err.message, err.stack);
    return {
      statusCode: 500,
      headers: { 'Content-Type': 'application/json', ...corsHeaders },
      body: JSON.stringify({ error: 'Internal server error', details: err.message }),
    };
  }
};
