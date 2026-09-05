import { createRequire } from "node:module"
import { Result } from "effect"
import { NativeLoadError, NativeOperationError, NativeReportedError } from "#errors.ts"
import type { SchemaSpec, ValueSpec, ValueTypeSpec } from "#spec.ts"

/**
 * The opaque managed database capability. The native runtime registry —
 * not this wrapper — owns the LMDB environment, the kernel directory
 * lock and every session; a retained wrapper after a completed close is
 * inert. Minted only by the managed handshake
 * (`runtimeDirectoryDbOpen` → `runtimeDbTake` in #runtime-native.ts).
 */
type DbHandle = { readonly __brand: "bumbledb.db" }

/** Owned generation evidence (a value clone, never a borrow). */
type WitnessHandle = { readonly __brand: "bumbledb.witness" }

type BuilderHandle = { readonly __brand: "bumbledb.builder" }

type OwnedHandle = { readonly __brand: "bumbledb.owned" }

/**
 * The sealed per-theory log schema (`crates/bumbledb-log` successor
 * lanes): the validated core schema plus its fingerprint, shared by the
 * command/decision grammar. Immutable plain data; no lifecycle verbs.
 */
type LogSchemaHandle = { readonly __brand: "bumbledb.logSchema" }

interface WireMutationReport {
	readonly submitted: bigint
	readonly changed: bigint
}

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
 * One marshalled cell. An application-owned Id128 crosses as its
 * canonical 32-lowercase-hex string (chapter 32); there is no fresh
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
 * exactly the frozen core roster, spelled as the migration plan JSON spells
 * the same expressions (one grammar, no second evaluator). `var` binds a
 * rule variable ordinal on this lane.
 */
type NumericCastIr = "toF64" | "toF64Exact" | "toI64Exact" | "toU64Exact"

type ScalarExprIr =
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

// ---------------------------------------------------------------------------
// Successor log grammar wire (C06 through the C09 bridge). The 0.x
// braids/codec/manifest/sidecar lanes and their mint tables are deleted.
// ---------------------------------------------------------------------------

/**
 * A grammar lane's domain outcome: the payload, or a refusal row whose
 * `kind` is an identity string spelled exactly as `bumbledb-log`'s
 * identities emitter spells it (`logIdentities()` / log-identities.json).
 */
type LogResult<T, K extends string> =
	| { readonly ok: true; readonly value: T }
	| { readonly ok: false; readonly kind: K; readonly message: string }

/** `FrameError` kinds (the `frame` identity family). */
type LogFrameKind =
	| "limitExceeded"
	| "lengthOverflow"
	| "allocation"
	| "truncated"
	| "family"
	| "layout"
	| "kind"
	| "tag"
	| "invalidEpoch"
	| "stateIdentityMismatch"
	| "emptyChangeSummary"
	| "emptyEvidence"
	| "invalidTerminalStamp"
	| "invalidPreconditionEvidence"
	| "invalidPolicy"
	| "invalidSequence"
	| "invalidCount"
	| "trailingBytes"

/** Receipt-row kinds: the frame family plus the foreign-scope refusal. */
type LogReceiptKind = LogFrameKind | "foreignRow"

/** Command-lane kinds: frame family plus the two command-only arms. */
type LogCommandKind = LogFrameKind | "core" | "schemaMismatch"

/** Chain-step refusals from the decode-and-verify lane. */
type LogChainKind = "wrongParent" | "wrongSequence"

/** The complete database scope of every command/receipt/decision. */
interface LogIdentity {
	/** Application-owned Id128, canonical 32-lowercase-hex. */
	readonly databaseId: string
	readonly incarnationId: string
	/** The core schema fingerprint, 64 lowercase hex characters. */
	readonly schemaId: string
}

interface LogStateStamp {
	readonly incarnation: string
	readonly dataRevision: bigint
}

interface LogDecisionStamp {
	readonly seq: bigint
	readonly hash: Uint8Array
}

