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

/** One live MVCC read snapshot. */
type SnapshotHandle = { readonly __brand: "bumbledb.snapshot" }

/**
 * One exhumed store — the read-only, theory-less open (engine 70-api.md
 * § exhume). No close function exists for this handle anywhere on the
 * bridge (the zero-closables law): the engine side is dropped when the
 * handle is garbage-collected — reclamation only, never correctness.
 */
type ExhumeHandle = { readonly __brand: "bumbledb.exhume" }

/**
 * One live write transaction — the submitted delta with the engine's
 * final-state point-read view. Spent by `txCommit`/`txAbort`.
 */
type TxHandle = { readonly __brand: "bumbledb.tx" }

/** One prepared query/program (plan pinned at prepare). */
type PreparedHandle = { readonly __brand: "bumbledb.prepared" }

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
 * positions no schema field directs (IR literals, query params). The spec's
 * `ValueSpec` vocabulary plus the bind-time-only Allen mask.
 */
type TaggedValue = ValueSpec | { readonly kind: "allenMask"; readonly mask: number }

/**
 * One positional execution argument: a tagged scalar, or a param SET
 * (`Term.paramSet` positions) as `{ kind: "set", values }`.
 */
type QueryParam = TaggedValue | { readonly kind: "set"; readonly values: readonly TaggedValue[] }

/**
 * The IR mirror (`bumbledb::ir`, 1:1): relations, fields, predicates, and
 * params by NUMERIC id — the SDK resolves names through the manifest and
 * sends ids; the bridge never sees names in queries. A plain query is sent
 * as its degenerate one-predicate program.
 */
interface ProgramIr {
	readonly predicates: readonly PredicateDefIr[]
	readonly output: number
}

/** One predicate: the head shape its rules align against, and the rules. */
interface PredicateDefIr {
	readonly head: readonly HeadTermIr[]
	readonly rules: readonly RuleIr[]
}

/** One head position: a plain variable slot or an aggregate-op kind. */
type HeadTermIr = { readonly kind: "var" } | { readonly kind: "aggregate"; readonly op: HeadOpIr }

/** The var-free aggregate-op kind at a head position. */
type HeadOpIr = "sum" | "min" | "max" | "count" | "countDistinct" | "argMax" | "argMin" | "pack"

/** One rule: conjunctive body, anti-join atoms, condition trees. */
interface RuleIr {
	readonly finds: readonly FindTermIr[]
	readonly atoms: readonly AtomIr[]
	readonly negated: readonly AtomIr[]
	readonly conditions: readonly ConditionTreeIr[]
}

/** One find term (mirrors `ir::FindTerm`). */
type FindTermIr =
	| { readonly kind: "var"; readonly var: number }
	| { readonly kind: "aggregate"; readonly op: AggOpIr; readonly over?: number }
	| { readonly kind: "measure"; readonly var: number }
	| { readonly kind: "aggregateMeasure"; readonly op: AggOpIr; readonly over: number }

/** One aggregate operator (mirrors `ir::AggOp`; Arg ops carry their key). */
type AggOpIr =
	| { readonly kind: "sum" }
	| { readonly kind: "min" }
	| { readonly kind: "max" }
	| { readonly kind: "count" }
	| { readonly kind: "countDistinct" }
	| { readonly kind: "argMax"; readonly key: number }
	| { readonly kind: "argMin"; readonly key: number }
	| { readonly kind: "pack" }

/** Where an atom draws its facts: a stored relation or a program predicate. */
type AtomSourceIr =
	| { readonly kind: "edb"; readonly relation: number }
	| { readonly kind: "idb"; readonly pred: number }

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

/** The `Allen` comparison's mask position: a literal mask or a param. */
type MaskTermIr =
	| { readonly kind: "literal"; readonly mask: number }
	| { readonly kind: "param"; readonly param: number }

/** One comparison operator (mirrors `ir::CmpOp`). */
type CmpOpIr =
	| { readonly kind: "eq" }
	| { readonly kind: "ne" }
	| { readonly kind: "lt" }
	| { readonly kind: "le" }
	| { readonly kind: "gt" }
	| { readonly kind: "ge" }
	| { readonly kind: "allen"; readonly mask: MaskTermIr }
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
type StatementKindTag = "functionality" | "containment" | "cardinality"

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
 * the form's direction/count payloads, and the decoded offending facts.
 */
