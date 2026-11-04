const cdk = require('aws-cdk-lib');
const iam = require('aws-cdk-lib/aws-iam');
const cognito = require('aws-cdk-lib/aws-cognito');
const s3 = require('aws-cdk-lib/aws-s3');
const s3deploy = require('aws-cdk-lib/aws-s3-deployment');
const lambda = require('aws-cdk-lib/aws-lambda');
const apigw = require('aws-cdk-lib/aws-apigateway');
const cloudfront = require('aws-cdk-lib/aws-cloudfront');
const cloudfront_origins = require('aws-cdk-lib/aws-cloudfront-origins');
const secretsmanager = require('aws-cdk-lib/aws-secretsmanager');
const path = require('path');

class RawVaultStack extends cdk.Stack {
  constructor(scope, id, props) {
    super(scope, id, props);
    const config = props.config;

    // ── Helper: Add CORS OPTIONS method to any API Gateway resource ────
    const addCorsOptions = (resource) => {
      const corsOptionsIntegration = new apigw.MockIntegration({
        integrationResponses: [
          {
            statusCode: '200',
            responseParameters: {
              'method.response.header.Access-Control-Allow-Headers':
                "'Content-Type,X-Amz-Date,Authorization,X-Api-Key,X-Amz-Security-Token'",
              'method.response.header.Access-Control-Allow-Methods': "'GET,POST,PUT,DELETE,OPTIONS'",
              'method.response.header.Access-Control-Allow-Origin': `'${frontendUrl}'`,
              'method.response.header.Access-Control-Allow-Credentials': "'true'",
              'method.response.header.Access-Control-Max-Age': "'86400'",
            },
          },
        ],
        passthroughBehavior: apigw.PassthroughBehavior.NEVER,
        requestTemplates: {
          'application/json': '{"statusCode": 200}',
        },
      });
      resource.addMethod('OPTIONS', corsOptionsIntegration, {
        authorizationType: apigw.AuthorizationType.NONE,
        methodResponses: [
          {
            statusCode: '200',
            responseParameters: {
              'method.response.header.Access-Control-Allow-Headers': true,
              'method.response.header.Access-Control-Allow-Methods': true,
              'method.response.header.Access-Control-Allow-Origin': true,
              'method.response.header.Access-Control-Allow-Credentials': true,
              'method.response.header.Access-Control-Max-Age': true,
            },
          },
        ],
      });
    };

    // ── 1. S3 Bucket (existing) ──────────────────────────
    const storageBucket = s3.Bucket.fromBucketName(this, 'RawVaultBucket', config.PHOTO_BUCKET);

    // ── 1b. Derived bucket (generated data: renditions, catalog, read model) ──
    // Separate from the photo bucket on purpose: originals are never written to by new code.
    const derivedBucket = new s3.Bucket(this, 'DerivedBucket', {
      blockPublicAccess: s3.BlockPublicAccess.BLOCK_ALL,
      encryption: s3.BucketEncryption.S3_MANAGED,
      enforceSSL: true,
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    // Secret salt for photo ids. Rendition URLs contain the id, so without a secret they could be
    // computed from a folder and file name. Created always and RETAINED: changing it changes every id.
    const idSalt = new secretsmanager.Secret(this, 'IdSalt', {
      description: 'RawVault photo id salt (keeps rendition URLs unguessable). Do not rotate casually.',
      generateSecretString: { passwordLength: 48, excludePunctuation: true },
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    // ── 2. Cognito User Pool (import existing) ───────────
    const userPool = cognito.UserPool.fromUserPoolId(this, 'ExistingUserPool', config.COGNITO_USER_POOL_ID);
    const userPoolClient = cognito.UserPoolClient.fromUserPoolClientId(this, 'RawVaultAppClient', {
      userPoolId: userPool.userPoolId,
      userPoolClientId: config.COGNITO_USER_POOL_CLIENT_ID,
    });

    // ── 3. Lambda Functions ────────────────────────────────
    const authLambda = new lambda.Function(this, 'AuthFunction', {
      functionName: 'rawvault-auth',
      runtime: lambda.Runtime.NODEJS_20_X,
      handler: 'index.handler',
      code: lambda.Code.fromAsset('src/lambda/auth'),
      timeout: cdk.Duration.seconds(30),
      memorySize: 256,
      environment: {
        USER_POOL_ID: userPool.userPoolId,
        CLIENT_ID: config.COGNITO_USER_POOL_CLIENT_ID,
        COGNITO_DOMAIN: config.COGNITO_DOMAIN,
      },
    });

    const listImagesLambda = new lambda.Function(this, 'ListImagesFunction', {
      functionName: 'rawvault-list-images',
      runtime: lambda.Runtime.NODEJS_20_X,
      handler: 'index.handler',
      code: lambda.Code.fromAsset('src/lambda/list-images'),
      timeout: cdk.Duration.seconds(60),
      memorySize: 512,
      environment: {
        USER_POOL_ID: userPool.userPoolId,
        STORAGE_BUCKET: storageBucket.bucketName,
      },
    });

    const downloadSignerLambda = new lambda.Function(this, 'DownloadSignerFunction', {
      functionName: 'rawvault-download-signer',
      runtime: lambda.Runtime.NODEJS_20_X,
      handler: 'index.handler',
      code: lambda.Code.fromAsset('src/lambda/download-signer'),
      timeout: cdk.Duration.seconds(30),
      memorySize: 256,
      environment: {
        USER_POOL_ID: userPool.userPoolId,
        STORAGE_BUCKET: storageBucket.bucketName,
      },
    });

    const previewGeneratorLambda = new lambda.Function(this, 'PreviewGeneratorFunction', {
      functionName: 'rawvault-preview-generator',
      runtime: lambda.Runtime.PROVIDED_AL2023,
      architecture: lambda.Architecture.ARM_64,
      handler: 'bootstrap',
      code: lambda.Code.fromAsset('src/lambda/preview-generator-rust/target/lambda/bootstrap'),
      timeout: cdk.Duration.minutes(15),
      memorySize: 1024,
      environment: {
        STORAGE_BUCKET: storageBucket.bucketName,
        USER_POOL_ID: userPool.userPoolId,
      },
    });

    const previewTriggerLambda = new lambda.Function(this, 'PreviewTriggerFunction', {
      functionName: 'rawvault-preview-trigger',
      runtime: lambda.Runtime.NODEJS_20_X,
      handler: 'index.handler',
      code: lambda.Code.fromAsset('src/lambda/preview-trigger'),
      timeout: cdk.Duration.seconds(30),
      memorySize: 256,
      environment: {
        USER_POOL_ID: userPool.userPoolId,
        STORAGE_BUCKET: storageBucket.bucketName,
      },
    });

    const batchDownloadSignerLambda = new lambda.Function(this, 'BatchDownloadSignerFunction', {
      functionName: 'rawvault-batch-download-signer',
      runtime: lambda.Runtime.NODEJS_20_X,
      handler: 'index.handler',
      code: lambda.Code.fromAsset('src/lambda/batch-download-signer'),
      timeout: cdk.Duration.seconds(30),
      memorySize: 512,
      environment: {
        STORAGE_BUCKET: storageBucket.bucketName,
      },
    });

    // ── 4. Frontend Hosting — S3 + CloudFront ──────────────
    // Must be defined BEFORE the API so we can use its domain for CORS
    const frontendBucket = new s3.Bucket(this, 'RawVaultFrontendBucket', {
      blockPublicAccess: s3.BlockPublicAccess.BLOCK_ALL,
      encryption: s3.BucketEncryption.S3_MANAGED,
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    // ── S3 Access Identity for CloudFront (to serve from photos bucket) ─
    const downloadOAI = new cloudfront.OriginAccessIdentity(this, 'DownloadOAI', {
      comment: 'CloudFront OAI for S3 downloads',
    });

    // ── CloudFront Distribution for S3 downloads (preview/raw files) ──
    const downloadCloudFront = new cloudfront.Distribution(this, 'DownloadCloudFront', {
      comment: 'RawVault S3 download CDN',
      defaultRootObject: '',
      errorResponses: [
        { httpStatus: 403, responseHttpStatus: 403, responsePagePath: '/index.html' },
        { httpStatus: 404, responseHttpStatus: 404, responsePagePath: '/index.html' },
      ],
      defaultBehavior: {
        origin: cloudfront_origins.S3BucketOrigin.withOriginAccessIdentity(storageBucket),
        viewerProtocolPolicy: cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
        allowedMethods: cloudfront.AllowedMethods.ALLOW_GET_HEAD,
        cachePolicy: cloudfront.CachePolicy.CACHING_OPTIMIZED,
        originRequestPolicy: cloudfront.OriginRequestPolicy.CORS_S3_ORIGIN,
        compress: true,
        ttl: {
          min: 0,
          default: 86400,
          max: 31536000, // 1 year for preview files
        },
      },
      priceClass: cloudfront.PriceClass.PRICE_CLASS_100,
      enableLogging: false,
    });

    // Grant CloudFront OAI read access to storage bucket
    storageBucket.grantRead(downloadOAI);

    // ── CloudFront for renditions (derived bucket, private origin via OAC) ──
    // Serves only /derived/*; the catalog and per-photo records in the same bucket stay unreachable.
    // Access control is the unguessable, salted photo id in the path (an "unlisted link" model):
    // fast and cheap, but anyone holding a URL can view that one image. Signed cookies would make it
    // strictly private at the cost of more moving parts; see docs/adr/0001-iceberg-catalog.md.
    const derivedOnly = new cloudfront.Function(this, 'DerivedOnlyFunction', {
      runtime: cloudfront.FunctionRuntime.JS_2_0,
      code: cloudfront.FunctionCode.fromInline(
        "function handler(event) { var uri = event.request.uri; " +
        "if (uri.indexOf('/derived/') !== 0) { return { statusCode: 403, statusDescription: 'Forbidden' }; } " +
        "return event.request; }"),
    });
    const renditionCdn = new cloudfront.Distribution(this, 'RenditionCdn', {
      comment: 'RawVault renditions (derived bucket)',
      defaultBehavior: {
        origin: cloudfront_origins.S3BucketOrigin.withOriginAccessControl(derivedBucket),
        viewerProtocolPolicy: cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
        allowedMethods: cloudfront.AllowedMethods.ALLOW_GET_HEAD,
        cachePolicy: cloudfront.CachePolicy.CACHING_OPTIMIZED,
        compress: false, // JPEGs are already compressed
        functionAssociations: [{ function: derivedOnly, eventType: cloudfront.FunctionEventType.VIEWER_REQUEST }],
      },
      httpVersion: cloudfront.HttpVersion.HTTP2_AND_3,
      priceClass: cloudfront.PriceClass.PRICE_CLASS_100,
      enableLogging: false,
    });

    const cloudFrontDistribution = new cloudfront.Distribution(this, 'FrontendDistribution', {
      comment: 'RawVault Frontend',
      defaultRootObject: 'index.html',
      errorResponses: [
        { httpStatus: 403, responseHttpStatus: 200, responsePagePath: '/index.html' },
        { httpStatus: 404, responseHttpStatus: 200, responsePagePath: '/index.html' },
      ],
      defaultBehavior: {
        origin: cloudfront_origins.S3BucketOrigin.withOriginAccessIdentity(frontendBucket),
        viewerProtocolPolicy: cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
        allowedMethods: cloudfront.AllowedMethods.ALLOW_GET_HEAD,
        cachePolicy: cloudfront.CachePolicy.CACHING_DISABLED,
        originRequestPolicy: cloudfront.OriginRequestPolicy.CORS_S3_ORIGIN,
      },
      priceClass: cloudfront.PriceClass.PRICE_CLASS_100,
      enableLogging: false,
    });

    const frontendDist = path.resolve(__dirname, '../../../frontend/dist');
    new s3deploy.BucketDeployment(this, 'DeployFrontendFiles', {
      sources: [s3deploy.Source.asset(frontendDist)],
      destinationBucket: frontendBucket,
      distribution: cloudFrontDistribution,
      distributionPaths: ['/*'],
    });

    // ── 5. API Gateway ─────────────────────────────────────
    const frontendUrl = `https://${cloudFrontDistribution.distributionDomainName}`;
    // Rust API: serves the timeline read model from the derived bucket (read-only, no access to originals).
    const apiLambda = new lambda.Function(this, 'ApiFunction', {
      functionName: 'rawvault-api',
      runtime: lambda.Runtime.PROVIDED_AL2023,
      architecture: lambda.Architecture.ARM_64,
      handler: 'bootstrap',
      code: lambda.Code.fromAsset(path.resolve(__dirname, '../../../backend/target/lambda/api')),
      timeout: cdk.Duration.seconds(10),
      memorySize: 256,
      environment: {
        DERIVED_BUCKET: derivedBucket.bucketName,
        ALLOWED_ORIGIN: frontendUrl,
        API_STAGE: 'prod',
      },
    });
    derivedBucket.grantRead(apiLambda);

    const api = new apigw.RestApi(this, 'RawVaultAPI', {
      restApiName: 'RawVault API',
      description: 'RawVault RAW Image Preview & Delivery API',
      // Month pages are JSON (up to a few hundred KB); gzip shrinks them roughly tenfold for clients that accept it.
      minCompressionSize: cdk.Size.bytes(1024),
      defaultCorsPreflight: {
        allowOrigins: [frontendUrl],
        allowMethods: ["GET", "POST", "PUT", "DELETE", "OPTIONS"],
        allowHeaders: ['Content-Type', 'Authorization', 'X-Amz-Date', 'X-Amz-Security-Token', 'Origin'],
        allowCredentials: true,
        maxAge: cdk.Duration.hours(1),
      },
      deployOptions: {
        stageName: 'prod',
        loggingLevel: apigw.MethodLoggingLevel.INFO,
        dataTraceEnabled: true,
        throttlingRateLimit: 100,
        throttlingBurstLimit: 50,
      },
    });

    // ── 6. Cognito Authorizer ──────────────────────────────
    const cognitoAuthorizer = new apigw.CognitoUserPoolsAuthorizer(this, 'CognitoAuthorizer', {
      cognitoUserPools: [userPool],
      identitySource: 'method.request.header.Authorization',
    });
    cognitoAuthorizer._attachToApi(api);
    // Method options, NOT integration options: passing these to LambdaIntegration is silently ignored
    // and leaves the method open (authorizationType NONE).
    const protectedMethod = {
      authorizer: cognitoAuthorizer,
      authorizationType: apigw.AuthorizationType.COGNITO,
    };
    const openMethod = { authorizationType: apigw.AuthorizationType.NONE };

    // API Gateway's own 4xx/5xx replies carry no CORS headers by default, which hides 401s from the browser.
    for (const [id, type] of [
      ['Unauthorized', apigw.ResponseType.UNAUTHORIZED],
      ['AccessDenied', apigw.ResponseType.ACCESS_DENIED],
      ['Default4xx', apigw.ResponseType.DEFAULT_4XX],
      ['Default5xx', apigw.ResponseType.DEFAULT_5XX],
    ]) {
      api.addGatewayResponse(`GatewayResponse${id}`, {
        type,
        responseHeaders: {
          'Access-Control-Allow-Origin': `'${frontendUrl}'`,
          'Access-Control-Allow-Credentials': "'true'",
        },
      });
    }

    // ── 7. API Resources ───────────────────────────────────
    const authResource = api.root.addResource('auth');
    const authTokenResource = authResource.addResource('token');
    authTokenResource.addMethod('POST', new apigw.LambdaIntegration(authLambda), openMethod);
    addCorsOptions(authTokenResource);

    const imagesResource = api.root.addResource('images');
    imagesResource.addMethod('GET', new apigw.LambdaIntegration(listImagesLambda), protectedMethod);
    addCorsOptions(imagesResource);

    const downloadResource = imagesResource.addResource('download');
    downloadResource.addMethod('GET', new apigw.LambdaIntegration(downloadSignerLambda), protectedMethod);
    addCorsOptions(downloadResource);

    const batchDownloadResource = downloadResource.addResource('batch');
    batchDownloadResource.addMethod('POST', new apigw.LambdaIntegration(batchDownloadSignerLambda), protectedMethod);
    addCorsOptions(batchDownloadResource);

    const previewResource = api.root.addResource('preview');
    previewResource.addMethod('GET', new apigw.LambdaIntegration(previewGeneratorLambda), protectedMethod);
    addCorsOptions(previewResource);

    const previewTriggerResource = api.root.addResource('preview-trigger');
    previewTriggerResource.addMethod('POST', new apigw.LambdaIntegration(previewTriggerLambda), {
      ...protectedMethod,
      methodResponses: [
        { statusCode: '200' },
        { statusCode: '400' },
        { statusCode: '401' },
        { statusCode: '500' },
      ],
    });
    addCorsOptions(previewTriggerResource);

    // Timeline (Rust API Lambda)
    const timelineResource = api.root.addResource('timeline');
    timelineResource.addMethod('GET', new apigw.LambdaIntegration(apiLambda), protectedMethod);
    addCorsOptions(timelineResource);
    const monthsResource = timelineResource.addResource('months');
    const monthResource = monthsResource.addResource('{month}');
    monthResource.addMethod('GET', new apigw.LambdaIntegration(apiLambda), protectedMethod);
    addCorsOptions(monthResource);

    // ── 7b. Indexing pipeline (OFF unless `-c enablePipeline=true`) ────────
    // Renders originals into the derived bucket and publishes the timeline. It is invoked by hand,
    // never by an event or API route. Enabling it is what gives a role READ access to the photo
    // bucket, so it is a deliberate, owner-approved step: `cdk deploy -c enablePipeline=true`.
    if (this.node.tryGetContext('enablePipeline') === 'true' || this.node.tryGetContext('enablePipeline') === true) {
      const pipelineLambda = new lambda.Function(this, 'PipelineFunction', {
        functionName: 'rawvault-pipeline',
        runtime: lambda.Runtime.PROVIDED_AL2023,
        architecture: lambda.Architecture.ARM_64,
        handler: 'bootstrap',
        code: lambda.Code.fromAsset(path.resolve(__dirname, '../../../backend/target/lambda/pipeline')),
        timeout: cdk.Duration.minutes(15),
        // Decoded frames are large (a 16-bit 60 MP TIFF is ~360 MB); Lambda CPU scales with memory too.
        memorySize: 3008,
        environment: {
          PHOTO_BUCKET: storageBucket.bucketName,
          DERIVED_BUCKET: derivedBucket.bucketName,
          ID_SALT: idSalt.secretValue.unsafeUnwrap(),
        },
      });
      derivedBucket.grantReadWrite(pipelineLambda);
      // Photo bucket: list under the originals prefix and read objects. Nothing else, no writes, no deletes.
      pipelineLambda.addToRolePolicy(new iam.PolicyStatement({
        actions: ['s3:ListBucket'],
        resources: [storageBucket.bucketArn],
        conditions: { StringLike: { 's3:prefix': ['all images/*', 'all images/'] } },
      }));
      pipelineLambda.addToRolePolicy(new iam.PolicyStatement({
        actions: ['s3:GetObject'],
        resources: [storageBucket.arnForObjects('all images/*')],
      }));
      new cdk.CfnOutput(this, 'PipelineFunctionName', {
        value: pipelineLambda.functionName,
        description: 'Invoke manually: plan / run / publish',
      });
    }

    // ── 8. IAM Permissions ─────────────────────────────────
    storageBucket.grantRead(listImagesLambda);
    storageBucket.grantRead(downloadSignerLambda);
    storageBucket.grantReadWrite(previewGeneratorLambda);
    storageBucket.grantRead(batchDownloadSignerLambda);

    previewTriggerLambda.addToRolePolicy(new iam.PolicyStatement({
      actions: ['lambda:InvokeFunction'],
      resources: [previewGeneratorLambda.functionArn],
    }));

    authLambda.addToRolePolicy(new iam.PolicyStatement({
      actions: ['cognito-idp:DescribeUserPoolClient'],
      resources: [`arn:aws:cognito-idp:${this.region}:${this.account}:userpool/${config.COGNITO_USER_POOL_ID}`],
    }));

    // ── 9. Outputs ─────────────────────────────────────────
    new cdk.CfnOutput(this, 'FrontendURL', {
      value: frontendUrl,
      description: 'Frontend URL (CloudFront)',
    });
    new cdk.CfnOutput(this, 'DownloadCDNURL', {
      value: `https://${downloadCloudFront.distributionDomainName}`,
      description: 'Download CDN URL (for S3 files)',
    });
    new cdk.CfnOutput(this, 'FrontendBucketName', {
      value: frontendBucket.bucketName,
      description: 'Frontend S3 Bucket Name',
    });
    new cdk.CfnOutput(this, 'DerivedBucketName', {
      value: derivedBucket.bucketName,
      description: 'Derived data bucket (renditions, catalog, read model)',
    });
    new cdk.CfnOutput(this, 'RenditionCdnURL', {
      value: `https://${renditionCdn.distributionDomainName}`,
      description: 'Base URL for VITE_RENDITION_BASE_URL (serves /derived/*)',
    });
    new cdk.CfnOutput(this, 'UserPoolId', {
      value: userPool.userPoolId,
      description: 'Cognito User Pool ID',
    });
    new cdk.CfnOutput(this, 'UserPoolClientId', {
      value: config.COGNITO_USER_POOL_CLIENT_ID,
      description: 'Cognito User Pool Client ID',
    });
    new cdk.CfnOutput(this, 'StorageBucketName', {
      value: storageBucket.bucketName,
      description: 'S3 Storage Bucket Name',
    });
    new cdk.CfnOutput(this, 'APIEndpoint', {
      value: api.url,
      description: 'API Gateway Endpoint URL',
    });
  }
}

module.exports = { RawVaultStack };
