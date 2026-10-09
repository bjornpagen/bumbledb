/**
 * Attachment blobs: immutable, content-addressed objects in the app's own bucket, separate from the
 * database's buckets. The blob is uploaded before the row that references it is committed, so a
 * crash leaves an orphan upload, never a reference to missing bytes.
 */
import { createHash } from "node:crypto"
import { PutObjectCommand, S3Client } from "@aws-sdk/client-s3"
import { Effect, Schema } from "effect"

export class BlobStoreUnavailable extends Schema.TaggedError<BlobStoreUnavailable>()("BlobStoreUnavailable", {
	detail: Schema.String
}) {}

const MAX_BLOB_BYTES = 4_000_000

let client: S3Client | undefined
function s3(): S3Client {
	// Provider-chain credentials: the deployed role refreshes automatically.
	client ??= new S3Client({})
	return client
}

export interface StoredBlob {
	readonly key: string
	readonly bytes: bigint
}

/**
 * Upload one bounded immutable blob; the key is its SHA-256, so a retried
 * upload of the same bytes is a harmless overwrite of identical content.
 */
export const putBlob = Effect.fn("blob.putBlob")(function* (tenantId: string, body: Uint8Array) {
	if (body.byteLength === 0 || body.byteLength > MAX_BLOB_BYTES) {
		return yield* new BlobStoreUnavailable({ detail: `blob size ${body.byteLength} outside (0, ${MAX_BLOB_BYTES}]` })
	}
	const bucket = process.env.APP_BLOB_BUCKET
	const prefix = process.env.APP_BLOB_PREFIX ?? "blobs"
	if (bucket === undefined) {
		return yield* new BlobStoreUnavailable({ detail: "APP_BLOB_BUCKET is not configured" })
	}
	const digest = createHash("sha256").update(body).digest("hex")
	const key = `${prefix}/${tenantId}/${digest}`
	yield* Effect.callback<void, BlobStoreUnavailable>((resume, signal) => {
		s3()
			.send(new PutObjectCommand({ Bucket: bucket, Key: key, Body: body }), { abortSignal: signal })
			.then(() => resume(Effect.void))
			.catch((cause: unknown) => resume(Effect.fail(new BlobStoreUnavailable({ detail: String(cause) }))))
	})
	return { key, bytes: BigInt(body.byteLength) } satisfies StoredBlob
})
