/**
 * Scripted `MigrationCodec` double for the migration GENERATOR tests
 * (`migrations-*.test.ts`). It is a harness for the LANGUAGE layer only —
 * the repo workflow, diff admission, write ordering, drift refusal and
 * deterministic output — never a second production codec: the one production
 * binding is `#migrations/native.ts` over the native migration codec.
 * Rust conformance tests cover the production digest and frame encodings;
 * this double uses test-only digests.
 *
 * The double is deterministic and self-consistent: fake 64-hex digests are
 * derived from canonical JSON spellings, recorded manifests re-verify from
 * the exact file texts the generator wrote, and any tampered plan or edited
 * entry recomputes to a different digest and refuses — the same OBSERVABLE
 * discipline as the native chain pass, with different (fake) bytes.
 */

import type { SchemaSpec } from "@bjornpagen/bumbledb"
import { NativeRuntime } from "@bjornpagen/bumbledb"
import { Effect } from "effect"
import type { JsonValue } from "#migrations/canonical.ts"
import { compactJson, renderJson } from "#migrations/canonical.ts"
import type { ChainPayload, ChainRequest, EntryPayload, MigrationCodec, SchemaIdentity } from "#migrations/codec.ts"
import { decodeManifestData, decodePlanData } from "#migrations/decode.ts"
import { drift, repository } from "#migrations/fail.ts"
import type { HeldRepositoryLock, RepositoryExclusion } from "#migrations/lock.ts"

// ---------------------------------------------------------------------------
// Deterministic fake 32-byte digest (64 lowercase hex). FNV-1a over UTF-8,
// widened by domain-separated rounds. Test-only bytes, never a real digest.
// ---------------------------------------------------------------------------

const FNV_OFFSET = 0xcbf29ce484222325n
const FNV_PRIME = 0x100000001b3n
const MASK = 0xffffffffffffffffn

function fnv(text: string): bigint {
	let hash = FNV_OFFSET
	for (let index = 0; index < text.length; index += 1) {
		hash ^= BigInt(text.charCodeAt(index))
		hash = (hash * FNV_PRIME) & MASK
	}
	return hash
}

export function fakeDigest(domain: string, text: string): string {
	let out = ""
	for (let round = 0; round < 4; round += 1) {
		out += fnv(`${domain}\0${round}\0${text}`).toString(16).padStart(16, "0")
	}
	return out
}

// ---------------------------------------------------------------------------
// SchemaSpec → the theory-snapshot grammar the diff walks (the double's
// stand-in for native `schema_file::render`). Deterministic: declaration
// order in, one canonical text out. Statements are carried opaquely so a
// law-only change still changes the fake SchemaId.
// ---------------------------------------------------------------------------

function jsonSafe(value: unknown): JsonValue {
	if (value === null || typeof value === "boolean" || typeof value === "string") {
		return value
	}
	if (typeof value === "number") {
		return Number.isFinite(value) ? value : String(value)
	}
	if (typeof value === "bigint") {
		return value.toString(10)
	}
	if (value instanceof Uint8Array) {
		let hex = ""
		for (const byte of value) {
			hex += byte.toString(16).padStart(2, "0")
		}
		return { bytes: hex }
	}
	if (Array.isArray(value)) {
		return value.map(jsonSafe)
	}
	if (typeof value === "object") {
		const out: Record<string, JsonValue> = {}
		for (const key of Object.keys(value as Record<string, unknown>)) {
			const entry = (value as Record<string, unknown>)[key]
			if (entry !== undefined) {
				out[key] = jsonSafe(entry)
			}
		}
		return out
	}
	return String(value)
}

function typeJson(valueType: SchemaSpec["relations"][number]["fields"][number]["valueType"]): JsonValue {
	switch (valueType.kind) {
		case "bool":
		case "u64":
		case "i64":
		case "f64":
		case "uuid":
		case "string":
			return valueType.kind
		case "fixedBytes":
			return { fixedBytes: valueType.len }
		case "interval": {
			if (valueType.width === undefined) {
				return { interval: valueType.element }
			}
			return { fixedInterval: { element: valueType.element, width: valueType.width.toString(10) } }
		}
	}
}

