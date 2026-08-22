/**
 * `Db` — the living half of the SDK (PRD-07): open/create a store from a
 * `Schema`, write typed facts through delta transactions with race-free
 * final-state point reads, receive rejections as typed violation VALUES
 * keyed to statements, and read through a synchronous instance callback —
 * all typed by the schema's relations record.
 *
 * A store read is one callback: `db.read((instance, witness) => …)`. The
 * instance is invalid the moment the callback returns; the witness is a
 * cloneable token and may escape. There is no handle-shaped read and no
 * `using snap = db.read()`. Builder, owned instance, and witness
 * implement `Symbol.dispose`. Prepared plans are plain values whose
 * engine-side half is reclaimed by a GC finalizer — reclamation only,
 * never correctness.
 *
 * PROCESS MODEL: one process, one exclusive-lock handle per store. The
 * `Db` value owns the LMDB environment's exclusive lock until process
 * exit (or until GC reclaims the native handle); a second engine-level
 * open of the same store is refused by the engine (`EnvironmentLocked`),
 * matching Rust. Resume = reopen in a fresh process, or hold the one `Db`
 * this process already opened. The host owns composition; retry is host
 * policy.
 *
 * REJECTION IS DATA: a rejected commit is a domain outcome (it becomes the
 * LLM repair prompt downstream), returned as a {@link WriteOutcome}
 * carrying {@link Violation} values. A moved generation on
 * {@link Db.writeFrom} is the `{ tag: "moved" }` arm, not an exception.
 * Genuine failures — I/O, used-after-scope, spent handle, marshal shape —
 * throw `@superbuilders/errors` wrapped errors instead.
 */

import * as path from "node:path"
import * as errors from "@superbuilders/errors"
import { isClosedMember, sealedFieldsOf } from "#closed.ts"
import { rosterOf } from "#fields.ts"
import { lower } from "#lower.ts"
import { cellOf, factOf, handleOf, isFreshField, type KeyFact, keyRowOf, recordOf, rowOf } from "#marshal.ts"

import type {
	AdmitResult,
	BuilderHandle,
	DbHandle,
	FactValue,
	InstanceHandle,
	Manifest,
	NativeWriteOutcome,
	OwnedHandle,
	PreparedHandle,
	TxHandle,
	WireFreshRange,
	WireMutationReport,
	Violation as WireViolation,
	ViolationFact as WireViolationFact,
	WitnessHandle
} from "#native.ts"
import { bridged, bridgedAsync, errorFromThrow, native } from "#native.ts"
import type { FindColumn } from "#query/atom.ts"
import type { Query } from "#query/lower.ts"
import { lowerQuery } from "#query/lower.ts"
import { decodeAnswers, wireParams } from "#query/run.ts"
import type { ParamEntry, ParamsRecord } from "#query/scope.ts"
import type { AnyRelation, Fact, FreshKeys } from "#relation.ts"
import type { AnySchema, Schema, SchemaRelation, SchemaRelations } from "#schema.ts"
import { isStatement, type KeyStatement, type Statement } from "#statements.ts"

/**
 * The ordinary (writable, scannable) relations of a schema's record — the
 * only values the runtime methods accept: closed relations lack the
 * relation shape entirely, so passing one is a type error.
 */
type MemberRelation<Rels extends SchemaRelations> = Extract<Rels[keyof Rels], AnyRelation>

/**
 * Facts consumed vs facts that changed the in-memory final-state view.
 * The length-1 report is `{ submitted: 1n, changed: 0n | 1n }`.
 */
interface MutationReport {
	readonly submitted: bigint
	readonly changed: bigint
}

/**
 * The ONE collection-write spelling: typed fact objects, iterable
 * (proposals/one-representation/20). The column transport (`ColumnBatch`,
 * the union's second arm) is DELETED (70-deletions D1) — it existed only
 * because the row spelling was slow, and a transport kept beside its
 * replacement is a mode; the row spelling is now also the fast one (the
 * facts cross as one flat row-major cells array beside the explicit row
 * count — {@link FlatCollection}).
 */
type CollectionWrite<R extends AnyRelation> = Iterable<Fact<R>>

/**
 * One projected collection as it crosses the bridge: the exact row count
 * beside the flat cells. `rows` is EXPLICIT because the cells alone
 * cannot state it — a nullary relation's N facts project to 0 cells, so
 * any bridge-side `cells.length / arity` derivation would silently
 * collapse them to an empty write (nullary relations are LEGAL; the
 * engine pins them) — and the JS side is the ONE side that knows N.
 */
interface FlatCollection {
	readonly rows: bigint
	readonly cells: readonly FactValue[]
}

/**
 * The flat projector: every fact's cells land in ONE row-major
 * `FactValue` array (length rows×arity) — no JS array per fact exists
 * anywhere between the caller's objects and the native crossing
 * (proposals/one-representation/20, V1) — and the row count is counted
 * while projecting (the {@link FlatCollection} law: the stated count is
 * what the bridge verifies against `rows × arity`, exactly, for every
 * arity). The per-cell judgment is `cellOf` — the one cell judge `rowOf`
 * also speaks (closed handle→id, well-formedness, interval shape) — and
 * the missing-field refusal is `rowOf`'s, byte for byte; only the output
 * form differs (flat, never per-row).
 */
function rowsOf<R extends AnyRelation>(relation: R, facts: Iterable<Fact<R>>): FlatCollection {
	const data = relation.data
	const cells: FactValue[] = []
	let rows = 0n
	for (const fact of facts) {
		rows += 1n
		const record = recordOf(fact)
		for (const declared of data.fields) {
			const value = record[declared.name]
			if (value === undefined) {
				throw errors.new(`relation ${data.name}: fact is missing field ${declared.name}`)
			}
			cells.push(cellOf(`relation ${data.name} field ${declared.name}`, declared.field, value))
		}
	}
	return { rows, cells }
}

function mutateCollection<R extends AnyRelation>(
	relation: R,
	facts: CollectionWrite<R>,
	apply: (rows: bigint, cells: readonly FactValue[]) => WireMutationReport
): MutationReport {
	const flat = rowsOf(relation, facts)
	const report = apply(flat.rows, flat.cells)
	return Object.freeze({ submitted: report.submitted, changed: report.changed })
}

/**
 * Half-open fresh-id range from one `reserve`. Empty cannot yield a
 * minted id — `start` exists only on the nonempty arm.
 */
type FreshRange =
	| {
			readonly empty: true
			readonly count: 0n
			at(index: bigint): undefined
			[Symbol.iterator](): IterableIterator<bigint>
	  }
	| {
			readonly empty: false
			readonly start: bigint
			readonly endExclusive: bigint
			readonly count: bigint
			at(index: bigint): bigint | undefined
			[Symbol.iterator](): IterableIterator<bigint>
	  }

function freshRangeOf(wire: WireFreshRange): FreshRange {
	if (wire.empty) {
		return Object.freeze({
			empty: true,
			count: 0n,
			at(_index: bigint) {
				return undefined
			},
			*[Symbol.iterator](): IterableIterator<bigint> {}
		})
	}
	const start = wire.start
	const endExclusive = wire.endExclusive
	const count = endExclusive - start
	return Object.freeze({
		empty: false,
		start,
		endExclusive,
		get count() {
			return count
		},
		at(index: bigint) {
			if (index < 0n || index >= count) {
				return undefined
			}
			return start + index
		},
		*[Symbol.iterator](): IterableIterator<bigint> {
			for (let id = start; id < endExclusive; id++) {
				yield id
			}
		}
	})
}

/**
 * The key object of a key-statement-selected `get`: exactly the selected
 * `key()` statement's projection fields, each at the relation's own BARE
 * structural value type — the {@link KeyFact} rule generalized from the
 * primary key to ANY declared key statement.
 */
type DeclaredKeyFact<R extends AnyRelation, Projection extends readonly string[]> = {
	readonly [K in Projection[number] & keyof Fact<R>]: Fact<R>[K]
}

/**
 * One offending fact of a violation: the cited relation's name (a member
 * of the schema's record) and the fact decoded to a named natural-value
 * object — partial exactly as the engine cites it. Closed-referencing
 * cells arrive as handle NAMES (the marshal bijection's read half), so the
 * record and the violation's `canonical` string — which the engine already
 * renders with handle names — agree on the one spelling.
 */
interface OffendingFact<Rels extends SchemaRelations> {
	readonly relation: keyof Rels & string
	readonly fact: Readonly<Record<string, FactValue>>
}

/**
 * Shared body of every violation arm: the engine's canonical rendering and
 * the cited facts. `statement` is NOT here — its presence is the
 * discriminant. Implied auto-keys have no SDK spelling (`statement` is
 * the value `undefined`); every declared form carries the IDENTICAL
 * statement value the schema declared (consumers `===`-match it).
 */
type ViolationBody<Rels extends SchemaRelations> = {
	readonly canonical: string
	readonly facts: readonly OffendingFact<Rels>[]
}

/**
 * A functionality violation of an engine-materialized fresh-implied or
 * closed auto-key. These slots have no declared spelling (`schema()`
 * rejects an explicit duplicate); `statement` is present and `undefined`.
 */
type ImpliedKeyViolation<Rels extends SchemaRelations> = ViolationBody<Rels> & {
	readonly kind: "functionality"
	readonly statement: undefined
}

/**
 * A functionality violation of a declared `key()` statement. `statement`
 * is the IDENTICAL SDK value the schema declared.
 */
type DeclaredKeyViolation<Rels extends SchemaRelations> = ViolationBody<Rels> & {
	readonly kind: "functionality"
	readonly statement: Statement
}

/**
 * A containment violation of a declared `contained()` statement (no
 * `orientation` — that property exists exactly on {@link MirrorViolation}).
 */
type ContainmentViolation<Rels extends SchemaRelations> = ViolationBody<Rels> & {
	readonly kind: "containment"
	readonly statement: Statement
	readonly direction: "sourceUnsatisfied" | "targetRequired"
}

