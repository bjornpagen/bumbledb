/**
 * C11 data model — the generated repo-local migration artifacts,
 * mirroring the native codec exactly
 * (`crates/bumbledb-log/src/migration/{plan,manifest}.rs` and
 * `crates/bumbledb-log/src/schema_file.rs`, P09). Everything here is plain
 * readonly data: a plan file is inert canonical JSON, never executable
 * TypeScript, and a well-shaped object alone is not a checked plan — the
 * native migration codec is the one validation, rendering and digest
 * authority. Every 32-byte digest is 64 lowercase hex computed natively;
 * TypeScript never reimplements the canonical framing, hashing or scalar
 * arithmetic.
 *
 * Human labels versus digests: `id` (`0001-note-pinned`) is a stable human
 * label (1..=64 of `[a-z0-9-]`, unique per manifest) for review and file
 * names. It never participates in identity; `planDigest`/`prefixDigest`/
 * `planSetDigest` are the commitments, and a changed plan under a reused
 * label is drift.
 */
import type { ExecutionPolicy, Schema, SchemaRelations } from "@bjornpagen/bumbledb"
import type { DatabaseIdentity, DecisionDigest, OperationId, PlanSetDigest } from "#identity.ts"
import type { MigrationIntent } from "#migrations/intent.ts"

/**
 * One canonical value cell (`migration::json` grammar, shared by plans and
 * schema snapshots): a single-arm object. Integers are decimal strings;
 * `$f64` is the canonical bit image (canonical quiet NaN, canonical +0) as
 * 16 lowercase hex digits; `id128` is 32 lowercase hex; bytes are lowercase
 * hex; intervals are `[start, end]` pairs in their element spelling.
 */
export type PlanValue =
	| { readonly bool: boolean }
	| { readonly u64: string }
	| { readonly i64: string }
	| { readonly $f64: string }
	| { readonly id128: string }
	| { readonly string: string }
	| { readonly fixedBytes: string }
	| { readonly intervalU64: readonly [string, string] }
	| { readonly intervalI64: readonly [string, string] }
	| { readonly intervalF64: readonly [string, string] }

/**
 * The generated expression fragment: exactly the core `ScalarExpr` roster
 * with the migration context's named SOURCE-field reference in the variable
 * position. Source-field arithmetic may stay unresolved until native compile
 * binds it. The core `ScalarEvaluator` is the one meaning. No closure,
 * module path or opaque "run this code" node exists.
 */
export type PlanExpression =
	| { readonly kind: "field"; readonly name: string }
	| { readonly kind: "literal"; readonly value: PlanValue }
	| { readonly kind: "negate"; readonly expr: PlanExpression }
	| { readonly kind: "isNaN"; readonly expr: PlanExpression }
	| { readonly kind: "isFinite"; readonly expr: PlanExpression }
	| {
			readonly kind: "add" | "subtract" | "multiply" | "divide"
			readonly left: PlanExpression
			readonly right: PlanExpression
	  }
	| {
			readonly kind: "cast"
			readonly cast: "toF64" | "toF64Exact" | "toI64Exact" | "toU64Exact"
			readonly expr: PlanExpression
	  }

export interface PlanFieldMap {
	readonly target: string
	readonly expression: PlanExpression
}

/**
 * The finite operation roster. Coverage is total over ORDINARY relations
 * (closed relations are sealed schema axioms, unnameable by data ops):
 * every ordinary source relation appears exactly once as `map-relation.
 * source` or `drop-relation.source`; every ordinary target relation exactly
 * once as `map-relation.target` or `empty-relation.target`. Seeds follow
 * their producing operation; `validate-schema` is the required final
 * operation and must name `toSchemaId`.
 */
export type PlanOperation =
	| {
			readonly kind: "map-relation"
			readonly source: string
			readonly target: string
			readonly fields: readonly PlanFieldMap[]
	  }
	| { readonly kind: "empty-relation"; readonly target: string }
	| { readonly kind: "drop-relation"; readonly source: string }
	| { readonly kind: "seed"; readonly target: string; readonly rows: ReadonlyArray<readonly PlanValue[]> }
	| { readonly kind: "validate-schema"; readonly schemaId: string }

/**
 * One explicit data-loss acknowledgement: a dropped source relation
 * (`field` absent) or a source field no expression references. Every actual
 * loss needs exactly one acknowledgement and stale acknowledgements refuse
 * — both directions are re-judged natively.
 */
export interface PlanLoss {
	readonly relation: string
	readonly field?: string
}

/** One generated `NNNN-label.plan.json`. Immutable once recorded. */
export interface MigrationPlan {
	readonly planVersion: 1
	/** Decimal manifest index, contiguous from "0". */
	readonly sequence: string
	/** The stable human label, e.g. `0001-note-pinned`. */
	readonly id: string
	readonly fromSchemaId: string
	readonly toSchemaId: string
	readonly operations: readonly PlanOperation[]
	readonly destructive: readonly PlanLoss[]
}

