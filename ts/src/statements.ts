/**
 * Dependency statements as typed values (`docs/architecture/30-dependencies.md`
 * owns the semantics; `docs/architecture/70-api.md` the surface): the FD key
 * form, conditional containment, the bidirectional `==` abbreviation, and
 * the cardinality window. A statement value is opaque and inert — no
 * methods, no fluent continuation: a fact about the theory, not a builder.
 *
 * Every field reference is checked against the relation it names in the
 * TYPE — existence through {@link FaceFields} (`on(R, "nope")` does not
 * compile) and STRUCTURAL compatibility through {@link SameShapes}: the two
 * faces' projected kind/width/element/roster quadruples are read off the
 * schema type (the minimal kernel — descriptors are pure structure) and
 * constrained positionwise equal, so a u64 face against a str face, a
 * bytes width mismatch, an interval element mismatch, or a bare column
 * against a closed reference is a compile error. TWO of those walls carry
 * construction-time runtime twins here for untyped callers: ARITY
 * ({@link assertArityAgreement} — cleanup-0.5.0 ruling 9: an
 * arity-mismatched pairing fails at the statement, never by silent
 * truncation) and the ROSTER slot ({@link assertRosterAgreement} —
 * roster IDENTITY, positionwise: a closed vocabulary's referencing column
 * is spelled with the vocabulary's own id descriptor, the ONE spelling, so
 * a plain u64 column can never alias a vocabulary through a declared law
 * and the SDK's descriptor-keyed closed judgments stay sound). Domains are
 * NOT compared here — there is no domain to compare at construction: the
 * statements themselves are what define the equivalence classes, and the
 * domain wall lives where they aggregate — `schema()` (the
 * one-generator-per-class law) and query joins (class names off the schema
 * type). What is only a SEMANTIC property — the target side of a
 * containment resolving a declared key of its relation — is DELIBERATELY
 * not (and cannot be) stated here: whether `B(y)` is a key of `B` depends
 * on which `key()` statements the surrounding `schema()` collects, a set no
 * face type can see; it stays the engine's typed `SchemaError` judgment at
 * `Db.create`/`Db.open` (the two-boundary split, engine as final
 * authority).
 */

import * as errors from "@superbuilders/errors"
import { isClosedMember, sealedFieldOf } from "#closed.ts"
import type { Count } from "#count.ts"
import { type AnyFace, type FaceData, renderFace, type SameArity, type SameShapes } from "#face.ts"
import { type ClosedRoster, rosterOf } from "#fields.ts"
import type { AnyRelation, RelationFields } from "#relation.ts"
import { renderWindow, type WindowSpec } from "#spec.ts"

/** A `key()` statement's runtime description — owner and projection carried at exact types. */
interface KeyData<R extends AnyRelation, Projection extends readonly string[]> {
	readonly kind: "key"
	readonly owner: R
	readonly projection: Projection
}

/**
 * A containment statement's runtime description — the two faces carried at
 * their EXACT types (owner names and projection tuples are honest runtime
 * properties, and they are the type-level carrier `schema()`'s law-typing
 * pairs slots through). The defaults are the wide shape renderers and the
 * wire lowering consume.
 */
interface ContainmentData<Src extends FaceData = FaceData, Tgt extends FaceData = FaceData> {
	readonly kind: "containment"
	readonly source: Src
	readonly target: Tgt
	readonly bidirectional: boolean
}

/** A window statement's runtime description — target-left, faces at exact types like {@link ContainmentData}. */
interface WindowData<Tgt extends FaceData = FaceData, Src extends FaceData = FaceData> {
	readonly kind: "window"
	readonly target: Tgt
	readonly window: WindowSpec
	readonly source: Src
}

/** One statement's runtime description, tagged by form. */
type StatementData = KeyData<AnyRelation, readonly string[]> | ContainmentData | WindowData

/**
 * The admission brand — a module-private symbol, deliberately unexported
 * (the `count.ts` pattern): `Statement` is a public structural type, so
 * without this brand a forged plain object of the right shape would walk
 * past the construction-time arity and roster walls into `schema()` — and
 * the roster wall is the one the engine cannot backstop (the wire carries
 * plain u64s, no rosters). The symbol makes the four constructors the ONLY
 * mints, so a statement that skipped the walls is unspellable.
 */
const admitted: unique symbol = Symbol("bumbledb.statement.admitted")

/** An opaque statement value — what `schema()` assembles into a theory. Only the four constructors produce one. */
interface Statement {
	readonly data: StatementData
	readonly [admitted]: true
}

/**
 * Narrows any value to an admitted statement — the probe is the
 * module-private {@link admitted} brand only the four constructors set, so
 * no host-built value (fact cells are structurally OPEN — an interval with
 * an excess `kind` property is a legal cell) can ever be misread as one.
 * The keyed-get selector dispatch and `schema()`'s admission both judge
 * through here.
 */
function isStatement(value: unknown): value is Statement {
	return typeof value === "object" && value !== null && admitted in value
}

