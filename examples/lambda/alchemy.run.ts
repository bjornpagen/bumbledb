import * as Alchemy from "alchemy"
import * as AWS from "alchemy/AWS"
import * as Duration from "effect/Duration"
import * as Effect from "effect/Effect"

export default Alchemy.Stack(
	"BumbledbLambda",
	{ providers: AWS.providers(), state: AWS.state() },
	Effect.gen(function* () {
		const bucket = yield* AWS.S3.Bucket("Log", {})
		const prefix = "log"
		// Intended prefix Get/Put/Delete. Function always mints its own role and cannot take this one.
		const role = yield* AWS.IAM.Role("Fn", {
			assumeRolePolicyDocument: {
				Version: "2012-10-17",
				Statement: [
					{
						Effect: "Allow",
						Principal: { Service: "lambda.amazonaws.com" },
						Action: ["sts:AssumeRole"]
					}
				]
			},
			inlinePolicies: {
				Prefix: {
					Version: "2012-10-17",
					Statement: [
						{
							Effect: "Allow",
							Action: ["s3:GetObject", "s3:PutObject", "s3:DeleteObject"],
							Resource: [`arn:aws:s3:::${bucket.bucketName}/${prefix}/*`]
						}
					]
				}
			}
		})
		const duty = yield* AWS.Lambda.LayerVersion("Duty", {
			path: "./layer/duty",
			compatibleRuntimes: ["nodejs24.x"],
			compatibleArchitectures: ["arm64"]
		})
		const fn = yield* AWS.Lambda.Function("Api", {
			main: "./src/handler.ts",
			functionUrl: true,
			runtime: "nodejs24.x",
			architecture: "arm64",
			memorySize: 512,
			timeout: Duration.seconds(60),
			env: { BUCKET: bucket.bucketName, PREFIX: prefix },
			build: {
				install: [
					"@bjornpagen/bumbledb",
					"@bjornpagen/bumbledb-linux-arm64",
					"@bjornpagen/bumbledb-log"
				]
			},
			layers: [duty]
		})
		yield* AWS.Scheduler.every("5 minutes").toLambda(fn, { input: { duty: true } })
		return {
			url: fn.functionUrl,
			bucketName: bucket.bucketName,
			intendedRoleArn: role.roleArn
		}
	})
)
