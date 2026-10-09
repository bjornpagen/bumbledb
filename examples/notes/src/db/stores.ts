/**
 * Where each tenant's log and cache live. With `BUMBLEDB_LOG_BUCKET` set, the log is in S3 (the log
 * bucket an S3 Express directory bucket, checkpoints in `BUMBLEDB_CKPT_BUCKET`) under
 * `tenants/<tenant>/`; otherwise it is a durable directory under the app's data directory. The cache
 * is always local and disposable.
 */
import * as path from "node:path"
import { S3Client } from "@aws-sdk/client-s3"
import type { ObjectStore } from "@bjornpagen/bumbledb"
import { FsStore, S3Store } from "@bjornpagen/bumbledb"

const dataDir = () => process.env.BUMBLEDB_DATA_DIR ?? path.join(process.cwd(), ".bdb")

let client: S3Client | undefined

export function storeFor(tenant: string): ObjectStore {
	const log = process.env.BUMBLEDB_LOG_BUCKET
	if (log === undefined) return FsStore.make(path.join(dataDir(), "tenants", tenant))
	const checkpoints = process.env.BUMBLEDB_CKPT_BUCKET
	if (checkpoints === undefined) throw new Error("BUMBLEDB_CKPT_BUCKET is required with BUMBLEDB_LOG_BUCKET")
	client ??= new S3Client({})
	return S3Store.make({ client, log: { bucket: log }, checkpoints: { bucket: checkpoints }, prefix: `tenants/${tenant}/` })
}

export function cacheFor(tenant: string): { readonly directory: string } {
	return { directory: path.join(process.env.BUMBLEDB_CACHE_DIR ?? path.join(dataDir(), "cache"), tenant) }
}
