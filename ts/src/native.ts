import { NativeOperationError, NativeReportedError } from "./errors.ts"
import { loadAddon } from "./native/load.ts"
import type { SchemaSpec, ValueSpec, ValueTypeSpec } from "./spec.ts"

/**
 * The opaque managed database capability. The native runtime registry —
 * not this wrapper — owns the LMDB environment, the kernel directory
 * lock and every session; a retained wrapper after a completed close is
 * inert. Minted only by the managed handshake
 * (`runtimeDirectoryDbOpen` → `runtimeDbTake` in #runtime-native.ts).
 */
type DbHandle = { readonly __brand: "bumbledb.db" }

/** A discrete half-open interval `[start, end)` over u64/i64. */
interface IntervalValue {
	readonly start: bigint
	readonly end: bigint
}

/**
 * A dense-line half-open interval with canonical binary64 endpoints:
 * NaN-free, strictly ordered; ±Infinity are unbounded endpoints.
 */
interface F64IntervalValue {
	readonly start: number
	readonly end: number
}

/**
 * One marshalled cell. An application-owned Uuid crosses as its
 * canonical hyphenated UUID string; there is no fresh
 * range, reservation counter or issuance value anywhere on this wire.
 */
type FactValue = boolean | bigint | number | string | Uint8Array | IntervalValue | F64IntervalValue

type TaggedValue = ValueSpec

type QueryParam = TaggedValue | { readonly kind: "set"; readonly values: readonly TaggedValue[] }

type QueryIr =
	| {
			readonly kind: "cq"
			readonly interiors: readonly InteriorIr[]
			readonly head: readonly HeadTermIr[]
			readonly rules: readonly RuleIr[]
	  }
	| {
			readonly kind: "reach"
			readonly interiors: readonly InteriorIr[]
			readonly rec: RecIr
			readonly head: readonly HeadTermIr[]
			readonly rules: readonly RuleIr[]
	  }

interface InteriorIr {
	readonly head: readonly HeadTermIr[]
	readonly rules: readonly RuleIr[]
}

interface RecIr {
	readonly head: readonly HeadTermIr[]
	readonly base: readonly RuleIr[]
	readonly rec: readonly RuleIr[]
}

type HeadTermIr =
	| { readonly kind: "var" }
	| { readonly kind: "compute" }
	| { readonly kind: "aggregate"; readonly op: HeadOpIr }

type HeadOpIr = "sum" | "mean" | "min" | "max" | "count" | "pack"

/**
 * The computed-find scalar expression (C05 `FindTerm::Compute(ScalarExpr)`):
 * exactly the frozen core roster, spelled as the canonical scalar JSON spells
 * the same expressions (one grammar, no second evaluator). `var` binds a
 * rule variable ordinal on this lane.
 */
type NumericCastIr = "toF64" | "toF64Exact" | "toI64Exact" | "toU64Exact"

type ScalarExprIr =
	| { readonly kind: "measure"; readonly expr: ScalarExprIr }
	| {
			readonly kind: "mulDiv"
			readonly a: ScalarExprIr
			readonly b: ScalarExprIr
			readonly divisor: ScalarExprIr
			readonly rounding: import("./scalar.ts").Rounding
	  }
	| { readonly kind: "var"; readonly var: number }
	| { readonly kind: "literal"; readonly value: TaggedValue }
	| { readonly kind: "negate"; readonly expr: ScalarExprIr }
	| { readonly kind: "add"; readonly left: ScalarExprIr; readonly right: ScalarExprIr }
	| { readonly kind: "subtract"; readonly left: ScalarExprIr; readonly right: ScalarExprIr }
	| { readonly kind: "multiply"; readonly left: ScalarExprIr; readonly right: ScalarExprIr }
	| { readonly kind: "divide"; readonly left: ScalarExprIr; readonly right: ScalarExprIr }
	| { readonly kind: "cast"; readonly cast: NumericCastIr; readonly expr: ScalarExprIr }
	| { readonly kind: "isNaN"; readonly expr: ScalarExprIr }
	| { readonly kind: "isFinite"; readonly expr: ScalarExprIr }

interface RuleIr {
	readonly finds: readonly FindTermIr[]
	readonly atoms: readonly AtomIr[]
	readonly negated: readonly AtomIr[]
	readonly conditions: readonly ConditionTreeIr[]
}

type FoldOpIr =
	| { readonly kind: "sum" }
	| { readonly kind: "mean" }
	| { readonly kind: "min" }
	| { readonly kind: "max" }

