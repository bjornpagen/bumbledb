/**
 * The repo-local migration repository: layout, bounded reads and structural
 * integrity. Recorded artifacts are immutable — generation only appends a new
 * plan/snapshot and rewrites the manifest/index/contract; an edited recorded
 * file, an unrecorded stray plan, a missing snapshot or a reordered manifest
 * is DRIFT and refuses before anything else happens. Digest-level tampering
 * is judged natively (`migration::manifest::{verify_manifest, bind_plans}`)
 * from the exact file texts read here; TypeScript never recomputes a hash.
 */
import { readdir } from "node:fs/promises"
import * as path from "node:path"
import { Effect } from "effect"
import type { LogError } from "#errors.ts"
import { decodeManifestData } from "#migrations/decode.ts"
import { drift, repository as repositoryError } from "#migrations/fail.ts"
import { readBounded } from "#migrations/fsops.ts"
import type { MigrationManifest, MigrationRepository } from "#migrations/types.ts"

const MAX_MANIFEST_BYTES = 8 * 1024 * 1024
const MAX_PLAN_BYTES = 64 * 1024 * 1024
const MAX_SNAPSHOT_BYTES = 64 * 1024 * 1024
const MAX_DIRECTORY_ENTRIES = 20000

export function manifestPath(directory: string): string {
	return path.join(directory, "manifest.json")
}

export function planPath(directory: string, id: string): string {
	return path.join(directory, `${id}.plan.json`)
}

export function snapshotPath(directory: string, sequence: number): string {
	return path.join(directory, "meta", `${sequence.toString(10).padStart(4, "0")}.schema.json`)
}

export function indexPath(directory: string): string {
	return path.join(directory, "index.ts")
}

export function contractPath(repo: MigrationRepository): string {
	return repo.contract ?? path.join(repo.directory, "runtime-contract.json")
}

/** The full human file id: `NNNN-<label>`. */
export function planId(sequence: number, label: string): string {
	return `${sequence.toString(10).padStart(4, "0")}-${label}`
}

export interface RepositoryState {
	/** `manifest.json` text, or null for a repo without any recorded plan. */
	readonly manifestText: string | null
	readonly manifest: MigrationManifest | null
	/** Recorded plan file texts, in manifest order. */
	readonly planTexts: readonly string[]
	/** Recorded snapshot file texts, in manifest order (targets). */
	readonly snapshotTexts: readonly string[]
	/**
	 * Unrecorded files whose name matches the NEXT sequence — leftovers of an
	 * interrupted generation (the manifest is written last, so an unlisted
	 * next-sequence file was never recorded). Generation deterministically
	 * rewrites them; any other unlisted file is drift, not a leftover.
	 */
	readonly staleDrafts: readonly string[]
}

/** Latest recorded target snapshot text, or null at the empty base. */
export function latestSnapshot(state: RepositoryState): string | null {
	return state.snapshotTexts.length === 0 ? null : (state.snapshotTexts[state.snapshotTexts.length - 1] ?? null)
}

function listNames(operation: string, directory: string): Effect.Effect<readonly string[], LogError> {
	return Effect.tryPromise({
		try: async () => {
			let names: string[]
			try {
				names = await readdir(directory)
			} catch (cause) {
				if (typeof cause === "object" && cause !== null && "code" in cause && cause.code === "ENOENT") {
					return []
				}
				throw cause
			}
			if (names.length > MAX_DIRECTORY_ENTRIES) {
				throw { code: `directory exceeds ${MAX_DIRECTORY_ENTRIES} entries` }
			}
			return names
		},
		catch: (cause) =>
			repositoryError(
				operation,
				directory,
				typeof cause === "object" && cause !== null && "code" in cause && typeof cause.code === "string"
					? cause.code
					: "io failure"
			)
	})
}

/**
 * Read and structurally verify the recorded repository. Every recorded entry
 * must have exactly its plan file and target snapshot; strays refuse as
 * drift. Digest recomputation is the caller's native chain pass.
 */