interface Violation {
	readonly statementId: number
	readonly kind: StatementKindTag
	readonly canonical: string
	readonly direction?: "sourceUnsatisfied" | "targetRequired"
	readonly count?: bigint
	readonly facts: readonly ViolationFact[]
}

/**
 * `dbCreate`/`dbOpen`'s domain outcome. `schemaError` covers both spec
 * resolution (unresolvable names, banned spellings — every issue in one
 * message) and schema validation at the declaration boundary;
 * `newtypeMismatch` is the coherence wall's own kind — a spec whose
 * statement pairs faces with disagreeing newtype labels (the engine twin
 * of the schema-level class wall; unreachable through the typed builder,
 * which computes every label from the laws, so only a raw spec can reach
 * it); `fingerprintMismatch` is `dbOpen`'s stored-theory refusal.
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
 * adoption-era refusals as data — `descriptorMissing` (the store predates
 * self-describing stores and has not been adopted; the remedy is one
 * fingerprint-matching `dbOpen` under the creating schema),
 * `formatMismatch`, and `corruption` (the persisted descriptor fails its
 * integrity gates). Genuine failures — a missing path, a held exclusive
 * lock — throw.
 */
type ExhumeResult =
	| { readonly ok: true; readonly exhume: ExhumeHandle }
	| {
			readonly ok: false
			readonly kind: "descriptorMissing" | "formatMismatch" | "corruption"
			readonly message: string
	  }

/**
 * `dbWriteFrom`'s domain outcome: the live witnessed transaction, or the
 * typed stale-premise verdict (a state-changing commit landed after the
 * witness snapshot; retry policy is host-side).
 */
type WriteFromResult =
	| { readonly ok: true; readonly tx: TxHandle }
	| {
			readonly ok: false
			readonly kind: "generationMoved"
			readonly witnessed: bigint
			readonly current: bigint
	  }

/**
 * `txCommit`'s domain outcome: the committed generation, or the COMPLETE
 * violation set (every violated statement cited once, per direction for a
 * containment, in materialized statement order).
 */
type CommitResult =
	| { readonly ok: true; readonly generation: bigint }
	| { readonly ok: false; readonly violations: readonly Violation[] }