/**
 * A containment violation of one slot of a declared `mirrors()` statement.
 * BOTH materialized slots render as the one `==` utterance in the written
 * orientation (identical `canonical` strings; the engine's `render.rs`
 * never emits a bare `<=` for a mirrored pair). `direction` is relative
 * to the violated SLOT's own orientation, so it alone cannot say which
 * side of the `==` was violated: `written` is the `source <= target` slot
 * as the statement was spelled, `mirrored` the engine-materialized
 * `target <= source` partner.
 */
type MirrorViolation<Rels extends SchemaRelations> = ViolationBody<Rels> & {
	readonly kind: "containment"
	readonly statement: Statement
	readonly direction: "sourceUnsatisfied" | "targetRequired"
	readonly orientation: "written" | "mirrored"
}

/**
 * A capacity violation of a declared `capacity()` statement. `measure` is
 * the engine's witnessed group total — u128-wide, crossing whole as
 * bigint (C3: truncation is unrepresentable).
 */
type CapacityViolation<Rels extends SchemaRelations> = ViolationBody<Rels> & {
	readonly kind: "capacity"
	readonly statement: Statement
	readonly measure: bigint
}

/**
 * One violated statement of a rejected commit, as a typed value. The
 * arms are a true discriminant: `statement === undefined` is exactly the
 * implied-auto-key arm; every declared form carries `Statement` (not
 * `Statement | undefined`, not an omit-optional). `canonical` is the
 * ENGINE's rendering. `direction` / `measure` pass through from the
 * engine VERBATIM.
 */
type Violation<Rels extends SchemaRelations> =
	| ImpliedKeyViolation<Rels>
	| DeclaredKeyViolation<Rels>
	| ContainmentViolation<Rels>
	| MirrorViolation<Rels>
	| CapacityViolation<Rels>

/**
 * The abandoned arm of a write result (ruled 2026-07-23, R10): present in
 * the type EXACTLY when the callback can abandon — the conditional
 * distributes over `R`, so a callback with no `Abandon` arm contributes
 * `never` and the arm vanishes from the sum. The outcome is in the type;
 * a dead arm is never handled.
 */
type AbandonedArm<R> = R extends Abandon<infer P> ? { readonly tag: "abandoned"; readonly abandoned: P } : never

/** A callback return that is not Promise-like. TypeScript `never` is not a runtime boundary. */
type SyncResult<R> = R extends PromiseLike<unknown> ? never : R

interface Committed<T> {
	readonly value: T
	readonly generation: bigint
}

type Admission<Rels extends SchemaRelations, T> =
	| { readonly tag: "accepted"; readonly value: T }
	| { readonly tag: "rejected"; readonly violations: readonly Violation<Rels>[] }

/**
 * A write's domain outcome. One discriminant: narrow on `tag`. The
 * abandoned arm is present exactly when the callback can abandon.
 */
type WriteOutcome<Rels extends SchemaRelations, R> =
	| { readonly tag: "accepted"; readonly value: Committed<Exclude<R, Abandon<unknown>>> }
	| { readonly tag: "rejected"; readonly violations: readonly Violation<Rels>[] }
	| AbandonedArm<R>

type WriteFromOutcome<Rels extends SchemaRelations, R> =
	| WriteOutcome<Rels, R>
	| { readonly tag: "moved"; readonly witnessed: bigint; readonly current: bigint }

/**
 * The delta-building callback of a write: runs synchronously against the
 * live transaction. Returning {@link abandon}`(payload)` rolls the
 * transaction back (R10) — the result type carries the payload arm exactly
 * then.
 */
type DeltaBuild<Rels extends SchemaRelations, R = void> = (tx: WriteTx<Rels>) => R

/**
 * The runtime discriminant of {@link Abandon} values — a property probe is
 * how `write`/`writeFrom` distinguish "abort without committing" from an
 * ordinary callback result, never a guess about the host's own value shapes.
 */
const abandonMark: unique symbol = Symbol("bumbledb.abandon")

/**
 * The abandon sentinel {@link abandon} builds: returning one from a `write`
 * or `writeFrom` callback rolls the transaction back WITHOUT
 * committing (no empty commit is ever issued) and surfaces the payload as
 * `{ tag: "abandoned", abandoned: payload }` (ruled 2026-07-23, R10 — the
 * sentinel's contract is unconditional, whichever write verb received it).
 */
interface Abandon<P> {
	readonly [abandonMark]: true
	readonly payload: P
}

/**
 * Wraps a payload in the {@link Abandon} sentinel — the one way a write
 * callback declines to commit: `return abandon(payload)` aborts the delta
 * (nothing is committed, not even an empty commit) and the write resolves
 * to `{ tag: "abandoned", abandoned: payload }`, from `write` and `writeFrom`
 * alike (R10).
 */
function abandon<P>(payload: P): Abandon<P> {
	return Object.freeze({ [abandonMark]: true as const, payload })
}

/**
 * The abandon payload type a write callback's return type implies: the
 * payload of its `Abandon` arm, `never` when the callback can never
 * abandon (the `abandoned` outcome is then statically unreachable and
 * {@link AbandonedArm} erases it from the sum).
 */
type AbandonedPayload<R> = R extends Abandon<infer P> ? P : never

/**
 * Narrows a write callback result to the abandon sentinel. The probe is
 * the private {@link abandonMark} symbol only {@link abandon} sets, and
 * `R`'s `Abandon` arm is the only way a sentinel can flow out of the
 * callback — so the narrowed payload type is sound by construction.
 */
function isAbandon<R>(value: R): value is R & Abandon<AbandonedPayload<R>> {
	return typeof value === "object" && value !== null && abandonMark in value
}

/**
 * The abandon outcome's trusted admission seam: the value's shape is the
 * checkable half (the sentinel mark only {@link abandon} mints, and the
 * outcome carrying that sentinel's own payload), and the sentinel's
 * existence IS the proof `R` carries an `Abandon` arm — so the outcome is
 * admitted at the conditional {@link AbandonedArm} face the type tier
 * cannot resolve over an open `R`.
 */
function isAbandonedOutcome<Rels extends SchemaRelations, R>(
	outcome: { readonly tag: "abandoned"; readonly abandoned: AbandonedPayload<R> },
	sentinel: Abandon<AbandonedPayload<R>>
): outcome is { readonly tag: "abandoned"; readonly abandoned: AbandonedPayload<R> } & WriteOutcome<Rels, R> {
	return isAbandon(sentinel) && outcome.abandoned === sentinel.payload
}

/** Builds the abandoned write outcome from the callback's own sentinel (the R10 arm's one mint). */
function abandonedOutcome<Rels extends SchemaRelations, R>(
	sentinel: Abandon<AbandonedPayload<R>>
): WriteOutcome<Rels, R> {
	const outcome = Object.freeze({ tag: "abandoned" as const, abandoned: sentinel.payload })
	if (!isAbandonedOutcome<Rels, R>(outcome, sentinel)) {
		throw errors.new("bumbledb abandon outcome construction incomplete")
	}
	return outcome
}

/**
 * One live write transaction: the submitted delta with the engine's
 * FINAL-STATE point-read view (base + pending delta — the exact state the
 * commit judgment judges, so check-then-act is race-free by construction).
 * Spent when its owning `write`/`writeFrom` call resolves the attempt;
 * any later use throws.
 */
interface WriteTx<Rels extends SchemaRelations> {
	/**
	 * Records a collection of inserts. Singleton is `[fact]`. Empty is
	 * lawful. Returns how many facts were consumed and how many changed
	 * the in-memory final-state view. Every fact is complete — omitted
	 * fresh cells are a type error; mint first with {@link WriteTx.reserve}.
	 */
	insert<R extends MemberRelation<Rels>>(relation: R, facts: CollectionWrite<R>): MutationReport
	/**
	 * Records a collection of deletes. Singleton is `[fact]`. Returns
	 * how many facts were consumed and how many changed the view.
	 */
	delete<R extends MemberRelation<Rels>>(relation: R, facts: Iterable<Fact<R>>): MutationReport
	/**
	 * Mints `count` consecutive fresh values for a `.fresh` field.
	 * `count === 0n` is empty and does not yield a start.
	 */
	reserve<R extends MemberRelation<Rels>>(relation: R, field: FreshKeys<R> & string, count: bigint): FreshRange
	/** Final-state membership of one complete fact. */
	contains<R extends MemberRelation<Rels>>(relation: R, fact: Fact<R>): boolean
	/**
	 * Final-state point lookup through the relation's primary key (the
	 * {@link KeyFact} rule); `undefined` on a miss.
	 */
	get<R extends MemberRelation<Rels>>(relation: R, key: KeyFact<R>): Fact<R> | undefined
	/**
	 * Final-state point lookup through a DECLARED `key()` statement of this
	 * schema — the key object is typed by the statement's own projection;
	 * `undefined` on a miss.
	 */
	get<R extends MemberRelation<Rels>, const P extends readonly string[]>(
		relation: R,
		keyStatement: KeyStatement<R, P>,
		key: DeclaredKeyFact<R, P>
	): Fact<R> | undefined
}

const witnessTypes: unique symbol = Symbol("bumbledb.witness.types")

/**
 * Cloneable generation evidence from one store read. May cross `await`.
 * Disposal is idempotent; later use throws {@link ErrSpentHandle}.
 */
interface Witness<Rels extends SchemaRelations> extends Disposable {
	readonly [witnessTypes]?: Rels
}

/**
 * The borrowed instance one `db.read((instance, witness) => …)` callback
 * receives. Invalid the moment the callback returns. A stashed value
 * throws {@link ErrUseAfterScope}. Not a handle: there is no `db.read()`.
 */
