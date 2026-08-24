/**
 * `s3Store`: the five verbs over one S3-compatible target. The official
 * `@aws-sdk/client-s3` client signs and talks; this module maps HTTP
 * outcomes onto the sums. One storage class for every key. The vendor
 * ETag header is the opaque token, carried verbatim. R2 rides region
 * `auto` and a required endpoint.
 */

import { DeleteObjectCommand, GetObjectCommand, PutObjectCommand, S3Client } from "@aws-sdk/client-s3"
import * as errors from "@superbuilders/errors"
import { wrapStore } from "#errors.ts"
import type { StoreKey } from "#keys.ts"
import type { Etag, ObjectStore } from "#store.ts"

interface StaticKeys {
	readonly accessKeyId: string
	readonly secretAccessKey: string
	readonly sessionToken?: string
}

type S3Credentials = StaticKeys | (() => StaticKeys | Promise<StaticKeys>)

interface S3Config {
	readonly endpoint?: string
	readonly region: string
	readonly bucket: string
	readonly credentials: S3Credentials
	readonly prefix?: string
}

function asEtag(raw: string): Etag {
	return raw as Etag
}

function normalizePrefix(raw: string | undefined): string {
	if (raw === undefined || raw.length === 0) {
		return ""
	}
	const trimmed = raw.replace(/^\/+|\/+$/g, "")
	if (trimmed.length === 0 || trimmed.split("/").some((seg) => seg.length === 0)) {
		throw errors.new(`store prefix is not a slash path: ${raw}`)
	}
	return trimmed
}

function joinPrefix(prefix: string, key: string): string {
	return prefix.length === 0 ? key : `${prefix}/${key}`
}

async function resolveKeys(credentials: S3Credentials): Promise<StaticKeys> {
	if (typeof credentials === "function") {
		return await credentials()
	}
	return credentials
}

function httpStatus(error: unknown): number | undefined {
	if (error === null || typeof error !== "object" || !("$metadata" in error)) {
		return undefined
	}
	const metadata = error.$metadata
	if (metadata === null || typeof metadata !== "object" || !("httpStatusCode" in metadata)) {
		return undefined
	}
	const status = metadata.httpStatusCode
	return typeof status === "number" ? status : undefined
}

function asError(error: unknown): Error {
	if (error instanceof Error) {
		return error
	}
	return errors.new(String(error))
}

function etagOf(raw: string | undefined, op: string, key: StoreKey): Etag {
	if (raw === undefined || raw.length === 0) {
		throw wrapStore(errors.new("vendor omitted ETag"), `${op} ${key}`)
	}
	return asEtag(raw)
}

function s3Store(config: S3Config): ObjectStore {
	if (config.region === "auto" && config.endpoint === undefined) {
		throw errors.new("region auto needs an endpoint")
	}
	const prefix = normalizePrefix(config.prefix)
	const client = new S3Client({
		region: config.region,
		credentials: async function credentials() {
			const keys = await resolveKeys(config.credentials)
			return {
				accessKeyId: keys.accessKeyId,
				secretAccessKey: keys.secretAccessKey,
				...(keys.sessionToken === undefined ? {} : { sessionToken: keys.sessionToken })
			}
		},
		requestChecksumCalculation: "WHEN_REQUIRED",
		responseChecksumValidation: "WHEN_REQUIRED",
		...(config.endpoint === undefined ? {} : { endpoint: config.endpoint, forcePathStyle: true })
	})

	function objectKey(key: StoreKey): string {
		return joinPrefix(prefix, key)
	}

	return {
		async get(key) {
			const ran = await errors.try(
				client.send(
					new GetObjectCommand({
						Bucket: config.bucket,
						Key: objectKey(key)
					})
				)
			)
			if (ran.error) {
				if (httpStatus(ran.error) === 404) {
					return null
				}
				throw wrapStore(asError(ran.error), `get ${key}`)
			}
			if (ran.data.Body === undefined) {
				throw wrapStore(errors.new("vendor omitted body"), `get ${key}`)
			}
			return {
				bytes: await ran.data.Body.transformToByteArray(),
				etag: etagOf(ran.data.ETag, "get", key)
			}
		},

		async getIfChanged(key, etag) {
			const ran = await errors.try(
				client.send(
					new GetObjectCommand({
						Bucket: config.bucket,
						Key: objectKey(key),
						IfNoneMatch: etag
					})
				)
			)
			if (ran.error) {
				if (httpStatus(ran.error) === 304) {
					return { tag: "unchanged" }
				}
				throw wrapStore(asError(ran.error), `getIfChanged ${key}`)
			}
			if (ran.data.Body === undefined) {
				throw wrapStore(errors.new("vendor omitted body"), `getIfChanged ${key}`)
			}
			return {
				tag: "changed",
				fetched: {
					bytes: await ran.data.Body.transformToByteArray(),
					etag: etagOf(ran.data.ETag, "getIfChanged", key)
				}
			}
		},

		async putCreate(key, bytes) {
			const ran = await errors.try(
				client.send(
					new PutObjectCommand({
						Bucket: config.bucket,
						Key: objectKey(key),
						Body: bytes,
						IfNoneMatch: "*"
					})
				)
			)
			if (ran.error) {
				if (httpStatus(ran.error) === 412) {
					return { tag: "exists" }
				}
				throw wrapStore(asError(ran.error), `putCreate ${key}`)
			}
			return { tag: "created", etag: etagOf(ran.data.ETag, "putCreate", key) }
		},

		async putSwap(key, bytes, etag) {
			const ran = await errors.try(
				client.send(
					new PutObjectCommand({
						Bucket: config.bucket,
						Key: objectKey(key),
						Body: bytes,
						IfMatch: etag
					})
				)
			)
			if (ran.error) {
				const status = httpStatus(ran.error)
				if (status === 412 || status === 404) {
					return { tag: "moved" }
				}
				throw wrapStore(asError(ran.error), `putSwap ${key}`)
			}
			return { tag: "swapped", etag: etagOf(ran.data.ETag, "putSwap", key) }
		},

		async delete(key) {
			const ran = await errors.try(
				client.send(
					new DeleteObjectCommand({
						Bucket: config.bucket,
						Key: objectKey(key)
					})
				)
			)
			if (ran.error) {
				if (httpStatus(ran.error) === 404) {
					return
				}
				throw wrapStore(asError(ran.error), `delete ${key}`)
			}
		}
	}
}

export type { S3Config, S3Credentials, StaticKeys }
export { joinPrefix, s3Store }
