/**
 * Alchemy deployment (chapter 33 "Alchemy: ordinary infrastructure,
 * attached permissions"): one Next.js server on nodejs24.x/arm64, the
 * bumbledb log bucket as EXPLICITLY provisioned durable data (separate
 * from website asset/ISR buckets and from the app's blob bucket), and the
 * data-writer policy ATTACHED to the actual server function via
 * `server.bind` — returning an unattached role ARN is not permission
 * configuration (the audited defect this replaces).
 *
 * Credentials refresh through the supported provider chain at runtime; no
 * static keys exist anywhere. Admin/GC/backup identities are separate,
 * prefix-constrained roles provisioned by the operator — normal app code
 * exposes no migrations or GC. The web Function URL is public: every app
 * route authenticates even when callers bypass the CDN (src/auth.ts).
 */
import * as Alchemy from "alchemy"
import * as AWS from "alchemy/AWS"
import * as Effect from "effect/Effect"

const logPrefix = "log"
const blobPrefix = "blobs"

export const Website = AWS.Website.Nextjs("Website", {
	runtime: "nodejs24.x",
	architecture: "arm64",
	env: {
		// The deployed tenant registry file ships with the server unit; the
		// runtime contract path is the traced default. SESSION_SECRET arrives
		// from the platform's secret store, never from this file.
		BUMBLEDB_TARGET: "linux-arm64",
		NODE_ENV: "production"
	}
})

export default Alchemy.Stack(
	"Notes",
	{ providers: AWS.providers(), state: AWS.state() },
	Effect.gen(function* () {
		// Durable data buckets — never recreated by a failed read.
		const log = yield* AWS.S3.Bucket("BumbledbLog", {})
		const blobs = yield* AWS.S3.Bucket("AppBlobs", {})
		const site = yield* Website
		if (site.server) {
			// The data-writer policy, ATTACHED to the server function. The
			// role holding s3:GetObject/PutObject on the log prefix is trusted
			// to run the protocol; S3 IAM cannot interpret HEAD fields as
			// application authorization, so keep this scope tenant-data-tight.
			yield* site.server.bind`BumbledbDataWriter(${site.server})`({
				policyStatements: [
					{
						Effect: "Allow",
						Action: ["s3:GetObject", "s3:PutObject"],
						Resource: [`arn:aws:s3:::${log.bucketName}/${logPrefix}/*`]
					},
					{
						Effect: "Allow",
						Action: ["s3:PutObject"],
						Resource: [`arn:aws:s3:::${blobs.bucketName}/${blobPrefix}/*`]
					}
				]
			})
		}
		return {
			url: site.url,
			logBucket: log.bucketName,
			blobBucket: blobs.bucketName
		}
	})
)