interface ReadInstance<Rels extends SchemaRelations> {
	/**
	 * The committed generation this instance witnessed — read inside the
	 * lease's own transaction.
	 */
	readonly generation: bigint
	/** Full-relation export in row-id order, decoded to bare structural facts. */
	scan<R extends MemberRelation<Rels>>(relation: R): Fact<R>[]
	/**
	 * Exact cardinality of `relation` at this lease's snapshot — a
	 * structural read of the engine's maintained counter (folded
	 * transactionally at every commit, pinned equal to the scan count),
	 * never a scan, never an estimate. `bigint` by the wire law: engine
	 * cardinality is u64, which is not a JavaScript safe integer by
	 * construction. `count` and `scan` run inside the lease's one read
	 * transaction, so both observe the same snapshot. Closed relations
	 * are a type error, exactly as `scan` — a sealed extension is schema
	 * data whose length the caller already declared.
	 */
	count<R extends MemberRelation<Rels>>(relation: R): bigint
	/**
	 * Committed-state point lookup through the relation's primary key
	 * (the {@link KeyFact} rule); `undefined` on a miss.
	 */
	get<R extends MemberRelation<Rels>>(relation: R, key: KeyFact<R>): Fact<R> | undefined
	/**
	 * Committed-state point lookup through a DECLARED `key()` statement of
	 * this schema — the key object is typed by the statement's own
	 * projection; `undefined` on a miss.
	 */
	get<R extends MemberRelation<Rels>, const P extends readonly string[]>(
		relation: R,
		keyStatement: KeyStatement<R, P>,
		key: DeclaredKeyFact<R, P>
	): Fact<R> | undefined
	/** Committed-state membership of one complete fact. */
	contains<R extends MemberRelation<Rels>>(relation: R, fact: Fact<R>): boolean
	/**
	 * Executes a prepared query against this instance with the typed
	 * params object; returns the answer SET as plain rows with bare
	 * structural values (no order — the host sorts). This is the ONE
	 * execution spelling ({@link Prepared} carries no `execute`).
	 */
	execute<Row, Params extends ParamsRecord>(prepared: Prepared<Rels, Row, Params>, params: Params): Row[]
	prepare<Row, Params extends ParamsRecord>(q: Query<Rels, Row, Params>): Prepared<Rels, Row, Params>
}

/**
 * The module-private inference slot of {@link Prepared}: an optional symbol
 * property (never set at runtime) that keeps the prepared value's `Row` and
 * `Params` type arguments load-bearing, so `execute` infers the typed rows
 * and the typed params object from the value alone — the query module's
 * `inferred` pattern, local to this module. A type-level carrier only:
 * values stay bare, nothing is asserted.
 */
const preparedTypes: unique symbol = Symbol("bumbledb.prepared.types")

/**
 * One prepared query as a plain VALUE: explicit visible compilation
 * (`db.prepare(q)` lowers, pins the plan, and surfaces every engine roster
 * refusal), no lifecycle. Execution happens ONLY through
 * `instance.execute(prepared, params)`. The engine-side plan is reclaimed by a GC
 * finalizer when this value becomes unreachable (reclamation only, never
 * correctness — an unreclaimed plan is idle memory, and process exit frees
 * everything).
 */
interface Prepared<Rels extends SchemaRelations, Row, Params extends ParamsRecord> {
	readonly [preparedTypes]?: { readonly rels: Rels; readonly row: Row; readonly params: Params }
}

/**
 * An open store. There is no close: read through `read`,
 * write through `write`/`writeFrom`, and let the process own the
 * environment's lifetime (the engine fsyncs every commit, so durability
 * never waits on a close). A second `open`/`create` of the same path
 * while this handle lives is the engine's `EnvironmentLocked`.
 */
interface Db<Rels extends SchemaRelations> {
	/** The theory this store was opened with (fingerprint-verified by the engine). */
	readonly schema: Schema<Rels>
	/**
	 * One store read: runs `body` SYNCHRONOUSLY inside the engine lease
	 * and returns its result. The {@link ReadInstance} is invalidated
	 * when `body` returns — a stashed use throws {@link ErrUseAfterScope}.
	 * The {@link Witness} is a clone and may escape. A thenable return
	 * throws {@link ErrAsyncCallback}.
	 */
	read<R>(body: (instance: ReadInstance<Rels>, witness: Witness<Rels>) => SyncResult<R>): SyncResult<R>
	/**
	 * One delta transaction: builds the delta synchronously through `fn`,
	 * commits, and returns the domain outcome. A throw from `fn` aborts
	 * the delta (LMDB untouched) and rethrows wrapped. `fn` may decline to
	 * commit by returning {@link abandon}`(payload)`: the transaction rolls
	 * back — nothing is committed, not even an empty commit — and the
	 * outcome is `{ tag: "abandoned", abandoned: payload }`.
	 */
	write<R>(fn: (tx: WriteTx<Rels>) => SyncResult<R>): WriteOutcome<Rels, SyncResult<R>>
	/**
	 * Witnessed write: commits only if no state-changing commit landed
	 * since `witness` was minted. A moved generation is
	 * `{ tag: "moved" }` — retry is host policy; this method never loops.
	 */
	writeFrom<R>(witness: Witness<Rels>, fn: (tx: WriteTx<Rels>) => SyncResult<R>): WriteFromOutcome<Rels, SyncResult<R>>
	/**
	 * Prepares a query value built against THIS schema (identity is the
	 * membership rule): lowers it to the engine IR, pins the plan, and
	 * returns the typed {@link Prepared} value. Every IR roster refusal —
	 * rule caps, rec roster, type rules — is the ENGINE's typed
	 * judgment and throws here carrying its message intact.
	 */
	prepare<Row, Params extends ParamsRecord>(q: Query<Rels, Row, Params>): Prepared<Rels, Row, Params>
}

/** One relation's runtime tables: engine id, the identical schema member, field ids, primary key. */
interface RelationEntry {
	readonly id: number
	readonly member: SchemaRelation
	readonly fieldIds: ReadonlyMap<string, number>
	readonly primaryKey: PrimaryKey | undefined
}

/** One relation's primary candidate key: its materialized statement id and projection. */
interface PrimaryKey {
	readonly statementId: number
	readonly projection: readonly string[]
}

/**
 * One materialized-statement slot as the SDK mirrors it. Implied auto-keys
 * omit `statement` (the engine owns those slots); every declared form
 * carries the SDK value that lowered to it. Functionality forms also
 * carry the key's owner and projection (what keyed point reads resolve
 * through).
 */
type ImpliedKeyEntry = {
	readonly kind: "functionality"
	readonly owner: string
	readonly projection: readonly string[]
}

type DeclaredKeyEntry = {
	readonly kind: "functionality"
	readonly statement: Statement
	readonly owner: string
	readonly projection: readonly string[]
}

type StatementEntry =
	| ImpliedKeyEntry
	| DeclaredKeyEntry
	| { readonly kind: "containment"; readonly statement: Statement }
	| { readonly kind: "mirrors"; readonly statement: Statement; readonly orientation: "written" | "mirrored" }
	| { readonly kind: "capacity"; readonly statement: Statement }

/**
 * Mirrors the engine's materialized statement order
 * (`SchemaDescriptor::materialized_statements`, pinned by the fingerprint):
 * one auto-key per fresh field (relation declaration order, then field
 * order), one closed auto-key per closed relation (declaration order),
 * then the declared statements in declaration order — a `mirrors`
 * statement occupying TWO adjacent slots (the engine lowers `==` to two
 * containments, `source <= target` first), both owned by the one SDK
 * value. This positional match is how statement ids resolve back to SDK
 * statement values without the engine ever learning a wire format.
 */
function materializedEntries(theory: AnySchema): StatementEntry[] {
	const entries = impliedKeyEntries(theory)
	for (const statement of theory.statements) {
		entries.push(...declaredEntries(statement))
	}
	return entries
}

/**
 * The engine-materialized implied keys, in the engine's pinned order: one
 * auto-key per fresh field (relation declaration order, then field order),
 * then one closed auto-key `R(id) -> R` per closed relation (declaration
 * order). These slots carry no SDK statement value — the engine owns them
 * (`schema()` rejects an explicit duplicate).
 */
function impliedKeyEntries(theory: AnySchema): StatementEntry[] {
	const entries: StatementEntry[] = []
	for (const member of Object.values(theory.relations)) {
		if (isClosedMember(member)) {
			continue
		}
		for (const declared of member.data.fields) {
			if (isFreshField(declared.field)) {
				entries.push({
					kind: "functionality",
					owner: member.name,
					projection: [declared.name]
				})
			}
		}
	}
	for (const member of Object.values(theory.relations)) {
		if (isClosedMember(member)) {
			entries.push({
				kind: "functionality",
				owner: member.name,
				projection: ["id"]
			})
		}
	}
	return entries
}

/**
 * One declared statement's materialized slots: a key or capacity statement
 * occupies one, a `mirrors` occupies two adjacent slots (the engine lowers
 * `==` to two containments, `source <= target` first), both owned by the
 * one SDK value.
 */
function declaredEntries(statement: Statement): StatementEntry[] {
	const data = statement.data
	switch (data.kind) {
		case "key": {
			return [
				{
					kind: "functionality",
					statement,
					owner: data.owner.name,
					projection: data.projection
				}
			]
		}
		case "containment": {
			return [{ kind: "containment", statement }]
		}
		case "mirrors": {
			return [
				{ kind: "mirrors", statement, orientation: "written" },
				{ kind: "mirrors", statement, orientation: "mirrored" }
			]
		}
		case "capacity": {
			return [{ kind: "capacity", statement }]
		}
	}
}

/**
 * Narrows a callback result to a thenable — the async-callback probe both
 * commit sites share: an `async` build callback typechecks (`Promise<void>`
 * is assignable where a `void` return is expected), so the refusal has to
 * be a runtime probe on the returned value.
 */
function isThenable(value: unknown): boolean {
	return typeof value === "object" && value !== null && "then" in value && typeof value.then === "function"
}

/**
 * Narrows a keyed-get middle argument to a statement value (vs a key
 * object) through the statement module's admission brand — a
 * REPRESENTATION, never a shape probe: fact cell shapes are structurally
 * OPEN (an interval value carrying an excess `kind` property is a legal
 * cell), so no property probe could ever be sound here, but no host-built
 * key object can spell the module-private brand symbol.
 */
