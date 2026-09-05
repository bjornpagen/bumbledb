/**
 * Bounded, interruption-safe repository filesystem work. Generation holds
 * the native kernel lock (see `#migrations/lock.ts`) before any read or
 * cleanup. Reads use one open file descriptor, bounded chunks, fatal UTF-8
 * and an aggregate byte cap — never stat then whole-file read. Immutable
 * plan/snapshot artifacts are uniquely owned (O_EXCL / no-clobber); matching
 * existing content is verified, never overwritten. The sole authoritative
 * manifest is replaced last by a uniquely named temp + fsync + rename +
 * directory sync. Derived index/contract files are repairable the same way.
 *
 * Every promise I/O is tracked. Cancellation joins pending work before the
 * caller releases the repository lock. No PID liveness, stale-file unlink,
 * or lock-token guessing lives here.
 */
import { randomUUID } from "node:crypto"
import { open } from "node:fs/promises"
import type { FileHandle } from "node:fs/promises"
import { mkdir, rename, rm } from "node:fs/promises"
import * as path from "node:path"
import { Effect } from "effect"
import type { LogError } from "#errors.ts"
import { repository } from "#migrations/fail.ts"

const CHUNK = 64 * 1024
const decoder = new TextDecoder("utf-8", { fatal: true })

const pending = new Set<Promise<unknown>>()

function track<A>(work: Promise<A>): Promise<A> {
	pending.add(work)
	return work.finally(() => {
		pending.delete(work)
	})
}

function ioDetail(cause: unknown): string {
	if (typeof cause === "object" && cause !== null && "code" in cause && typeof cause.code === "string") {
		return cause.code
	}
	if (cause instanceof TypeError) {
		return "invalid UTF-8"
	}
	return "io failure"
}

function ioError(operation: string, filePath: string, cause: unknown): LogError {
	return repository(operation, filePath, ioDetail(cause))
}

function isMissing(cause: unknown): boolean {
	return typeof cause === "object" && cause !== null && "code" in cause && cause.code === "ENOENT"
}

function isExists(cause: unknown): boolean {
	return typeof cause === "object" && cause !== null && "code" in cause && cause.code === "EEXIST"
}

/**
 * Join every in-flight filesystem promise. Uninterruptible: the repository
 * lock must outlive canceled I/O.
 */
export const joinPendingIo: Effect.Effect<void> = Effect.tryPromise({
	try: () => track(Promise.allSettled([...pending])).then(() => undefined),
	catch: () => undefined
}).pipe(Effect.asVoid, Effect.uninterruptible)

function tryIo<A>(operation: string, filePath: string, work: () => Promise<A>): Effect.Effect<A, LogError> {
	return Effect.tryPromise({
		try: () => track(work()),
		catch: (cause) => ioError(operation, filePath, cause)
	})
}

async function openExisting(filePath: string): Promise<FileHandle | null> {
	try {
		return await open(filePath, "r")
	} catch (cause) {
		if (isMissing(cause)) {
			return null
		}
		throw cause
	}
}

function decodeBytes(operation: string, filePath: string, pieces: readonly Buffer[]): Effect.Effect<string, LogError> {
	return Effect.try({
		try: () => decoder.decode(pieces.length === 0 ? new Uint8Array() : Buffer.concat(pieces)),
		catch: (cause) => ioError(operation, filePath, cause)
	})
}

/**
 * Read one UTF-8 file through a single descriptor in bounded chunks.
 * Invalid UTF-8 is fatal. A growing file stops at the aggregate cap.
 * Absent files are `null`, never an invented empty artifact. Yields between
 * chunks so concurrent growth is observable.
 */
export function readBounded(
	operation: string,
	filePath: string,
	maxBytes: number
): Effect.Effect<string | null, LogError> {
	return Effect.gen(function* () {
		const opened = yield* tryIo(operation, filePath, () => openExisting(filePath))
		if (opened === null) {
			return null
		}
		return yield* Effect.acquireUseRelease(
			Effect.succeed(opened),
			(handle) =>
				Effect.gen(function* () {
					const pieces: Buffer[] = []
					let total = 0
					const buffer = Buffer.alloc(CHUNK)
					for (;;) {
						const bytesRead = yield* tryIo(operation, filePath, async () => {
							const result = await handle.read(buffer, 0, buffer.length, null)
							return result.bytesRead
						})
						if (bytesRead === 0) {
							break
						}
						total += bytesRead
						if (total > maxBytes) {
							return yield* Effect.fail(repository(operation, filePath, `file exceeds the ${maxBytes}-byte bound`))
						}
						pieces.push(Buffer.from(buffer.subarray(0, bytesRead)))
						yield* Effect.yieldNow
					}
					return yield* decodeBytes(operation, filePath, pieces)
				}),
			(handle) =>
				tryIo(operation, filePath, () => handle.close()).pipe(
					Effect.catch(() => Effect.void),
					Effect.asVoid
				)
		)
	})
}

