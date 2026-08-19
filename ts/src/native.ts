import { createRequire } from "node:module"
import * as errors from "@superbuilders/errors"
import type { SchemaSpec, ValueSpec, ValueTypeSpec } from "#spec.ts"

/**
 * The complete typed surface of the bumbledb-node napi bridge. ALL FFI
 * typing lives in this one file — no other module may know the `.node`
 * artifact exists. The bridge is a dumb bridge (PRD-04): descriptor in as
 * data, queries in as IR data, facts in/out as value rows, rejections out as
 * structured violation sets; anything smart lives in the SDK or the engine.
 *
 * Marshaling law: fact rows cross as NATURAL JS values, schema-directed by
 * the engine descriptor (`boolean ⇄ bool`, `bigint ⇄ u64/i64`,
 * `string ⇄ str`, `Uint8Array ⇄ bytes<N>`, `{ start, end }` ⇄ interval);
 * IR, spec, and query params cross as TAGGED plain objects mirroring the
 * engine's own data enums 1:1. Every u64/i64 crosses as `bigint`, never
 * `number`. Domain outcomes (schema errors, fingerprint mismatches,
 * rejections, generation moves, IR errors) are DATA; marshaling/shape
 * violations and use-after-close THROW.
 */

/** The opaque database handle (owns the LMDB environment + exclusive lock). */
type DbHandle = { readonly __brand: "bumbledb.db" }

/** One live borrowed instance valid only inside a read callback. */
type InstanceHandle = { readonly __brand: "bumbledb.instance" }

/** One cloneable generation witness. May outlive the read that minted it. */
type WitnessHandle = { readonly __brand: "bumbledb.witness" }

/** One unproved heap builder. Spent by `instanceBuilderAdmit` / close. */
type BuilderHandle = { readonly __brand: "bumbledb.builder" }

/** One admitted heap instance. */
type OwnedHandle = { readonly __brand: "bumbledb.owned" }

/**
 * One exhumed store — the read-only, theory-less open (engine 70-api.md
 * § exhume). Lifetimes are disposables (ruled 2026-07-23, R12):
 * `exhumeClose` is the deterministic teardown the SDK's `Symbol.dispose`
 * rides — releasing the environment (and the store's exclusive lock)
 * scope-shaped, never a GC race; the engine-side drop remains the
 * reclamation-only backstop for a collected-but-undisposed handle.
 */
type ExhumeHandle = { readonly __brand: "bumbledb.exhume" }

/**
 * One live write transaction — the submitted delta with the engine's
 * final-state point-read view. Valid only inside a write callback.
 */
type TxHandle = { readonly __brand: "bumbledb.tx" }

/** One prepared query (plan pinned at prepare). */
type PreparedHandle = { readonly __brand: "bumbledb.prepared" }

/**
 * Engine mutation report as it crosses napi: both counts are engine
 * values, never reconstructed from JS length.
 */
interface WireMutationReport {
	readonly submitted: bigint
	readonly changed: bigint
}

/**
 * Engine fresh-id range as it crosses napi. Empty cannot yield a start —
 * `start` is a minted id only on the nonempty arm. (C wires empty as
 * `BDB_FRESH_RANGE_TAG_EMPTY`. The JS wire is `{ empty: true }`, not that
 * C sentinel.)
 */
type WireFreshRange =
	| { readonly empty: true }
	| { readonly empty: false; readonly start: bigint; readonly endExclusive: bigint }

/** A half-open interval `[start, end)` as it crosses the boundary. */
interface IntervalValue {
	readonly start: bigint
	readonly end: bigint
}

/**
 * One fact-row cell as a natural JS value. The expected engine type comes
 * from the schema descriptor (marshaling is schema-directed, never guessed):
 * `boolean` for bool, `bigint` for u64/i64, `string` for str, `Uint8Array`
 * for bytes<N> (width-checked), `{ start, end }` for intervals.
 */
type FactValue = boolean | bigint | string | Uint8Array | IntervalValue