function isStatementValue<R extends AnyRelation, P extends readonly string[]>(
	value: KeyFact<R> | KeyStatement<R, P>
): value is KeyStatement<R, P> {
	return isStatement(value)
}

/**
 * THE one selector dispatch of the `get` overload pair (primary-key vs
 * key-statement, `docs/architecture/70-api.md` § the freeze): judges the
 * middle argument once and hands the narrowed pieces to the chosen
 * continuation. Every keyed get on a read scope, write tx, and builder
 * dispatches through here, so the two mismatch refusals speak with one voice.
 */
function selectKeyRead<R extends AnyRelation, P extends readonly string[], T>(
	keyOrStatement: KeyFact<R> | KeyStatement<R, P>,
	declaredKey: DeclaredKeyFact<R, P> | undefined,
	byStatement: (statement: KeyStatement<R, P>, key: DeclaredKeyFact<R, P>) => T,
	byPrimary: (key: KeyFact<R>) => T
): T {
	if (declaredKey !== undefined) {
		if (!isStatementValue(keyOrStatement)) {
			throw errors.new("keyed get takes a key() statement value as its second argument")
		}
		return byStatement(keyOrStatement, declaredKey)
	}
	if (isStatementValue(keyOrStatement)) {
		throw errors.new("keyed get with a statement selector also takes the key object — get(relation, keyStatement, key)")
	}
	return byPrimary(keyOrStatement)
}

/** The id-resolution tables one open builds: relation entries by name, statement slots by id. */
interface Tables {
	readonly relations: ReadonlyMap<string, RelationEntry>
	readonly statements: readonly StatementEntry[]
}

/**
 * Builds the id-resolution tables from the manifest, verifying the SDK's
 * positional mirror against the engine's reported order — any drift
 * (count, kind, id, or membership) is a construction-time failure, never a
 * silent misattribution of a violation to the wrong statement value. The
 * declaration-ordinal law the query lowering leans on is verified in the
 * same walks: relation ids and sealed field ids both equal declaration
 * order, so a constructed `Tables` IS the proof and `prepare` inherits it
 * structurally — never a silently misaddressed query.
 */
function tablesOf(theory: AnySchema, manifest: Manifest): Tables {
	const entries = materializedEntries(theory)
	if (entries.length !== manifest.statements.length) {
		throw errors.new(
			`bumbledb manifest drift: the SDK lowering yields ${entries.length} materialized statements, the engine reports ${manifest.statements.length}`
		)
	}
	manifest.statements.forEach(function verifySlot(statement, index) {
		const entry = entries[index]
		if (entry === undefined || statement.id !== index) {
			throw errors.new(
				`bumbledb manifest drift: statement ${statement.id} is ${statement.kind}, the SDK mirror at ${index} expected ${entry?.kind}`
			)
		}
		const engineKind = entry.kind === "mirrors" ? "containment" : entry.kind
		if (engineKind !== statement.kind) {
			throw errors.new(
				`bumbledb manifest drift: statement ${statement.id} is ${statement.kind}, the SDK mirror at ${index} expected ${engineKind}`
			)
		}
	})
	const relations = new Map<string, RelationEntry>()
	for (const relation of manifest.relations) {
		const member = theory.relations[relation.name]
		if (member === undefined) {
			throw errors.new(`bumbledb manifest drift: relation ${relation.name} is not in schema ${theory.name}`)
		}
		const fieldIds = new Map<string, number>()
		for (const field of relation.fields) {
			fieldIds.set(field.name, field.id)
		}
		sealedFieldsOf(member).forEach(function verifyField(declared, fieldOrdinal) {
			if (fieldIds.get(declared.name) !== fieldOrdinal) {
				throw errors.new(
					`bumbledb manifest drift: ${relation.name}.${declared.name} has engine field id ${fieldIds.get(declared.name)}, its sealed ordinal is ${fieldOrdinal}`
				)
			}
		})
		let primaryKey: PrimaryKey | undefined
		entries.forEach(function firstOwnedKey(entry, index) {
			if (primaryKey === undefined && entry.kind === "functionality" && entry.owner === relation.name) {
				primaryKey = Object.freeze({ statementId: index, projection: entry.projection })
			}
		})
		relations.set(relation.name, Object.freeze({ id: relation.id, member, fieldIds, primaryKey }))
	}
	Object.keys(theory.relations).forEach(function verifyRelation(name, ordinal) {
		const entry = relations.get(name)
		if (entry === undefined) {
			throw errors.new(`bumbledb manifest drift: schema relation ${name} is not in the manifest`)
		}
		if (entry.id !== ordinal) {
			throw errors.new(
				`bumbledb manifest drift: relation ${name} has engine id ${entry.id}, its declaration ordinal is ${ordinal} — query lowering depends on declaration order = ids`
			)
		}
	})
	return Object.freeze({ relations, statements: Object.freeze(entries) })
}

function tablesFromTheory(theory: AnySchema): Tables {
	const entries = materializedEntries(theory)
	const relations = new Map<string, RelationEntry>()
	Object.keys(theory.relations).forEach(function byOrdinal(name, ordinal) {
		const member = theory.relations[name]
		if (member === undefined) {
			throw errors.new(`bumbledb theory has no relation ${name}`)
		}
		const fieldIds = new Map<string, number>()
		sealedFieldsOf(member).forEach(function byField(declared, fieldOrdinal) {
			fieldIds.set(declared.name, fieldOrdinal)
		})
		let primaryKey: PrimaryKey | undefined
		entries.forEach(function firstOwnedKey(entry, index) {
			if (primaryKey === undefined && entry.kind === "functionality" && entry.owner === name) {
				primaryKey = Object.freeze({ statementId: index, projection: entry.projection })
			}
		})
		relations.set(name, Object.freeze({ id: ordinal, member, fieldIds, primaryKey }))
	})
	return Object.freeze({ relations, statements: Object.freeze(entries) })
}

/** The point-read half a transaction and a read scope share, over their own handle. */
interface PointReads {
	contains(relationId: number, row: readonly FactValue[]): boolean
	get(relationId: number, statementId: number, key: readonly FactValue[]): FactValue[] | null
}

/**
 * One borrowed instance's PRIVATE lifetime record. Held in
 * {@link instanceStates} — the native handle is never a public value.
 */
interface InstanceState {
	readonly handle: InstanceHandle
	live: boolean
	readonly owner: object
}

const instanceStates = new WeakMap<object, InstanceState>()

interface WitnessState {
	readonly handle: WitnessHandle
	spent: boolean
	readonly owner: object
}

const witnessStates = new WeakMap<object, WitnessState>()

const witnessReclaimer = new FinalizationRegistry<WitnessHandle>(function reclaimWitness(handle) {
	const closed = errors.trySync(function closeWitness() {
		native.witnessClose(handle)
	})
	if (closed.error) {
		return
	}
})

/**
 * One prepared value's PRIVATE engine half: the pinned plan handle, the
 * owning store's identity token, and the query's marshaling tables (params
 * in declaration order, select columns in head order). Held in
 * {@link preparedPlans} — the plan handle is never a public value.
 */
interface PreparedPlan {
	readonly handle: PreparedHandle
	readonly owner: object
	readonly params: readonly ParamEntry[]
	readonly finds: readonly FindColumn[]
}

/** The private engine halves of this module's prepared values. */
const preparedPlans = new WeakMap<object, PreparedPlan>()

/**
 * Reclaims the engine-side plan of a garbage-collected {@link Prepared}
 * value. RECLAMATION ONLY, never correctness: a plan the collector never
 * visits is idle engine memory until process exit, and a failure to close
 * is swallowed (there is no one left to care — the owning value is gone).
 */
const planReclaimer = new FinalizationRegistry<PreparedHandle>(function reclaimPlan(handle) {
	const closed = errors.trySync(function closePlan() {
		native.preparedClose(handle)
	})
	if (closed.error) {
		return
	}
})

const ErrAsyncCallback = errors.new(
	"bumbledb asyncCallback: a read or write callback returned a thenable — the callback is synchronous"
)
const ErrSpentHandle = errors.new("bumbledb spentHandle: a consumed builder, instance, or witness was used")
const ErrUseAfterScope = errors.new(
	"bumbledb useAfterScope: a stashed read instance or write transaction was used after its callback returned"
)
const ErrForeignPrepared = errors.new("bumbledb foreignPrepared: a prepared query met a foreign instance")
const ErrForeignWitness = errors.new("bumbledb foreignWitness: a witness met a foreign store")

/**
 * The shared typed read surface: store leases and owned instances both
 * expose scan/count/get/contains/execute/prepare. The native ops are the only
 * difference — one way to read, two handle kinds.
 */
interface CatalogNative {
	scan(relationId: number): FactValue[][]
	count(relationId: number): bigint
	contains(relationId: number, values: readonly FactValue[]): boolean
	get(relationId: number, statementId: number, keyValues: readonly FactValue[]): FactValue[] | null
	prepare(query: ReturnType<typeof lowerQuery>): ReturnType<typeof native.instancePrepare>
	execute(prepared: PreparedHandle, params: ReturnType<typeof wireParams>): FactValue[][]
}

