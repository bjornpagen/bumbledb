/**
 * Bounded, interruption-safe repository filesystem work. Generation writes
 * every artifact through a temporary sibling + fsync + atomic rename, and the
 * manifest is always written LAST — a crash mid-generation leaves either the
 * old recorded chain or the new one, never a half-listed plan: plan/snapshot
 * files are inert until the manifest names them, and recorded files are never
 * rewritten (append-only history; only the derived index/contract files are
 * regenerated). Reads are size-bounded before any byte is loaded. All I/O is
 * inside Effects; single-file commits are uninterruptible-bounded with
 * temporary-file cleanup on every failure path.
 */
import { open, mkdir, readFile, rename, rm, stat } from "node:fs/promises"
import * as path from "node:path"
import { Effect } from "effect"
import type { LogError } from "#errors.ts"
import { repository } from "#migrations/fail.ts"

function ioDetail(cause: unknown): string {
	if (typeof cause === "object" && cause !== null && "code" in cause && typeof cause.code === "string") {
		return cause.code
	}
	return "io failure"
}

function ioError(operation: string, filePath: string, cause: unknown): LogError {
	return repository(operation, filePath, ioDetail(cause))
}

function isMissing(cause: unknown): boolean {
	return typeof cause === "object" && cause !== null && "code" in cause && cause.code === "ENOENT"
}

/**
 * Read one UTF-8 file with a byte cap checked BEFORE loading. Absent files
 * are `null`, never an invented empty artifact.
 */
export function readBounded(
	operation: string,
	filePath: string,
	maxBytes: number
): Effect.Effect<string | null, LogError> {
	return Effect.tryPromise({
		try: async () => {
			let size: number
			try {
				size = (await stat(filePath)).size
			} catch (cause) {
				if (isMissing(cause)) {
					return null
				}
				throw cause
			}
			if (size > maxBytes) {
				throw { code: `file exceeds the ${maxBytes}-byte bound` }
			}
			try {
				return await readFile(filePath, "utf8")
			} catch (cause) {
				if (isMissing(cause)) {
					return null
				}
				throw cause
			}
		},
		catch: (cause) => ioError(operation, filePath, cause)
	})
}

/** Create the directory (and parents). Idempotent. */
export function ensureDirectory(operation: string, directory: string): Effect.Effect<void, LogError> {
	return Effect.tryPromise({
		try: async () => {
			await mkdir(directory, { recursive: true })
		},
		catch: (cause) => ioError(operation, directory, cause)
	})
}

async function commitFile(filePath: string, text: string): Promise<void> {
	const temporary = `${filePath}.tmp-${process.pid}`
	try {
		const handle = await open(temporary, "w")
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
	// Make the rename durable at the directory level.
	try {
		const dir = await open(path.dirname(filePath), "r")
		try {
			await dir.sync()
		} finally {
			await dir.close()
		}
	} catch {
		// Directory fsync is best-effort on platforms that refuse it; the
		// rename itself was atomic and the file contents are synced.
	}
}

/**
 * Write one file atomically (temp + fsync + rename). The write is a bounded
 * single-file commit: it is uninterruptible so an interrupt can never leave
 * the temporary in place silently, and every failure removes it.
 */
export function writeAtomic(operation: string, filePath: string, text: string): Effect.Effect<void, LogError> {
	return Effect.tryPromise({
		try: () => commitFile(filePath, text),
		catch: (cause) => ioError(operation, filePath, cause)
	}).pipe(Effect.uninterruptible)
}

/**
 * Remove one file (idempotent). Used ONLY for unrecorded interrupted-
 * generation leftovers; recorded history files are never removed.
 */
export function removeFile(operation: string, filePath: string): Effect.Effect<void, LogError> {
	return Effect.tryPromise({
		try: async () => {
			await rm(filePath, { force: true })
		},
		catch: (cause) => ioError(operation, filePath, cause)
	})
}

