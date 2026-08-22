import { createRequire } from "node:module"
import * as errors from "@superbuilders/errors"
import type { SchemaSpec, ValueSpec, ValueTypeSpec } from "#spec.ts"

/** The opaque database handle (owns the LMDB environment + exclusive lock). */
type DbHandle = { readonly __brand: "bumbledb.db" }

type InstanceHandle = { readonly __brand: "bumbledb.instance" }

type WitnessHandle = { readonly __brand: "bumbledb.witness" }

type BuilderHandle = { readonly __brand: "bumbledb.builder" }

type OwnedHandle = { readonly __brand: "bumbledb.owned" }

type TxHandle = { readonly __brand: "bumbledb.tx" }

type PreparedHandle = { readonly __brand: "bumbledb.prepared" }

interface WireMutationReport {
	readonly submitted: bigint
	readonly changed: bigint
}

type WireFreshRange =
	| { readonly empty: true }
	| { readonly empty: false; readonly start: bigint; readonly endExclusive: bigint }

interface IntervalValue {
	readonly start: bigint
	readonly end: bigint
}

type FactValue = boolean | bigint | string | Uint8Array | IntervalValue

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

type HeadTermIr = { readonly kind: "var" } | { readonly kind: "aggregate"; readonly op: HeadOpIr }

type HeadOpIr = "sum" | "min" | "max" | "count" | "pack"

interface RuleIr {
	readonly finds: readonly FindTermIr[]
	readonly atoms: readonly AtomIr[]
	readonly negated: readonly AtomIr[]
	readonly conditions: readonly ConditionTreeIr[]
}

type FoldOpIr = { readonly kind: "sum" } | { readonly kind: "min" } | { readonly kind: "max" }

type FindTermIr =
	| { readonly kind: "var"; readonly var: number }
	| { readonly kind: "count" }
	| { readonly kind: "aggregate"; readonly op: FoldOpIr; readonly over: number }
	| { readonly kind: "pack"; readonly over: number }

declare const parsedQueryBrand: unique symbol

type ParsedQuery = QueryIr & { readonly [parsedQueryBrand]: true }

type AggOpIr =
	| { readonly kind: "sum" }
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

interface ManifestStatement {
	readonly id: number
	readonly kind: StatementKindTag
	readonly spelling: string
}

interface Manifest {
	readonly relations: readonly ManifestRelation[]
	readonly statements: readonly ManifestStatement[]
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

type CreateResult =
	| { readonly tag: "accepted"; readonly db: DbHandle }
	| { readonly tag: "rejected"; readonly violations: readonly Violation[] }
	| { readonly tag: "schemaError"; readonly message: string }
	| { readonly tag: "newtypeMismatch"; readonly message: string }

/**
 * `dbOpen`'s domain outcome. `schemaError` spans both spec
 * resolution (unresolvable names, banned spellings — every issue in one
 * message) and schema validation at the declaration boundary;
 * `newtypeMismatch` is the coherence wall's own kind; `fingerprintMismatch`
 * is `dbOpen`'s stored-theory refusal.
 */
type DbOpenResult =
	| { readonly ok: true; readonly db: DbHandle }
	| {
			readonly ok: false
			readonly kind: "schemaError" | "newtypeMismatch" | "fingerprintMismatch"
			readonly message: string
	  }

type NativeWriteOutcome =
	| { readonly tag: "accepted"; readonly generation: bigint }
	| { readonly tag: "rejected"; readonly violations: readonly Violation[] }
	| { readonly tag: "abandoned" }
	| { readonly tag: "moved"; readonly witnessed: bigint; readonly current: bigint }

type AdmitResult =
	| { readonly tag: "accepted"; readonly value: OwnedHandle }
	| { readonly tag: "rejected"; readonly violations: readonly Violation[] }

type PrepareResult =
	| { readonly ok: true; readonly prepared: PreparedHandle }
	| { readonly ok: false; readonly kind: "irError"; readonly message: string }

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
	| "freshExhausted"
	| "closedRelationWrite"
	| "commitSync"
	| "transactionPoisoned"
	| "foreignPrepared"
	| "foreignWitness"
	| "param"
	| "capacityRayMeasure"
	| "derivedBudgetExceeded"
	| "overflow"
	| "resultBytesOverflow"
	| "corruption"

type AdmissionTag = "accepted" | "rejected"
type WriteTag = "accepted" | "rejected" | "abandoned" | "moved"
type OpenKind = "schemaError" | "newtypeMismatch" | "fingerprintMismatch"
type PrepareKind = "irError"

interface Native {
	engineVersion(): string

