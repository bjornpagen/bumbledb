/** Fails if any bucket of this stage still exists. */
import { HeadBucketCommand, ListDirectoryBucketsCommand, S3Client } from "@aws-sdk/client-s3"
import { buckets, region, stage } from "../buckets.ts"

const client = new S3Client({ region })
const names = buckets(stage())
const left: string[] = []
const directory = await client.send(new ListDirectoryBucketsCommand({}))
if (directory.Buckets?.some(({ Name }) => Name === names.log)) left.push(names.log)
try {
	await client.send(new HeadBucketCommand({ Bucket: names.checkpoints }))
	left.push(names.checkpoints)
} catch (error) {
	if ((error as { $metadata?: { httpStatusCode?: number } }).$metadata?.httpStatusCode !== 404) throw error
}
if (left.length > 0) throw new Error(`still deployed: ${left.join(", ")}`)
console.log(`stage ${stage()}: nothing left`)
