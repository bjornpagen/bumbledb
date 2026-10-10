/**
 * Alchemy deployment: one Next.js server function, the database's checkpoint bucket (S3 Standard,
 * versioned) and the app's blob bucket. The log bucket is an S3 Express directory bucket in the
 * function's region, provisioned out of band and named by BUMBLEDB_LOG_BUCKET; it has no lifecycle
 * rule. The server may read and create objects in both database buckets and may delete only
 * checkpoints. Run `pnpm migrate <tenant>...` against the deployed buckets before shifting traffic.
 */
import * as Alchemy from "alchemy"
import * as AWS from "alchemy/AWS"
import * as Effect from "effect/Effect"

function required(name: string): string {
	const value = process.env[name]
	if (value === undefined || value === "") throw new Error(`${name} is required`)
	return value
}

const logBucket = required("BUMBLEDB_LOG_BUCKET")
const checkpointBucket = required("BUMBLEDB_CKPT_BUCKET")
const blobPrefix = "blobs"

export const Website = AWS.Website.Nextjs("Website", {
	runtime: "nodejs24.x",
	architecture: "arm64",
	env: {
		BUMBLEDB_LOG_BUCKET: logBucket,
		BUMBLEDB_CKPT_BUCKET: checkpointBucket,
		NODE_ENV: "production"
	}
})

export default Alchemy.Stack(
	"Notes",
	{ providers: AWS.providers(), state: AWS.state() },
	Effect.gen(function* () {
		yield* AWS.S3.Bucket("BumbledbCheckpoints", { bucketName: checkpointBucket, versioning: "Enabled" })
		const blobs = yield* AWS.S3.Bucket("AppBlobs", {})
		const site = yield* Website
		if (site.server) {
			yield* site.server.bind`BumbledbDataWriter(${site.server})`({
				policyStatements: [
					{
						Effect: "Allow",
						Action: ["s3express:CreateSession"],
						Resource: [`arn:aws:s3express:*:*:bucket/${logBucket}`]
					},
					{
						Effect: "Allow",
						Action: ["s3:GetObject", "s3:PutObject"],
						Resource: [`arn:aws:s3:::${checkpointBucket}/tenants/*`]
					},
					{ Effect: "Allow", Action: ["s3:ListBucket"], Resource: [`arn:aws:s3:::${checkpointBucket}`] },
					{
						Effect: "Allow",
						Action: ["s3:DeleteObject"],
						Resource: [`arn:aws:s3:::${checkpointBucket}/tenants/*/ckpt/*`]
					},
					{ Effect: "Allow", Action: ["s3:PutObject"], Resource: [`arn:aws:s3:::${blobs.bucketName}/${blobPrefix}/*`] }
				]
			})
		}
		return { url: site.url, blobBucket: blobs.bucketName }
	})
)
