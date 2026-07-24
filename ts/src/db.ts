/**
 * `Db` — the living half of the SDK (PRD-07): open/create a store from a
 * `Schema`, write typed facts through delta transactions with race-free
 * final-state point reads, receive rejections as typed violation VALUES
 * keyed to statements, read through scoped snapshots, and run the witnessed
 * read-compute-write loop — all typed by the schema's relations record.
 *
 * ZERO CLOSABLES: no value this module returns carries a close, dispose, or
 * release spelling. `Db` values are CACHED per canonical path for the life
 * of the process (a best-effort exit hook closes the cached environments;
 * correctness never depends on it — the engine fsyncs every commit, so a
 * process that dies without the hook loses nothing that was committed).
 * Snapshots are internal: `read(fn)` opens one before `fn` and closes it
 * after unconditionally, and the {@link ReadScope} handed to `fn` is
 * invalidated the moment `fn` returns. Prepared plans are plain values whose
 * engine-side half is reclaimed by a GC finalizer — reclamation only, never
 * correctness.
 *
 * PROCESS MODEL: one process, one exclusive-lock handle per store. The
 * cached `Db` value owns the LMDB environment's exclusive lock until
 * process exit; a second engine-level open of the same store (an aliased
 * path spelling, or another process) is refused by the engine. The
 * run-store process model (PRD-16) depends on this being true: resume =
 * reopen, which is either this process's cached value or a fresh process's
 * open.
 *
 * REJECTION IS DATA: a rejected commit is a domain outcome (it becomes the
 * LLM repair prompt downstream), returned as a {@link WriteResult} carrying
 * {@link Violation} values. Genuine failures — I/O, used-after-scope,
 * marshal shape — throw `@superbuilders/errors` wrapped errors instead.
 */

import * as path from "node:path"
import * as errors from "@superbuilders/errors"
import { isClosedMember, sealedFieldsOf } from "#closed.ts"
import type { Exhumed } from "#exhume.ts"
import { exhumeStore } from "#exhume.ts"
import { rosterOf } from "#fields.ts"
import { lower } from "#lower.ts"
import {
	factOf,
	handleOf,
	isFreshField,
	isMintedFresh,
	type KeyFact,
	keyRowOf,
	type Minted,
	recordOf,
	rowOf
} from "#marshal.ts"

import type {
	DbHandle,
	FactValue,
	Manifest,
	PreparedHandle,
	SnapshotHandle,
	Staleness,
	StatementKindTag,
	TxHandle,
	Violation as WireViolation,
	ViolationFact as WireViolationFact
} from "#native.ts"
import { bridged, native } from "#native.ts"
import type { FindColumn } from "#query/atom.ts"
import type { Query } from "#query/lower.ts"
import { lowerQuery } from "#query/lower.ts"
import { decodeAnswers, wireParams } from "#query/run.ts"
import type { ParamEntry, ParamsRecord } from "#query/scope.ts"
import type { AnyRelation, Fact, InsertFact } from "#relation.ts"
import type { AnySchema, Schema, SchemaRelation, SchemaRelations } from "#schema.ts"
import { isStatement, type KeyStatement, type Statement } from "#statements.ts"

/**
 * The ordinary (writable, scannable) relations of a schema's record — the
 * only values the runtime methods accept: closed relations lack the
 * relation shape entirely, so passing one is a type error.
 */
type MemberRelation<Rels extends SchemaRelations> = Extract<Rels[keyof Rels], AnyRelation>

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
 * One violated statement of a rejected commit, as a typed value.
 * `statement` is the IDENTICAL SDK statement value the schema declared —
 * consumers `===`-match it against their own constants; it is `undefined`
 * exactly for the engine-materialized fresh-implied and closed auto-keys,
 * which have no declared spelling (`schema()` rejects an explicit
 * duplicate of them). `canonical` is the ENGINE's rendering of the
 * violated materialized statement — for a `mirrors` statement BOTH
 * materialized slots render as the one `==` utterance in the written
 * orientation (identical strings; the engine's `render.rs` renders each
 * partner of a mirrored pair as the `==` spelling, never a bare `<=`
 * direction). `direction` (`sourceUnsatisfied` | `targetRequired`) and
 * `count` are the containment/window form payloads, passed through from
 * the engine VERBATIM — `direction` is relative to the violated SLOT's
 * own orientation, so for a `mirrors` statement it alone cannot say which
 * side of the `==` was violated: the slot identity is carried by
 * `orientation`, present exactly for `mirrors` slots — `written` is the
 * `source <= target` slot as the statement was spelled, `mirrored` the
 * engine-materialized `target <= source` partner.
 */
interface Violation<Rels extends SchemaRelations> {
	readonly statement: Statement | undefined
	readonly kind: StatementKindTag
	readonly canonical: string
	readonly direction?: "sourceUnsatisfied" | "targetRequired"
	readonly orientation?: "written" | "mirrored"
	readonly count?: bigint
	readonly facts: readonly OffendingFact<Rels>[]
}

/**
 * A commit's domain outcome: the committed generation, or the COMPLETE
 * violation set (every violated statement cited once, per direction for a
 * containment, in materialized statement order). Narrows on `.ok`.
 */
type WriteResult<Rels extends SchemaRelations> =
	| { readonly ok: true; readonly generation: bigint }
	| { readonly ok: false; readonly violations: readonly Violation<Rels>[] }

/** The delta-building callback of a write: runs synchronously against the live transaction. */
type DeltaBuild<Rels extends SchemaRelations> = (tx: Tx<Rels>) => void

/**
 * The runtime discriminant of {@link Abandon} values — a property probe is
 * how `writeWitnessed` distinguishes "abort without committing" from an
 * ordinary callback result, never a guess about the host's own value shapes.
 */
const abandonMark: unique symbol = Symbol("bumbledb.abandon")

