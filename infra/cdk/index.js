#!/usr/bin/env node
const cdk = require('aws-cdk-lib');
const { LawtStack } = require('./lib/lawt-stack.js');
const { loadConfig } = require('./lib/config.js');

const app = new cdk.App();
const config = loadConfig();

new LawtStack(app, config.STACK_NAME, {
  env: {
    account: config.AWS_ACCOUNT,
    region: config.AWS_REGION,
  },
  config,
  description: 'lawt: RAW photo timeline (AWS CDK)',
});
