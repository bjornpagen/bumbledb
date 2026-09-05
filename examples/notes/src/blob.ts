/**
 * Blob-first attachment storage (OPS-003): the immutable blob goes to the
 * app's OWN S3 bucket under a content-addressed key FIRST; the database
 * fact referencing it commits second. A crash between the two leaves an
 * orphan upload (cheap, sweepable by app policy), never a durable
 * reference to missing bytes. This bucket/prefix is application data,
 * explicitly separate from the bumbledb log bucket and from website
 * asset/ISR buckets — the database SDK has no JS S3 client; this one is
 * ordinary app code using the deployed role's provider-chain credentials
 * (refreshed by the SDK, never static keys).
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
