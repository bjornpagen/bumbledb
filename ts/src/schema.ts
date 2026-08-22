/**
 * `schema()` — assembles relations and statements into a theory value (the
 * `Theory` analog; what `Db.create`/`Db.open` take). Construction-time
 * validation is the macro-EXPANSION-boundary analog: membership,
 * implied-key duplicates, duplicate statements, a belt-and-braces handle
 * re-verification — and the TARGET-KEY WALL ({@link verifyTargetKeys}),
 * the value tier of the two-tier containment law
 * (60-containment-parity): every containment/mirrors/capacity target
 * projection must set-match a key of its relation, judged HERE with the
 * engine's exact rule so `lower()` never emits an engine-refused
 * containment. The type tier is `law.ts`'s `TargetKeyWall` (best effort,
 * statically known tuples); every OTHER semantic judgment (key-internal
 * legality, fresh-on-u64, …) stays the engine's `SchemaError` at
 * `Db.create` — the engine is the final authority for every boundary,
 * this wall just makes the SDK agree with it first.
 */

import * as errors from "@superbuilders/errors"
import type { AnyClosed } from "#closed.ts"
import { isClosedMember, sealedFieldOf } from "#closed.ts"
import type { FaceData } from "#face.ts"
import { assertDeclarationOrderKey, assertDeclarationRecord, rosterOf } from "#fields.ts"
import { type ClassesOf, classesComplete, computeClasses, type LawfulStatements, type SchemaClasses } from "#law.ts"
import type { AnyRelation } from "#relation.ts"
import type { LiteralSetSpec, LiteralSpec } from "#spec.ts"
import { isStatement, renderStatement, type Statement } from "#statements.ts"

/**
 * The implied keys of one walk over the relation record, carried in BOTH
 * spellings its two consumers read: `rendered` — each key in the canonical
 * statement rendering, for the explicit-duplicate check (string identity
 * with the renderer as the single spelling authority) — and `roster` — the
 * same keys as PROJECTIONS per relation, the implied half of the
 * target-key wall's key population ({@link verifyTargetKeys}).
 */
interface ImpliedKeys {
	readonly rendered: ReadonlySet<string>
	readonly roster: ReadonlyMap<string, ReadonlyArray<readonly string[]>>
}

/**
 * Validates the relation record and collects the implied keys: the
 * fresh-implied `R(field) -> R` per minted field and the closed auto-key
 * `R(id) -> R` per closed relation — one walk, both consumers' spellings
 * ({@link ImpliedKeys}).
 */
function collectImplied(name: string, relations: SchemaRelations): ImpliedKeys {
	assertDeclarationRecord(`schema ${name} relations`, relations)
	const rendered = new Set<string>()
	const roster = new Map<string, ReadonlyArray<readonly string[]>>()
	for (const [recordKey, member] of Object.entries(relations)) {
		assertDeclarationOrderKey(`schema ${name} relation`, recordKey)
		if (member.name !== recordKey) {
			throw errors.new(
				`schema ${name}: record key ${recordKey} holds relation ${member.name} — the key must equal the relation's declared name`
			)
		}
		const projections: Array<readonly string[]> = []
		if (isClosedMember(member)) {
			projections.push(Object.freeze(["id"]))
		} else {
			for (const declared of member.data.fields) {
				if ("fresh" in declared.field && declared.field.fresh === true) {
					projections.push(Object.freeze([declared.name]))
				}
			}
		}
		for (const projection of projections) {
			rendered.add(`${member.name}(${projection.join(", ")}) -> ${member.name}`)
		}
		roster.set(member.name, Object.freeze(projections))
	}
	return { rendered, roster }
}

/** The relation values a statement addresses, for membership checking. */
function statementOwners(statement: Statement): readonly SchemaRelation[] {
	const data = statement.data
	if (data.kind === "key") {
		return [data.owner]
	}
	return [data.source.owner, data.target.owner]
}

/**
 * Requires every relation a statement addresses to be the IDENTICAL value
 * the schema record declares — same-name-different-value is a forgery, not
 * a membership.
 */