function catalogMethods<Rels extends SchemaRelations>(
	theory: Schema<Rels>,
	tables: Tables,
	owner: object,
	assertLive: () => void,
	ops: CatalogNative
): Pick<ReadInstance<Rels>, "scan" | "count" | "get" | "contains" | "execute" | "prepare"> {
	function planOf(prepared: object): PreparedPlan {
		const plan = preparedPlans.get(prepared)
		if (plan === undefined) {
			throw errors.wrap(ErrForeignPrepared, "bumbledb execute target is not a prepared value of this SDK")
		}
		if (plan.owner !== owner) {
			throw errors.wrap(
				ErrForeignPrepared,
				`bumbledb prepared value was prepared by a different store than this one (schema ${theory.name})`
			)
		}
		return plan
	}
	function contains<R extends MemberRelation<Rels>>(relation: R, fact: Fact<R>): boolean {
		assertLive()
		const entry = ordinaryEntry(tables, theory, relation)
		return bridged("bumbledb instance contains", function readContains() {
			return ops.contains(entry.id, rowOf(relation.data, recordOf(fact)))
		})
	}
	function get<R extends MemberRelation<Rels>, const P extends readonly string[]>(
		relation: R,
		keyOrStatement: KeyFact<R> | KeyStatement<R, P>,
		declaredKey?: DeclaredKeyFact<R, P>
	): Fact<R> | undefined {
		assertLive()
		const entry = ordinaryEntry(tables, theory, relation)
		return selectKeyRead(
			keyOrStatement,
			declaredKey,
			function byStatement(statement, key) {
				const selected = declaredKeyOf(tables, theory, relation, statement)
				const row = bridged("bumbledb instance get", function readGet() {
					return ops.get(entry.id, selected.statementId, keyRowOf(relation.data, selected.projection, recordOf(key)))
				})
				return row === null ? undefined : factOf(relation, row)
			},
			function byPrimary(key) {
				const primaryKey = entry.primaryKey
				if (primaryKey === undefined) {
					throw errors.new(
						`relation ${relation.name} has no candidate key — keyed get requires a fresh field or a declared key statement`
					)
				}
				const row = bridged("bumbledb instance get", function readGet() {
					return ops.get(
						entry.id,
						primaryKey.statementId,
						keyRowOf(relation.data, primaryKey.projection, recordOf(key))
					)
				})
				return row === null ? undefined : factOf(relation, row)
			}
		)
	}
	function scan<R extends MemberRelation<Rels>>(relation: R): Fact<R>[] {
		assertLive()
		const entry = ordinaryEntry(tables, theory, relation)
		const rows = bridged("bumbledb instance scan", function readScan() {
			return ops.scan(entry.id)
		})
		return rows.map(function decodeRow(row) {
			return factOf(relation, row)
		})
	}
	function count<R extends MemberRelation<Rels>>(relation: R): bigint {
		assertLive()
		const entry = ordinaryEntry(tables, theory, relation)
		return bridged("bumbledb instance count", function readCount() {
			return ops.count(entry.id)
		})
	}
	function execute<Row, Params extends ParamsRecord>(prepared: Prepared<Rels, Row, Params>, params: Params): Row[] {
		assertLive()
		const plan = planOf(prepared)
		const wire = wireParams(plan.params, recordOf(params))
		const rows = bridged("execute bumbledb prepared query", function callExecute() {
			return ops.execute(plan.handle, wire)
		})
		return decodeAnswers<Row>(plan.finds, rows)
	}
	function prepare<Row, Params extends ParamsRecord>(q: Query<Rels, Row, Params>): Prepared<Rels, Row, Params> {
		assertLive()
		if (q.schema !== theory) {
			throw errors.new(
				`query was built against schema ${q.schema.name}, not the identical schema value this store opened with — schema identity is the membership rule`
			)
		}
		const queryIr = lowerQuery(q)
		const outcome = bridged("prepare bumbledb query", function callPrepare() {
			return ops.prepare(queryIr)
		})
		if (!outcome.ok) {
			throwPrepareRefusal(outcome.message)
		}
		const prepared: Prepared<Rels, Row, Params> = Object.freeze({})
		preparedPlans.set(
			prepared,
			Object.freeze({
				handle: outcome.prepared,
				owner,
				params: q.data.params,
				finds: q.data.finds
			})
		)
		planReclaimer.register(prepared, outcome.prepared)
		return prepared
	}
	return { scan, count, get, contains, execute, prepare }
}

function ordinaryEntry(tables: Tables, theory: AnySchema, relation: AnyRelation): RelationEntry {
	const entry = tables.relations.get(relation.name)
	if (entry === undefined || entry.member !== relation) {
		throw errors.new(`relation ${relation.name} is not a member of schema ${theory.name}`)
	}
	if (isClosedMember(relation)) {
		throw errors.new(
			`relation ${relation.name} is closed — its extension is schema data (axioms), never scanned or written`
		)
	}
	return entry
}

/** Resolves a key-statement-selected read: identity is the membership rule. */
function declaredKeyOf(tables: Tables, theory: AnySchema, relation: AnyRelation, statement: Statement): PrimaryKey {
	const statementId = tables.statements.findIndex(function byIdentity(candidate) {
		return "statement" in candidate && candidate.statement === statement
	})
	const entry = tables.statements[statementId]
	if (entry === undefined) {
		throw errors.new(
			`keyed get statement is not a declared statement of schema ${theory.name} — statement identity is the membership rule`
		)
	}
	if (entry.kind !== "functionality") {
		throw errors.new("keyed get takes a key() statement — containments and capacity statements key nothing")
	}
	if (entry.owner !== relation.name) {
		throw errors.new(
			`keyed get statement keys ${entry.owner}, not ${relation.name} — the statement must be a declared key of the relation it reads`
		)
	}
	return Object.freeze({ statementId, projection: entry.projection })
}

function overlayMethods<Rels extends SchemaRelations>(
	theory: Schema<Rels>,
	tables: Tables,
	assertLive: () => void,
	reads: PointReads
): Pick<WriteTx<Rels>, "contains" | "get"> {
	function contains<R extends MemberRelation<Rels>>(relation: R, fact: Fact<R>): boolean {
		assertLive()
		const entry = ordinaryEntry(tables, theory, relation)
		return reads.contains(entry.id, rowOf(relation.data, recordOf(fact)))
	}
	function readThroughKey<R extends MemberRelation<Rels>>(
		relation: R,
		entry: RelationEntry,
		selected: PrimaryKey,
		key: Readonly<Record<string, unknown>>
	): Fact<R> | undefined {
		const row = reads.get(entry.id, selected.statementId, keyRowOf(relation.data, selected.projection, key))
		if (row === null) {
			return undefined
		}
		return factOf(relation, row)
	}
	function get<R extends MemberRelation<Rels>>(relation: R, key: KeyFact<R>): Fact<R> | undefined
	function get<R extends MemberRelation<Rels>, const P extends readonly string[]>(
		relation: R,
		keyStatement: KeyStatement<R, P>,
		key: DeclaredKeyFact<R, P>
	): Fact<R> | undefined
	function get<R extends MemberRelation<Rels>, const P extends readonly string[]>(
		relation: R,
		keyOrStatement: KeyFact<R> | KeyStatement<R, P>,
		declaredKey?: DeclaredKeyFact<R, P>
	): Fact<R> | undefined {
		assertLive()
		const entry = ordinaryEntry(tables, theory, relation)
		return selectKeyRead(
			keyOrStatement,
			declaredKey,
			function byStatement(statement, key) {
				return readThroughKey(relation, entry, declaredKeyOf(tables, theory, relation, statement), recordOf(key))
			},
			function byPrimary(key) {
				const primaryKey = entry.primaryKey
				if (primaryKey === undefined) {
					throw errors.new(
						`relation ${relation.name} has no candidate key — keyed get requires a fresh field or a declared key statement`
					)
				}
				return readThroughKey(relation, entry, primaryKey, recordOf(key))
			}
		)
	}
	return { contains, get }
}

function createReadInstance<Rels extends SchemaRelations>(
	nativeHandle: InstanceHandle,
	theory: Schema<Rels>,
	tables: Tables,
	owner: object
): ReadInstance<Rels> {
	const state: InstanceState = { handle: nativeHandle, live: true, owner }
	function assertLive(): void {
		if (!state.live) {
			throw errors.wrap(
				ErrUseAfterScope,
				"bumbledb read instance is invalidated — its owning callback already returned"
			)
		}
	}
	const methods = catalogMethods(theory, tables, owner, assertLive, {
		scan(relationId) {
			return native.instanceScan(state.handle, relationId)
		},
		count(relationId) {
			return native.instanceCount(state.handle, relationId)
		},
		contains(relationId, values) {
			return native.instanceContains(state.handle, relationId, values)
		},
		get(relationId, statementId, keyValues) {
			return native.instanceGet(state.handle, relationId, statementId, keyValues)
		},
		prepare(query) {
			return native.instancePrepare(state.handle, query)
		},
		execute(prepared, params) {
			return native.preparedExecute(prepared, state.handle, params)
		}
	})
	const instance: ReadInstance<Rels> = Object.freeze({
		get generation() {
			assertLive()
			return bridged("bumbledb instance generation", function readGeneration() {
				return native.instanceGeneration(state.handle)
			})
		},
		...methods
	})
	instanceStates.set(instance, state)
	return instance
}

/**
 * Constructs one open `Db` over an already-admitted handle: builds the
 * id-resolution tables once and closes over them — the `Db` owns handle
 * and tables and nothing else. Handle lifetime is the process's: the store
 * cache holds the environment handle until the exit hook closes it.
 */