/**
 * A containment (or `==` bijection) statement as a TYPED value: `data`
 * carries both faces at their exact types, so the schema-level class laws
 * can read every paired (relation, field) slot off the statement type —
 * spell the statement list inline in `schema()` and the equivalence
 * classes compute at the type level too. Structurally still a plain
 * {@link Statement}.
 */
interface ContainedStatement<Src extends FaceData, Tgt extends FaceData> extends Statement {
	readonly data: ContainmentData<Src, Tgt>
}

/** A window statement as a TYPED value — the {@link ContainedStatement} of the window form. */
interface WindowStatement<Tgt extends FaceData, Src extends FaceData> extends Statement {
	readonly data: WindowData<Tgt, Src>
}

/**
 * A `key()` statement as a TYPED value: its `data` carries the owner
 * relation and the projection tuple at their EXACT types (honest runtime
 * properties — no phantom), which is what the key-statement-selected
 * `get(relation, keyStatement, key)` overload types its key object by
 * (`docs/architecture/70-api.md` § the freeze, the multi-key typed get) and
 * what resolves each projected field's descriptor through the owner's
 * schema type. Structurally still a plain {@link Statement}.
 */
interface KeyStatement<R extends AnyRelation, Projection extends readonly string[]> extends Statement {
	readonly data: KeyData<R, Projection>
}

/** Renders one face position's closedness for the roster-agreement diagnostics. */
function renderRosterSide(roster: ClosedRoster | undefined): string {
	return roster === undefined ? "a bare column" : `a ${roster.name} reference`
}

/**
 * The runtime twin of {@link SameArity} (cleanup-0.5.0 ruling 9): the two
 * faces must project equally many fields, judged at CONSTRUCTION for
 * untyped callers too — without it an arity-mismatched containment
 * silently truncates to the shorter projection (this module's positionwise
 * walk and `law.ts`'s `unionSlot` both skip unpaired positions) until
 * `Db.create`'s colder engine refusal. The error carries the two faces'
 * own facts: names, arities, and the rendered statement.
 */
function assertArityAgreement(source: FaceData, target: FaceData, statement: Statement): void {
	if (source.projection.length !== target.projection.length) {
		throw errors.new(
			`${source.owner.name}(${source.projection.join(", ")}) and ${target.owner.name}(${target.projection.join(", ")}) project ${source.projection.length} vs ${target.projection.length} fields — positional pairing requires both faces to project equally many — ${renderStatement(statement)}`
		)
	}
}

/**
 * The runtime twin of {@link SameShapes}'s roster slot: the two faces'
 * projected descriptors must agree POSITIONWISE on closedness — the same
 * roster (value identity — vocabulary identity is value identity, the
 * SDK's membership rule everywhere) or none. Without this wall a plain u64
 * column could alias a closed vocabulary through a declared containment
 * (`docs/architecture/10-data-model.md` spells the ENGINE encoding that
 * way), and every descriptor-keyed closed judgment — the orderable ban,
 * the name↔id marshal, answer decode — would silently miss it. The
 * vocabulary's own descriptor (`Kind.id`) is the ONE spelling of a closed
 * reference at this surface (the canonical-utterance law); the engine
 * cannot backstop this one — the wire carries plain u64s, no rosters.
 * Arity agreement ({@link assertArityAgreement}) runs first, so the
 * positionwise walk here never sees an unpaired position from a well-typed
 * OR an untyped caller.
 */
function assertRosterAgreement(source: FaceData, target: FaceData, statement: Statement): void {
	source.projection.forEach(function agreeAt(fieldName, position) {
		const targetField = target.projection[position]
		if (targetField === undefined) {
			return
		}
		const sourceRoster = rosterOf(sealedFieldOf(source.owner, fieldName))
		const targetRoster = rosterOf(sealedFieldOf(target.owner, targetField))
		if (sourceRoster !== targetRoster) {
			throw errors.new(
				`${source.owner.name}.${fieldName} is ${renderRosterSide(sourceRoster)} but ${target.owner.name}.${targetField} is ${renderRosterSide(targetRoster)} — closedness rides the descriptor: a closed reference is spelled with the vocabulary's own id descriptor (one meaning, one spelling), so faces pair closed-with-closed through one roster or bare-with-bare, never across — ${renderStatement(statement)}`
			)
		}
	})
}

/**
 * `R(X) -> R` — the FD key form, composite keys as tuples. No selection
 * parameter exists (the FD-with-selection shape is unrepresentable, as in
 * the grammar), and only ordinary relations are accepted: a closed
 * relation's key `R(id) -> R` is materialized by the engine, so an
 * explicit one would only ever be a duplicate. Every projected name is
 * checked against `R`'s field block in the type, and the tuple is carried
 * in the returned value's type ({@link KeyStatement}) — keyed point reads
 * through THIS statement are typed field-for-field, descriptors resolvable
 * through the owner's schema type.
 */
function key<
	R extends AnyRelation,
	const Projection extends readonly [keyof RelationFields<R> & string, ...(keyof RelationFields<R> & string)[]]