function verifyMembership(name: string, relations: SchemaRelations, statement: Statement, rendered: string): void {
	for (const owner of statementOwners(statement)) {
		const member = relations[owner.name]
		if (member === undefined) {
			throw errors.new(`schema ${name}: relation ${owner.name} is not declared in this schema — ${rendered}`)
		}
		if (member !== owner) {
			throw errors.new(
				`schema ${name}: statement references a different relation value named ${owner.name} than the one this schema declares — ${rendered}`
			)
		}
	}
}

/** Flattens one binding's literal set into its literals. */
function bindingLiterals(set: LiteralSetSpec): readonly LiteralSpec[] {
	if (set.kind === "one") {
		return [set.literal]
	}
	return set.literals
}

/**
 * Re-verifies one binding's handle literals against the field's roster —
 * belt-and-braces over what `where()` already resolved and the type level
 * already blocked, so a forged binding fails here rather than at the
 * engine boundary with a colder message.
 */
function verifyBindingHandles(
	name: string,
	face: FaceData,
	binding: { readonly field: string; readonly set: LiteralSetSpec },
	rendered: string
): void {
	const roster = rosterOf(sealedFieldOf(face.owner, binding.field))
	for (const literal of bindingLiterals(binding.set)) {
		if (literal.kind !== "handle") {
			continue
		}
		if (roster === undefined) {
			throw errors.new(
				`schema ${name}: ${face.owner.name}.${binding.field} is not a closed-relation reference — the handle literal ${literal.handle} is legal only on a field carrying a closed relation's roster — ${rendered}`
			)
		}
		if (!roster.handles.includes(literal.handle)) {
			throw errors.new(`schema ${name}: closed relation ${roster.name} has no handle ${literal.handle} — ${rendered}`)
		}
	}
}

/** Walks every face of a statement through the handle re-verification. */
function verifyHandles(name: string, statement: Statement, rendered: string): void {
	const data = statement.data
	if (data.kind === "key") {
		return
	}
	for (const face of [data.source, data.target]) {
		for (const binding of face.selection) {
			verifyBindingHandles(name, face, binding, rendered)
		}
	}
}

/**
 * Resolves the closed relation a `(relation, field)` pair references
 * through the DECLARED containments — the identical walk the engine's
 * canonical renderer performs (`schema/render.rs` `closed_target_of`): one
 * hop, source projecting exactly `[field]`, target projecting exactly the
 * closed relation's `[id]`, first declared match wins; a `mirrors`
 * contributes both of its materialized orientations. `undefined` = the
 * engine would render the field's selection literals as raw row ids.
 */
function closedTargetOf(statements: readonly Statement[], owner: string, field: string): string | undefined {
	for (const statement of statements) {
		const data = statement.data
		if (data.kind !== "containment" && data.kind !== "mirrors") {
			continue
		}
		const pairs: Array<readonly [FaceData, FaceData]> = [[data.source, data.target]]
		if (data.kind === "mirrors") {
			pairs.push([data.target, data.source])
		}
		for (const [source, target] of pairs) {
			if (
				source.owner.name === owner &&
				source.projection.length === 1 &&
				source.projection[0] === field &&
				target.projection.length === 1 &&
				target.projection[0] === "id" &&
				isClosedMember(target.owner)
			) {
				return target.owner.name
			}
		}
	}
	return undefined
}

/**
 * Admits a handle spelling only when the schema also declares the
 * containment the ENGINE's canonical renderer resolves it through
 * (`docs/architecture/10-data-model.md` § closed relations: a closed
 * reference is the plain u64 column PLUS a declared containment). Without
 * it the two renderers drift — `renderStatement` prints the handle name,
 * the engine's violation `canonical` prints the raw row id — and the
 * paste-back law (`violation.canonical === renderStatement(statement)`)
 * breaks. Runs over the COMPLETE statement list, so declaration order
 * never matters. The closed relation's own `id` field resolves directly
 * (the walk's field-0 case).
 */
