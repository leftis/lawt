'use strict';

const fs = require('fs');
const path = require('path');

// One settings file for the whole project: <repo>/lawt.env (git-ignored).
// Read literally, like `docker run --env-file`: KEY=value per line, no quote removal,
// no `$` expansion, `#` comments and blank lines skipped. Real environment variables win.
const ENV_FILE = path.resolve(__dirname, '../../../lawt.env');

const REQUIRED = {
  AWS_ACCOUNT: 'AWS account id that owns the stack',
  AWS_REGION: 'AWS region to deploy to, for example eu-west-1',
  PHOTO_BUCKET: 'Existing S3 bucket with your original photos (only ever read)',
  COGNITO_USER_POOL_ID: 'Existing Cognito user pool id, for example eu-west-1_AbCdEfGhI',
  COGNITO_USER_POOL_CLIENT_ID: 'App client id of that pool (needs a client secret and the code grant)',
  COGNITO_DOMAIN: 'Hosted UI host, for example my-prefix.auth.eu-west-1.amazoncognito.com',
};

const OPTIONAL = {
  APP_NAME: 'lawt', // prefix of function names and other physical names
  STACK_NAME: 'LawtStack', // CloudFormation stack name
};

function readEnvFile(file) {
  if (!fs.existsSync(file)) return {};
  const out = {};
  for (const line of fs.readFileSync(file, 'utf8').split(/\r?\n/)) {
    if (line === '' || line.startsWith('#')) continue;
    const eq = line.indexOf('=');
    if (eq > 0) out[line.slice(0, eq)] = line.slice(eq + 1);
  }
  return out;
}

/** Loads and validates the deployment settings; throws one error listing everything missing. */
function loadConfig() {
  const fromFile = readEnvFile(ENV_FILE);
  const pick = (key) => process.env[key] || fromFile[key] || '';
  const cfg = {};
  const missing = [];
  for (const key of Object.keys(REQUIRED)) {
    cfg[key] = pick(key);
    if (!cfg[key]) missing.push(`  ${key}: ${REQUIRED[key]}`);
  }
  if (missing.length > 0) {
    throw new Error(
      `Missing settings. Copy lawt.env.example to lawt.env and fill in:\n${missing.join('\n')}`,
    );
  }
  for (const [key, fallback] of Object.entries(OPTIONAL)) cfg[key] = pick(key) || fallback;
  return cfg;
}

module.exports = { loadConfig };
