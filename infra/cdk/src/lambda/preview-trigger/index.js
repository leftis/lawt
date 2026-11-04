const { LambdaClient, InvokeCommand } = require('@aws-sdk/client-lambda');
const lambdaClient = new LambdaClient({});

exports.handler = async (event) => {
  const rawPath = event.queryStringParameters?.raw_path;

  if (!rawPath) {
    return {
      statusCode: 400,
      headers: {
        'Content-Type': 'application/json',
        'Access-Control-Allow-Origin': '*',
      },
      body: JSON.stringify({ error: 'Missing query parameter: raw_path' }),
    };
  }

  const key = rawPath;
  const invokePayload = { key: key };

  try {
    // Fire-and-forget: async invocation avoids timeout on large RAW files (50 MB Fuji RAF etc.)
    // The Rust Lambda can take 30-60s; synchronous call would time out the trigger.
    await lambdaClient.send(
      new InvokeCommand({
        FunctionName: 'rawvault-preview-generator',
        InvocationType: 'Event',          // async — returns 202 immediately
        Payload: JSON.stringify(invokePayload),
      })
    );

    return {
      statusCode: 200,
      headers: {
        'Content-Type': 'application/json',
        'Access-Control-Allow-Origin': '*',
      },
      body: JSON.stringify({
        message: 'Preview generation triggered',
        rawPath: rawPath,
        status: 'processing',
      }),
    };
  } catch (err) {
    return {
      statusCode: 500,
      headers: {
        'Content-Type': 'application/json',
        'Access-Control-Allow-Origin': '*',
      },
      body: JSON.stringify({ error: 'Failed to trigger preview generation', details: err.message }),
    };
  }
};