function verifyClosedReferences(name: string, statements: readonly Statement[]): void {
	for (const statement of statements) {
		const data = statement.data
		if (data.kind === "key") {
			continue
		}
		const rendered = renderStatement(statement)
		for (const face of [data.source, data.target]) {
			for (const binding of face.selection) {
				verifyClosedReferenceBinding(name, statements, face, binding, rendered)
			}
		}
	}
}

/** One binding's closed-reference resolution check (the {@link verifyClosedReferences} leaf). */
function verifyClosedReferenceBinding(
	name: string,
	statements: readonly Statement[],
	face: FaceData,
	binding: { readonly field: string; readonly set: LiteralSetSpec },
	rendered: string
): void {
	const spellsHandle = bindingLiterals(binding.set).some(function isHandle(literal) {
		return literal.kind === "handle"
	})
	if (!spellsHandle) {
		return
	}
	const roster = rosterOf(sealedFieldOf(face.owner, binding.field))
	if (roster === undefined) {
		return
	}
	if (isClosedMember(face.owner) && binding.field === "id") {
		return
	}
	const resolved = closedTargetOf(statements, face.owner.name, binding.field)
	if (resolved !== roster.name) {
		throw errors.new(
			`schema ${name}: ${face.owner.name}.${binding.field} spells a ${roster.name} handle, but no declared containment resolves the closed reference — a closed reference is the plain u64 column plus its declared containment; declare contained(on(${face.owner.name}, "${binding.field}"), on(${roster.name}, "id")) — ${rendered}`
		)
	}
}

/**
 * THE TARGET-KEY WALL, value tier (60-containment-parity — the runtime
 * twin of `law.ts`'s `TargetKeyWall`, the engine's `resolve_target_key` /
 * `resolve_capacity_target` mirrored exactly): every `contained`/
 * `mirrors`/`capacity` statement's target projection must resolve a key
 * of the target relation, judged over the SAME key population the engine
 * materializes — the fresh-implied and closed auto-keys
 * ({@link collectImplied}'s roster) first, then the declared `key()`
 * statements in written order (a key may be declared after its probe, so
 * this wall runs over the COMPLETE list, never inside the statement
 * loop). `mirrors` materializes as two containments source-first, so both
 * orientations judge their own target. SOUNDNESS BAR: the set-match +
 * closed-id rule below is the engine's COMPLETE rule for this law
 * (`matching_functionality` compares field SETS — permutations resolve,
 * subsets/supersets refuse), so this wall never rejects what the engine
 * accepts; every other schema judgment stays engine-first.
 */
function verifyTargetKeys(
	name: string,
	statements: readonly Statement[],
	implied: ReadonlyMap<string, ReadonlyArray<readonly string[]>>
): void {
	const declared = new Map<string, Array<readonly string[]>>()
	for (const statement of statements) {
		const data = statement.data
		if (data.kind !== "key") {
			continue
		}
		const keys = declared.get(data.owner.name)
		if (keys === undefined) {
			declared.set(data.owner.name, [data.projection])
		} else {
			keys.push(data.projection)
		}
	}
	for (const statement of statements) {
		const data = statement.data
		if (data.kind === "key") {
			continue
		}
		const rendered = renderStatement(statement)
		// A containment or capacity judges its one target face; `mirrors`
		// materializes as the two adjacent containments (source-first —
		// macro parity), so the written target's face judges first, then
		// the reverse orientation's target (the written source).
		const faces = data.kind === "mirrors" ? [data.target, data.source] : [data.target]
		for (const face of faces) {
			verifyTargetKeyFace(name, face, implied, declared, rendered)
		}
	}
}

/**
 * One target face's key resolution (the {@link verifyTargetKeys} leaf).
 * Closed target: the handle id is the ONE probe-able identity of a closed
 * relation, so the projection must be exactly `["id"]` — its own refusal
 * (the engine's `ClosedTargetNotHandle`): the rule is CLOSEDNESS, not key
 * absence. Ordinary target: the projection's field-name SET must equal
 * some roster member's set (the engine's `matching_functionality` —
 * permutations resolve, subsets and supersets do not). The refusal speaks
 * the engine's shape in NAMES, with the engine's pointwise hint verbatim
 * when the projection carries an interval position.
 */