	dbCreate(path: string, spec: SchemaSpec): Promise<CreateResult>

	dbOpen(path: string, spec: SchemaSpec): Promise<DbOpenResult>

	dbManifest(db: DbHandle): Manifest

	dbFingerprint(db: DbHandle): string

	dbGeneration(db: DbHandle): bigint

	dbFromInstance(path: string, instance: OwnedHandle): Promise<DbHandle>

	/**
	 * Runs `callback` synchronously inside the engine read lease. The
	 * instance handle is invalid after the callback returns; the witness
	 * handle is a clone and may escape.
	 */
	dbRead<R>(db: DbHandle, callback: (instance: InstanceHandle, witness: WitnessHandle) => R): R
	instanceGeneration(instance: InstanceHandle): bigint
	instanceScan(instance: InstanceHandle, relationId: number): FactValue[][]

	instanceCount(instance: InstanceHandle, relationId: number): bigint
	instanceContains(instance: InstanceHandle, relationId: number, values: readonly FactValue[]): boolean
	instanceGet(
		instance: InstanceHandle,
		relationId: number,
		keyStatementId: number,
		keyValues: readonly FactValue[]
	): FactValue[] | null
	instancePrepare(instance: InstanceHandle, query: ParsedQuery): PrepareResult
	witnessClose(witness: WitnessHandle): void

	dbWrite(db: DbHandle, callback: (tx: TxHandle) => boolean): NativeWriteOutcome

	dbWriteFrom(db: DbHandle, witness: WitnessHandle, callback: (tx: TxHandle) => boolean): NativeWriteOutcome
	/**
	 * Records a collection of inserts into the delta; returns the engine
	 * `{ submitted, changed }` report. `cells` is ONE flat row-major array
	 * (length rows×arity) in sealed field order, and `rows` is the EXPLICIT
	 * row count the caller states — the one collection crossing
	 * (proposals/one-representation/20): the JS side alone knows N when the
	 * roster is fieldless (N nullary facts project to 0 cells, so no
	 * derivation can recover N), and the bridge verifies
	 * `cells.length === rows × arity` exactly against its resident sealed
	 * roster before building the engine's shape-proved collection in a
	 * single pass. Empty (`rows === 0n`, no cells) is lawful and still a
	 * mutation. Nothing is judged until commit; shape violations throw
	 * typed, naming relation and field.
	 */
	txInsert(tx: TxHandle, relationId: number, rows: bigint, cells: readonly FactValue[]): WireMutationReport

	txDelete(tx: TxHandle, relationId: number, rows: bigint, cells: readonly FactValue[]): WireMutationReport

	txContains(tx: TxHandle, relationId: number, values: readonly FactValue[]): boolean

	txGet(tx: TxHandle, relationId: number, keyStatementId: number, keyValues: readonly FactValue[]): FactValue[] | null

	txReserve(tx: TxHandle, relationId: number, fieldId: number, count: bigint): WireFreshRange

	dbPrepare(db: DbHandle, query: ParsedQuery): PrepareResult

	preparedExecute(prepared: PreparedHandle, instance: InstanceHandle, params: readonly QueryParam[]): FactValue[][]

	preparedClose(prepared: PreparedHandle): void

	instanceBuilderNew(spec: SchemaSpec): BuilderHandle

