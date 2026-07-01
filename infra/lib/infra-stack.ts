import * as path from 'path';
import * as cdk from 'aws-cdk-lib';
import * as s3 from 'aws-cdk-lib/aws-s3';
import * as s3deploy from 'aws-cdk-lib/aws-s3-deployment';
import * as lambda from 'aws-cdk-lib/aws-lambda';

export class InfraStack extends cdk.Stack {
  constructor(scope: cdk.App, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    const siteBucket = new s3.Bucket(this, 'SiteBucket', {
      publicReadAccess: true,
      // Update this specific block to explicitly turn off all blocks:
      blockPublicAccess: new s3.BlockPublicAccess({
        blockPublicAcls: false,
        blockPublicPolicy: false,
        ignorePublicAcls: false,
        restrictPublicBuckets: false,
      }),
      websiteIndexDocument: 'index.html',
      removalPolicy: cdk.RemovalPolicy.DESTROY,
      autoDeleteObjects: true,
    });

    // Ship the static site as part of the stack, so `cdk deploy` is the single
    // source of truth for both the bucket and its contents (no separate s3 sync).
    new s3deploy.BucketDeployment(this, 'SiteContent', {
      sources: [s3deploy.Source.asset(path.join(__dirname, '..', '..', 'frontend'))],
      destinationBucket: siteBucket,
    });

    // Rust weather-forecast backend on the provided.al2023 runtime (Graviton).
    // Build the artifact before `cdk synth`/`deploy` with:
    //   cd backend && cargo lambda build --release --arm64
    // We reference the built `bootstrap` binary directly rather than using the
    // cargo-lambda-cdk auto-bundler, which mis-handles the space in this repo's
    // path ("Code Stuff") when shelling out to `cargo lambda build`.
    const forecastFn = new lambda.Function(this, 'ForecastFn', {
      runtime: lambda.Runtime.PROVIDED_AL2023,
      architecture: lambda.Architecture.ARM_64,
      handler: 'bootstrap', // ignored by the custom runtime, but required.
      code: lambda.Code.fromAsset(
        path.join(__dirname, '..', '..', 'backend', 'target', 'lambda', 'bootstrap'),
      ),
      memorySize: 128,
      timeout: cdk.Duration.seconds(10),
    });

    // Public HTTPS endpoint for the function; no static site coupling.
    // Read-only, unauthenticated forecast data, so CORS is wide open for now.
    const forecastUrl = forecastFn.addFunctionUrl({
      authType: lambda.FunctionUrlAuthType.NONE,
      cors: {
        allowedOrigins: ['*'],
        allowedMethods: [lambda.HttpMethod.GET],
      },
    });

    new cdk.CfnOutput(this, 'ForecastFunctionUrl', {
      value: forecastUrl.url,
      description: 'Base URL of the forecast API; call GET <url>forecast',
    });

    new cdk.CfnOutput(this, 'SiteWebsiteUrl', {
      value: siteBucket.bucketWebsiteUrl,
      description: 'Public URL of the static site',
    });
  }
}