/**
 * One tagged engine value — the 1:1 mirror of `bumbledb::Value` for the
 * positions no schema field directs (IR literals, query params).
 */
type TaggedValue = ValueSpec

/**
 * One positional execution argument: a tagged scalar, or a param SET
 * (`Term.paramSet` positions) as `{ kind: "set", values }`.
 */
type QueryParam = TaggedValue | { readonly kind: "set"; readonly values: readonly TaggedValue[] }

/**
 * The IR mirror (`bumbledb::ir`, 1:1): relations, fields, interiors, and
 * params by NUMERIC id — the SDK resolves names through the manifest and
 * sends ids; the bridge never sees names in queries. Q1 is a tagged sum:
 * CQ carries no rec; Reach carries `rec` by value.
 */
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

/** One named interior: the head shape its rules align against, and the rules. */
interface InteriorIr {
	readonly head: readonly HeadTermIr[]
	readonly rules: readonly RuleIr[]
}

/** The linear rec on a Reach query: shared head, base arms, rec arms. */
interface RecIr {
	readonly head: readonly HeadTermIr[]
	readonly base: readonly RuleIr[]
	readonly rec: readonly RuleIr[]
}

/** One head position: a plain variable slot or an aggregate-op kind. */
type HeadTermIr = { readonly kind: "var" } | { readonly kind: "aggregate"; readonly op: HeadOpIr }

/** The var-free aggregate-op kind at a head position. */
type HeadOpIr = "sum" | "min" | "max" | "count" | "pack"

/** One rule: conjunctive body, anti-join atoms, condition trees. */
interface RuleIr {
	readonly finds: readonly FindTermIr[]
	readonly atoms: readonly AtomIr[]
	readonly negated: readonly AtomIr[]
	readonly conditions: readonly ConditionTreeIr[]
}

/** One find term (mirrors `ir::FindTerm`). Count is nullary; pack and folds carry `over`. */
type FoldOpIr = { readonly kind: "sum" } | { readonly kind: "min" } | { readonly kind: "max" }

type FindTermIr =
	| { readonly kind: "var"; readonly var: number }
	| { readonly kind: "count" }
	| { readonly kind: "aggregate"; readonly op: FoldOpIr; readonly over: number }
	| { readonly kind: "pack"; readonly over: number }
	| { readonly kind: "measure"; readonly var: number }
	| { readonly kind: "aggregateMeasure"; readonly op: FoldOpIr; readonly over: number }

/** Host brand: only {@link parseQueryIr} and `lowerQuery` inhabit this. Phantom — not a runtime key. */
declare const parsedQueryBrand: unique symbol

/** A `QueryIr` that passed the host shape parse (rec/main nonempty, aggregate finds split). */
type ParsedQuery = QueryIr & { readonly [parsedQueryBrand]: true }

/** One aggregate operator (mirrors `ir::AggOp`; Arg ops carry their key). */
type AggOpIr =
	| { readonly kind: "sum" }
	| { readonly kind: "min" }
	| { readonly kind: "max" }
	| { readonly kind: "count" }
	| { readonly kind: "pack" }

/** Where an atom draws its facts: a stored relation or a derived table. */
type AtomSourceIr =
	| { readonly kind: "edb"; readonly relation: number }
	| { readonly kind: "interior"; readonly interior: number }

/**
 * One atom: named-field bindings as `[fieldId, term]` pairs; absence of a
 * field is the wildcard.
 */
interface AtomIr {
	readonly source: AtomSourceIr
	readonly bindings: ReadonlyArray<readonly [number, TermIr]>
}

/** One term of an atom binding or comparison (mirrors `ir::Term`). */
type TermIr =
	| { readonly kind: "var"; readonly var: number }
	| { readonly kind: "param"; readonly param: number }
	| { readonly kind: "paramSet"; readonly param: number }
	| { readonly kind: "literal"; readonly value: TaggedValue }
	| { readonly kind: "measure"; readonly var: number }

