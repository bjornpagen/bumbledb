/**
 * `s3Store`: the five verbs over one S3-compatible target. `aws4fetch`
 * signs SigV4 exactly as it ships — no forked signer, no option
 * surface beyond endpoint, region, bucket, credentials, and key
 * prefix. One storage class for every key. The vendor ETag header
 * is the opaque token, carried verbatim.
 */

import * as errors from "@superbuilders/errors"
import { AwsClient } from "aws4fetch"
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

function encodeKey(key: string): string {
	return key.split("/").map(encodeURIComponent).join("/")
}

function objectUrl(config: { endpoint?: string; region: string; bucket: string }, key: string): string {
	const encoded = encodeKey(key)
	if (config.endpoint !== undefined) {
		return `${config.endpoint.replace(/\/+$/, "")}/${encodeURIComponent(config.bucket)}/${encoded}`
	}
	return `https://${config.bucket}.s3.${config.region}.amazonaws.com/${encoded}`
}

async function resolveKeys(credentials: S3Credentials): Promise<StaticKeys> {
	if (typeof credentials === "function") {
		return await credentials()
	}
	return credentials
}

function clientOf(region: string, keys: StaticKeys): AwsClient {
	return new AwsClient({
		accessKeyId: keys.accessKeyId,
		secretAccessKey: keys.secretAccessKey,
		service: "s3",
		region,
		...(keys.sessionToken === undefined ? {} : { sessionToken: keys.sessionToken })
	})
}

function headerEtag(headers: Headers, op: string, key: StoreKey): Etag {
	const raw = headers.get("etag")
	if (raw === null || raw.length === 0) {
		throw wrapStore(errors.new("vendor omitted ETag"), `${op} ${key}`)
	}
	return asEtag(raw)
}

function s3Store(config: S3Config): ObjectStore {
	if (config.region === "auto" && config.endpoint === undefined) {
		throw errors.new("region auto needs an endpoint")
	}
	const prefix = normalizePrefix(config.prefix)
	const staticClient = typeof config.credentials === "function" ? null : clientOf(config.region, config.credentials)

	async function client(): Promise<AwsClient> {
		if (staticClient !== null) {
			return staticClient
		}
		return clientOf(config.region, await resolveKeys(config.credentials))
	}

	function urlOf(key: StoreKey): string {
		return objectUrl(
			{
				region: config.region,
				bucket: config.bucket,
				...(config.endpoint === undefined ? {} : { endpoint: config.endpoint })
			},
			joinPrefix(prefix, key)
		)
	}

	async function signed(
		key: StoreKey,
		op: string,
		init: { method: string; headers?: Record<string, string>; body?: Uint8Array }
	): Promise<Response> {
		const signedClient = await client()
		const ran = await errors.try(signedClient.fetch(urlOf(key), init))
		if (ran.error) {
			throw wrapStore(ran.error, `${op} ${key}`)
		}
		return ran.data
	}

	return {
		async get(key) {
			const response = await signed(key, "get", { method: "GET" })
			if (response.status === 404) {
				return null
			}
			if (!response.ok) {
				throw wrapStore(errors.new(`GET ${response.status}`), `get ${key}`)
			}
			const bytes = new Uint8Array(await response.arrayBuffer())
			return { bytes, etag: headerEtag(response.headers, "get", key) }
		},

		async getIfChanged(key, etag) {
			const response = await signed(key, "getIfChanged", {
				method: "GET",
				headers: { "If-None-Match": etag }
			})
			if (response.status === 304) {
				return { tag: "unchanged" }
			}
			if (!response.ok) {
				throw wrapStore(errors.new(`GET ${response.status}`), `getIfChanged ${key}`)
			}
			const bytes = new Uint8Array(await response.arrayBuffer())
			return {
				tag: "changed",
				fetched: { bytes, etag: headerEtag(response.headers, "getIfChanged", key) }
			}
		},

		async putCreate(key, bytes) {
			const response = await signed(key, "putCreate", {
				method: "PUT",
				headers: { "If-None-Match": "*" },
				body: bytes
			})
			if (response.status === 412) {
				return { tag: "exists" }
			}
			if (!response.ok) {
				throw wrapStore(errors.new(`PUT ${response.status}`), `putCreate ${key}`)
			}
			return { tag: "created", etag: headerEtag(response.headers, "putCreate", key) }
		},

		async putSwap(key, bytes, etag) {
			const response = await signed(key, "putSwap", {
				method: "PUT",
				headers: { "If-Match": etag },
				body: bytes
			})
			if (response.status === 412) {
				return { tag: "moved" }
			}
			if (!response.ok) {
				throw wrapStore(errors.new(`PUT ${response.status}`), `putSwap ${key}`)
			}
			return { tag: "swapped", etag: headerEtag(response.headers, "putSwap", key) }
		},

		async delete(key) {
			const response = await signed(key, "delete", { method: "DELETE" })
			if (response.status === 404 || response.ok) {
				return
			}
			throw wrapStore(errors.new(`DELETE ${response.status}`), `delete ${key}`)
		}
	}
}

export type { S3Config, S3Credentials, StaticKeys }
export { joinPrefix, objectUrl, s3Store }