function openDb<Rels extends SchemaRelations>(handle: DbHandle, theory: Schema<Rels>, manifest: Manifest): Db<Rels> {
	const tables = tablesOf(theory, manifest)
	/** This store's identity token: read scopes and prepared values carry it, so cross-store use is a typed refusal. */
	const owner = Object.freeze({})

	function isMemberName(name: string): name is keyof Rels & string {
		return tables.relations.has(name)
	}

	function offendingFactOf(fact: WireViolationFact): OffendingFact<Rels> {
		const entry = tables.relations.get(fact.relation)
		if (entry === undefined || !isMemberName(fact.relation)) {
			throw errors.new(`bumbledb violation cites unknown relation ${fact.relation}`)
		}
		const declared = sealedFieldsOf(entry.member)
		const decoded: Record<string, FactValue> = {}
		for (const cell of fact.fields) {
			const cited = declared.find(function byName(candidate) {
				return candidate.name === cell.name
			})
			const roster = rosterOf(cited?.field)
			decoded[cell.name] =
				roster !== undefined
					? handleOf(`violation fact ${fact.relation} field ${cell.name}`, roster, cell.value)
					: cell.value
		}
		return Object.freeze({ relation: fact.relation, fact: Object.freeze(decoded) })
	}

	function violationOf(wire: WireViolation): Violation<Rels> {
		const entry = tables.statements[wire.statementId]
		if (entry === undefined) {
			throw errors.new(`bumbledb violation cites unknown statement id ${wire.statementId}`)
		}
		const facts = Object.freeze(wire.facts.map(offendingFactOf))
		const canonical = wire.canonical
		if (entry.kind === "functionality") {
			if (!("statement" in entry)) {
				return Object.freeze({ kind: "functionality", statement: undefined, canonical, facts })
			}
			return Object.freeze({ kind: "functionality", statement: entry.statement, canonical, facts })
		}
		if (entry.kind === "capacity") {
			if (wire.kind !== "capacity") {
				throw errors.new(`bumbledb violation ${wire.statementId} is a capacity slot without a measure`)
			}
			return Object.freeze({
				kind: "capacity",
				statement: entry.statement,
				canonical,
				measure: wire.measure,
				facts
			})
		}
		if (wire.kind !== "containment") {
			throw errors.new(`bumbledb violation ${wire.statementId} is a containment slot without a direction`)
		}
		if (entry.kind === "mirrors") {
			return Object.freeze({
				kind: "containment",
				statement: entry.statement,
				canonical,
				direction: wire.direction,
				orientation: entry.orientation,
				facts
			})
		}
		return Object.freeze({
			kind: "containment",
			statement: entry.statement,
			canonical,
			direction: wire.direction,
			facts
		})
	}

	function pointReadsOf(assertLive: () => void, reads: PointReads) {
		function contains<R extends MemberRelation<Rels>>(relation: R, fact: Fact<R>): boolean {
			assertLive()
			const entry = ordinaryEntry(tables, theory, relation)
			return reads.contains(entry.id, rowOf(relation.data, recordOf(fact)))
		}
		/** One keyed point read through an already-resolved key, decoded to a fact (`undefined` on a miss). */
		function readThroughKey<R extends MemberRelation<Rels>>(
			relation: R,
			entry: RelationEntry,
			selected: PrimaryKey,
			key: Readonly<Record<string, unknown>>
		): Fact<R> | undefined {
			const row = reads.get(entry.id, selected.statementId, keyRowOf(relation.data, selected.projection, key))
			if (row === null) {
				return undefined
			}
			return factOf(relation, row)
		}
		function get<R extends MemberRelation<Rels>>(relation: R, key: KeyFact<R>): Fact<R> | undefined
		function get<R extends MemberRelation<Rels>, const P extends readonly string[]>(
			relation: R,
			keyStatement: KeyStatement<R, P>,
			key: DeclaredKeyFact<R, P>
		): Fact<R> | undefined
		function get<R extends MemberRelation<Rels>, const P extends readonly string[]>(
			relation: R,
			keyOrStatement: KeyFact<R> | KeyStatement<R, P>,
			declaredKey?: DeclaredKeyFact<R, P>
		): Fact<R> | undefined {
			assertLive()
			const entry = ordinaryEntry(tables, theory, relation)
			return selectKeyRead(
				keyOrStatement,
				declaredKey,
				function byStatement(statement, key) {
					return readThroughKey(relation, entry, declaredKeyOf(tables, theory, relation, statement), recordOf(key))
				},
				function byPrimary(key) {
					const primaryKey = entry.primaryKey
					if (primaryKey === undefined) {
						throw errors.new(
							`relation ${relation.name} has no candidate key — keyed get requires a fresh field or a declared key statement`
						)
					}
					return readThroughKey(relation, entry, primaryKey, recordOf(key))
				}
			)
		}
		return { contains, get }
	}

	function pinPrepared<Row, Params extends ParamsRecord>(
		preparedHandle: PreparedHandle,
		q: Query<Rels, Row, Params>
	): Prepared<Rels, Row, Params> {
		const prepared: Prepared<Rels, Row, Params> = Object.freeze({})
		preparedPlans.set(
			prepared,
			Object.freeze({
				handle: preparedHandle,
				owner,
				params: q.data.params,
				finds: q.data.finds
			})
		)
		planReclaimer.register(prepared, preparedHandle)
		return prepared
	}

	function makeWitness(nativeHandle: WitnessHandle): Witness<Rels> {
		const state: WitnessState = { handle: nativeHandle, spent: false, owner }
		const witness: Witness<Rels> = Object.freeze({
			[Symbol.dispose](): void {
				if (state.spent) {
					return
				}
				state.spent = true
				bridged("close bumbledb witness", function closeWitness() {
					native.witnessClose(nativeHandle)
				})
			}
		})
		witnessStates.set(witness, state)
		witnessReclaimer.register(witness, nativeHandle)
		return witness
	}

	function makeInstance(nativeHandle: InstanceHandle): ReadInstance<Rels> {
		return createReadInstance(nativeHandle, theory, tables, owner)
	}

	function read<R>(body: (instance: ReadInstance<Rels>, witness: Witness<Rels>) => SyncResult<R>): SyncResult<R> {
		let captured: R | undefined
		const result = bridged("bumbledb read", function runRead() {
			return native.dbRead(handle, function onRead(nativeInstance, nativeWitness) {
				const instance = makeInstance(nativeInstance)
				const witness = makeWitness(nativeWitness)
				const value = body(instance, witness)
				const state = instanceStates.get(instance)
				if (state !== undefined) {
					state.live = false
				}
				if (isThenable(value)) {
					throw errors.wrap(ErrAsyncCallback, "bumbledb read callback returned a thenable")
				}
				captured = value
				return value
			})
		})
		return (captured ?? result) as SyncResult<R>
	}

	function makeTx(resolveTx: () => TxHandle): { readonly tx: WriteTx<Rels>; spend(): void } {
		const txState = { spent: false }
		function assertLive(): void {
			if (txState.spent) {
				throw errors.wrap(ErrUseAfterScope, "bumbledb write transaction is spent")
			}
		}
		const reads = pointReadsOf(assertLive, {
			contains(relationId, row) {
				const txHandle = resolveTx()
				return bridged("bumbledb tx contains", function readContains() {
					return native.txContains(txHandle, relationId, row)
				})
			},
			get(relationId, statementId, key) {
				const txHandle = resolveTx()
				return bridged("bumbledb tx get", function readGet() {
					return native.txGet(txHandle, relationId, statementId, key)
				})
			}
		})
		function insert<R extends MemberRelation<Rels>>(relation: R, facts: CollectionWrite<R>): MutationReport {
			assertLive()
			const entry = ordinaryEntry(tables, theory, relation)
			const txHandle = resolveTx()
			return mutateCollection(relation, facts, function applyCells(rows, cells) {
				return bridged("bumbledb tx insert", function record() {
					return native.txInsert(txHandle, entry.id, rows, cells)
				})
			})
		}
		function remove<R extends MemberRelation<Rels>>(relation: R, facts: Iterable<Fact<R>>): MutationReport {
			assertLive()
			const entry = ordinaryEntry(tables, theory, relation)
			const txHandle = resolveTx()
			const flat = rowsOf(relation, facts)
			const report = bridged("bumbledb tx delete", function record() {
				return native.txDelete(txHandle, entry.id, flat.rows, flat.cells)
			})
			return Object.freeze({ submitted: report.submitted, changed: report.changed })
		}
		function reserve<R extends MemberRelation<Rels>>(
			relation: R,
			field: FreshKeys<R> & string,
			count: bigint
		): FreshRange {
			assertLive()
			const entry = ordinaryEntry(tables, theory, relation)
			const declared = relation.data.fields.find(function byName(candidate) {
				return candidate.name === field
			})
			if (declared === undefined || !isFreshField(declared.field)) {
				throw errors.new(`relation ${relation.name}: field ${field} is not a fresh cell`)
			}
			const fieldId = entry.fieldIds.get(field)
			if (fieldId === undefined) {
				throw errors.new(`bumbledb manifest drift: relation ${relation.name} has no field id for ${field}`)
			}
			const txHandle = resolveTx()
			const range = bridged("bumbledb tx reserve", function mint() {
				return native.txReserve(txHandle, entry.id, fieldId, count)
			})
			return freshRangeOf(range)
		}
		const tx: WriteTx<Rels> = Object.freeze({
			insert,
			delete: remove,
			reserve,
			contains: reads.contains,
			get: reads.get
		})
		function spend(): void {
			txState.spent = true
		}
		return { tx, spend }
	}

	function mapNativeWrite<R>(nativeOutcome: NativeWriteOutcome, built: R | undefined): WriteFromOutcome<Rels, R> {
		if (nativeOutcome.tag === "moved") {
			return Object.freeze({
				tag: "moved" as const,
				witnessed: nativeOutcome.witnessed,
				current: nativeOutcome.current
			})
		}
		if (nativeOutcome.tag === "rejected") {
			return Object.freeze({
				tag: "rejected" as const,
				violations: Object.freeze(nativeOutcome.violations.map(violationOf))
			})
		}
		if (nativeOutcome.tag === "abandoned") {
			if (built === undefined || !isAbandon(built)) {
				throw errors.new("bumbledb write abandoned without an abandon sentinel")
			}
			return abandonedOutcome<Rels, R>(built)
		}
		return Object.freeze({
			tag: "accepted" as const,
			value: Object.freeze({
				value: built as Exclude<R, Abandon<unknown>>,
				generation: nativeOutcome.generation
			})
		})
	}

	function runWrite<R>(
		invoke: (callback: (tx: TxHandle) => boolean) => NativeWriteOutcome,
		fn: (tx: WriteTx<Rels>) => SyncResult<R>
	): WriteFromOutcome<Rels, SyncResult<R>> {
		let built: SyncResult<R> | undefined
		const nativeOutcome = bridged("bumbledb write", function callWrite() {
			return invoke(function onWrite(txHandle) {
				const made = makeTx(function resolveTx() {
					return txHandle
				})
				const result = errors.trySync(function buildDelta() {
					return fn(made.tx)
				})
				made.spend()
				if (result.error) {
					throw errors.wrap(result.error, "build write delta")
				}
				if (isThenable(result.data)) {
					throw errors.wrap(ErrAsyncCallback, "bumbledb write callback returned a thenable")
				}
				built = result.data
				return !isAbandon(result.data)
			})
		})
		return mapNativeWrite(nativeOutcome, built)
	}

	function write<R>(fn: (tx: WriteTx<Rels>) => SyncResult<R>): WriteOutcome<Rels, SyncResult<R>> {
		const outcome = runWrite(function invoke(callback) {
			return native.dbWrite(handle, callback)
		}, fn)
		if (outcome.tag === "moved") {
			throw errors.new("bumbledb write reported moved — unconditional writes cannot move")
		}
		return outcome
	}

	function writeFrom<R>(
		witness: Witness<Rels>,
		fn: (tx: WriteTx<Rels>) => SyncResult<R>
	): WriteFromOutcome<Rels, SyncResult<R>> {
		const state = witnessStates.get(witness)
		if (state === undefined) {
			throw errors.wrap(ErrForeignWitness, "bumbledb writeFrom witness is not a witness of this SDK")
		}
		if (state.owner !== owner) {
			throw errors.wrap(
				ErrForeignWitness,
				`bumbledb writeFrom witness belongs to a different store (schema ${theory.name})`
			)
		}
		if (state.spent) {
			throw errors.wrap(ErrSpentHandle, "bumbledb writeFrom witness has been disposed")
		}
		return runWrite(function invoke(callback) {
			return native.dbWriteFrom(handle, state.handle, callback)
		}, fn)
	}

	function prepare<Row, Params extends ParamsRecord>(q: Query<Rels, Row, Params>): Prepared<Rels, Row, Params> {
		if (q.schema !== theory) {
			throw errors.new(
				`query was built against schema ${q.schema.name}, not the identical schema value this store opened with — schema identity is the membership rule`
			)
		}
		const queryIr = lowerQuery(q)
		const outcome = bridged("prepare bumbledb query", function callPrepare() {
			return native.dbPrepare(handle, queryIr)
		})
		if (!outcome.ok) {
			throwPrepareRefusal(outcome.message)
		}
		return pinPrepared(outcome.prepared, q)
	}

	return Object.freeze({
		schema: theory,
		read,
		write,
		writeFrom,
		prepare
	})
}

