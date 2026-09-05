import type { ScalarExpr, SchemaSpec } from "@bjornpagen/bumbledb"

/** All IDs below are verified by the native codec, never by a type assertion. */
export interface SchemaSnapshot {
	readonly id: string
	readonly spec: SchemaSpec
	readonly json: string
}

export type MigrationIntentEntry =
	| { readonly kind: "rename-relation"; readonly from: string; readonly to: string }
	| { readonly kind: "rename-field"; readonly relation: string; readonly from: string; readonly to: string }
	| { readonly kind: "drop-relation"; readonly relation: string }
	| { readonly kind: "drop-field"; readonly relation: string; readonly field: string }
	| { readonly kind: "backfill" | "convert"; readonly relation: string; readonly field: string; readonly expression: ScalarExpr<unknown> }
	| { readonly kind: "seed"; readonly relation: string; readonly rows: Iterable<Readonly<Record<string, unknown>>> }

/** A projection variable uses the core evaluator's positional source field. */
export type MigrationField = {
	readonly target: string
	readonly expression: ScalarExpr<unknown>
}

export type MigrationOperation =
	| { readonly kind: "map-relation"; readonly source: string; readonly target: string; readonly fields: readonly MigrationField[] }
	| { readonly kind: "empty-relation"; readonly target: string }
	| { readonly kind: "drop-relation"; readonly source: string; readonly acknowledged: true }
	| { readonly kind: "seed"; readonly target: string; readonly rows: readonly Readonly<Record<string, unknown>>[] }
	| { readonly kind: "validate-schema"; readonly schemaId: string }

/** Unverified authoring data. Only canonicalizeMigrationPlan can mint executable bytes. */
export interface MigrationPlanInput {
	readonly planVersion: 1
	readonly sequence: bigint
	readonly id: string
	readonly fromSchemaId: string
	readonly toSchemaId: string
	readonly operations: readonly MigrationOperation[]
	readonly destructive: readonly { readonly relation: string; readonly field?: string }[]
}

export interface MigrationEntry {
	readonly sequence: string
	readonly id: string
	readonly fromSchemaId: string
	readonly toSchemaId: string
	readonly planDigest: string
	readonly prefixDigest: string
}

export interface MigrationManifest {
	readonly manifestVersion: 1
	readonly planVersion: 1
	readonly baseSchemaId: string
	readonly basePrefixDigest: string
	readonly entries: readonly MigrationEntry[]
}

export interface RuntimeContract {
	readonly contractVersion: 1
	readonly schemaId: string
	readonly prefixDigest: string
	readonly steps: string
}

export interface MigrationArtifacts {
	readonly plan: { readonly id: string; readonly json: string; readonly digest: string }
	readonly snapshot: SchemaSnapshot
	readonly manifest: MigrationManifest
	readonly manifestJson: string
	readonly runtimeContract: RuntimeContract
	readonly runtimeContractJson: string
}

export interface GenerationReport {
	readonly status: "unchanged" | "generated"
	readonly contract: RuntimeContract
	readonly files: readonly string[]
}