function verifyTargetKeyFace(
	name: string,
	face: FaceData,
	implied: ReadonlyMap<string, ReadonlyArray<readonly string[]>>,
	declared: ReadonlyMap<string, ReadonlyArray<readonly string[]>>,
	rendered: string
): void {
	if (isClosedMember(face.owner)) {
		if (face.projection.length === 1 && face.projection[0] === "id") {
			return
		}
		throw errors.new(
			`schema ${name}: ${rendered}: closed target ${face.owner.name} is addressed by its synthetic id only — projection (${face.projection.join(", ")}) must be exactly (id) (rewrite the target side as on(${face.owner.name}, "id"))`
		)
	}
	const roster = [...(implied.get(face.owner.name) ?? []), ...(declared.get(face.owner.name) ?? [])]
	const want = new Set(face.projection)
	const matched = roster.some(function sameFieldSet(key) {
		// Raw length equality on BOTH sides first — the belt against
		// multiset drift: `new Set` collapses a duplicate spelling, so a
		// collapsed key or projection could set-match a shorter partner the
		// engine's FieldSet refuses (duplicates are refused at the key()
		// mint too; this comparison trusts neither wall alone).
		if (key.length !== face.projection.length || want.size !== face.projection.length) {
			return false
		}
		const keySet = new Set(key)
		if (keySet.size !== want.size) {
			return false
		}
		for (const field of keySet) {
			if (!want.has(field)) {
				return false
			}
		}
		return true
	})
	if (matched) {
		return
	}
	const available = roster.length === 0 ? "none" : roster.map((key) => `(${key.join(", ")})`).join("; ")
	const pointwise = face.projection.some(function carriesInterval(fieldName) {
		const descriptor = sealedFieldOf(face.owner, fieldName)
		return descriptor !== undefined && descriptor.kind === "interval"
	})
	const hint = pointwise ? "; hint: declare the exact pointwise key `R(prefix…, interval) -> R`" : ""
	throw errors.new(
		`schema ${name}: ${rendered}: target projection (${face.projection.join(", ")}) matches no declared key of ${face.owner.name} — available keys: ${available}${hint}`
	)
}

/** One member of a schema's relation record. */
type SchemaRelation = AnyRelation | AnyClosed

/** The relation record a schema is generic over — what `Db` and queries key on. */
type SchemaRelations = Record<string, SchemaRelation>

/**
 * A theory value: named relations, the DECLARED dependency statements, and
 * the LAW-COMPUTED class map (`classes` — relation → field → class name,
 * `undefined` = bare). The class map is THE domain authority: `schema()`
 * computes it FROM the statement list at both tiers (the type through
 * {@link ClassesOf}, the value through the union-find twin), queries
 * compare class names off it, and the wire lowering emits it as the spec
 * `newtype` labels. Nothing is ever synthesized: `statements` is exactly
 * the declared list, in written order.
 */
interface Schema<Rels extends SchemaRelations, Classes extends SchemaClasses = SchemaClasses> {
	readonly name: string
	readonly relations: Rels
	readonly statements: readonly Statement[]
	readonly classes: Classes
}

/** Any schema value, whatever its relation record. */
type AnySchema = Schema<SchemaRelations>

/**
 * Forces the class map to EVALUATE at the `schema()` boundary: a two-level
 * mapped copy, type-identical to `C` — but instantiation resolves it to
 * the finished relation → field → class record, so hovering a schema value
 * (or anything carrying its `Classes` parameter — queries, `Db`) shows the
 * computed record instead of the unevaluated `ClassesOf<...>` application
 * dragging the whole statement-tuple type along. The conditional wrapper
 * is the display mechanism, not a judgment: resolving it drops the alias
 * reference, so tsc renders the finished record (measured against tsc's
 * own type rendering; a bare mapped alias still displays by name).
 * Display-only by construction; `classesComplete` guards the same type at
 * the value tier.
 */