>(relation: R, fields: Projection): KeyStatement<R, Projection> {
	if (isClosedMember(relation)) {
		throw errors.new(
			`key(${relation.name}, ...): closedness already materializes ${relation.name}(id) -> ${relation.name} — an explicit key on a closed relation is rejected as a duplicate`
		)
	}
	const data: KeyData<R, Projection> = Object.freeze({
		kind: "key",
		owner: relation,
		projection: Object.freeze(fields)
	})
	return Object.freeze({ data, [admitted]: true as const })
}

/**
 * `A(X|φ) <= B(Y|ψ)` — conditional inclusion, source left. Arity mismatch
 * between the two faces is a type error ({@link SameArity}); a structurally
 * mismatched pair is a type error ({@link SameShapes} — positionwise
 * equality of the projected kind/width/element triples). The target side
 * must resolve a declared key of B — a SEMANTIC property of the whole
 * statement set that no face type can state, DELIBERATELY judged by the
 * engine at `Db.create`/`Db.open` (`SchemaError`), never re-checked here.
 */
function contained<A extends AnyFace, B extends AnyFace>(
	source: A,
	target: B & SameArity<A, B> & SameShapes<A, B>
): ContainedStatement<A["data"], B["data"]> {
	const data: ContainmentData<A["data"], B["data"]> = Object.freeze({
		kind: "containment",
		source: source.data,
		target: target.data,
		bidirectional: false
	})
	const statement = Object.freeze({ data, [admitted]: true as const })
	assertArityAgreement(data.source, data.target, statement)
	assertRosterAgreement(data.source, data.target, statement)
	return statement
}

/**
 * `A(X|φ) == B(Y|ψ)` — the bidirectional abbreviation, one utterance: the
 * selected `==` bijection, a keyed one-to-one correspondence between the
 * two faces (each side contains the other). It lowers to the two adjacent
 * containments in the `A <= B` first order (macro parity — the engine
 * performs the split, source-first) and renders as `==` once, in the
 * written orientation. Faces pair by arity AND structural shape, exactly
 * as {@link contained}.
 */
function mirrors<A extends AnyFace, B extends AnyFace>(
	source: A,
	target: B & SameArity<A, B> & SameShapes<A, B>
): ContainedStatement<A["data"], B["data"]> {
	const data: ContainmentData<A["data"], B["data"]> = Object.freeze({
		kind: "containment",
		source: source.data,
		target: target.data,
		bidirectional: true
	})
	const statement = Object.freeze({ data, [admitted]: true as const })
	assertArityAgreement(data.source, data.target, statement)
	assertRosterAgreement(data.source, data.target, statement)
	return statement
}

/**
 * `B(Y|ψ) <={window} A(X|φ)` — the cardinality window. READ CAREFULLY: the
 * LEFT face is the window's TARGET, the per-group parent (B-family,
 * target-left — macro parity), and the RIGHT face is the counted source.
 * `window(on(Holder, "id"), atMost(3n), on(Account, "holder"))` says: each
 * Holder id groups at most three Account rows by holder. The two faces
 * pair by arity AND structural shape ({@link SameShapes}), exactly as
 * containment — the grouping join reads the same positionwise field
 * pairing.
 */
function window<B extends AnyFace, A extends AnyFace>(
	target: B,
	count: Count,
	source: A & SameArity<B, A> & SameShapes<B, A>
): WindowStatement<B["data"], A["data"]> {
	const data: WindowData<B["data"], A["data"]> = Object.freeze({
		kind: "window",
		target: target.data,
		window: count.window,
		source: source.data
	})
	const statement = Object.freeze({ data, [admitted]: true as const })
	assertArityAgreement(data.source, data.target, statement)
	assertRosterAgreement(data.source, data.target, statement)
	return statement
}

/**
 * Renders one statement in the CANONICAL macro spelling
 * (`docs/architecture/70-api.md`; the engine's `schema/render.rs` emits the
 * same shapes for violations) — `Account(id) -> Account`,
 * `Account(holder) <= Holder(id)`,
 * `Account(id | kind == Savings) == SavingsTerms(account)`,
 * `Holder(id) <={0..3} Account(holder)` — so TS-side errors and
 * engine-side diagnostics read identically. A renderer, never a parser:
 * strings are output-only.
 */
function renderStatement(statement: Statement): string {
	const data = statement.data
	switch (data.kind) {
		case "key":
			return `${data.owner.name}(${data.projection.join(", ")}) -> ${data.owner.name}`
		case "containment": {
			const operator = data.bidirectional ? "==" : "<="
			return `${renderFace(data.source)} ${operator} ${renderFace(data.target)}`
		}
		case "window":
			return `${renderFace(data.target)} <=${renderWindow(data.window)} ${renderFace(data.source)}`
	}
}

export type {
	ContainedStatement,
	ContainmentData,
	KeyData,
	KeyStatement,
	Statement,
	StatementData,
	WindowData,
	WindowStatement
}
export { contained, isStatement, key, mirrors, renderStatement, window }
