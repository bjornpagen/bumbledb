/**
 * The buckets the real-S3 conformance suite runs against: the log on an S3 Express One Zone directory
 * bucket (Alchemy has no directory bucket resource, so Cloud Control creates it) and checkpoints on a
 * versioned S3 Standard bucket. State is local; the stack lives for one test run and is destroyed after.
 */
import * as Alchemy from "alchemy"
import * as AWS from "alchemy/AWS"
import * as Effect from "effect/Effect"
import { buckets, zone } from "./buckets.ts"

export default Alchemy.Stack(
	"BumbledbS3",
	{ providers: AWS.providers(), state: Alchemy.localState() },
	Effect.gen(function* () {
		const names = buckets(yield* Alchemy.Stage)
		const log = yield* AWS.CloudControl.Resource("Log", {
			typeName: "AWS::S3Express::DirectoryBucket",
			desiredState: { BucketName: names.log, DataRedundancy: "SingleAvailabilityZone", LocationName: zone }
		})
		const checkpoints = yield* AWS.S3.Bucket("Checkpoints", {
			bucketName: names.checkpoints,
			versioning: "Enabled",
			forceDestroy: true
		})
		return { log: log.identifier, checkpoints: checkpoints.bucketName }
	})
)
