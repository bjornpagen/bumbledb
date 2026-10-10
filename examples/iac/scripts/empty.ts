/** Deletes every object in this stage's log bucket; Cloud Control refuses to delete a non-empty directory bucket. */
import { DeleteObjectsCommand, ListObjectsV2Command, NoSuchBucket, S3Client } from "@aws-sdk/client-s3"
import { buckets, region, stage } from "../buckets.ts"

const client = new S3Client({ region })
const bucket = buckets(stage()).log
try {
	for (;;) {
		const page = await client.send(new ListObjectsV2Command({ Bucket: bucket }))
		const keys = (page.Contents ?? []).map(({ Key }) => ({ Key }))
		if (keys.length === 0) break
		const result = await client.send(new DeleteObjectsCommand({ Bucket: bucket, Delete: { Objects: keys } }))
		if (result.Errors?.length) throw new Error(`could not delete ${result.Errors.length} objects from ${bucket}`)
	}
} catch (error) {
	if (!(error instanceof NoSuchBucket)) throw error
}
