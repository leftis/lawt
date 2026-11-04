#!/usr/bin/env node
const cdk = require('aws-cdk-lib');
const { RawVaultStack } = require('./lib/rawvault-stack.js');
const { loadConfig } = require('./lib/config.js');

const app = new cdk.App();
const config = loadConfig();

new RawVaultStack(app, 'RawVaultStack', {
  env: {
    account: config.AWS_ACCOUNT,
    region: config.AWS_REGION,
  },
  config,
  description: 'RawVault - High-Performance RAW Image Preview & Delivery System (AWS CDK)',
});