/** `dbPrepare`'s domain outcome (IR roster errors are data). */
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
	dbCreate(path: string, spec: SchemaSpec): DbOpenResult
	/**
	 * Opens an existing durable store, verifying format version, store
	 * kind, and schema fingerprint (`fingerprintMismatch` as data).
	 */
	dbOpen(path: string, spec: SchemaSpec): DbOpenResult
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
	 * witness is always the SNAPSHOT handle (`dbWriteFrom`), never this
	 * integer: an integer witness would be a claim a caller could fabricate
	 * or stale-cache (the engine's recorded refusal).
	 */
	dbGeneration(db: DbHandle): bigint

	/**
	 * Opens a store FROM ITS OWN PERSISTED DESCRIPTOR (the read-only,
	 * theory-less open; engine 70-api.md § exhume) — no schema crosses in.
	 * The three adoption-era refusals return as data ({@link ExhumeResult});
	 * genuine failures throw. The handle has no close anywhere on this
	 * bridge (zero closables): GC reclaims the engine side and releases the
	 * store's exclusive lock — reclamation only, never correctness.
	 */
	dbExhume(path: string): ExhumeResult
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

	/** Opens one MVCC read snapshot as a live handle. */
	dbSnapshot(db: DbHandle): SnapshotHandle
	/** Closes the snapshot, releasing its LMDB reader slot. */
	snapshotClose(snap: SnapshotHandle): void
	/** Full-relation export in row-id order (one row per fact). */
	snapshotScan(snap: SnapshotHandle, relationId: number): FactValue[][]
	/** Committed-state membership of one fact (sealed field order). */
	snapshotContains(snap: SnapshotHandle, relationId: number, values: readonly FactValue[]): boolean
	/**
	 * Committed-state point lookup through a key statement (`keyValues` in
	 * the statement's projection order); `null` on a miss.
	 */
	snapshotGet(
		snap: SnapshotHandle,
		relationId: number,
		keyStatementId: number,
		keyValues: readonly FactValue[]
	): FactValue[] | null

	/**
	 * Begins a write transaction: the submitted delta. One write
	 * transaction may be open per db handle at a time (single-writer
	 * engine; a second begin throws rather than deadlocking the process).
	 */
	dbWriteBegin(db: DbHandle): TxHandle
	/**
	 * Begins a WITNESSED write transaction: commits only if no
	 * state-changing commit landed since `snap` was taken —
	 * `generationMoved` as data otherwise (the optimistic
	 * read-compute-write loop's entry; retry policy stays host-side).
	 */
	dbWriteFrom(db: DbHandle, snap: SnapshotHandle): WriteFromResult
	/**
	 * Records an insert into the delta; `true` iff the final state changed.
	 * Nothing is judged until commit; shape violations throw typed.
	 */
	txInsert(tx: TxHandle, relationId: number, values: readonly FactValue[]): boolean
	/** Records a delete into the delta; `true` iff the final state changed. */
	txDelete(tx: TxHandle, relationId: number, values: readonly FactValue[]): boolean
	/**
	 * Final-state membership (base + pending delta — the exact view the
	 * commit judgment judges; check-then-act is race-free by construction).
	 */
	txContains(tx: TxHandle, relationId: number, values: readonly FactValue[]): boolean
	/** Final-state point lookup through a key statement; `null` on a miss. */
	txGet(tx: TxHandle, relationId: number, keyStatementId: number, keyValues: readonly FactValue[]): FactValue[] | null
	/**
	 * Mints the next fresh value for `(relationId, fieldId)` and returns it
	 * — the engine's alloc-then-insert dyn-lane mint (there is no
	 * insert-with-omitted-fields spelling; include the minted id in the
	 * full row).
	 */
	txAlloc(tx: TxHandle, relationId: number, fieldId: number): bigint
	/**
	 * Commits the delta: every dependency statement judged against the
	 * final state; a rejection carries the complete violation rendering.
	 * The handle is spent either way.
	 */
	txCommit(tx: TxHandle): CommitResult
	/** Aborts the delta (LMDB was never touched). The handle is spent. */
	txAbort(tx: TxHandle): void

	/**
	 * Prepares a program (IR as data, ids only; plan pinned at prepare).
	 * Roster errors return as data.
	 */
	dbPrepare(db: DbHandle, program: ProgramIr): PrepareResult
	/**
	 * Executes against a snapshot with positional params. One-copy owned
	 * rows out, column order = the program's head order; answers are a set
	 * — the host sorts.
	 */
	preparedExecute(prepared: PreparedHandle, snap: SnapshotHandle, params: readonly QueryParam[]): FactValue[][]
	/** The pull-based plan-drift signal against a snapshot. */
	preparedStaleness(prepared: PreparedHandle, snap: SnapshotHandle): Staleness
	/** Releases the prepared query. */
	preparedClose(prepared: PreparedHandle): void
}

/**
 * The sole platform this release ships (PRD-03 ruling 1: prebuilt-only,
 * darwin-arm64). The per-platform-package structure below makes adding
 * `darwin-x64`/`linux-*`/`win32-*` pure addition — one more `os`/`cpu`-gated
 * package plus a CI matrix — never a redesign, so this string is the only
 * place the shipped set is named for the unsupported-platform message.
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

export type {
	AggOpIr,
	AtomIr,
	AtomSourceIr,
	CmpOpIr,
	CommitResult,
	ComparisonIr,
	ConditionTreeIr,
	DbHandle,
	DbOpenResult,
	ExhumeHandle,
	ExhumeResult,
	FactValue,
	FindTermIr,
	HeadOpIr,
	HeadTermIr,
	IntervalValue,
	Manifest,
	ManifestField,
	ManifestRelation,
	ManifestRow,
	ManifestStatement,
	MaskTermIr,
	Native,
	OccurrenceDrift,
	PredicateDefIr,
	PreparedHandle,
	PrepareResult,
	ProgramIr,
	QueryParam,
	RuleIr,
	SnapshotHandle,
	Staleness,
	StatementKindTag,
	TaggedValue,
	TermIr,
	TxHandle,
	Violation,
	ViolationFact,
	WriteFromResult
}
export { loadNativeBinding, native }