/**
 * The engine twin of the schema-level class wall, as a matchable value
 * (`errors.is`): the shared lowering rejected a spec whose statement pairs
 * faces with disagreeing newtype labels — the faces of a dependency agree
 * on their newtype, or neither carries one. UNREACHABLE through the typed
 * builder (the SDK computes every label from the laws, so its lowered
 * specs cohere by construction); a raw spec handed to the bridge is the
 * one road here, and the runtime referee that proves the engine judges
 * what the types claim.
 */
const ErrNewtypeMismatch = errors.new(
	"bumbledb newtypeMismatch: a statement pairs faces whose newtypes disagree — the faces of a dependency agree on their newtype, or neither carries one"
)
const ErrSchemaError = errors.new("bumbledb schemaError: the declaration failed validation")
const ErrFingerprintMismatch = errors.new("bumbledb fingerprintMismatch: the store's schema does not match this theory")
const ErrIrError = errors.new("bumbledb irError: the query failed validation")

function throwOpenRefusal(
	verb: string,
	canonical: string,
	kind: "schemaError" | "newtypeMismatch" | "fingerprintMismatch",
	message: string
): never {
	const detail = `${verb} ${canonical}: ${message}`
	if (kind === "newtypeMismatch") {
		throw errors.wrap(ErrNewtypeMismatch, detail)
	}
	if (kind === "schemaError") {
		throw errors.wrap(ErrSchemaError, detail)
	}
	throw errors.wrap(ErrFingerprintMismatch, detail)
}

function throwPrepareRefusal(message: string): never {
	throw errors.wrap(ErrIrError, `prepare: ${message}`)
}

function openFromHandle<Rels extends SchemaRelations>(dbHandle: DbHandle, theory: Schema<Rels>): Db<Rels> {
	const manifest = bridged("fetch bumbledb manifest", function fetchManifest() {
		return native.dbManifest(dbHandle)
	})
	return openDb(dbHandle, theory, manifest)
}

async function createStore<Rels extends SchemaRelations>(
	storePath: string,
	theory: Schema<Rels>
): Promise<Admission<Rels, Db<Rels>>> {
	const canonical = path.resolve(storePath)
	const spec = lower(theory)
	const created = await bridgedAsync(`create bumbledb store at ${canonical}`, function callBridge() {
		return native.dbCreate(canonical, spec)
	})
	if (created.tag === "schemaError" || created.tag === "newtypeMismatch") {
		throwOpenRefusal("create", canonical, created.tag, created.message)
	}
	if (created.tag === "rejected") {
		return Object.freeze({
			tag: "rejected" as const,
			violations: Object.freeze(
				created.violations.map(function mapWire(wire) {
					return mapViolationWithoutStore<Rels>(theory, wire)
				})
			)
		})
	}
	return Object.freeze({ tag: "accepted" as const, value: openFromHandle(created.db, theory) })
}

function mapViolationWithoutStore<Rels extends SchemaRelations>(
	theory: Schema<Rels>,
	wire: WireViolation
): Violation<Rels> {
	const entries = materializedEntries(theory)
	const entry = entries[wire.statementId]
	if (entry === undefined) {
		throw errors.new(`bumbledb violation cites unknown statement id ${wire.statementId}`)
	}
	function offending(fact: WireViolationFact): OffendingFact<Rels> {
		const member = theory.relations[fact.relation]
		if (member === undefined || !(fact.relation in theory.relations)) {
			throw errors.new(`bumbledb violation cites unknown relation ${fact.relation}`)
		}
		const declared = sealedFieldsOf(member)
		const decoded: Record<string, FactValue> = {}
		for (const cell of fact.fields) {
			const cited = declared.find(function byName(candidate) {
				return candidate.name === cell.name
			})
			const roster = rosterOf(cited?.field)
			decoded[cell.name] =
				roster !== undefined
					? handleOf(`violation fact ${fact.relation} field ${cell.name}`, roster, cell.value)
					: cell.value
		}
		return Object.freeze({ relation: fact.relation as keyof Rels & string, fact: Object.freeze(decoded) })
	}
	const facts = Object.freeze(wire.facts.map(offending))
	const canonical = wire.canonical
	if (entry.kind === "functionality") {
		if (!("statement" in entry)) {
			return Object.freeze({ kind: "functionality", statement: undefined, canonical, facts })
		}
		return Object.freeze({ kind: "functionality", statement: entry.statement, canonical, facts })
	}
	if (entry.kind === "capacity") {
		if (wire.kind !== "capacity") {
			throw errors.new(`bumbledb violation ${wire.statementId} is a capacity slot without a measure`)
		}
		return Object.freeze({
			kind: "capacity",
			statement: entry.statement,
			canonical,
			measure: wire.measure,
			facts
		})
	}
	if (wire.kind !== "containment") {
		throw errors.new(`bumbledb violation ${wire.statementId} is a containment slot without a direction`)
	}
	if (entry.kind === "mirrors") {
		return Object.freeze({
			kind: "containment",
			statement: entry.statement,
			canonical,
			direction: wire.direction,
			orientation: entry.orientation,
			facts
		})
	}
	return Object.freeze({
		kind: "containment",
		statement: entry.statement,
		canonical,
		direction: wire.direction,
		facts
	})
}

async function openStore<Rels extends SchemaRelations>(storePath: string, theory: Schema<Rels>): Promise<Db<Rels>> {
	const canonical = path.resolve(storePath)
	const spec = lower(theory)
	const opened = await bridgedAsync(`open bumbledb store at ${canonical}`, function callBridge() {
		return native.dbOpen(canonical, spec)
	})
	if (!opened.ok) {
		throwOpenRefusal("open", canonical, opened.kind, opened.message)
	}
	return openFromHandle(opened.db, theory)
}

interface OwnedInstance<Rels extends SchemaRelations> extends Disposable {
	prepare<Row, Params extends ParamsRecord>(q: Query<Rels, Row, Params>): Prepared<Rels, Row, Params>
	execute<Row, Params extends ParamsRecord>(prepared: Prepared<Rels, Row, Params>, params: Params): Row[]
	scan<R extends MemberRelation<Rels>>(relation: R): Fact<R>[]
	count<R extends MemberRelation<Rels>>(relation: R): bigint
	contains<R extends MemberRelation<Rels>>(relation: R, fact: Fact<R>): boolean
	get<R extends MemberRelation<Rels>>(relation: R, key: KeyFact<R>): Fact<R> | undefined
	get<R extends MemberRelation<Rels>, const P extends readonly string[]>(
		relation: R,
		keyStatement: KeyStatement<R, P>,
		key: DeclaredKeyFact<R, P>
	): Fact<R> | undefined
}

interface InstanceBuilder<Rels extends SchemaRelations> extends Disposable {
	load<R extends MemberRelation<Rels>>(relation: R, facts: CollectionWrite<R>): MutationReport
	delete<R extends MemberRelation<Rels>>(relation: R, facts: Iterable<Fact<R>>): MutationReport
	reserve<R extends MemberRelation<Rels>>(relation: R, field: FreshKeys<R> & string, count: bigint): FreshRange
	contains<R extends MemberRelation<Rels>>(relation: R, fact: Fact<R>): boolean
	get<R extends MemberRelation<Rels>>(relation: R, key: KeyFact<R>): Fact<R> | undefined
	get<R extends MemberRelation<Rels>, const P extends readonly string[]>(
		relation: R,
		keyStatement: KeyStatement<R, P>,
		key: DeclaredKeyFact<R, P>
	): Fact<R> | undefined
	admit(): Promise<Admission<Rels, OwnedInstance<Rels>>>
}