type FindTermIr =
	| {
			readonly kind: "segments"
			readonly op: import("./query/segments.ts").SegmentOp
			readonly left: number
			readonly right: number
	  }
	| { readonly kind: "var"; readonly var: number }
	| { readonly kind: "compute"; readonly expr: ScalarExprIr }
	| { readonly kind: "count" }
	| { readonly kind: "aggregate"; readonly op: FoldOpIr; readonly over: number }
	| { readonly kind: "pack"; readonly over: number }

declare const parsedQueryBrand: unique symbol

type ParsedQuery = QueryIr & { readonly [parsedQueryBrand]: true }

type AggOpIr =
	| { readonly kind: "sum" }
	| { readonly kind: "mean" }
	| { readonly kind: "min" }
	| { readonly kind: "max" }
	| { readonly kind: "count" }
	| { readonly kind: "pack" }

type AtomSourceIr =
	| { readonly kind: "edb"; readonly relation: number }
	| { readonly kind: "interior"; readonly interior: number }

interface AtomIr {
	readonly source: AtomSourceIr
	readonly bindings: ReadonlyArray<readonly [number, TermIr]>
}

type TermIr =
	| { readonly kind: "var"; readonly var: number }
	| { readonly kind: "param"; readonly param: number }
	| { readonly kind: "paramSet"; readonly param: number }
	| { readonly kind: "literal"; readonly value: TaggedValue }

type CmpOpIr =
	| { readonly kind: "eq" }
	| { readonly kind: "ne" }
	| { readonly kind: "lt" }
	| { readonly kind: "le" }
	| { readonly kind: "gt" }
	| { readonly kind: "ge" }
	| { readonly kind: "allen"; readonly mask: number }
	| { readonly kind: "pointIn" }

interface ComparisonIr {
	readonly op: CmpOpIr
	readonly lhs: TermIr
	readonly rhs: TermIr
}

type ConditionTreeIr =
	| { readonly kind: "leaf"; readonly cmp: ComparisonIr }
	| { readonly kind: "and"; readonly children: readonly ConditionTreeIr[] }
	| { readonly kind: "or"; readonly children: readonly ConditionTreeIr[] }

type StatementKindTag = "functionality" | "containment" | "capacity"

interface ManifestField {
	readonly name: string
	readonly id: number
	readonly valueType: ValueTypeSpec
	/** The field's host newtype name off the declared spec; a closed relation's synthetic `id` slot carries the handle newtype. Absent on a bare column. */
	readonly newtype?: string
}

interface ManifestRow {
	readonly handle: string
	readonly id: bigint
	readonly values: ReadonlyArray<{ readonly name: string; readonly value: FactValue }>
}

interface ManifestRelation {
	readonly name: string
	readonly id: number
	readonly fields: readonly ManifestField[]
	readonly extension?: readonly ManifestRow[]
}

interface SealedSide {
	readonly relation: number
	readonly projection: readonly number[]
	readonly selection: ReadonlyArray<{ readonly field: number; readonly values: readonly FactValue[] }>
}

type SealedWeight =
	| { readonly kind: "unit" }
	| { readonly kind: "field"; readonly field: number }
	| { readonly kind: "duration"; readonly field: number }

type SealedHi =
	| { readonly kind: "unbounded" }
	| { readonly kind: "lit"; readonly value: bigint }
	| { readonly kind: "targetField"; readonly field: number }
	| { readonly kind: "targetDuration"; readonly field: number }

type SealedStatement =
	| {
			readonly id: number
			readonly kind: "functionality"
			readonly relation: number
			readonly projection: readonly number[]
	  }
	| {
			readonly id: number
			readonly kind: "containment"
			readonly source: SealedSide
			readonly target: SealedSide
	  }
	| {
			readonly id: number
			readonly kind: "capacity"
			readonly target: SealedSide
			readonly weight: SealedWeight
			readonly lo: bigint
			readonly hi: SealedHi
			readonly source: SealedSide
	  }

interface SealedDescriptor {
	readonly relations: readonly ManifestRelation[]
	readonly statements: readonly SealedStatement[]
	readonly fingerprint: string
}

interface ViolationFact {
	readonly relation: string
	readonly fields: ReadonlyArray<{ readonly name: string; readonly value: FactValue }>
}

