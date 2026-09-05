/**
 * `s3Store`: the five verbs over one S3-compatible target. The official
 * `@aws-sdk/client-s3` client signs and talks; this module maps HTTP
 * outcomes onto the sums. One storage class for every key. The vendor
 * ETag header is the opaque token, carried verbatim. R2 rides region
 * `auto` and a required endpoint. A 409, a timeout, or any other
 * unproved conditional-write result is Ambiguous; the GET-verify law
 * resolves it. Credentials are consulted per request. Conditional
 * writes are not retried by the client: a re-sent PUT is an unproved
 * outcome. Body-stream failures wrap ErrStore.
 */
import { DeleteObjectCommand, GetObjectCommand, PutObjectCommand, S3Client } from "@aws-sdk/client-s3"
import { Result } from "effect"
import { bytesEqual } from "#bytes.ts"
import { LogInputError, wrapStore } from "#errors.ts"
import type { StoreKey } from "#keys.ts"
import { parsePrefix } from "#keys.ts"
import type { Create, Etag, Fetched, ObjectStore, Swap } from "#store.ts"

interface StaticKeys {
	readonly accessKeyId: string
	readonly secretAccessKey: string
	readonly sessionToken?: string
}
/** Static keys, or a caller-owned refresh the store invokes before each signed request. */
type S3Credentials = StaticKeys | (() => StaticKeys | Promise<StaticKeys>)
interface S3Config {
	readonly endpoint?: string
	readonly region: string
	readonly bucket: string
	readonly credentials: S3Credentials
	readonly prefix?: string
}
const READ_ATTEMPTS = 6
const READ_BASE_MS = 50
const READ_CAP_MS = 2000
function asEtag(raw: string): Etag {
	return raw as Etag
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
function etagOf(raw: string | undefined, op: string, key: StoreKey): Etag {
	if (raw === undefined || raw.length === 0) {
		throw wrapStore(new LogInputError({ message: "vendor omitted ETag" }), `${op} ${key}`)
	}
	return asEtag(raw)
}
function isTransientName(error: unknown): boolean {
	if (error === null || typeof error !== "object" || !("name" in error)) {
		return true
	}
	const name = error.name
	return (
		name === "TimeoutError" ||
		name === "RequestTimeout" ||
		name === "TimeoutErrorException" ||
		name === "NetworkingError" ||
		name === "Conflict" ||
		name === "ConflictException" ||
		name === undefined
	)
}
/** 409 Conflict is Ambiguous, never a proved Exists or Moved. */
function isUnprovedWrite(error: unknown): boolean {
	const status = httpStatus(error)
	if (status === 409) {
		return true
	}
	if (status !== undefined && status >= 500 && status < 600) {
		return true
	}
	if (status !== undefined) {
		return false
	}
	return isTransientName(error)
}
function isRetryableRead(error: unknown): boolean {
	const status = httpStatus(error)
	if (status !== undefined) {
		return status >= 500 && status < 600
	}
	return isTransientName(error)
}
async function sleep(ms: number): Promise<void> {
	await new Promise(function later(resolve) {
		setTimeout(resolve, ms)
	})
}
function jitteredMs(ceiling: number): number {
	return Math.floor(Math.random() * (ceiling + 1))
}
async function bodyBytes(body: unknown, op: string, key: StoreKey): Promise<Uint8Array> {
	const ran = await Promise.resolve(
		(async function consume() {
			if (body === null || typeof body !== "object") {
				throw new LogInputError({ message: "vendor omitted body stream" })
			}
			const stream = body as {
				transformToByteArray?: () => Promise<Uint8Array>
			}
			if (typeof stream.transformToByteArray !== "function") {
				throw new LogInputError({ message: "vendor omitted body stream" })
			}
			return await stream.transformToByteArray()
		})()
	).then(Result.succeed, (cause: unknown) => Result.fail(cause))
	if (Result.isFailure(ran)) {
		throw wrapStore(ran.failure, `${op} ${key}`)
	}
	return ran.success
}
function s3Store(config: S3Config): ObjectStore {
	if (config.region === "auto" && config.endpoint === undefined) {
		throw new LogInputError({ message: "region auto needs an endpoint" })
	}
	const prefix = parsePrefix(config.prefix ?? "")
	const client = new S3Client({
		region: config.region,
		maxAttempts: 1,
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
	type Load =
		| {
				readonly tag: "hit"
				readonly fetched: Fetched
		  }
		| {
				readonly tag: "miss"
		  }
		| {
				readonly tag: "retry"
				readonly error: unknown
		  }
		| {
				readonly tag: "fail"
				readonly error: unknown
		  }
	async function load(key: StoreKey): Promise<Load> {
		const ran = await Promise.resolve(
			client.send(
				new GetObjectCommand({
					Bucket: config.bucket,
					Key: objectKey(key)
				})
			)
		).then(Result.succeed, (cause: unknown) => Result.fail(cause))
		if (Result.isFailure(ran)) {
			if (httpStatus(ran.failure) === 404) {
				return { tag: "miss" }
			}
			const error = wrapStore(ran.failure, `get ${key}`)
			return isRetryableRead(ran.failure) ? { tag: "retry", error } : { tag: "fail", error }
		}
		if (ran.success.Body === undefined) {
			return { tag: "fail", error: wrapStore(new LogInputError({ message: "vendor omitted body" }), `get ${key}`) }
		}
		const body = await Promise.resolve(bodyBytes(ran.success.Body, "get", key)).then(Result.succeed, (cause: unknown) =>
			Result.fail(cause)
		)
		if (Result.isFailure(body)) {
			return { tag: "retry", error: body.failure }
		}
		const tag = await Promise.resolve(
			(async function etag() {
				return etagOf(ran.success.ETag, "get", key)
			})()
		).then(Result.succeed, (cause: unknown) => Result.fail(cause))
		if (Result.isFailure(tag)) {
			return { tag: "fail", error: tag.failure }
		}
		return { tag: "hit", fetched: { bytes: body.success, etag: tag.success } }
	}
	async function getOnce(key: StoreKey): Promise<Fetched | null> {
		const step = await load(key)
		if (step.tag === "hit") {
			return step.fetched
		}
		if (step.tag === "miss") {
			return null
		}
		throw step.error
	}
	async function retryGet(key: StoreKey): Promise<Fetched | null> {
		let attempt = 0
		for (;;) {
			const step = await load(key)
			if (step.tag === "hit") {
				return step.fetched
			}
			if (step.tag === "miss") {
				return null
			}
			if (step.tag === "fail") {
				throw step.error
			}
			attempt += 1
			if (attempt === READ_ATTEMPTS) {
				throw step.error
			}
			const ceiling = Math.min(READ_CAP_MS, READ_BASE_MS * 2 ** (attempt - 1))
			await sleep(jitteredMs(ceiling))
		}
	}
	async function proveCreate(key: StoreKey, attempted: Uint8Array): Promise<Create> {
		const fetched = await retryGet(key)
		if (fetched === null) {
			return { tag: "ambiguous" }
		}
		if (bytesEqual(fetched.bytes, attempted)) {
			return { tag: "created", etag: fetched.etag }
		}
		return { tag: "exists" }
	}
	async function proveSwap(key: StoreKey, attempted: Uint8Array): Promise<Swap> {
		const fetched = await retryGet(key)
		if (fetched === null) {
			return { tag: "moved" }
		}
		if (bytesEqual(fetched.bytes, attempted)) {
			return { tag: "swapped", etag: fetched.etag }
		}
		return { tag: "moved" }
	}
	return {
		async get(key) {
			return await getOnce(key)
		},
		async getIfChanged(key, etag) {
			const ran = await Promise.resolve(
				client.send(
					new GetObjectCommand({
						Bucket: config.bucket,
						Key: objectKey(key),
						IfNoneMatch: etag
					})
				)
			).then(Result.succeed, (cause: unknown) => Result.fail(cause))
			if (Result.isFailure(ran)) {
				if (httpStatus(ran.failure) === 304) {
					return { tag: "unchanged" }
				}
				throw wrapStore(ran.failure, `getIfChanged ${key}`)
			}
			if (ran.success.Body === undefined) {
				throw wrapStore(new LogInputError({ message: "vendor omitted body" }), `getIfChanged ${key}`)
			}
			return {
				tag: "changed",
				fetched: {
					bytes: await bodyBytes(ran.success.Body, "getIfChanged", key),
					etag: etagOf(ran.success.ETag, "getIfChanged", key)
				}
			}
		},
		async putCreate(key, bytes) {
			const ran = await Promise.resolve(
				client.send(
					new PutObjectCommand({
						Bucket: config.bucket,
						Key: objectKey(key),
						Body: bytes,
						IfNoneMatch: "*"
					})
				)
			).then(Result.succeed, (cause: unknown) => Result.fail(cause))
			if (Result.isFailure(ran)) {
				if (httpStatus(ran.failure) === 412) {
					return { tag: "exists" }
				}
				if (isUnprovedWrite(ran.failure)) {
					return await proveCreate(key, bytes)
				}
				throw wrapStore(ran.failure, `putCreate ${key}`)
			}
			if (ran.success.ETag === undefined || ran.success.ETag.length === 0) {
				return await proveCreate(key, bytes)
			}
			return { tag: "created", etag: asEtag(ran.success.ETag) }
		},
		async putSwap(key, bytes, etag) {
			const ran = await Promise.resolve(
				client.send(
					new PutObjectCommand({
						Bucket: config.bucket,
						Key: objectKey(key),
						Body: bytes,
						IfMatch: etag
					})
				)
			).then(Result.succeed, (cause: unknown) => Result.fail(cause))
			if (Result.isFailure(ran)) {
				const status = httpStatus(ran.failure)
				if (status === 412 || status === 404) {
					return { tag: "moved" }
				}
				if (isUnprovedWrite(ran.failure)) {
					return await proveSwap(key, bytes)
				}
				throw wrapStore(ran.failure, `putSwap ${key}`)
			}
			if (ran.success.ETag === undefined || ran.success.ETag.length === 0) {
				return await proveSwap(key, bytes)
			}
			return { tag: "swapped", etag: asEtag(ran.success.ETag) }
		},
		async delete(key) {
			const ran = await Promise.resolve(
				client.send(
					new DeleteObjectCommand({
						Bucket: config.bucket,
						Key: objectKey(key)
					})
				)
			).then(Result.succeed, (cause: unknown) => Result.fail(cause))
			if (Result.isFailure(ran)) {
				if (httpStatus(ran.failure) === 404) {
					return
				}
				throw wrapStore(ran.failure, `delete ${key}`)
			}
		}
	}
}

export type { S3Config, S3Credentials }
export { joinPrefix, s3Store }