const ownedRecords = new WeakMap<object, { handle: OwnedHandle; theory: AnySchema; spent: boolean; owner: object }>()
const builderRecords = new WeakMap<object, { handle: BuilderHandle; theory: AnySchema; spent: boolean }>()

const ownedReclaimer = new FinalizationRegistry<OwnedHandle>(function reclaimOwned(handle) {
	const closed = errors.trySync(function closeOwned() {
		native.ownedInstanceClose(handle)
	})
	if (closed.error) {
		return
	}
})

const builderReclaimer = new FinalizationRegistry<BuilderHandle>(function reclaimBuilder(handle) {
	const closed = errors.trySync(function closeBuilder() {
		native.instanceBuilderClose(handle)
	})
	if (closed.error) {
		return
	}
})

function wrapOwned<Rels extends SchemaRelations>(nativeHandle: OwnedHandle, theory: Schema<Rels>): OwnedInstance<Rels> {
	const owner = Object.freeze({})
	const rec = { handle: nativeHandle, theory, spent: false, owner }
	const tables = tablesFromTheory(theory)
	function assertLive(): void {
		if (rec.spent) {
			throw errors.wrap(ErrSpentHandle, "bumbledb owned instance has been disposed")
		}
	}
	const methods = catalogMethods(theory, tables, owner, assertLive, {
		scan(relationId) {
			return native.ownedScan(nativeHandle, relationId)
		},
		count(relationId) {
			return native.ownedCount(nativeHandle, relationId)
		},
		contains(relationId, values) {
			return native.ownedContains(nativeHandle, relationId, values)
		},
		get(relationId, statementId, keyValues) {
			return native.ownedGet(nativeHandle, relationId, statementId, keyValues)
		},
		prepare(query) {
			return native.ownedPrepare(nativeHandle, query)
		},
		execute(prepared, params) {
			return native.ownedExecute(prepared, nativeHandle, params)
		}
	})
	const instance: OwnedInstance<Rels> = Object.freeze({
		...methods,
		[Symbol.dispose](): void {
			if (rec.spent) {
				return
			}
			try {
				native.ownedInstanceClose(nativeHandle)
			} catch (caught) {
				const error = errorFromThrow(caught)
				if (/leased for publish/.test(error.message)) {
					throw errors.wrap(ErrSpentHandle, "bumbledb owned instance is leased for publish")
				}
				throw errors.wrap(error, "close bumbledb owned instance")
			}
			rec.spent = true
			ownedReclaimer.unregister(instance)
		}
	})
	ownedRecords.set(instance, rec)
	ownedReclaimer.register(instance, nativeHandle, instance)
	return instance
}

function wrapBuilder<Rels extends SchemaRelations>(
	nativeHandle: BuilderHandle,
	theory: Schema<Rels>
): InstanceBuilder<Rels> {
	const rec = { handle: nativeHandle, theory, spent: false }
	const tables = tablesFromTheory(theory)
	function assertLive(): void {
		if (rec.spent) {
			throw errors.wrap(ErrSpentHandle, "bumbledb instance builder has been spent")
		}
	}
	const overlay = overlayMethods(theory, tables, assertLive, {
		contains(relationId, row) {
			return bridged("bumbledb builder contains", function readContains() {
				return native.instanceBuilderContains(nativeHandle, relationId, row)
			})
		},
		get(relationId, statementId, key) {
			return bridged("bumbledb builder get", function readGet() {
				return native.instanceBuilderGet(nativeHandle, relationId, statementId, key)
			})
		}
	})
	const builder: InstanceBuilder<Rels> = Object.freeze({
		load<R extends MemberRelation<Rels>>(relation: R, facts: CollectionWrite<R>): MutationReport {
			assertLive()
			const entry = ordinaryEntry(tables, theory, relation)
			return mutateCollection(relation, facts, function applyCells(rows, cells) {
				return bridged("bumbledb builder load", function loadCells() {
					return native.instanceBuilderLoad(nativeHandle, entry.id, rows, cells)
				})
			})
		},
		delete<R extends MemberRelation<Rels>>(relation: R, facts: Iterable<Fact<R>>): MutationReport {
			assertLive()
			const entry = ordinaryEntry(tables, theory, relation)
			const flat = rowsOf(relation, facts)
			const report = bridged("bumbledb builder delete", function remove() {
				return native.instanceBuilderDelete(nativeHandle, entry.id, flat.rows, flat.cells)
			})
			return Object.freeze({ submitted: report.submitted, changed: report.changed })
		},
		reserve<R extends MemberRelation<Rels>>(relation: R, field: FreshKeys<R> & string, count: bigint): FreshRange {
			assertLive()
			const entry = ordinaryEntry(tables, theory, relation)
			const declared = relation.data.fields.find(function byName(candidate) {
				return candidate.name === field
			})
			if (declared === undefined || !isFreshField(declared.field)) {
				throw errors.new(`relation ${relation.name}: field ${field} is not a fresh cell`)
			}
			const fieldId = entry.fieldIds.get(field)
			if (fieldId === undefined) {
				throw errors.new(`bumbledb manifest drift: relation ${relation.name} has no field id for ${field}`)
			}
			const range = bridged("bumbledb builder reserve", function mint() {
				return native.instanceBuilderReserve(nativeHandle, entry.id, fieldId, count)
			})
			return freshRangeOf(range)
		},
		contains: overlay.contains,
		get: overlay.get,
		async admit(): Promise<Admission<Rels, OwnedInstance<Rels>>> {
			if (rec.spent) {
				throw errors.wrap(ErrSpentHandle, "bumbledb instance builder has been spent")
			}
			rec.spent = true
			builderReclaimer.unregister(builder)
			let outcome: AdmitResult
			try {
				outcome = await native.instanceBuilderAdmit(nativeHandle)
			} catch (caught) {
				throw errors.wrap(errorFromThrow(caught), "admit bumbledb instance")
			}
			if (outcome.tag === "rejected") {
				return Object.freeze({
					tag: "rejected" as const,
					violations: Object.freeze(
						outcome.violations.map(function mapWire(wire) {
							return mapViolationWithoutStore<Rels>(theory, wire)
						})
					)
				})
			}
			return Object.freeze({
				tag: "accepted" as const,
				value: wrapOwned(outcome.value, theory)
			})
		},
		[Symbol.dispose](): void {
			if (rec.spent) {
				return
			}
			rec.spent = true
			builderReclaimer.unregister(builder)
			bridged("close bumbledb instance builder", function closeBuilder() {
				native.instanceBuilderClose(nativeHandle)
			})
		}
	})
	builderRecords.set(builder, rec)
	builderReclaimer.register(builder, nativeHandle, builder)
	return builder
}

const InstanceBuilder = Object.freeze({
	create<Rels extends SchemaRelations>(theory: Schema<Rels>): InstanceBuilder<Rels> {
		const spec = lower(theory)
		const handle = bridged("create bumbledb instance builder", function make() {
			return native.instanceBuilderNew(spec)
		})
		return wrapBuilder(handle, theory)
	}
})

/**
 * The store lifecycle — `Db.create(path, schema)` / `Db.open(path, schema)`.
 * Create refuses an already-initialized directory; open verifies format
 * version and the schema fingerprint. A second live handle on the same
 * path is the engine's `EnvironmentLocked`. There is no close anywhere:
 * the process owns the environment until GC/exit (durability is the
 * engine's per-commit fsync). Resume = reopen in a fresh process, or
 * hold the `Db` this process opened.
 */
const Db = Object.freeze({
	/** Creates a fresh durable store at `path` from the schema. */
	async create<Rels extends SchemaRelations>(
		storePath: string,
		theory: Schema<Rels>
	): Promise<Admission<Rels, Db<Rels>>> {
		return createStore(storePath, theory)
	},
	/**
	 * Opens an existing durable store at `path` with the same theory.
	 * Format 8 open never back-fills a descriptor. A second open of a
	 * still-live path is `EnvironmentLocked`.
	 */
	async open<Rels extends SchemaRelations>(storePath: string, theory: Schema<Rels>): Promise<Db<Rels>> {
		return openStore(storePath, theory)
	},
	async fromInstance<Rels extends SchemaRelations>(
		storePath: string,
		instance: OwnedInstance<Rels>
	): Promise<Db<Rels>> {
		const rec = ownedRecords.get(instance)
		if (rec === undefined) {
			throw errors.wrap(ErrSpentHandle, "bumbledb fromInstance target is not an owned instance of this SDK")
		}
		if (rec.spent) {
			throw errors.wrap(ErrSpentHandle, "bumbledb fromInstance target has been disposed")
		}
		const canonical = path.resolve(storePath)
		const dbHandle = await bridgedAsync(`publish bumbledb instance at ${canonical}`, function publish() {
			return native.dbFromInstance(canonical, rec.handle)
		})
		return openFromHandle(dbHandle, rec.theory as Schema<Rels>)
	}
})

export type {
	Abandon,
	AbandonedArm,
	Admission,
	CapacityViolation,
	Committed,
	ContainmentViolation,
	DeclaredKeyFact,
	DeclaredKeyViolation,
	DeltaBuild,
	FreshRange,
	ImpliedKeyViolation,
	MemberRelation,
	MirrorViolation,
	MutationReport,
	OffendingFact,
	OwnedInstance,
	Prepared,
	ReadInstance,
	SyncResult,
	Violation,
	Witness,
	WriteFromOutcome,
	WriteOutcome,
	WriteTx
}
export {
	abandon,
	Db,
	ErrAsyncCallback,
	ErrFingerprintMismatch,
	ErrForeignPrepared,
	ErrForeignWitness,
	ErrIrError,
	ErrNewtypeMismatch,
	ErrSchemaError,
	ErrSpentHandle,
	ErrUseAfterScope,
	InstanceBuilder
}