/** One comparison operator (mirrors `ir::CmpOp`). */
type CmpOpIr =
	| { readonly kind: "eq" }
	| { readonly kind: "ne" }
	| { readonly kind: "lt" }
	| { readonly kind: "le" }
	| { readonly kind: "gt" }
	| { readonly kind: "ge" }
	| { readonly kind: "allen"; readonly mask: number }
	| { readonly kind: "pointIn" }

/** One comparison condition. */
interface ComparisonIr {
	readonly op: CmpOpIr
	readonly lhs: TermIr
	readonly rhs: TermIr
}

/**
 * The input condition grammar: any boolean combination of comparisons
 * (validation distributes to DNF engine-side).
 */
type ConditionTreeIr =
	| { readonly kind: "leaf"; readonly cmp: ComparisonIr }
	| { readonly kind: "and"; readonly children: readonly ConditionTreeIr[] }
	| { readonly kind: "or"; readonly children: readonly ConditionTreeIr[] }

/** A statement's form tag. */
type StatementKindTag = "functionality" | "containment" | "capacity"

/** One field's name, dense id, and structural type. */
interface ManifestField {
	readonly name: string
	readonly id: number
	readonly valueType: ValueTypeSpec
}

/**
 * One closed-relation ground axiom as manifest data: handle →
 * declaration-order id → (column, value) pairs.
 */
interface ManifestRow {
	readonly handle: string
	readonly id: bigint
	readonly values: ReadonlyArray<{ readonly name: string; readonly value: FactValue }>
}

/**
 * One relation's names and ids; a closed relation's sealed field list opens
 * with the synthetic (`id`, u64) handle field and carries its extension.
 */
interface ManifestRelation {
	readonly name: string
	readonly id: number
	readonly fields: readonly ManifestField[]
	readonly extension?: readonly ManifestRow[]
}

/** One statement's identity, form tag, and canonical spelling. */
interface ManifestStatement {
	readonly id: number
	readonly kind: StatementKindTag
	readonly spelling: string
}

/**
 * The theory's manifest: every name → id pairing as plain data (PRD-02's
 * tables, one JS object) — called once per open by the SDK.
 */
interface Manifest {
	readonly relations: readonly ManifestRelation[]
	readonly statements: readonly ManifestStatement[]
}

/** One offending fact of a violation, decoded to named natural values. */
interface ViolationFact {
	readonly relation: string
	readonly fields: ReadonlyArray<{ readonly name: string; readonly value: FactValue }>
}

/**
 * One violated statement of a rejected commit, rendered to plain data: the
 * statement id (materialized order), form tag, CANONICAL spelling (the
 * engine's one renderer — a bijection on legal statements, paste-back-able),
 * the form's direction/measure payloads, and the decoded offending facts.
 * `measure` is the capacity form's witnessed group total — the engine
 * accumulates in u128 and the value crosses WHOLE as bigint (C3:
 * truncation is unrepresentable).
 */
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

/**
 * `dbCreate`'s domain outcome. Admission is `accepted` / `rejected`.
 * Declaration-boundary refusals ride as their own tags (not theory
 * admission) and the SDK throws them.
 */
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

/**
 * `dbExhume`'s domain outcome: the live exhume handle, or one of the three
 * typed refusals as data — `descriptorMissing` (a format-8 store whose
 * descriptor key is absent; open never back-fills), `formatMismatch`
 * (including every format-7 store), and `corruption` (the persisted
 * descriptor fails its integrity gates). Genuine failures — a missing
 * path, a held exclusive lock — throw.
 */
type ExhumeResult =
	| { readonly ok: true; readonly exhume: ExhumeHandle }
	| {
			readonly ok: false
			readonly kind: "descriptorMissing" | "formatMismatch" | "corruption"
			readonly message: string
	  }

/**
 * `dbWrite` / `dbWriteFrom` native outcome. The SDK attaches the callback
 * return onto the accepted arm. Moved is data, never an error kind.
 */