/** Create the directory (and parents). Idempotent. */
export function ensureDirectory(operation: string, directory: string): Effect.Effect<void, LogError> {
	return tryIo(operation, directory, async () => {
		await mkdir(directory, { recursive: true })
	})
}

async function syncDirectory(directory: string): Promise<void> {
	const dir = await open(directory, "r")
	try {
		await dir.sync()
	} finally {
		await dir.close()
	}
}

async function writeExclusive(filePath: string, text: string): Promise<"created" | "identical"> {
	try {
		const handle = await open(filePath, "wx")
		try {
			await handle.writeFile(text, "utf8")
			await handle.sync()
		} finally {
			await handle.close()
		}
		await syncDirectory(path.dirname(filePath))
		return "created"
	} catch (cause) {
		if (!isExists(cause)) {
			throw cause
		}
		const existingHandle = await openExisting(filePath)
		if (existingHandle === null) {
			throw { code: "immutable artifact vanished during identical-content check" }
		}
		try {
			const pieces: Buffer[] = []
			let total = 0
			const cap = Math.max(Buffer.byteLength(text, "utf8"), 1)
			const buffer = Buffer.alloc(CHUNK)
			for (;;) {
				const { bytesRead } = await existingHandle.read(buffer, 0, buffer.length, null)
				if (bytesRead === 0) {
					break
				}
				total += bytesRead
				if (total > cap) {
					throw { code: "immutable artifact exists with different content" }
				}
				pieces.push(Buffer.from(buffer.subarray(0, bytesRead)))
			}
			const existing = decoder.decode(pieces.length === 0 ? new Uint8Array() : Buffer.concat(pieces))
			if (existing === text) {
				return "identical"
			}
		} finally {
			await existingHandle.close()
		}
		throw { code: "immutable artifact exists with different content" }
	}
}

/**
 * Publish one uniquely owned immutable artifact. No-clobber: an existing
 * file is accepted only when its bytes are identical. Uninterruptible.
 */
export function writeImmutable(operation: string, filePath: string, text: string): Effect.Effect<void, LogError> {
	return tryIo(operation, filePath, () => writeExclusive(filePath, text).then(() => undefined)).pipe(Effect.uninterruptible)
}

async function replaceFile(filePath: string, text: string): Promise<void> {
	const temporary = `${filePath}.tmp-${randomUUID()}`
	try {
		const handle = await open(temporary, "wx")
		try {
			await handle.writeFile(text, "utf8")
			await handle.sync()
		} finally {
			await handle.close()
		}
		await rename(temporary, filePath)
	} catch (cause) {
		await rm(temporary, { force: true }).catch(() => undefined)
		throw cause
	}
	await syncDirectory(path.dirname(filePath))
}

/**
 * Atomically durably replace the sole authoritative manifest. Last commit
 * of a generation. Uninterruptible.
 */
export function writeManifest(operation: string, filePath: string, text: string): Effect.Effect<void, LogError> {
	return tryIo(operation, filePath, () => replaceFile(filePath, text)).pipe(Effect.uninterruptible)
}

/**
 * Repair a derived index/contract file idempotently (temp + fsync + rename).
 * Uninterruptible.
 */
export function writeDerived(operation: string, filePath: string, text: string): Effect.Effect<void, LogError> {
	return tryIo(operation, filePath, () => replaceFile(filePath, text)).pipe(Effect.uninterruptible)
}

/**
 * Remove one file (idempotent). Used ONLY for unrecorded interrupted-
 * generation leftovers; recorded history files are never removed.
 */
export function removeFile(operation: string, filePath: string): Effect.Effect<void, LogError> {
	return tryIo(operation, filePath, async () => {
		await rm(filePath, { force: true })
	})
}