/**
 * The abandon sentinel {@link abandon} builds: returning one from a
 * `writeWitnessed` callback aborts the attempt WITHOUT committing (no empty
 * commit is ever issued) and surfaces the payload as
 * `{ ok: false, abandoned: payload }`.
 */
interface Abandon<P> {
	readonly [abandonMark]: true
	readonly payload: P
}

/**
 * Wraps a payload in the {@link Abandon} sentinel — the one way a
 * `writeWitnessed` callback declines to commit: `return abandon(payload)`
 * aborts the delta (nothing is committed, not even an empty commit) and the
 * write resolves to `{ ok: false, abandoned: payload }`.
 */
function abandon<P>(payload: P): Abandon<P> {
	return Object.freeze({ [abandonMark]: true as const, payload })
}

/**
 * The abandon payload type a `writeWitnessed` callback's return type
 * implies: the payload of its `Abandon` arm, `never` when the callback can
 * never abandon (the `abandoned` outcome is then statically unreachable).
 */
type AbandonedPayload<R> = R extends Abandon<infer P> ? P : never

/**
 * Narrows a `writeWitnessed` callback result to the abandon sentinel. The
 * probe is the private {@link abandonMark} symbol only {@link abandon} sets,
 * and `R`'s `Abandon` arm is the only way a sentinel can flow out of the
 * callback — so the narrowed payload type is sound by construction.
 */
function isAbandon<R>(value: R): value is R & Abandon<AbandonedPayload<R>> {
	return typeof value === "object" && value !== null && abandonMark in value
}

/**
 * `writeWitnessed`'s domain outcome: the committed generation, the COMPLETE
 * engine violation set (rejection-as-data, exactly {@link WriteResult}'s
 * false arm), or the callback's own abandon payload. Narrows on `.ok`, then
 * on `"violations" in result`.
 */
type WitnessedWriteResult<Rels extends SchemaRelations, R> =
	| { readonly ok: true; readonly generation: bigint }
	| { readonly ok: false; readonly violations: readonly Violation<Rels>[] }
	| { readonly ok: false; readonly abandoned: AbandonedPayload<R> }

/**
 * One live write transaction: the submitted delta with the engine's
 * FINAL-STATE point-read view (base + pending delta — the exact state the
 * commit judgment judges, so check-then-act is race-free by construction).
 * Spent when its owning `write`/`writeWitnessed` call resolves the attempt;
 * any later use throws.
 */