type NativeWriteOutcome =
	| { readonly tag: "accepted"; readonly generation: bigint }
	| { readonly tag: "rejected"; readonly violations: readonly Violation[] }
	| { readonly tag: "abandoned" }
	| { readonly tag: "moved"; readonly witnessed: bigint; readonly current: bigint }

/**
 * Builder `admit` native outcome.
 */
type AdmitResult =
	| { readonly tag: "accepted"; readonly value: OwnedHandle }
	| { readonly tag: "rejected"; readonly violations: readonly Violation[] }

/** `dbPrepare`/`instancePrepare`'s domain outcome (IR roster errors are data). */
type PrepareResult =
	| { readonly ok: true; readonly prepared: PreparedHandle }
	| { readonly ok: false; readonly kind: "irError"; readonly message: string }

/** One occurrence's plan drift (pinned vs live row counts). */
interface OccurrenceDrift {
	readonly relation: number
	readonly pinned: bigint
	readonly live: bigint
	readonly ratio: number
}

/**
 * The pull-based plan-drift report: engine-policy-free — no threshold
 * exists engine-side; the host owns reprepare policy.
 */
interface Staleness {
	readonly perOccurrence: readonly OccurrenceDrift[]
	readonly maxRatio: number
}

type ErrorFamilyKind =
	| "formatMismatch"
	| "schemaMismatch"
	| "alreadyInitialized"
	| "destinationExists"
	| "publishedButUnsynced"
	| "environmentLocked"
	| "storeKindMismatch"
	| "descriptorMissing"
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
	| "measureOfRay"
	| "capacityRayMeasure"
	| "derivedBudgetExceeded"
	| "overflow"
	| "resultBytesOverflow"
	| "corruption"

type AdmissionTag = "accepted" | "rejected"
type WriteTag = "accepted" | "rejected" | "abandoned" | "moved"
type OpenKind = "schemaError" | "newtypeMismatch" | "fingerprintMismatch"
type ExhumeKind = "descriptorMissing" | "formatMismatch" | "corruption"
type PrepareKind = "irError"

/**
 * The plan-as-data report (ruled 2026-07-23, R13): the engine's
 * `ExecutionStats` rendered to plain objects — camelCase keys, u64
 * counters as `bigint`. A diagnostic surface, EXPLICITLY UNFROZEN: the
 * shape follows the plan representation wherever it goes and no
 * compatibility claim attaches, so this typing names the stable spine
 * (version, emits, the plan sections) and leaves each section's leaves
 * open for the host to introspect.
 */
interface Explain {
	readonly introspectionVersion: number
	readonly emits: bigint
	readonly disjointRules?: Readonly<Record<string, unknown>>
	readonly subsumed: ReadonlyArray<Readonly<Record<string, unknown>>>
	readonly dead: ReadonlyArray<Readonly<Record<string, unknown>>>
	readonly rules: ReadonlyArray<Readonly<Record<string, unknown>>>
	readonly interiors: ReadonlyArray<Readonly<Record<string, unknown>>>
	readonly reach?: Readonly<Record<string, unknown>>
}

interface Native {
	/**
	 * Proof-of-life export (PRD-03): a non-empty string naming the bridge
	 * crate version and the engine's storage format version — evidence the
	 * cargo path dependency compiled, linked, and loaded through Node-API.
	 */
	engineVersion(): string