	instanceBuilderLoad(
		builder: BuilderHandle,
		relationId: number,
		rows: bigint,
		cells: readonly FactValue[]
	): WireMutationReport
	instanceBuilderDelete(
		builder: BuilderHandle,
		relationId: number,
		rows: bigint,
		cells: readonly FactValue[]
	): WireMutationReport
	instanceBuilderReserve(builder: BuilderHandle, relationId: number, fieldId: number, count: bigint): WireFreshRange
	instanceBuilderContains(builder: BuilderHandle, relationId: number, values: readonly FactValue[]): boolean
	instanceBuilderGet(
		builder: BuilderHandle,
		relationId: number,
		keyStatementId: number,
		keyValues: readonly FactValue[]
	): FactValue[] | null
	instanceBuilderClose(builder: BuilderHandle): void
	instanceBuilderAdmit(builder: BuilderHandle): Promise<AdmitResult>
	ownedInstanceClose(instance: OwnedHandle): void
	ownedScan(instance: OwnedHandle, relationId: number): FactValue[][]

	ownedCount(instance: OwnedHandle, relationId: number): bigint
	ownedContains(instance: OwnedHandle, relationId: number, values: readonly FactValue[]): boolean
	ownedGet(
		instance: OwnedHandle,
		relationId: number,
		keyStatementId: number,
		keyValues: readonly FactValue[]
	): FactValue[] | null
	ownedPrepare(instance: OwnedHandle, query: ParsedQuery): PrepareResult
	ownedExecute(prepared: PreparedHandle, instance: OwnedHandle, params: readonly QueryParam[]): FactValue[][]
}

const SHIPPED_PLATFORMS = "darwin-arm64"

const requireNative = createRequire(import.meta.url)

interface NativeBinding extends Native {
	dbClose(db: DbHandle): void
}

function loadNativeBinding(platform: string, arch: string): NativeBinding {
	const platformPackage = `@bjornpagen/bumbledb-${platform}-${arch}`

	const present = errors.trySync(() => requireNative.resolve(`${platformPackage}/package.json`))
	if (present.error) {
		throw errors.wrap(
			present.error,
			`no native binary for ${platform}-${arch}: @bjornpagen/bumbledb ships ${SHIPPED_PLATFORMS} only`
		)
	}

	const loaded = errors.trySync(() => requireNative(platformPackage))
	if (loaded.error) {
		throw errors.wrap(loaded.error, `load the ${platformPackage} native binary (package present but unloadable)`)
	}
	return loaded.data
}

const binding: NativeBinding = loadNativeBinding(process.platform, process.arch)
const native: Native = binding

function dbClose(db: DbHandle): void {
	binding.dbClose(db)
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
		const error = errors.new(`bumbledb ${caught.kind}: ${caught.message}`)
		Object.defineProperty(error, "kind", { value: caught.kind, enumerable: true })
		return error
	}
	return errors.new(String(caught))
}

function bridged<T>(context: string, run: () => T): T {
	try {
		return run()
	} catch (caught) {
		const inner = errorFromThrow(caught)
		throw errors.wrap(inner, `${context}: ${inner.message}`)
	}
}

async function bridgedAsync<T>(context: string, run: () => Promise<T>): Promise<T> {
	try {
		return await run()
	} catch (caught) {
		const inner = errorFromThrow(caught)
		throw errors.wrap(inner, `${context}: ${inner.message}`)
	}
}

export type {
	AdmissionTag,
	AdmitResult,
	AggOpIr,
	AtomIr,
	AtomSourceIr,
	BuilderHandle,
	CmpOpIr,
	ComparisonIr,
	ConditionTreeIr,
	CreateResult,
	DbHandle,
	DbOpenResult,
	ErrorFamilyKind,
	FactValue,
	FindTermIr,
	FoldOpIr,
	HeadOpIr,
	HeadTermIr,
	InstanceHandle,
	InteriorIr,
	IntervalValue,
	Manifest,
	ManifestField,
	ManifestRelation,
	ManifestRow,
	ManifestStatement,
	Native,
	NativeWriteOutcome,
	OpenKind,
	OwnedHandle,
	ParsedQuery,
	PreparedHandle,
	PrepareKind,
	PrepareResult,
	QueryIr,
	QueryParam,
	RecIr,
	RuleIr,
	StatementKindTag,
	TaggedValue,
	TermIr,
	TxHandle,
	Violation,
	ViolationFact,
	WireFreshRange,
	WireMutationReport,
	WitnessHandle,
	WriteTag
}
export { bridged, bridgedAsync, dbClose, errorFromThrow, loadNativeBinding, native, SHIPPED_PLATFORMS }