interface Tx<Rels extends SchemaRelations> {
	/**
	 * Records one insert. Omitted fresh fields are MINTED through the
	 * engine's alloc lane and returned as bare bigints; supplying them instead
	 * preserves identity (the resupply idiom). Returns the relation's
	 * fresh cells, minted or resupplied.
	 */
	insert<R extends MemberRelation<Rels>>(relation: R, fact: InsertFact<R>): Minted<R>
	/** Records one delete; `true` iff the final state changed. */
	delete<R extends MemberRelation<Rels>>(relation: R, fact: Fact<R>): boolean
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

/**
 * The read view one `db.read(fn)` call scopes: an MVCC snapshot pinned at
 * its generation, valid EXACTLY for the synchronous extent of `fn`. The
 * value is invalidated when `fn` returns — every later verb call throws a
 * typed used-after-scope error; the underlying snapshot (and its LMDB
 * reader slot) is already closed. No close spelling exists here because
 * there is nothing the host could ever need to close.
 */
interface ReadScope<Rels extends SchemaRelations> {
	/**
	 * The committed generation this scope witnessed — captured atomically
	 * with the snapshot: writes are synchronous and this process holds the
	 * store's only write handle, so nothing can commit between the snapshot
	 * open and the generation read.
	 */
	readonly generation: bigint
	/** Full-relation export in row-id order, decoded to bare structural facts. */
	scan<R extends MemberRelation<Rels>>(relation: R): Fact<R>[]
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
	 * Executes a prepared query against this scope's snapshot with the
	 * typed params object; returns the answer SET as plain rows with
	 * bare structural values (no order — the host sorts). This is the ONE
	 * execution spelling ({@link Prepared} carries no `execute`).
	 */
	execute<Row, Params extends ParamsRecord>(prepared: Prepared<Rels, Row, Params>, params: Params): Row[]
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
 * `snap.execute(prepared, params)` / `db.execute(prepared, params)` — the
 * symmetry rule's one spelling. The engine-side plan is reclaimed by a GC
 * finalizer when this value becomes unreachable (reclamation only, never
 * correctness — an unreclaimed plan is idle memory, and process exit frees
 * everything).
 */
interface Prepared<Rels extends SchemaRelations, Row, Params extends ParamsRecord> {
	/**
	 * The pull-based plan-drift report against a read scope's snapshot —
	 * engine-policy-free: no threshold exists engine-side; the host owns
	 * re-prepare.
	 */
	staleness(snap: ReadScope<Rels>): Staleness
	readonly [preparedTypes]?: { readonly row: Row; readonly params: Params }
}

/**
 * An open store, cached per canonical path for the life of the process.
 * There is no close: read through `read`/the read sugar, write through
 * `write`/`writeWitnessed`, and let the process own the environment's
 * lifetime (the engine fsyncs every commit, so durability never waits on a
 * close).
 */
interface Db<Rels extends SchemaRelations> {
	/** The theory this store was opened with (fingerprint-verified by the engine). */
	readonly schema: Schema<Rels>
	/**
	 * One scoped snapshot read: opens an MVCC snapshot, runs `fn`
	 * SYNCHRONOUSLY against it, and closes the snapshot unconditionally
	 * before returning `fn`'s result. The {@link ReadScope} is invalidated
	 * when `fn` returns — a used-after-scope call throws a typed error.
	 */
	read<T>(fn: (snap: ReadScope<Rels>) => T): T
	/** `db.scan(r)` === `db.read(snap => snap.scan(r))` — the symmetry rule. */
	scan<R extends MemberRelation<Rels>>(relation: R): Fact<R>[]
	/** `db.get(r, k)` === `db.read(snap => snap.get(r, k))` — the symmetry rule. */
	get<R extends MemberRelation<Rels>>(relation: R, key: KeyFact<R>): Fact<R> | undefined
	/** `db.get(r, s, k)` === `db.read(snap => snap.get(r, s, k))` — the symmetry rule, keyed form. */
	get<R extends MemberRelation<Rels>, const P extends readonly string[]>(
		relation: R,
		keyStatement: KeyStatement<R, P>,
		key: DeclaredKeyFact<R, P>
	): Fact<R> | undefined
	/** `db.contains(r, f)` === `db.read(snap => snap.contains(r, f))` — the symmetry rule. */
	contains<R extends MemberRelation<Rels>>(relation: R, fact: Fact<R>): boolean
	/** `db.execute(p, params)` === `db.read(snap => snap.execute(p, params))` — the symmetry rule. */
	execute<Row, Params extends ParamsRecord>(prepared: Prepared<Rels, Row, Params>, params: Params): Row[]
	/**
	 * One delta transaction: builds the delta synchronously through `fn`,
	 * commits, and returns the domain outcome. A throw from `fn` aborts
	 * the delta (LMDB untouched) and rethrows wrapped.
	 */
	write(fn: DeltaBuild<Rels>): WriteResult<Rels>
	/**
	 * The ONE witnessed-write form: snapshot → `fn` (premise reads via
	 * `snap`, delta via `tx`) → witnessed commit, which lands only if no
	 * state-changing commit intervened since the snapshot. On a moved
	 * generation the WHOLE `fn` reruns on a fresh snapshot: this process
	 * holds the store's only write handle, so every generation move is
	 * self-inflicted by the host's own interleaved writes, and each rerun
	 * witnesses a strictly newer generation — the benign race converges.
	 * The loop's honesty bound is {@link WITNESSED_ATTEMPT_CAP}: a callback
	 * that moves the generation on EVERY attempt (a plain `db.write` before
	 * its first tx verb) would spin forever, so past the cap the typed
	 * {@link ErrWitnessedLivelock} is thrown instead of a silent loop (the
	 * engine's ruling: the error, never a loop — retry is host policy, and
	 * the cap is that policy's own diagnostic). `fn` may decline to commit
	 * by returning {@link abandon}`(payload)` — the outcome is then
	 * `{ ok: false, abandoned: payload }` and NO commit (not even an empty
	 * one) is issued.
	 */
	writeWitnessed<R>(fn: (snap: ReadScope<Rels>, tx: Tx<Rels>) => R): WitnessedWriteResult<Rels, R>
	/**
	 * Prepares a query value built against THIS schema (identity is the
	 * membership rule): lowers it to the engine IR, pins the plan, and
	 * returns the typed {@link Prepared} value. Every IR roster refusal —
	 * rule caps, strata legality, type rules — is the ENGINE's typed
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
 * One materialized-statement slot as the SDK mirrors it: the form tag, the
 * SDK statement value that lowered to it (`undefined` for the
 * engine-materialized implied keys), and — for functionality forms — the
 * key's owner and projection (what keyed point reads resolve through).
 */
interface StatementEntry {
	readonly kind: StatementKindTag
	readonly statement: Statement | undefined
	readonly key: { readonly owner: string; readonly projection: readonly string[] } | undefined
	/**
	 * The slot's orientation relative to the written statement — set exactly
	 * for the two slots of a `mirrors` (`false` = the written `source <=
	 * target`, `true` = the materialized `target <= source` partner), so a
	 * violation can say WHICH side of the `==` was violated.
	 */
	readonly reversed?: boolean
}

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
					statement: undefined,
					key: { owner: member.name, projection: [declared.name] }
				})
			}
		}
	}
	for (const member of Object.values(theory.relations)) {
		if (isClosedMember(member)) {
			entries.push({
				kind: "functionality",
				statement: undefined,
				key: { owner: member.name, projection: ["id"] }
			})
		}
	}
	return entries
}

/**
 * One declared statement's materialized slots: a key or window occupies
 * one, a `mirrors` occupies two adjacent slots (the engine lowers `==` to
 * two containments, `source <= target` first), both owned by the one SDK
 * value.
 */