type LogCondition =
	| { readonly kind: "unconditional" }
	| { readonly kind: "exactState"; readonly state: LogStateStamp }

interface LogCommandId {
	readonly receiptEpoch: bigint
	readonly requestId: string
}

interface LogCommandMetadata {
	readonly identity: LogIdentity
	readonly id: LogCommandId
	readonly condition: LogCondition
}

interface LogCommandRef {
	readonly identity: LogIdentity
	readonly receiptEpoch: bigint
	readonly requestId: string
	readonly digest: Uint8Array
}

type LogOutcomeWire =
	| { readonly kind: "committed"; readonly added: bigint; readonly removed: bigint; readonly result?: Uint8Array }
	| { readonly kind: "noChange"; readonly result?: Uint8Array }
	| { readonly kind: "preconditionFailed"; readonly expected: LogStateStamp; readonly observed: LogStateStamp }
	| { readonly kind: "invariantRejected"; readonly evidence: Uint8Array }

interface LogReceipt {
	readonly command: LogCommandRef
	readonly decisionAt: LogDecisionStamp
	readonly stateAt: LogStateStamp
	readonly outcome: LogOutcomeWire
}

interface LogLimits {
	readonly envelopeBytes: bigint
	readonly changeBytes: bigint
	readonly evidenceBytes: bigint
	readonly resultBytes: bigint
}

type LogAccess =
	| { readonly kind: "active" }
	| {
			readonly kind: "frozen"
			readonly operation: string
			readonly intent:
				| { readonly kind: "erasure" }
				| { readonly kind: "migration"; readonly planSetDigest: Uint8Array; readonly target: string }
	  }

type LogLifecycle =
	| {
			readonly kind: "live"
			readonly access: LogAccess
			readonly decision: LogDecisionStamp
			readonly state: LogStateStamp
			readonly receipts: { readonly openEpoch: bigint; readonly retiredThrough: bigint }
	  }
	| {
			readonly kind: "deleted"
			readonly operation: string
			readonly reason:
				| { readonly kind: "erasure" }
				| {
						readonly kind: "migrationAborted"
						readonly sourceDatabase: string
						readonly sourceIncarnation: string
						readonly planSetDigest: Uint8Array
				  }
	  }

type LogActivation =
	| { readonly kind: "notActivated" }
	| {
			readonly kind: "activated"
			readonly operation: string
			readonly targetGenesis: Uint8Array
			readonly cause:
				| { readonly kind: "create" }
				| { readonly kind: "restore" }
				| { readonly kind: "migration"; readonly planSetDigest: Uint8Array }
	  }

interface LogAuthority {
	readonly identity: LogIdentity
	readonly revision: bigint
	readonly lifecycle: LogLifecycle
	readonly activation: LogActivation
}

type LogGenesisProvenance =
	| { readonly kind: "create" }
	| { readonly kind: "restore"; readonly sourceEvidence: Uint8Array }
	| {
			readonly kind: "migration"
			readonly sourceDatabase: string
			readonly sourceIncarnation: string
			readonly planSetDigest: Uint8Array
	  }

interface LogGenesis {
	readonly identity: LogIdentity
	readonly initialApplicationDigest: Uint8Array
	readonly initialSystemDigest: Uint8Array
	readonly provenance: LogGenesisProvenance
}

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

interface ManifestStatement {
	readonly id: number
	readonly kind: StatementKindTag
	readonly spelling: string
}

interface Manifest {
	readonly relations: readonly ManifestRelation[]
	readonly statements: readonly ManifestStatement[]
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
// surface — a JS callback inside a native transaction — is deleted (P06),
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
	| "derivedBudgetExceeded"
	| "overflow"
	| "scalar"
	| "resultBytesOverflow"
	| "corruption"

type AdmissionTag = "accepted" | "rejected"
type WriteTag = "accepted" | "rejected" | "abandoned" | "moved"
type OpenKind = "schemaError" | "newtypeMismatch" | "fingerprintMismatch" | "destinationExists"
type PrepareKind = "irError"

interface Native {
	engineVersion(): string