type Violation =
	| {
			readonly statementId: number
			readonly kind: "functionality"
			readonly canonical: string
			readonly facts: readonly ViolationFact[]
	  }
	| {
			readonly statementId: number
			readonly kind: "containment"
			readonly canonical: string
			readonly direction: "sourceUnsatisfied" | "targetRequired"
			readonly facts: readonly ViolationFact[]
	  }
	| {
			readonly statementId: number
			readonly kind: "capacity"
			readonly canonical: string
			readonly measure: bigint
			readonly facts: readonly ViolationFact[]
	  }

// The managed create/open outcome is `ManagedDbOutcome` in
// #runtime-native.ts (accepted | rejected | refused); reads, writes and
// queries are worker-affine session verbs there too. The historical
// raw-pointer `dbRead`/`dbWrite`/`tx*`/`instance*`/`prepared*` synchronous
// surface — a JS callback inside a native transaction — is deleted,
// as are the fresh/reserve issuance verbs (`WireFreshRange`).

type ErrorFamilyKind =
	| "formatMismatch"
	| "schemaMismatch"
	| "alreadyInitialized"
	| "destinationExists"
	| "publishedButUnsynced"
	| "environmentLocked"
	| "io"
	| "lmdb"
	| "readersFull"
	| "schema"
	| "validation"
	| "factShape"
	| "closedRelationWrite"
	| "commitSync"
	| "transactionPoisoned"
	| "foreignPrepared"
	| "foreignWitness"
	| "param"
	| "capacityRayMeasure"
	| "overflow"
	| "scalar"
	| "resultBytesOverflow"
	| "corruption"
	| "store"

type AdmissionTag = "accepted" | "rejected"
type WriteTag = "accepted" | "rejected" | "abandoned" | "moved"
type OpenKind = "schemaError" | "newtypeMismatch" | "fingerprintMismatch" | "destinationExists"
type PrepareKind = "irError"

interface Native {
	engineVersion(): string

	/** Small identity-sized inputs only; bulk hashing rides `runtimeHash`. */
	blake3Hash(data: Uint8Array): Uint8Array

	descriptor(spec: SchemaSpec): SealedDescriptor
}

let binding: Native | undefined

function ensureNativeBinding(): Native {
	binding ??= loadAddon<Native>(process.platform, process.arch)
	return binding
}

/** Lazy singleton: the addon loads on first native access, not module evaluation. */
const native: Native = new Proxy({} as Native, {
	get(_target, property, receiver) {
		return Reflect.get(ensureNativeBinding(), property, receiver)
	},
	set(_target, property, value) {
		return Reflect.set(ensureNativeBinding(), property, value)
	}
})

function nativeBindingIsLoaded(): boolean {
	return binding !== undefined
}

function isEngineThrow(value: unknown): value is { kind: ErrorFamilyKind; message: string } {
	if (typeof value !== "object" || value === null) {
		return false
	}
	const rec = value as { kind?: unknown; message?: unknown }
	return typeof rec.kind === "string" && typeof rec.message === "string"
}

function errorFromThrow(caught: unknown): Error {
	if (caught instanceof Error) {
		return caught
	}
	if (isEngineThrow(caught)) {
		return new NativeReportedError({
			kind: caught.kind,
			message: `bumbledb ${caught.kind}: ${caught.message}`,
			cause: caught
		})
	}
	return new NativeReportedError({ kind: "Unknown", message: String(caught), cause: caught })
}

function bridged<T>(context: string, run: () => T): T {
	try {
		return run()
	} catch (caught) {
		if (caught instanceof Error) throw caught
		throw new NativeOperationError({ operation: context, cause: caught })
	}
}

export type {
	AdmissionTag,
	AggOpIr,
	AtomIr,
	AtomSourceIr,
	CmpOpIr,
	ComparisonIr,
	ConditionTreeIr,
	DbHandle,
	ErrorFamilyKind,
	F64IntervalValue,
	FactValue,
	FindTermIr,
	HeadOpIr,
	HeadTermIr,
	InteriorIr,
	IntervalValue,
	NumericCastIr,
	OpenKind,
	ParsedQuery,
	PrepareKind,
	QueryIr,
	QueryParam,
	RecIr,
	RuleIr,
	ScalarExprIr,
	SealedDescriptor,
	SealedHi,
	SealedSide,
	SealedStatement,
	SealedWeight,
	StatementKindTag,
	TaggedValue,
	TermIr,
	Violation,
	ViolationFact,
	WriteTag
}
export { bridged, errorFromThrow, native, nativeBindingIsLoaded }
