/**
 * The one native-codec seam of the migration GENERATOR (C11). Everything
 * semantic — schema validation, canonical SchemaId, snapshot rendering, plan
 * validation/rendering/digesting, manifest verification/appending, plan-set
 * digests — happens in the native codec (P09: `schema_file::{schema_id,
 * render}`, `migration::plan`, `migration::manifest`) reached through the P06
 * bridge. TypeScript never reimplements framing, hashing or scalar
 * arithmetic; this interface only names the two bounded read entrypoints the
 * generator calls. `#migrations/native.ts` binds the production codec;
 * authored tests drive the same generator through a scripted codec because
 * physical digest bytes remain provisional until the F3 format freeze (C12).
 *
 * This module is types only — importing it performs no native work and does
 * not load the addon.
 */
import type { ExecutionPolicy, NativeRuntime, SchemaSpec } from "@bjornpagen/bumbledb"
import type { Effect } from "effect"
import type { LogError } from "#errors.ts"
import type { JsonValue } from "#migrations/canonical.ts"

export interface SchemaIdentity {
	/** Canonical schema fingerprint, 64 lowercase hex (core v6 stream). */
	readonly schemaId: string
	/** The canonical `schema_file::render` text, written verbatim to meta/. */
	readonly snapshot: string
}

export interface EntryPayload {
	readonly sequence: string
	readonly id: string
	readonly fromSchemaId: string
	readonly toSchemaId: string
	readonly planDigest: string
	readonly prefixDigest: string
}

export interface ChainRequest {
	/** Parsed `manifest.json` tree when the repo has one; null roots a fresh chain. */
	readonly manifest: JsonValue | null
	/** Required when `manifest` is null: the declared empty-base schema. */
	readonly baseSchemaId: string | null
	/** Every recorded plan's parsed tree, in manifest order (`bind_plans`). */
	readonly plans: readonly JsonValue[]
	/** A new plan to validate + append (data tree, `parse_plan` grammar). */
	readonly append: JsonValue | null
	/** Request a `plan_set_digest` over entries [first, first+count). */
	readonly planSet: { readonly first: number; readonly count: number } | null
}

export interface ChainPayload {
	readonly headPrefixDigest: string
	readonly planSetDigest: string | null
	readonly appended: {
		readonly entry: EntryPayload
		/** Canonical `render_plan` text: written verbatim, never reformatted. */
		readonly planText: string
		/** Canonical `render_manifest` text after the append. */
		readonly manifestText: string
	} | null
}

export interface MigrationCodec {
	/** Validate + fingerprint + render the current schema natively. */
	schemaIdentity(spec: SchemaSpec, work: ExecutionPolicy): Effect.Effect<SchemaIdentity, LogError, NativeRuntime>
	/**
	 * One native chain pass: parse + verify the manifest, bind every recorded
	 * plan's canonical digest, optionally validate/append a new plan (returning
	 * its canonical rendered text, the new manifest text and the new entry),
	 * and optionally compute a pending plan-set digest. Nothing is trusted
	 * from text.
	 */
	verifyChain(request: ChainRequest, work: ExecutionPolicy): Effect.Effect<ChainPayload, LogError, NativeRuntime>
}