	/**
	 * Creates a fresh DURABLE store at `path` (frozen ruling 3: no ephemeral
	 * kind crosses this bridge). Refuses an already-initialized directory
	 * (throws); schema failures return as data.
	 */
	dbCreate(path: string, spec: SchemaSpec): Promise<CreateResult>
	/**
	 * Opens an existing durable store, verifying format version, store
	 * kind, and schema fingerprint (`fingerprintMismatch` as data).
	 */
	dbOpen(path: string, spec: SchemaSpec): Promise<DbOpenResult>
	/**
	 * Closes the handle. Dependent handles each hold the engine alive; the
	 * environment (and its exclusive lock) releases when the last closes.
	 */
	dbClose(db: DbHandle): void
	/** The PRD-02 manifest — every name → id table, one plain object. */
	dbManifest(db: DbHandle): Manifest
	/**
	 * The open store's schema fingerprint, 64 lowercase hex chars — the
	 * cross-host identity readback (`dbCreate` stored this exact value,
	 * `dbOpen` verified it). The engine computes; the bridge hex-encodes.
	 * Test-facing (the cross-host fingerprint lock); the SDK surface stays
	 * bijective with the Rust surface, which exposes no fingerprint
	 * accessor on `Db` — so no `Db` method wraps this.
	 */
	dbFingerprint(db: DbHandle): string
	/**
	 * The current committed generation — diagnostics only. The write-side
	 * witness is always a {@link WitnessHandle}, never this integer.
	 */
	dbGeneration(db: DbHandle): bigint
	/** Publishes an admitted heap instance at `path` without re-judgment. */
	dbFromInstance(path: string, instance: OwnedHandle): Promise<DbHandle>

	/**
	 * Opens a store FROM ITS OWN PERSISTED DESCRIPTOR (the read-only,
	 * theory-less open; engine 70-api.md § exhume) — no schema crosses in.
	 * The three typed refusals return as data ({@link ExhumeResult});
	 * genuine failures throw. The handle's deterministic teardown is
	 * `exhumeClose` (R12); GC reclamation remains the backstop only.
	 */
	dbExhume(path: string): Promise<ExhumeResult>
	/**
	 * Closes the exhume handle, releasing its environment (and the store's
	 * exclusive lock) deterministically — the native teardown under the
	 * SDK's `Symbol.dispose` (ruled 2026-07-23, R12: lifetimes are
	 * disposables, never `close()` methods to remember).
	 */
	exhumeClose(exhume: ExhumeHandle): void
	/**
	 * The exhumed store's persisted schema as manifest-shaped data — the
	 * engine's own manifest rendering of the STORED descriptor: relations
	 * in engine-id order, sealed field lists (a closed relation opens with
	 * the synthetic (`id`, u64) handle field) with structural value types,
	 * and closed-relation rosters.
	 */
	exhumeDescriptor(exhume: ExhumeHandle): Manifest
	/**
	 * Full-relation export by NAME in row-id order, values marshaled per
	 * the STORED descriptor (str already resolved through `_dict` inside
	 * the engine; a closed relation scans its sealed roster). Each call is
	 * one self-contained snapshot read; an unknown relation name throws.
	 */
	exhumeScan(exhume: ExhumeHandle, relationName: string): FactValue[][]

	/**
	 * Runs `callback` synchronously inside the engine read lease. The
	 * instance handle is invalid after the callback returns; the witness
	 * handle is a clone and may escape.
	 */
	dbRead<R>(db: DbHandle, callback: (instance: InstanceHandle, witness: WitnessHandle) => R): R
	instanceGeneration(instance: InstanceHandle): bigint
	instanceScan(instance: InstanceHandle, relationId: number): FactValue[][]
	instanceContains(instance: InstanceHandle, relationId: number, values: readonly FactValue[]): boolean
	instanceGet(
		instance: InstanceHandle,
		relationId: number,
		keyStatementId: number,
		keyValues: readonly FactValue[]
	): FactValue[] | null
	instancePrepare(instance: InstanceHandle, query: ParsedQuery): PrepareResult
	witnessClose(witness: WitnessHandle): void