export const readRepository = Effect.fn("bumbledb-log.migrations.readRepository")(function* (
	repo: MigrationRepository
) {
	const operation = "migrations.readRepository"
	const directory = repo.directory
	const manifestText = yield* readBounded(operation, manifestPath(directory), MAX_MANIFEST_BYTES)
	if (manifestText === null) {
		// A fresh repository: nothing recorded. A sequence-0 plan/snapshot file
		// is an interrupted first generation (manifest is written last); any
		// other stray file is drift.
		const names = yield* listNames(operation, directory)
		const staleDrafts: string[] = []
		for (const name of names) {
			if (!name.endsWith(".plan.json")) {
				continue
			}
			if (name.startsWith("0000-")) {
				staleDrafts.push(name)
				continue
			}
			return yield* Effect.fail(
				drift(operation, `plan file ${name} exists but manifest.json does not — a recorded chain cannot be partial`)
			)
		}
		const metaNames = yield* listNames(operation, path.join(directory, "meta"))
		for (const name of metaNames) {
			if (!name.endsWith(".schema.json")) {
				continue
			}
			if (name === "0000.schema.json") {
				staleDrafts.push(`meta/${name}`)
				continue
			}
			return yield* Effect.fail(
				drift(operation, `snapshot file meta/${name} exists but manifest.json does not — a recorded chain cannot be partial`)
			)
		}
		const state: RepositoryState = { manifestText: null, manifest: null, planTexts: [], snapshotTexts: [], staleDrafts }
		return state
	}
	let tree: unknown
	try {
		tree = JSON.parse(manifestText)
	} catch {
		return yield* Effect.fail(repositoryError(operation, manifestPath(directory), "manifest is not JSON"))
	}
	const decoded = decodeManifestData(tree)
	if (!decoded.ok) {
		return yield* Effect.fail(repositoryError(operation, manifestPath(directory), decoded.detail))
	}
	const manifest: MigrationManifest = decoded.value
	// Structural chain checks (native re-verifies with digests).
	const ids = new Set<string>()
	for (const [index, entry] of manifest.entries.entries()) {
		if (entry.sequence !== index.toString(10)) {
			return yield* Effect.fail(
				drift(operation, `manifest entry ${entry.id} has sequence ${entry.sequence}, expected ${index}`)
			)
		}
		if (ids.has(entry.id)) {
			return yield* Effect.fail(drift(operation, `manifest reuses the label ${entry.id}`))
		}
		ids.add(entry.id)
	}
	// No stray or missing files. A next-sequence leftover of an interrupted
	// generation is tolerated as a rewritable draft; anything else is drift.
	const nextPrefix = `${manifest.entries.length.toString(10).padStart(4, "0")}-`
	const staleDrafts: string[] = []
	const names = yield* listNames(operation, directory)
	for (const name of names) {
		if (!name.endsWith(".plan.json") || ids.has(name.slice(0, -".plan.json".length))) {
			continue
		}
		if (name.startsWith(nextPrefix)) {
			staleDrafts.push(name)
			continue
		}
		return yield* Effect.fail(drift(operation, `plan file ${name} is not recorded in the manifest`))
	}
	const metaNames = yield* listNames(operation, path.join(directory, "meta"))
	for (const name of metaNames) {
		if (!name.endsWith(".schema.json")) {
			continue
		}
		const stem = name.slice(0, -".schema.json".length)
		const ordinal = Number.parseInt(stem, 10)
		if (Number.isSafeInteger(ordinal) && ordinal >= 0 && ordinal < manifest.entries.length) {
			continue
		}
		if (ordinal === manifest.entries.length) {
			staleDrafts.push(`meta/${name}`)
			continue
		}
		return yield* Effect.fail(drift(operation, `snapshot file meta/${name} is not recorded in the manifest`))
	}
	const planTexts: string[] = []
	const snapshotTexts: string[] = []
	for (const [index, entry] of manifest.entries.entries()) {
		const plan = yield* readBounded(operation, planPath(directory, entry.id), MAX_PLAN_BYTES)
		if (plan === null) {
			return yield* Effect.fail(drift(operation, `recorded plan ${entry.id} has no ${entry.id}.plan.json file`))
		}
		planTexts.push(plan)
		const snapshot = yield* readBounded(operation, snapshotPath(directory, index), MAX_SNAPSHOT_BYTES)
		if (snapshot === null) {
			return yield* Effect.fail(
				drift(operation, `recorded plan ${entry.id} has no meta/${index.toString(10).padStart(4, "0")}.schema.json snapshot`)
			)
		}
		snapshotTexts.push(snapshot)
	}
	const state: RepositoryState = { manifestText, manifest, planTexts, snapshotTexts, staleDrafts }
	return state
})