export function renderSnapshot(spec: SchemaSpec): string {
	const relations: JsonValue[] = spec.relations.map((relation) => {
		const body: Record<string, JsonValue> = {
			name: relation.name,
			fields: relation.fields.map((field) => ({ name: field.name, type: typeJson(field.valueType) }))
		}
		if (relation.closed !== undefined) {
			// The native grammar spells closedness as an `extension` row array.
			body.extension = jsonSafe(relation.closed.rows)
		}
		return body
	})
	return renderJson({ relations, statements: spec.statements.map(jsonSafe) })
}

// ---------------------------------------------------------------------------
// The scripted codec. Digest discipline mirrors the native chain pass:
//   schemaId      = H("schema", snapshot text)
//   planDigest    = H("plan", compact plan JSON)
//   basePrefix    = H("prefix-base", baseSchemaId)
//   prefix[i]     = H("prefix", prefix[i-1] ‖ entry-frame-without-own-prefix)
// ---------------------------------------------------------------------------

function refuse(detail: string) {
	return Effect.fail(drift("migrations.double", detail))
}

function entryFrame(entry: Omit<EntryPayload, "prefixDigest">): string {
	return compactJson({
		sequence: entry.sequence,
		id: entry.id,
		fromSchemaId: entry.fromSchemaId,
		toSchemaId: entry.toSchemaId,
		planDigest: entry.planDigest
	})
}

export interface CodecLog {
	schemaCalls: number
	chainCalls: number
}

/**
 * Satisfy the generator's `NativeRuntime` requirement without acquiring the
 * real native registry: the scripted codec performs no native work, so the
 * service is never used — `die` proves it if anything ever does.
 */
export function withStubRuntime<A, E>(effect: Effect.Effect<A, E, NativeRuntime>): Effect.Effect<A, E> {
	return Effect.provideService(effect, NativeRuntime, {
		close: () => Effect.succeed({ kind: "closed" as const }),
		inspect: () => Effect.die("the migration generator double never touches the native runtime")
	})
}

