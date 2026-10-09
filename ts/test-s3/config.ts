/**
 * The real S3 endpoint `pnpm test:s3` runs against. Configuration comes from BUMBLEDB_S3_TARGET
 * (`aws` or `seaweedfs`), BUMBLEDB_S3_REGION, BUMBLEDB_S3_LOG_BUCKET, BUMBLEDB_S3_CKPT_BUCKET,
 * BUMBLEDB_S3_PREFIX (ending in `/`) and, for `seaweedfs`, BUMBLEDB_S3_ENDPOINT; credentials come
 * from the standard AWS environment. A missing variable fails the run.
 */
import { randomUUID } from "node:crypto"
import { S3Client } from "@aws-sdk/client-s3"
import type { ObjectStore } from "../src/database/io.ts"
import { S3Store } from "../src/database/s3.ts"

function required(name: string): string {
	const value = process.env[name]
	if (value === undefined || value === "") throw new Error(`${name} is required`)
	return value
}

export const target = required("BUMBLEDB_S3_TARGET")
if (target !== "aws" && target !== "seaweedfs") throw new Error("BUMBLEDB_S3_TARGET must be aws or seaweedfs")
const prefix = required("BUMBLEDB_S3_PREFIX")
if (!prefix.endsWith("/")) throw new Error("BUMBLEDB_S3_PREFIX must end in /")
const endpoint = target === "seaweedfs" ? required("BUMBLEDB_S3_ENDPOINT") : undefined
const log = { bucket: required("BUMBLEDB_S3_LOG_BUCKET") }
const checkpoints = { bucket: required("BUMBLEDB_S3_CKPT_BUCKET") }

const client = new S3Client({
	region: required("BUMBLEDB_S3_REGION"),
	...(endpoint === undefined ? {} : { endpoint, forcePathStyle: true })
})

/** A store whose keys live under a fresh directory of the run's prefix. */
export function freshStore(name: string): ObjectStore {
	return S3Store.make({ client, log, checkpoints, prefix: `${prefix}${name}-${randomUUID()}/` })
}