	/** Small identity-sized inputs only; bulk hashing rides `runtimeHash`. */
	blake3Hash(data: Uint8Array): Uint8Array

	descriptor(spec: SchemaSpec): SealedDescriptor

	dbManifest(db: DbHandle): Manifest

	dbFingerprint(db: DbHandle): string

	dbGeneration(db: DbHandle): bigint

	/**
	 * blake3 over the canonical catalog enumeration — the replication
	 * equality oracle: equal digests imply identical judged content
	 * regardless of page layout or allocation history.
	 */
	dbCatalogDigest(db: DbHandle): Uint8Array

	witnessClose(witness: WitnessHandle): void

	instanceBuilderNew(spec: SchemaSpec): BuilderHandle

	/**
	 * Records a collection of inserts into the draft; returns the engine
	 * `{ submitted, changed }` report. `cells` is ONE flat row-major array
	 * (length rows×arity) in sealed field order, and `rows` is the EXPLICIT
	 * row count the caller states; the bridge verifies
	 * `cells.length === rows × arity` exactly against its resident sealed
	 * roster before building the engine's shape-proved collection.
	 */
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
	instanceBuilderContains(builder: BuilderHandle, relationId: number, values: readonly FactValue[]): boolean
	instanceBuilderGet(
		builder: BuilderHandle,
		relationId: number,
		keyStatementId: number,
		keyValues: readonly FactValue[]
	): FactValue[] | null
	instanceBuilderClose(builder: BuilderHandle): void
	// Admission moved onto the one executor: `runtimeBuilderAdmit` →
	// `runtimeAdmitTake` in #runtime-native.ts. No AsyncTask/Promise verb.

	ownedInstanceClose(instance: OwnedHandle): void
	// Owned-instance point reads are bounded owned-heap work and stay
	// synchronous; scans and queries ride the executor
	// (`runtimeOwnedScan`/`runtimeOwnedQuery` in #runtime-native.ts).
	ownedCount(instance: OwnedHandle, relationId: number): bigint
	ownedContains(instance: OwnedHandle, relationId: number, values: readonly FactValue[]): boolean
	ownedGet(
		instance: OwnedHandle,
		relationId: number,
		keyStatementId: number,
		keyValues: readonly FactValue[]
	): FactValue[] | null

	// --- successor log grammar (small frames; command/decision lanes are
	// executor verbs in #runtime-native.ts) ---

	logIdentities(): string

	logSchema(spec: SchemaSpec): LogSchemaHandle

	logSchemaFingerprint(schema: LogSchemaHandle): string

	logReceiptKey(id: LogCommandId): Uint8Array

	logReceiptEncode(receipt: LogReceipt, limits: LogLimits): LogResult<Uint8Array, LogReceiptKind>

	logReceiptDecode(
		expected: { readonly identity: LogIdentity; readonly receiptEpoch: bigint; readonly requestId: string },
		bytes: Uint8Array,
		limits: LogLimits
	): LogResult<LogReceipt, LogReceiptKind>

	logReceiptDecodeAt(id: LogCommandId, bytes: Uint8Array, limits: LogLimits): LogResult<LogReceipt, LogReceiptKind>

	logControlEncode(authority: LogAuthority, cap: bigint): LogResult<Uint8Array, LogFrameKind>

	logControlDecode(bytes: Uint8Array, cap: bigint): LogResult<LogAuthority, LogFrameKind>

	logGenesisEncode(record: LogGenesis, cap: bigint): LogResult<Uint8Array, LogFrameKind>

	logGenesisDecode(bytes: Uint8Array, cap: bigint): LogResult<LogGenesis, LogFrameKind>

	logGenesisStamp(record: LogGenesis, cap: bigint): LogResult<LogDecisionStamp, LogFrameKind>