	/**
	 * Runs `callback` synchronously inside the engine write region.
	 * Return `true` to commit, `false` to abandon. Nested writes throw.
	 */
	dbWrite(db: DbHandle, callback: (tx: TxHandle) => boolean): NativeWriteOutcome
	/**
	 * Witnessed write: `moved` is data when the store advanced since the
	 * witness was minted. The callback does not run on that arm.
	 */
	dbWriteFrom(db: DbHandle, witness: WitnessHandle, callback: (tx: TxHandle) => boolean): NativeWriteOutcome
	/**
	 * Records a collection of inserts into the delta; returns the engine
	 * `{ submitted, changed }` report. `rows` is an array of value-arrays
	 * in sealed field order. Empty is lawful and still a mutation (poison
	 * is observed). Nothing is judged until commit; shape violations throw typed.
	 */
	txInsert(tx: TxHandle, relationId: number, rows: readonly (readonly FactValue[])[]): WireMutationReport
	/** Records a collection of deletes; returns the engine `{ submitted, changed }` report. */
	txDelete(tx: TxHandle, relationId: number, rows: readonly (readonly FactValue[])[]): WireMutationReport
	/**
	 * Final-state membership (base + pending delta — the exact view the
	 * commit judgment judges; check-then-act is race-free by construction).
	 */
	txContains(tx: TxHandle, relationId: number, values: readonly FactValue[]): boolean
	/** Final-state point lookup through a key statement; `null` on a miss. */
	txGet(tx: TxHandle, relationId: number, keyStatementId: number, keyValues: readonly FactValue[]): FactValue[] | null
	/**
	 * Mints `count` consecutive fresh values for `(relationId, fieldId)`.
	 * `count === 0n` is empty and does not yield a start.
	 */
	txReserve(tx: TxHandle, relationId: number, fieldId: number, count: bigint): WireFreshRange

	/**
	 * Prepares a query (IR as data, ids only; plan pinned at prepare).
	 * Roster errors return as data.
	 */
	dbPrepare(db: DbHandle, query: ParsedQuery): PrepareResult
	/**
	 * Executes against a live instance with positional params. One-copy owned
	 * rows out, column order = the query's head order; answers are a set
	 * — the host sorts.
	 */
	preparedExecute(prepared: PreparedHandle, instance: InstanceHandle, params: readonly QueryParam[]): FactValue[][]
	/**
	 * Plan introspection as data (ruled 2026-07-23, R13): runs the prepared
	 * query against a store read with counting instrumentation and returns
	 * the structured stats. Store-read only.
	 */
	preparedExplain(prepared: PreparedHandle, instance: InstanceHandle, params: readonly QueryParam[]): Explain
	/** The pull-based plan-drift signal against a store read. */
	preparedStaleness(prepared: PreparedHandle, instance: InstanceHandle): Staleness
	/** Releases the prepared query. */
	preparedClose(prepared: PreparedHandle): void

	instanceBuilderNew(spec: SchemaSpec): BuilderHandle
	instanceBuilderLoad(
		builder: BuilderHandle,
		relationId: number,
		rows: readonly (readonly FactValue[])[]
	): WireMutationReport
	instanceBuilderClose(builder: BuilderHandle): void
	instanceBuilderAdmit(builder: BuilderHandle): Promise<AdmitResult>
	ownedInstanceClose(instance: OwnedHandle): void
	ownedRead<R>(instance: OwnedHandle, callback: (instance: InstanceHandle) => R): R
}

/**
 * The sole platform this release ships (PRD-03 ruling 1: prebuilt-only,
 * darwin-arm64). The per-platform-package structure below makes adding
 * `darwin-x64`/`linux-*`/`win32-*` pure addition — one more `os`/`cpu`-gated
 * package plus a CI matrix — never a redesign. This constant names the
 * shipped set for the unsupported-platform message; the build's
 * `PUBLISH_PLATFORM` (`scripts/platform.ts` — src cannot import scripts,
 * the packaging boundary) and the `ts/.gitignore` carve-out spell the same
 * target, and the single-source pin in `test/build-platform.test.ts` holds
 * all three in lockstep.
 */
const SHIPPED_PLATFORMS = "darwin-arm64"