export function scriptedCodec(log?: CodecLog): MigrationCodec {
	function schemaIdentity(spec: SchemaSpec) {
		return Effect.sync(() => {
			if (log !== undefined) {
				log.schemaCalls += 1
			}
			const snapshot = renderSnapshot(spec)
			const identity: SchemaIdentity = { schemaId: fakeDigest("schema", snapshot), snapshot }
			return identity
		})
	}

	function verifyChain(request: ChainRequest) {
		return Effect.suspend((): Effect.Effect<ChainPayload, ReturnType<typeof drift>> => {
			if (log !== undefined) {
				log.chainCalls += 1
			}
			if (request.snapshots.length === 0) {
				return refuse("compiled chain input is mandatory; empty source is not a shortcut")
			}
			let baseSchemaId: string
			let entries: EntryPayload[] = []
			if (request.manifest !== null) {
				const decoded = decodeManifestData(request.manifest)
				if (!decoded.ok) {
					return refuse(decoded.detail)
				}
				baseSchemaId = decoded.value.baseSchemaId
				if (decoded.value.basePrefixDigest !== fakeDigest("prefix-base", baseSchemaId)) {
					return refuse("base prefix digest mismatch")
				}
				entries = [...decoded.value.entries]
			} else {
				if (request.baseSchemaId === null) {
					return refuse("fresh chain without a base schema")
				}
				baseSchemaId = request.baseSchemaId
			}
			if (request.plans.length !== entries.length) {
				return refuse("plan count does not match the manifest")
			}
			// Recompute every recorded digest from the exact plan trees.
			let prefix = fakeDigest("prefix-base", baseSchemaId)
			const labels = new Set<string>()
			for (const [index, entry] of entries.entries()) {
				const tree = request.plans[index]
				const plan = decodePlanData(tree)
				if (!plan.ok) {
					return refuse(`recorded plan ${index}: ${plan.detail}`)
				}
				if (
					entry.sequence !== index.toString(10) ||
					plan.value.sequence !== entry.sequence ||
					plan.value.id !== entry.id ||
					plan.value.fromSchemaId !== entry.fromSchemaId ||
					plan.value.toSchemaId !== entry.toSchemaId
				) {
					return refuse(`recorded plan ${index} disagrees with its manifest entry`)
				}
				if (labels.has(entry.id)) {
					return refuse(`label ${entry.id} reused`)
				}
				labels.add(entry.id)
				const expectedFrom = index === 0 ? baseSchemaId : (entries[index - 1]?.toSchemaId ?? "")
				if (entry.fromSchemaId !== expectedFrom) {
					return refuse(`entry ${index} does not chain from its predecessor`)
				}
				const digest = fakeDigest("plan", compactJson(tree as JsonValue))
				if (digest !== entry.planDigest) {
					return refuse(`recorded plan ${index} bytes do not match planDigest`)
				}
				prefix = fakeDigest("prefix", `${prefix}\0${entryFrame(entry)}`)
				if (prefix !== entry.prefixDigest) {
					return refuse(`entry ${index} prefixDigest mismatch`)
				}
			}
			let appended: ChainPayload["appended"] = null
			if (request.append !== null) {
				const plan = decodePlanData(request.append)
				if (!plan.ok) {
					return refuse(`append: ${plan.detail}`)
				}
				if (plan.value.sequence !== entries.length.toString(10)) {
					return refuse("append sequence is not the next entry")
				}
				const expectedFrom = entries.length === 0 ? baseSchemaId : (entries[entries.length - 1]?.toSchemaId ?? "")
				if (plan.value.fromSchemaId !== expectedFrom) {
					return refuse("append does not chain from the recorded head")
				}
				const last = plan.value.operations[plan.value.operations.length - 1]
				if (last === undefined || last.kind !== "validate-schema" || last.schemaId !== plan.value.toSchemaId) {
					return refuse("append must end with validate-schema naming toSchemaId")
				}
				if (labels.has(plan.value.id)) {
					return refuse(`label ${plan.value.id} reused`)
				}
				const planDigest = fakeDigest("plan", compactJson(request.append))
				const bare = {
					sequence: plan.value.sequence,
					id: plan.value.id,
					fromSchemaId: plan.value.fromSchemaId,
					toSchemaId: plan.value.toSchemaId,
					planDigest
				}
				prefix = fakeDigest("prefix", `${prefix}\0${entryFrame(bare)}`)
				const entry: EntryPayload = { ...bare, prefixDigest: prefix }
				const manifestJson: JsonValue = {
					manifestVersion: 1,
					planVersion: 1,
					baseSchemaId,
					basePrefixDigest: fakeDigest("prefix-base", baseSchemaId),
					entries: [...entries, entry].map((row) => ({
						sequence: row.sequence,
						id: row.id,
						fromSchemaId: row.fromSchemaId,
						toSchemaId: row.toSchemaId,
						planDigest: row.planDigest,
						prefixDigest: row.prefixDigest
					}))
				}
				appended = {
					entry,
					planText: renderJson(request.append),
					manifestText: renderJson(manifestJson)
				}
			}
			const payload: ChainPayload = {
				headPrefixDigest: prefix,
				planSetDigest: null,
				appended
			}
			return Effect.succeed(payload)
		})
	}

	return { schemaIdentity, verifyChain }
}

/** Language-layer exclusion: same-process busy refuse. Not a D21/D28 mock. */
const scriptedHeld = new Map<string, HeldRepositoryLock>()

export function scriptedExclusion(): RepositoryExclusion {
	return {
		acquire(operation, directory) {
			return Effect.gen(function* () {
				const key = directory.endsWith("/") ? directory.slice(0, -1) : directory
				if (scriptedHeld.has(key)) {
					return yield* Effect.fail(
						repository(operation, directory, "another generation holds the exclusive repository lock")
					)
				}
				const held: HeldRepositoryLock = {
					directory,
					release: Effect.sync(() => {
						scriptedHeld.delete(key)
					})
				}
				scriptedHeld.set(key, held)
				yield* Effect.addFinalizer(() => held.release)
				return held
			})
		}
	}
}
