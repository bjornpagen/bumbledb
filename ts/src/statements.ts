/**
 * Dependency statements as typed values (`docs/architecture/30-dependencies.md`
 * owns the semantics; `docs/architecture/70-api.md` the surface): the FD key
 * form, conditional containment, the bidirectional `==` abbreviation, and
 * the cardinality window. A statement value is opaque and inert — no
 * methods, no fluent continuation: a fact about the theory, not a builder.
 *
 * Every field reference is checked against the relation it names in the
 * TYPE — existence through {@link FaceFields} (`on(R, "nope")` does not
 * compile) and DOMAIN compatibility through {@link SameDomains}: the two
 * faces' projected domain labels are read structurally off the schema type
 * (S1's `F["domain"]`) and constrained positionwise equal, so a
 * cross-domain pair is a compile error by string-literal comparison of
 * descriptor shapes — never by a value brand (the structural design's
 * ratified check). What is only a SEMANTIC property — the target side of a
 * containment resolving a declared key of its relation — is DELIBERATELY
 * not (and cannot be) stated here: whether `B(y)` is a key of `B` depends
 * on which `key()` statements the surrounding `schema()` collects, a set no
 * face type can see; it stays the engine's typed `SchemaError` judgment at
 * `Db.create`/`Db.open` (the two-boundary split, engine as final
 * authority).
 */

import * as errors from "@superbuilders/errors"
import type { Count } from "#count.ts"
import { type AnyFace, type FaceData, renderFace, type SameArity, type SameDomains } from "#face.ts"
import type { AnyRelation, RelationFields } from "#relation.ts"
import { renderWindow, type WindowSpec } from "#spec.ts"

/** A `key()` statement's runtime description — owner and projection carried at exact types. */
interface KeyData<R extends AnyRelation, Projection extends readonly string[]> {
	readonly kind: "key"
	readonly owner: R
	readonly projection: Projection
}

/** One statement's runtime description, tagged by form. */
type StatementData =
	| KeyData<AnyRelation, readonly string[]>
	| {
			readonly kind: "containment"
			readonly source: FaceData
			readonly target: FaceData
			readonly bidirectional: boolean
	  }
	| {
			readonly kind: "window"
			readonly target: FaceData
			readonly window: WindowSpec
			readonly source: FaceData
	  }

/** An opaque statement value — what `schema()` assembles into a theory. */
interface Statement {
	readonly data: StatementData
}

/**
 * A `key()` statement as a TYPED value: its `data` carries the owner
 * relation and the projection tuple at their EXACT types (honest runtime
 * properties — no phantom), which is what the key-statement-selected
 * `get(relation, keyStatement, key)` overload types its key object by
 * (`docs/architecture/70-api.md` § the freeze, the multi-key typed get) and
 * what resolves each projected field's domain label through the owner's
 * schema type. Structurally still a plain {@link Statement}.
 */
interface KeyStatement<R extends AnyRelation, Projection extends readonly string[]> extends Statement {
	readonly data: KeyData<R, Projection>
}

/**
 * `R(X) -> R` — the FD key form, composite keys as tuples. No selection
 * parameter exists (the FD-with-selection shape is unrepresentable, as in
 * the grammar), and only ordinary relations are accepted: a closed
 * relation's key `R(id) -> R` is materialized by the engine, so an
 * explicit one would only ever be a duplicate. Every projected name is
 * checked against `R`'s field block in the type, and the tuple is carried
 * in the returned value's type ({@link KeyStatement}) — keyed point reads
 * through THIS statement are typed field-for-field, domains resolvable
 * through the owner's schema type.
 */
function key<
	R extends AnyRelation,
	const Projection extends readonly [keyof RelationFields<R> & string, ...(keyof RelationFields<R> & string)[]]
>(relation: R, fields: Projection): KeyStatement<R, Projection> {
	if (!("fields" in relation.data)) {
		throw errors.new(
			`key(${relation.name}, ...): closedness already materializes ${relation.name}(id) -> ${relation.name} — an explicit key on a closed relation is rejected as a duplicate`
		)
	}
	const data: KeyData<R, Projection> = Object.freeze({
		kind: "key",
		owner: relation,
		projection: Object.freeze(fields)
	})
	return Object.freeze({ data })
}

/**
 * `A(X|φ) <= B(Y|ψ)` — conditional inclusion, source left. Arity mismatch
 * between the two faces is a type error ({@link SameArity}); a cross-domain
 * pair is a type error ({@link SameDomains} — positionwise string-literal
 * equality of the projected S1 domain labels). The target side must
 * resolve a declared key of B — a SEMANTIC property of the whole statement
 * set that no face type can state, DELIBERATELY judged by the engine at
 * `Db.create`/`Db.open` (`SchemaError`), never re-checked here.
 */
function contained<A extends AnyFace, B extends AnyFace>(
	source: A,
	target: B & SameArity<A, B> & SameDomains<A, B>
): Statement {
	const data: StatementData = Object.freeze({
		kind: "containment",
		source: source.data,
		target: target.data,
		bidirectional: false
	})
	return Object.freeze({ data })
}

/**
 * `A(X|φ) == B(Y|ψ)` — the bidirectional abbreviation, one utterance: the
 * selected `==` bijection, a keyed one-to-one correspondence between the
 * two faces (each side contains the other). It lowers to the two adjacent
 * containments in the `A <= B` first order (macro parity — the engine
 * performs the split, source-first) and renders as `==` once, in the
 * written orientation. Faces pair by arity AND domain, exactly as
 * {@link contained}.
 */
function mirrors<A extends AnyFace, B extends AnyFace>(
	source: A,
	target: B & SameArity<A, B> & SameDomains<A, B>
): Statement {
	const data: StatementData = Object.freeze({
		kind: "containment",
		source: source.data,
		target: target.data,
		bidirectional: true
	})
	return Object.freeze({ data })
}

/**
 * `B(Y|ψ) <={window} A(X|φ)` — the cardinality window. READ CAREFULLY: the
 * LEFT face is the window's TARGET, the per-group parent (B-family,
 * target-left — macro parity), and the RIGHT face is the counted source.
 * `window(on(Holder, "id"), atMost(3n), on(Account, "holder"))` says: each
 * Holder id groups at most three Account rows by holder. The two faces
 * pair by arity AND domain ({@link SameDomains}), exactly as containment —
 * the grouping join reads the same positionwise field pairing.
 */
function window<B extends AnyFace, A extends AnyFace>(
	target: B,
	count: Count,
	source: A & SameArity<B, A> & SameDomains<B, A>
): Statement {
	const data: StatementData = Object.freeze({
		kind: "window",
		target: target.data,
		window: count.window,
		source: source.data
	})
	return Object.freeze({ data })
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

export type { KeyData, KeyStatement, Statement, StatementData }
export { contained, key, mirrors, renderStatement, window }