/**
 * CommonJS require anchored to this module, the only mechanism ESM has for
 * loading a Node-API addon without an experimental flag (static `import` of
 * `.node` files still sits behind `--experimental-addon-modules` on Node 24).
 * It resolves the per-platform binary package by name (see
 * {@link loadNativeBinding}); the addon never crosses as a relative path.
 * createRequire is the only unflagged Node-API addon loader in ESM, and this
 * file is the package's single sanctioned FFI boundary (the arch-split
 * packaging ruling).
 */
const requireNative = createRequire(import.meta.url)

/**
 * Resolves and loads the native bridge from its per-platform binary package
 * (`@bjornpagen/bumbledb-<platform>-<arch>`) — the Biome/esbuild/napi-rs
 * pattern. npm/pnpm install ONLY the `optionalDependency` whose `os`/`cpu`
 * match the host, so a matching host resolves the addon and every other host
 * resolves nothing. The two failure modes are distinct and both typed:
 *
 *   - the platform package is ABSENT (the expected state on any
 *     non-darwin-arm64 host, and on a foreign `platform`/`arch` passed under
 *     test) — an actionable unsupported-platform error naming the running
 *     `platform-arch` and the shipped set;
 *   - the platform package is PRESENT but its `bumbledb.node` will not load
 *     (a genuine ABI/corruption fault) — the wrapped loader error.
 *
 * Parameterized on `platform`/`arch` so the resolution law is exercised for
 * foreign hosts as a unit, without spawning a foreign process.
 */
function loadNativeBinding(platform: string, arch: string): Native {
	const platformPackage = `@bjornpagen/bumbledb-${platform}-${arch}`

	// Presence probe: the platform package's OWN manifest resolves iff the
	// matching optional dependency was installed. Its absence is the
	// expected, benign "unsupported platform" — never a corruption signal.
	const present = errors.trySync(() => requireNative.resolve(`${platformPackage}/package.json`))
	if (present.error) {
		throw errors.wrap(
			present.error,
			`no native binary for ${platform}-${arch}: @bjornpagen/bumbledb ships ${SHIPPED_PLATFORMS} only`
		)
	}

	// The package is present; load its addon (its `main` is `bumbledb.node`).
	// A failure HERE is corruption or an ABI mismatch, not an absent platform.
	const loaded = errors.trySync(() => requireNative(platformPackage))
	if (loaded.error) {
		throw errors.wrap(loaded.error, `load the ${platformPackage} native binary (package present but unloadable)`)
	}
	return loaded.data
}

/**
 * The loaded bumbledb-node bridge for the running host. Import this object
 * for every native call; the resolve-and-load happens once at module
 * initialization and an absent or unloadable artifact fails fast here rather
 * than at first use.
 */
const native: Native = loadNativeBinding(process.platform, process.arch)

/**
 * Engine throw identity: a real `Error` carrying `kind` from the
 * `ErrorFamily` table, or a leftover `{ kind, message }` object.
 */
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

/**
 * The bridge guard — THE one wrapper every native call crosses (db.ts and
 * exhume.ts both import it): runs one native call and wraps anything it
 * throws, so marshal-shape refusals and handle-lifecycle refusals cross as
 * genuine typed failures, never bare foreign errors. Engine throws keep
 * their forced kind.
 */
function bridged<T>(context: string, run: () => T): T {
	try {
		return run()
	} catch (caught) {
		throw errors.wrap(errorFromThrow(caught), context)
	}
}

/**
 * The async twin of {@link bridged}: every control-plane native is an
 * `AsyncTask` Promise, and this is the one wrapper those awaits cross.
 */
async function bridgedAsync<T>(context: string, run: () => Promise<T>): Promise<T> {
	try {
		return await run()
	} catch (caught) {
		throw errors.wrap(errorFromThrow(caught), context)
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
	ExhumeHandle,
	ExhumeKind,
	ExhumeResult,
	Explain,
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
	OccurrenceDrift,
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
	Staleness,
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
export { bridged, bridgedAsync, errorFromThrow, loadNativeBinding, native, SHIPPED_PLATFORMS }