function declaredEntries(statement: Statement): StatementEntry[] {
	const data = statement.data
	switch (data.kind) {
		case "key": {
			return [
				{
					kind: "functionality",
					statement,
					key: { owner: data.owner.name, projection: data.projection }
				}
			]
		}
		case "containment": {
			if (data.bidirectional) {
				return [
					{ kind: "containment", statement, key: undefined, reversed: false },
					{ kind: "containment", statement, key: undefined, reversed: true }
				]
			}
			return [{ kind: "containment", statement, key: undefined }]
		}
		case "window": {
			return [{ kind: "cardinality", statement, key: undefined }]
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
 * continuation. `Db.get` and the read scope's `get` both dispatch through
 * here, so the two mismatch refusals speak with one voice and the symmetry
 * rule (`db.get(...) === db.read(snap => snap.get(...))`) holds by
 * construction.
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

/** Maps a slot's reversal flag to the violation's `orientation` payload. */
function orientationOf(reversed: boolean | undefined): "written" | "mirrored" | undefined {
	if (reversed === undefined) {
		return undefined
	}
	if (reversed) {
		return "mirrored"
	}
	return "written"
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
		if (entry === undefined || statement.id !== index || entry.kind !== statement.kind) {
			throw errors.new(
				`bumbledb manifest drift: statement ${statement.id} is ${statement.kind}, the SDK mirror at ${index} expected ${entry?.kind}`
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
			if (primaryKey === undefined && entry.key !== undefined && entry.key.owner === relation.name) {
				primaryKey = Object.freeze({ statementId: index, projection: entry.key.projection })
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

/** The point-read half a transaction and a read scope share, over their own handle. */
interface PointReads {
	contains(relationId: number, row: readonly FactValue[]): boolean
	get(relationId: number, statementId: number, key: readonly FactValue[]): FactValue[] | null
}

/**
 * One read scope's PRIVATE lifetime record: its live snapshot handle, its
 * liveness flag (flipped exactly when the owning `read`/`writeWitnessed`
 * callback returns), and its owning store's identity token. Held in
 * {@link scopeStates} — the snapshot handle is never a public value.
 */
interface ScopeState {
	readonly handle: SnapshotHandle
	live: boolean
	readonly owner: object
}

/** The private lifetime records of this module's read scopes. */
const scopeStates = new WeakMap<object, ScopeState>()

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

/**
 * The internal retry signal a lazily-witnessed transaction throws when the
 * engine reports a moved generation at begin: `writeWitnessed` catches it
 * by identity (through cause chains, via `errors.is`) and reruns the whole
 * callback on a fresh snapshot. It never escapes the SDK.
 */
const generationMovedSignal = errors.new("bumbledb witnessed generation moved")

/**
 * The witnessed loop's attempt cap — a generous power of two. Benign
 * self-inflicted contention (the host's own commits landing between an
 * attempt's snapshot and its witnessed begin) converges in a handful of
 * retries because each rerun reads a FRESHER snapshot; a workload that moves
 * the generation on EVERY one of this many consecutive attempts is not
 * converging and never will (see {@link ErrWitnessedLivelock}).
 */
const WITNESSED_ATTEMPT_CAP = 64

/**
 * The typed livelock refusal `writeWitnessed` throws past
 * {@link WITNESSED_ATTEMPT_CAP} attempts: every attempt found the generation
 * moved, which is only sustainable when the callback ITSELF (even
 * indirectly) issues an interleaved plain `db.write` before its first tx
 * verb on every attempt — each rerun then re-moves the generation it is
 * about to witness, forever. That is host-policy pathology, not engine
 * judgment (the engine ships the error, never a loop), so it THROWS rather
 * than returning a result arm. Match with `errors.is`; the remedy is to
 * move the interleaved write out of the callback (or make it first-attempt
 * only — the delta belongs on `tx`, premise reads on `snap`).
 */
const ErrWitnessedLivelock = errors.new(
	"bumbledb writeWitnessed livelock: the generation moved on every attempt — the callback itself commits an interleaved write each try, so no snapshot can ever stay current"
)

/**
 * Fills one insert's omitted fresh cells through the engine's
 * alloc-then-insert dyn lane (there is no insert-with-omitted-fields wire
 * spelling) and collects every fresh cell — minted or resupplied — for the
 * insert's return. Mutates `values` in place with the minted cells.
 */
function mintFreshCells(
	txHandle: TxHandle,
	entry: RelationEntry,
	relation: AnyRelation,
	values: Record<string, unknown>
): Record<string, FactValue> {
	const fresh: Record<string, FactValue> = {}
	for (const declared of relation.data.fields) {
		if (!isFreshField(declared.field)) {
			continue
		}
		let cell = values[declared.name]
		if (cell === undefined) {
			const fieldId = entry.fieldIds.get(declared.name)
			if (fieldId === undefined) {
				throw errors.new(`bumbledb manifest drift: relation ${relation.name} has no field id for ${declared.name}`)
			}
			cell = bridged("bumbledb tx alloc", function mint() {
				return native.txAlloc(txHandle, entry.id, fieldId)
			})
			values[declared.name] = cell
		}
		if (typeof cell !== "bigint") {
			throw errors.new(
				`relation ${relation.name} field ${declared.name}: a fresh cell is a u64 bigint, got ${typeof cell}`
			)
		}
		fresh[declared.name] = cell
	}
	return fresh
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

	function resolveOrdinary(relation: AnyRelation): RelationEntry {
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
		return Object.freeze({
			statement: entry.statement,
			kind: wire.kind,
			canonical: wire.canonical,
			direction: wire.direction,
			orientation: orientationOf(entry.reversed),
			count: wire.count,
			facts: Object.freeze(wire.facts.map(offendingFactOf))
		})
	}

	/**
	 * Resolves a key-statement-selected read: the statement must be the
	 * IDENTICAL `key()` value this schema declared (identity is the
	 * membership rule) and must key `relation` — its materialized statement
	 * id comes from the positional mirror, so the engine point-reads through
	 * exactly the declared projection.
	 */
	function declaredKeyOf(relation: AnyRelation, statement: Statement): PrimaryKey {
		const statementId = tables.statements.findIndex(function byIdentity(candidate) {
			return candidate.statement === statement
		})
		const entry = tables.statements[statementId]
		if (entry === undefined) {
			throw errors.new(
				`keyed get statement is not a declared statement of schema ${theory.name} — statement identity is the membership rule`
			)
		}
		if (entry.kind !== "functionality" || entry.key === undefined) {
			throw errors.new("keyed get takes a key() statement — containments and windows key nothing")
		}
		if (entry.key.owner !== relation.name) {
			throw errors.new(
				`keyed get statement keys ${entry.key.owner}, not ${relation.name} — the statement must be a declared key of the relation it reads`
			)
		}
		return Object.freeze({ statementId, projection: entry.key.projection })
	}

	function pointReadsOf(assertLive: () => void, reads: PointReads) {
		function contains<R extends MemberRelation<Rels>>(relation: R, fact: Fact<R>): boolean {
			assertLive()
			const entry = resolveOrdinary(relation)
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
			const entry = resolveOrdinary(relation)
			return selectKeyRead(
				keyOrStatement,
				declaredKey,
				function byStatement(statement, key) {
					return readThroughKey(relation, entry, declaredKeyOf(relation, statement), recordOf(key))
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

	/**
	 * Resolves a prepared value's private plan, refusing foreign objects
	 * and prepared values of other stores as typed errors.
	 */
	function planOf(prepared: object): PreparedPlan {
		const plan = preparedPlans.get(prepared)
		if (plan === undefined) {
			throw errors.new("bumbledb execute target is not a prepared value of this SDK")
		}
		if (plan.owner !== owner) {
			throw errors.new(
				`bumbledb prepared value was prepared by a different store than this one (schema ${theory.name})`
			)
		}
		return plan
	}

	/**
	 * Builds one {@link ReadScope} over a live scope state. Every verb
	 * asserts liveness first: the owning call flips `state.live` the moment
	 * its callback returns, so a leaked scope is a typed refusal forever
	 * after.
	 */
	function makeScope(state: ScopeState, generation: bigint): ReadScope<Rels> {
		function assertLive(): void {
			if (!state.live) {
				throw errors.new("bumbledb read scope is invalidated — its owning read callback already returned")
			}
		}
		const reads = pointReadsOf(assertLive, {
			contains(relationId, row) {
				return bridged("bumbledb snapshot contains", function readContains() {
					return native.snapshotContains(state.handle, relationId, row)
				})
			},
			get(relationId, statementId, key) {
				return bridged("bumbledb snapshot get", function readGet() {
					return native.snapshotGet(state.handle, relationId, statementId, key)
				})
			}
		})
		function scan<R extends MemberRelation<Rels>>(relation: R): Fact<R>[] {
			assertLive()
			const entry = resolveOrdinary(relation)
			const rows = bridged("bumbledb snapshot scan", function readScan() {
				return native.snapshotScan(state.handle, entry.id)
			})
			return rows.map(function decodeRow(row) {
				return factOf(relation, row)
			})
		}
		function execute<Row, Params extends ParamsRecord>(prepared: Prepared<Rels, Row, Params>, params: Params): Row[] {
			assertLive()
			const plan = planOf(prepared)
			const wire = wireParams(plan.params, recordOf(params))
			const rows = bridged("execute bumbledb prepared query", function callExecute() {
				return native.preparedExecute(plan.handle, state.handle, wire)
			})
			return decodeAnswers<Row>(plan.finds, rows)
		}
		const scope: ReadScope<Rels> = Object.freeze({
			generation,
			scan,
			get: reads.get,
			contains: reads.contains,
			execute
		})
		scopeStates.set(scope, state)
		return scope
	}

	/**
	 * Live-handle accounting (diagnostic law, prod EINVAL 2026-07-17): every
	 * snapshot open/close is counted so a write-begin failure can report how
	 * many read handles were live at the fault — a leaked scope is invisible
	 * until the exact moment it matters, so the failure carries the census.
	 */
	let liveSnapshots = 0

	/** Opens one snapshot and its scope state (live until the owner flips it). */
	function openScopeState(): ScopeState {
		const snapHandle = bridged("open bumbledb snapshot", function openSnapshot() {
			return native.dbSnapshot(handle)
		})
		liveSnapshots += 1
		return { handle: snapHandle, live: true, owner }
	}

	/** Closes a scope's snapshot after the owner invalidated it. */
	function closeScopeState(state: ScopeState): void {
		bridged("close bumbledb snapshot", function closeSnapshot() {
			native.snapshotClose(state.handle)
		})
		liveSnapshots -= 1
	}

	/**
	 * Reads the committed generation for a just-opened scope, closing the
	 * scope's snapshot when the read faults: `dbGeneration` opens a transient
	 * engine read txn, so reader-table exhaustion is precisely the state in
	 * which it throws — with one snapshot already open. An unpaired fault
	 * here would park a snapshot worker and consume one of the engine's
	 * reader slots FOREVER (and undercount the liveSnapshots census), each
	 * fault ratcheting toward ReadersFull-for-the-process's-lifetime.
	 */
	function generationForScope(state: ScopeState): bigint {
		const generation = errors.trySync(function readGeneration() {
			return bridged("read bumbledb generation", function callGeneration() {
				return native.dbGeneration(handle)
			})
		})
		if (generation.error) {
			state.live = false
			closeScopeState(state)
			throw generation.error
		}
		return generation.data
	}

	function read<T>(fn: (snap: ReadScope<Rels>) => T): T {
		const state = openScopeState()
		const generation = generationForScope(state)
		const scope = makeScope(state, generation)
		const result = errors.trySync(function runRead() {
			return fn(scope)
		})
		state.live = false
		closeScopeState(state)
		if (result.error) {
			throw errors.wrap(result.error, "bumbledb read")
		}
		return result.data
	}

	function scan<R extends MemberRelation<Rels>>(relation: R): Fact<R>[] {
		return read(function scanInScope(snap) {
			return snap.scan(relation)
		})
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
		return read(function getInScope(snap) {
			return selectKeyRead(
				keyOrStatement,
				declaredKey,
				function byStatement(statement, key) {
					return snap.get(relation, statement, key)
				},
				function byPrimary(key) {
					return snap.get(relation, key)
				}
			)
		})
	}

	function contains<R extends MemberRelation<Rels>>(relation: R, fact: Fact<R>): boolean {
		return read(function containsInScope(snap) {
			return snap.contains(relation, fact)
		})
	}

	function execute<Row, Params extends ParamsRecord>(prepared: Prepared<Rels, Row, Params>, params: Params): Row[] {
		return read(function executeInScope(snap) {
			return snap.execute(prepared, params)
		})
	}

	/**
	 * Builds one {@link Tx} over a transaction-handle thunk: `write` passes
	 * an already-begun handle; `writeWitnessed` passes a LAZY thunk that
	 * begins the witnessed transaction on the first delta verb (so premise
	 * reads and the host's own interleaved writes can precede it) and
	 * throws {@link generationMovedSignal} when the witness is stale.
	 */
	function makeTx(resolveTx: () => TxHandle): { readonly tx: Tx<Rels>; spend(): void } {
		const txState = { spent: false }
		function assertLive(): void {
			if (txState.spent) {
				throw errors.new("bumbledb write transaction is spent")
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
		function insert<R extends MemberRelation<Rels>>(relation: R, fact: InsertFact<R>): Minted<R> {
			assertLive()
			const entry = resolveOrdinary(relation)
			const txHandle = resolveTx()
			/** The one spread copy of the write path: `mintFreshCells` writes minted cells in place, and they must never land in the caller's own fact object. */
			const values: Record<string, unknown> = { ...recordOf(fact) }
			const fresh = mintFreshCells(txHandle, entry, relation, values)
			const row = rowOf(relation.data, values)
			bridged("bumbledb tx insert", function record() {
				native.txInsert(txHandle, entry.id, row)
			})
			Object.freeze(fresh)
			if (!isMintedFresh(relation, fresh)) {
				throw errors.new(`relation ${relation.name}: minted fresh record is incomplete`)
			}
			return fresh
		}
		function remove<R extends MemberRelation<Rels>>(relation: R, fact: Fact<R>): boolean {
			assertLive()
			const entry = resolveOrdinary(relation)
			const txHandle = resolveTx()
			const row = rowOf(relation.data, recordOf(fact))
			return bridged("bumbledb tx delete", function record() {
				return native.txDelete(txHandle, entry.id, row)
			})
		}
		const tx: Tx<Rels> = Object.freeze({
			insert,
			delete: remove,
			contains: reads.contains,
			get: reads.get
		})
		function spend(): void {
			txState.spent = true
		}
		return { tx, spend }
	}

	function runDelta(txHandle: TxHandle, fn: DeltaBuild<Rels>): WriteResult<Rels> {
		const made = makeTx(function resolveTx() {
			return txHandle
		})
		const built = errors.trySync(function buildDelta() {
			return fn(made.tx)
		})
		made.spend()
		if (built.error) {
			bridged("abort bumbledb write transaction", function abort() {
				native.txAbort(txHandle)
			})
			throw errors.wrap(built.error, "build write delta")
		}
		if (isThenable(built.data)) {
			/**
			 * An `async` callback TYPECHECKS (Promise<void> is assignable where
			 * a `void` return is expected) but its body runs after the tx is
			 * spent: committing here would be a silent EMPTY commit reported
			 * ok while the callback's real inserts throw "spent" as unhandled
			 * rejections. Refused typed instead — abort, nothing committed
			 * (the same one-writer law as the thrown-callback path).
			 */
			bridged("abort bumbledb write transaction", function abort() {
				native.txAbort(txHandle)
			})
			throw errors.new(
				"bumbledb write callback returned a thenable — the delta build is synchronous; an async callback is refused, nothing was committed"
			)
		}
		const committed = errors.trySync(function commitDelta() {
			return bridged("commit bumbledb write transaction", function commit() {
				return native.txCommit(txHandle)
			})
		})
		if (committed.error) {
			/**
			 * A THROWN commit (engine I/O failure, bridge fault) must never
			 * leave the write transaction live: LMDB holds one writer per
			 * environment, and a leaked handle turns every later begin into
			 * EINVAL for the process's lifetime. The abort is best-effort —
			 * the native side may already have consumed the handle.
			 */
			const aborted = errors.trySync(function abortAfterFailedCommit() {
				native.txAbort(txHandle)
			})
			if (aborted.error) {
			}
			throw errors.wrap(committed.error, "commit bumbledb write transaction")
		}
		const outcome = committed.data
		if (outcome.ok) {
			return Object.freeze({ ok: true, generation: outcome.generation })
		}
		return Object.freeze({
			ok: false,
			violations: Object.freeze(outcome.violations.map(violationOf))
		})
	}

	function write(fn: DeltaBuild<Rels>): WriteResult<Rels> {
		const txHandle = bridged(`begin bumbledb write transaction (live snapshots: ${liveSnapshots})`, function begin() {
			return native.dbWriteBegin(handle)
		})
		return runDelta(txHandle, fn)
	}

	/**
	 * Commits an already-begun witnessed transaction and closes the
	 * attempt's snapshot: the committed generation, or the engine's
	 * complete violation set as data.
	 */
	function commitWitnessed<R>(state: ScopeState, txHandle: TxHandle): WitnessedWriteResult<Rels, R> {
		const committed = errors.trySync(function commitWitnessedDelta() {
			return bridged("commit bumbledb witnessed write transaction", function commit() {
				return native.txCommit(txHandle)
			})
		})
		if (committed.error) {
			/** Same one-writer law as `runDelta`: a thrown commit aborts before rethrowing. */
			const aborted = errors.trySync(function abortAfterFailedCommit() {
				native.txAbort(txHandle)
			})
			if (aborted.error) {
			}
			closeScopeState(state)
			throw errors.wrap(committed.error, "commit bumbledb witnessed write transaction")
		}
		const outcome = committed.data
		closeScopeState(state)
		if (outcome.ok) {
			return Object.freeze({ ok: true, generation: outcome.generation })
		}
		return Object.freeze({
			ok: false,
			violations: Object.freeze(outcome.violations.map(violationOf))
		})
	}

	/**
	 * One attempt of the witnessed loop: fresh snapshot, the callback over
	 * its scope and a LAZILY-begun witnessed transaction (the first delta
	 * verb begins it, so premise reads and the host's own interleaved
	 * writes can precede the witness check), then the witnessed commit —
	 * or the abandon abort, which never issues a commit. Returns
	 * `undefined` exactly when the generation moved and the whole callback
	 * must rerun on a fresh snapshot.
	 */
	function witnessedAttempt<R>(
		fn: (snap: ReadScope<Rels>, tx: Tx<Rels>) => R
	): WitnessedWriteResult<Rels, R> | undefined {
		const state = openScopeState()
		const generation = generationForScope(state)
		const scope = makeScope(state, generation)
		const pending: { tx: TxHandle | undefined } = { tx: undefined }
		function beginWitnessed(): TxHandle | undefined {
			const witnessed = bridged("begin witnessed bumbledb write transaction", function begin() {
				return native.dbWriteFrom(handle, state.handle)
			})
			if (!witnessed.ok) {
				return undefined
			}
			return witnessed.tx
		}
		const made = makeTx(function resolveWitnessedTx() {
			if (pending.tx === undefined) {
				const begun = beginWitnessed()
				if (begun === undefined) {
					throw generationMovedSignal
				}
				pending.tx = begun
			}
			return pending.tx
		})
		const built = errors.trySync(function computeWitnessed() {
			return fn(scope, made.tx)
		})
		made.spend()
		state.live = false
		/**
		 * Aborts the pending transaction if one was begun. A faulted abort
		 * still closes the attempt's snapshot BEFORE rethrowing — every
		 * openScopeState is paired with closeScopeState on every exit, or a
		 * reader slot and its snapshot worker leak for the process's lifetime.
		 */
		function abortPending(): void {
			const txHandle = pending.tx
			if (txHandle === undefined) {
				return
			}
			const aborted = errors.trySync(function abort() {
				native.txAbort(txHandle)
			})
			if (aborted.error) {
				closeScopeState(state)
				throw errors.wrap(aborted.error, "abort bumbledb witnessed write transaction")
			}
		}
		if (built.error) {
			abortPending()
			closeScopeState(state)
			if (errors.is(built.error, generationMovedSignal)) {
				return undefined
			}
			throw errors.wrap(built.error, "build witnessed write delta")
		}
		if (isThenable(built.data)) {
			/** The same async-callback refusal as `runDelta` — a thenable means the real delta build races the commit; nothing is committed. */
			abortPending()
			closeScopeState(state)
			throw errors.new(
				"bumbledb writeWitnessed callback returned a thenable — the delta build is synchronous; an async callback is refused, nothing was committed"
			)
		}
		if (isAbandon(built.data)) {
			abortPending()
			closeScopeState(state)
			return Object.freeze({ ok: false, abandoned: built.data.payload })
		}
		const late = errors.trySync(function resolveCommitTx() {
			if (pending.tx === undefined) {
				return beginWitnessed()
			}
			return pending.tx
		})
		if (late.error) {
			/** A faulted late begin must not leak the attempt's snapshot either. */
			closeScopeState(state)
			throw late.error
		}
		const txHandle = late.data
		if (txHandle === undefined) {
			closeScopeState(state)
			return undefined
		}
		return commitWitnessed(state, txHandle)
	}

	/**
	 * The witnessed retry loop. What it retries: the benign race — the
	 * host's OWN interleaved commit landing between an attempt's snapshot
	 * and its witnessed begin (every writer shares this handle, so a move
	 * is always self-inflicted) — by rerunning the WHOLE callback on a
	 * fresh snapshot, which converges because each rerun witnesses a
	 * strictly newer generation. What it refuses: the pathology where the
	 * callback itself (even indirectly) issues a plain `db.write` before
	 * its first tx verb, moving the generation on EVERY attempt — an
	 * unbounded loop would spin forever with no diagnostic, so past
	 * {@link WITNESSED_ATTEMPT_CAP} attempts the loop throws the typed
	 * {@link ErrWitnessedLivelock} instead (the engine's ruling: the
	 * error, never a loop — retry is host policy, and this is the host
	 * policy's own honesty bound).
	 */
	function writeWitnessed<R>(fn: (snap: ReadScope<Rels>, tx: Tx<Rels>) => R): WitnessedWriteResult<Rels, R> {
		for (let attempts = 0; attempts < WITNESSED_ATTEMPT_CAP; attempts += 1) {
			const attempt = witnessedAttempt(fn)
			if (attempt !== undefined) {
				return attempt
			}
		}
		throw errors.wrap(
			ErrWitnessedLivelock,
			`writeWitnessed livelock: the generation moved on all ${WITNESSED_ATTEMPT_CAP} attempts against schema ${theory.name}`
		)
	}

	function prepare<Row, Params extends ParamsRecord>(q: Query<Rels, Row, Params>): Prepared<Rels, Row, Params> {
		if (q.schema !== theory) {
			throw errors.new(
				`query was built against schema ${q.schema.name}, not the identical schema value this store opened with — schema identity is the membership rule`
			)
		}
		const program = lowerQuery(q)
		const outcome = bridged("prepare bumbledb program", function callPrepare() {
			return native.dbPrepare(handle, program)
		})
		if (!outcome.ok) {
			throw errors.new(`bumbledb ${outcome.kind} (prepare): ${outcome.message}`)
		}
		const preparedHandle = outcome.prepared
		function staleness(snap: ReadScope<Rels>): Staleness {
			const snapState = scopeStates.get(snap)
			if (snapState === undefined) {
				throw errors.new("bumbledb staleness witness is not a read scope of this SDK")
			}
			if (snapState.owner !== owner) {
				throw errors.new(
					`bumbledb read scope belongs to a different store than this prepared value (schema ${theory.name})`
				)
			}
			if (!snapState.live) {
				throw errors.new("bumbledb read scope is invalidated — its owning read callback already returned")
			}
			return bridged("read bumbledb prepared staleness", function callStaleness() {
				return native.preparedStaleness(preparedHandle, snapState.handle)
			})
		}
		const prepared: Prepared<Rels, Row, Params> = Object.freeze({ staleness })
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

	return Object.freeze({
		schema: theory,
		read,
		scan,
		get,
		contains,
		execute,
		write,
		writeWitnessed,
		prepare
	})
}

/**
 * One cached open store: the theory VALUE it was admitted with (identity is
 * the membership rule — the fingerprint check against a cached path is a
 * `===` on this), the `Db` value every same-path open returns, and the
 * environment handle the exit hook closes.
 */
interface CachedStore {
	readonly theory: AnySchema
	readonly db: unknown
	readonly handle: DbHandle
}

/**
 * The per-process store cache, keyed by canonical path
 * (`node:path.resolve` — absolute and normalized). Symlink aliasing is
 * deliberately not resolved here: an aliased spelling misses the cache and
 * reaches the engine, whose exclusive lock refuses a second live handle on
 * the same store — the backstop that keeps "one store, one handle" true.
 */
const openStores = new Map<string, CachedStore>()

/**
 * The in-process fingerprint check and its typing proof in one probe:
 * theory identity (`===`) implies `Rels` identity, because a cache entry's
 * `db` was constructed from that very theory value — so a hit narrows the
 * entry's `db` to `Db<Rels>` with no assertion anywhere.
 */
function holdsTheory<Rels extends SchemaRelations>(
	entry: CachedStore,
	theory: Schema<Rels>
): entry is CachedStore & { readonly db: Db<Rels> } {
	return entry.theory === theory
}

/**
 * The best-effort exit hook: closes every cached environment so LMDB
 * releases its locks tidily on a clean exit. CORRECTNESS NEVER RESTS HERE —
 * the engine fsyncs every commit, so a process killed before (or during)
 * this hook loses nothing that was committed.
 */
process.once("exit", function closeCachedStores() {
	for (const cached of openStores.values()) {
		const closed = errors.trySync(function closeEnvironment() {
			native.dbClose(cached.handle)
		})
		if (closed.error) {
		}
	}
})

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

/**
 * The one admission path both verbs share: canonical-path cache lookup
 * first (a hit returns the SAME `Db` value for the identical theory, a
 * typed fingerprint error for a different one, and a typed refusal for
 * `create` — the store a cache entry proves initialized is exactly what
 * create refuses). On a miss: lower the theory, run one bridge call, and
 * wrap the domain refusals — `schemaError` (spec resolution + schema
 * validation, every issue in one message), `newtypeMismatch` (the
 * coherence wall, {@link ErrNewtypeMismatch}), and `fingerprintMismatch`
 * (a different theory cannot open the store) — into typed errors carrying
 * the engine's message intact.
 */
function admit<Rels extends SchemaRelations>(
	verb: "create" | "open",
	storePath: string,
	theory: Schema<Rels>
): Db<Rels> {
	const canonical = path.resolve(storePath)
	const cached = openStores.get(canonical)
	if (cached !== undefined) {
		if (verb === "create") {
			throw errors.new(
				`create bumbledb store at ${canonical}: the store is already open in this process — create refuses an already-initialized directory`
			)
		}
		if (!holdsTheory(cached, theory)) {
			throw errors.new(
				`bumbledb fingerprintMismatch (open ${canonical}): the cached store was opened with schema ${cached.theory.name}, not this theory value — schema identity is the membership rule`
			)
		}
		return cached.db
	}
	const spec = lower(theory)
	const opened = bridged(`${verb} bumbledb store at ${canonical}`, function callBridge() {
		if (verb === "create") {
			return native.dbCreate(canonical, spec)
		}
		return native.dbOpen(canonical, spec)
	})
	if (!opened.ok) {
		if (opened.kind === "newtypeMismatch") {
			throw errors.wrap(ErrNewtypeMismatch, `${verb} ${canonical}: ${opened.message}`)
		}
		throw errors.new(`bumbledb ${opened.kind} (${verb} ${canonical}): ${opened.message}`)
	}
	const manifest = bridged("fetch bumbledb manifest", function fetchManifest() {
		return native.dbManifest(opened.db)
	})
	const db = openDb(opened.db, theory, manifest)
	openStores.set(canonical, Object.freeze({ theory, db, handle: opened.db }))
	return db
}

/**
 * The store lifecycle — `Db.create(path, schema)` / `Db.open(path, schema)`.
 * Create refuses an already-initialized directory; open verifies format
 * version, store kind, and the schema fingerprint. Both return values
 * CACHED per canonical path: a second open of the same path with the
 * identical theory value returns the SAME `Db`, and a different theory on
 * a cached path is a typed fingerprint error. There is no close anywhere:
 * the process owns every cached environment until exit (a best-effort exit
 * hook closes them; durability is the engine's per-commit fsync). One
 * store kind exists: durable — resume = reopen, meaning this process's
 * cached value or a fresh process's open.
 */
const Db = Object.freeze({
	/** Creates a fresh durable store at `path` from the schema; the value is cached for every later open. */
	async create<Rels extends SchemaRelations>(path: string, theory: Schema<Rels>): Promise<Db<Rels>> {
		return admit("create", path, theory)
	},
	/**
	 * Opens an existing durable store at `path` with the same theory — the
	 * cached value when this process already holds it. A fingerprint-matching
	 * open also BACK-FILLS the store's persisted schema descriptor when it is
	 * absent (self-describing stores, engine 50-storage.md § the `_meta`
	 * block), so a legacy store becomes exhumable after one ordinary open —
	 * adoption is automatic, never a separate verb.
	 */
	async open<Rels extends SchemaRelations>(path: string, theory: Schema<Rels>): Promise<Db<Rels>> {
		return admit("open", path, theory)
	},
	/**
	 * Opens a store READ-ONLY from its own persisted descriptor — the SDK's
	 * one schema-independent read path (no theory, no fingerprint check; the
	 * store rebirth tool's entry). Lives beside `open`/`create` so the path
	 * law stays in one place: the same `node:path.resolve` canonicalization,
	 * applied here. The value is NOT cached and carries no close: the
	 * engine-side handle (and the store's exclusive lock) is reclaimed by GC
	 * — reclamation only, never correctness. A store not yet adopted rejects
	 * with the typed `ErrExhumeNoDescriptor` (the remedy: one
	 * fingerprint-matching `Db.open` under the creating schema back-fills
	 * the descriptor).
	 */
	async exhume(storePath: string): Promise<Exhumed> {
		return exhumeStore(path.resolve(storePath))
	}
})

export type {
	Abandon,
	DeclaredKeyFact,
	DeltaBuild,
	MemberRelation,
	OffendingFact,
	Prepared,
	ReadScope,
	Tx,
	Violation,
	WitnessedWriteResult,
	WriteResult
}
export { abandon, Db, ErrNewtypeMismatch, ErrWitnessedLivelock, WITNESSED_ATTEMPT_CAP }