type EvaluatedClasses<C extends SchemaClasses> = C extends SchemaClasses
	? { readonly [N in keyof C]: { readonly [F in keyof C[N]]: C[N][F] } }
	: never

/**
 * Assembles a theory:
 * `schema("Ledger", { Kind, Account, Holder }, [ ...statements ])`.
 *
 * Rejected here, each with the offending statement rendered canonically:
 * a record key differing from its relation's declared name; a statement
 * whose relation is not (identically) a member of the record; an explicit
 * duplicate of a fresh-implied or closedness-implied key (macro parity:
 * "redundant here — and rejected as a duplicate"); a duplicate statement
 * (two statements rendering to one canonical utterance ARE one judgment);
 * a handle selection that its roster does not hold (belt-and-braces —
 * the type level already blocks it); a handle selection whose closed
 * reference no declared containment resolves (the engine's canonical
 * renderer would print the raw row id where `renderStatement` prints the
 * handle — the paste-back law demands the two spellings agree); and a
 * containment/mirrors/capacity target projection resolving no key of its
 * relation ({@link verifyTargetKeys} — the value tier of the two-tier
 * target-key wall, the engine's rule judged at assembly in names).
 *
 * The fresh-implied and closed auto-keys are NOT added to the statement
 * list: the engine materializes them itself, in its own pinned order
 * (`SchemaDescriptor::materialized_statements`), and restating them would
 * double them.
 *
 * THE LAW-TYPING happens here too (rulings 2/3 — the laws type the
 * columns): the statement list induces the equivalence classes over field
 * slots, at the TYPE level ({@link ClassesOf} — spell the statement list
 * inline so the tuple type stays precise) and at runtime (the union-find
 * twin), and the one-generator-per-class wall holds at both tiers — the
 * {@link LawfulStatements} verdict lands the compile error on the
 * statements argument; `computeClasses` throws the same content naming the
 * exact statement.
 */
function schema<const Rels extends SchemaRelations, const Stmts extends readonly Statement[]>(
	name: string,
	relations: Rels,
	statements: Stmts & LawfulStatements<Rels, Stmts>
): Schema<Rels, EvaluatedClasses<ClassesOf<Rels, Stmts>>> {
	const implied = collectImplied(name, relations)
	const seen = new Set<string>()
	for (const statement of statements) {
		/**
		 * The untyped caller's half of the admission brand: the type tier
		 * already refuses an unbranded structural literal, and this probe
		 * refuses the same forgery at runtime — a statement that skipped the
		 * construction-time arity and roster walls never enters the theory.
		 */
		if (!isStatement(statement)) {
			throw errors.new(
				`schema ${name}: a statement is minted only by key/contained/mirrors/capacity — a structural literal skips the construction-time arity and roster walls`
			)
		}
		const rendered = renderStatement(statement)
		verifyMembership(name, relations, statement, rendered)
		if (implied.rendered.has(rendered)) {
			throw errors.new(
				`schema ${name}: ${rendered} is redundant here (the fresh mark or closedness already implies it) — and rejected as a duplicate`
			)
		}
		if (seen.has(rendered)) {
			throw errors.new(`schema ${name}: duplicate statement — ${rendered}`)
		}
		seen.add(rendered)
		verifyHandles(name, statement, rendered)
	}
	verifyClosedReferences(name, statements)
	verifyTargetKeys(name, statements, implied.roster)
	const classes = computeClasses(name, relations, statements)
	if (!classesComplete<EvaluatedClasses<ClassesOf<Rels, Stmts>>>(classes, relations)) {
		throw errors.new(`schema ${name}: class-map construction incomplete`)
	}
	return Object.freeze({ name, relations, statements: Object.freeze([...statements]), classes })
}

export type { AnySchema, Schema, SchemaRelation, SchemaRelations }
export { schema }
