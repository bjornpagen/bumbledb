import { createReadStream, createWriteStream } from "node:fs"
import * as fs from "node:fs/promises"
import type { Readable } from "node:stream"
import { pipeline } from "node:stream/promises"
import type { S3Client } from "@aws-sdk/client-s3"
import { Effect } from "effect"
import { DbError } from "../errors.ts"
import type { Body, Bucket, Created, Deleted, Fetched, Listed, Millis, ObjectStore, Reply, Target } from "./io.ts"

type Sdk = typeof import("@aws-sdk/client-s3")

let sdk: Promise<Sdk> | undefined

interface S3StoreOptions {
	/** The application's own client: it owns credentials, region, endpoint and SDK retries. */
	readonly client: S3Client
	/** The commit log bucket, ideally an S3 Express directory bucket (`name--azid--x-s3`). */
	readonly log: { readonly bucket: string }
	/** The checkpoint bucket: an S3 Standard bucket, whose LIST is lexicographic. */
	readonly checkpoints: { readonly bucket: string }
	/** Prepended to every key; empty, or ending in `/`. */
	readonly prefix: string
}

interface Answer<O> {
	readonly output: O
	readonly date: Millis | null
}

interface Refusal {
	readonly status: number | undefined
	readonly date: Millis | null
}

function dateOf(headers: unknown): Millis | null {
	if (typeof headers !== "object" || headers === null || !("date" in headers) || typeof headers.date !== "string") {
		return null
	}
	const millis = Date.parse(headers.date)
	return Number.isNaN(millis) ? null : BigInt(millis)
}

function refusalOf(cause: unknown): Refusal {
	if (typeof cause !== "object" || cause === null) return { status: undefined, date: null }
	const metadata = "$metadata" in cause ? (cause.$metadata as { httpStatusCode?: number }) : undefined
	const response = "$response" in cause ? (cause.$response as { headers?: unknown } | undefined) : undefined
	return { status: metadata?.httpStatusCode, date: dateOf(response?.headers) }
}

/**
 * Sends one command and reads the response's `Date` header, which the SDK does not surface on
 * outputs. The middleware sits innermost in the deserialize step, so it sees every raw response.
 */
async function send<O>(
	client: S3Client,
	command: { middlewareStack: { add: (...args: never[]) => void } },
	signal: AbortSignal
): Promise<Answer<O>> {
	let date: Millis | null = null
	const capture =
		(next: (args: unknown) => Promise<{ response: unknown }>) =>
		async (args: unknown): Promise<{ response: unknown }> => {
			const result = await next(args)
			const response = result.response as { headers?: unknown } | undefined
			date = dateOf(response?.headers) ?? date
			return result
		}
	;(command.middlewareStack.add as (middleware: unknown, options: object) => void)(capture, {
		step: "deserialize",
		priority: "low",
		name: "bumbledbResponseDate"
	})
	const output = (await client.send(command as never, { abortSignal: signal })) as O
	return { output, date }
}

function millisOf(value: Date | undefined): Millis {
	return BigInt(value?.getTime() ?? 0)
}

/** A store over the application's AWS SDK v3 client. Every key lives under `prefix`. */
function make(options: S3StoreOptions): ObjectStore {
	const { client, prefix } = options
	const bucketName = (bucket: Bucket) => (bucket === "Log" ? options.log.bucket : options.checkpoints.bucket)
	const load = () => {
		sdk ??= import("@aws-sdk/client-s3")
		return sdk
	}

	function attempt<R>(
		operation: string,
		run: (sdk: Sdk, signal: AbortSignal) => Promise<Reply<R>>,
		refused: (refusal: Refusal) => Reply<R> | undefined
	): Effect.Effect<Reply<R>, DbError> {
		return Effect.tryPromise({
			try: async (signal) => {
				const module = await load()
				try {
					return await run(module, signal)
				} catch (cause) {
					const answer = refused(refusalOf(cause))
					if (answer !== undefined) return answer
					throw cause
				}
			},
			catch: (cause) =>
				new DbError({
					operation,
					reason: { _tag: "Io", kind: refusalOf(cause).status?.toString() ?? "Transport" }
				})
		})
	}

	return {
		get: (bucket: Bucket, key: string, target: Target) =>
			attempt<Fetched>(
				"S3Store.get",
				async ({ GetObjectCommand }, signal) => {
					const answer = await send<{ Body?: unknown; LastModified?: Date }>(
						client,
						new GetObjectCommand({ Bucket: bucketName(bucket), Key: `${prefix}${key}` }),
						signal
					)
					const body = answer.output.Body as (Readable & { transformToByteArray(): Promise<Uint8Array> }) | undefined
					const lastModified = millisOf(answer.output.LastModified)
					if (body === undefined) throw new Error("GetObject returned no body")
					if (target._tag === "Memory") {
						return {
							date: answer.date,
							result: { _tag: "Body", bytes: await body.transformToByteArray(), lastModified }
						}
					}
					await pipeline(body, createWriteStream(target.path))
					return { date: answer.date, result: { _tag: "Saved", lastModified } }
				},
				(refusal) => (refusal.status === 404 ? { date: refusal.date, result: { _tag: "Missing" } } : undefined)
			),
		putIfAbsent: (bucket: Bucket, key: string, body: Body) =>
			attempt<Created>(
				"S3Store.putIfAbsent",
				async ({ PutObjectCommand }, signal) => {
					const payload =
						body._tag === "Bytes"
							? { Body: body.bytes, ContentLength: body.bytes.byteLength }
							: { Body: createReadStream(body.path), ContentLength: (await fs.stat(body.path)).size }
					const answer = await send(
						client,
						new PutObjectCommand({ Bucket: bucketName(bucket), Key: `${prefix}${key}`, IfNoneMatch: "*", ...payload }),
						signal
					)
					return { date: answer.date, result: { _tag: "Created" } }
				},
				(refusal) => (refusal.status === 412 ? { date: refusal.date, result: { _tag: "Occupied" } } : undefined)
			),
		list: (keyPrefix: string, startAfter: string | null, maxKeys: number) =>
			attempt<Listed>(
				"S3Store.list",
				async ({ ListObjectsV2Command }, signal) => {
					const answer = await send<{ Contents?: ReadonlyArray<{ Key?: string }> }>(
						client,
						new ListObjectsV2Command({
							Bucket: options.checkpoints.bucket,
							Prefix: `${prefix}${keyPrefix}`,
							MaxKeys: maxKeys,
							...(startAfter === null ? {} : { StartAfter: `${prefix}${startAfter}` })
						}),
						signal
					)
					const keys = (answer.output.Contents ?? []).flatMap((object) =>
						object.Key?.startsWith(prefix) === true ? [object.Key.slice(prefix.length)] : []
					)
					return { date: answer.date, result: { _tag: "Keys", keys } }
				},
				() => undefined
			),
		delete: (key: string) =>
			attempt<Deleted>(
				"S3Store.delete",
				async ({ DeleteObjectCommand }, signal) => {
					const answer = await send(
						client,
						new DeleteObjectCommand({ Bucket: options.checkpoints.bucket, Key: `${prefix}${key}` }),
						signal
					)
					return { date: answer.date, result: { _tag: "Deleted" } }
				},
				() => undefined
			)
	}
}

const S3Store = Object.freeze({ make })

export type { S3StoreOptions }
export { S3Store }