/** One recorded chain identity inside `manifest.json`. */
export interface ManifestEntry {
	readonly sequence: string
	readonly id: string
	readonly fromSchemaId: string
	readonly toSchemaId: string
	readonly planDigest: string
	readonly prefixDigest: string
}

/**
 * The ordered chain of recorded identities. Prefix hashing is acyclic,
 * domain-separated and NATIVE: `basePrefixDigest` commits the manifest/plan
 * codec versions and the empty-base SchemaId; each entry's `prefixDigest`
 * commits the previous prefix plus the canonical entry frame excluding its
 * own prefix field. Verification is recomputation, never trust in text.
 */
export interface MigrationManifest {
	readonly manifestVersion: 1
	readonly planVersion: 1
	readonly baseSchemaId: string
	readonly basePrefixDigest: string
	readonly entries: readonly ManifestEntry[]
}

/**
 * The app's small deployed expectation (chapter 33 `runtime-contract.json`).
 * `schemaId`/`appliedPrefixDigest` are exactly P08's `RuntimeExpectation`
 * fields; `steps` is the decimal count of recorded manifest entries.
 */
export interface RuntimeContract {
	readonly contractVersion: 1
	readonly schemaId: string
	readonly appliedPrefixDigest: string
	readonly steps: string
}

/** What the generated static `index.ts` exports: data imports only. */
export interface GeneratedMigrations {
	readonly manifest: MigrationManifest
	readonly plans: readonly MigrationPlan[]
	/**
	 * Canonical `schema_file::render` texts: empty-base first, then each
	 * recorded plan target — exactly `entries.length + 1` rows. Mandatory
	 * for native chain compile (C8/D20); empty source is not a shortcut.
	 */
	readonly snapshots: readonly string[]
}

// ---------------------------------------------------------------------------
// The parsed theory (schema snapshot) shape the diff walks. Snapshot files
// are the native `schema_file::render` text verbatim; this is its bounded
// parsed image. `closed` relations carry a declared extension and are exempt
// from data operations.
// ---------------------------------------------------------------------------

export interface TheoryField {
	readonly name: string
	/** Canonical JSON spelling of the field type, for exact comparison. */
	readonly type: string
}

export interface TheoryRelation {
	readonly name: string
	readonly fields: readonly TheoryField[]
	readonly closed: boolean
}

export interface TheorySnapshot {
	readonly relations: readonly TheoryRelation[]
}

// ---------------------------------------------------------------------------
// Generator options and reports.
// ---------------------------------------------------------------------------

export interface MigrationRepository {
	/** The checked-in migrations directory, e.g. `bumbledb/migrations`. */
	readonly directory: string
	/**
	 * Where the runtime contract is written; default
	 * `<directory>/runtime-contract.json`.
	 */
	readonly contract?: string
}

export interface GenerateOptions<Rels extends SchemaRelations> {
	readonly schema: Schema<Rels>
	readonly intent?: MigrationIntent<Rels>
	/**
	 * Stable human label suffix for the new plan (`[a-z0-9-]`; the full id
	 * becomes `NNNN-<label>` and must stay within 64). When absent a
	 * deterministic label is derived from the diff.
	 */
	readonly label?: string
	readonly repository: MigrationRepository
	readonly work: ExecutionPolicy
}

export type CheckOptions<Rels extends SchemaRelations> = Omit<GenerateOptions<Rels>, "label">

export interface GenerationReport {
	readonly status: "unchanged" | "generated"
	readonly planId: string | null
	readonly contract: RuntimeContract
	/** Repository-relative paths written by this run (empty when unchanged). */
	readonly files: readonly string[]
	/**
	 * Repository-relative interrupted-generation leftovers this run removed
	 * (unrecorded next-sequence drafts only; recorded history is never touched).
	 */
	readonly removed: readonly string[]
}

export interface CheckReport {
	readonly status: "clean" | "generation-required"
	readonly detail: string
	readonly contract: RuntimeContract
}

// ---------------------------------------------------------------------------
// C11 activation reference. The rest of the admin vocabulary — AdminOutcome,
// MigrateValue, MigrationStatus, InitializeValue, ActivationReport,
// AbortReport, MigrationRef, AdminIdentityOptions — is P08's `#outcome.ts`/
// `#machine.ts` authority (chapter 35 Migration/admin section); the runner
// operations themselves are re-exported from P08's `#migration-ops.ts` by
// `#migrations/index.ts`. This module keeps only the generated-data roster.
// ---------------------------------------------------------------------------

/**
 * The activation reference returned inside `ready-to-switch` (P09
 * `ActivationRef`): durable in the target control before any activation
 * attempt, passed back verbatim to `activateMigration`/`abortMigration`.
 */
export interface ActivationRef {
	readonly operation: OperationId
	readonly planSetDigest: PlanSetDigest
	readonly target: DatabaseIdentity
	readonly targetGenesis: DecisionDigest
}