	logBlankDigests(): { readonly application: Uint8Array; readonly system: Uint8Array }
}

const SHIPPED_PLATFORMS = ["darwin-arm64", "linux-arm64", "linux-x64"] as const

const requireNative = createRequire(import.meta.url)

type NativeBinding = Native

function loadNativeBinding(platform: string, arch: string): NativeBinding {
	const platformPackage = `@bjornpagen/bumbledb-${platform}-${arch}`

	const present = Result.try(() => requireNative.resolve(`${platformPackage}/package.json`))
	if (Result.isFailure(present)) {
		throw new NativeLoadError({
			package: platformPackage,
			operation: "resolve",
			message: `no native binary for ${platform}-${arch}: @bjornpagen/bumbledb ships ${SHIPPED_PLATFORMS.join(", ")} only`,
			cause: present.failure
		})
	}

	const loaded = Result.try(() => requireNative(platformPackage))
	if (Result.isFailure(loaded)) {
		throw new NativeLoadError({
			package: platformPackage,
			operation: "load",
			message: `load the ${platformPackage} native binary (package present but unloadable)`,
			cause: loaded.failure
		})
	}
	return loaded.success
}

const binding: NativeBinding = loadNativeBinding(process.platform, process.arch)
const native: Native = binding

/**
 * @internal blake3 of the given bytes via the resident native binding —
 * the engine's own hash, lent to the replication driver
 * (`@bjornpagen/bumbledb-log`). Not SDK API; the export is deliberately
 * undocumented in the package surface. Small identity-sized inputs only.
 */
function internalBlake3(data: Uint8Array): Uint8Array {
	return bridged("bumbledb blake3", function hashBytes() {
		return binding.blake3Hash(data)
	})
}

/**
 * @internal the engine's own sealed descriptor as data — relation ids,
 * field ids and types in sealed order, closed rosters with resolved
 * axiom rows, materialized statements in engine order, and the real
 * fingerprint. Runs the pure seal path; no store opens. Not SDK API;
 * the export is deliberately undocumented in the package surface.
 */
function internalDescriptor(spec: SchemaSpec): SealedDescriptor {
	return bridged("bumbledb descriptor", function sealSpec() {
		return binding.descriptor(spec)
	})
}

/**
 * @internal the successor identity table emitted by the log core's one
 * speller (`bumbledb_log::identities::emit`). Not SDK API.
 */
function internalLogIdentities(): string {
	return bridged("bumbledb-log identities", function emitIdentities() {
		return binding.logIdentities()
	})
}

/**
 * @internal the sealed per-theory log schema off the same `SchemaSpec`
 * every other lane speaks — one validated core schema, lent to the
 * replication driver (`@bjornpagen/bumbledb-log`). Not SDK API.
 */
function internalLogSchema(spec: SchemaSpec): LogSchemaHandle {
	return bridged("bumbledb-log schema", function sealSchema() {
		return binding.logSchema(spec)
	})
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
	BuilderHandle,
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
	IntervalValue,
	LogAccess,
	LogActivation,
	LogAuthority,
	LogChainKind,
	LogCommandId,
	LogCommandKind,
	LogCommandMetadata,
	LogCommandRef,
	LogCondition,
	LogDecisionStamp,
	LogFrameKind,
	LogGenesis,
	LogGenesisProvenance,
	LogIdentity,
	LogLifecycle,
	LogLimits,
	LogOutcomeWire,
	LogReceipt,
	LogReceiptKind,
	LogResult,
	LogSchemaHandle,
	LogStateStamp,
	Manifest,
	NumericCastIr,
	OpenKind,
	OwnedHandle,
	ParsedQuery,
	PrepareKind,
	QueryIr,
	QueryParam,
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
	WireMutationReport,
	WitnessHandle,
	WriteTag
}
export {
	bridged,
	errorFromThrow,
	internalBlake3,
	internalDescriptor,
	internalLogIdentities,
	internalLogSchema,
	loadNativeBinding,
	native,
	SHIPPED_PLATFORMS
}
