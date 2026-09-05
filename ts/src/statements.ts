import {
	type BoundsOnTarget,
	type CapacityWeight,
	type CapacityWindow,
	isCapacityWeight,
	isCapacityWindow,
	type UnitDimensionBan,
	type UnitWindowBan,
	unitWeight,
	type WeightOnSource
} from "#capacity.ts"
import { isClosedMember, sealedFieldOf } from "#closed.ts"
import { AuthoringError } from "#errors.ts"
import { type AnyFace, type FaceData, renderFace, type SameArity, type SameShapes } from "#face.ts"
import { type AnyClosedRoster, rosterOf, rostersAgree } from "#fields.ts"
import type { AnyRelation, RelationFields } from "#relation.ts"
import { type CapacityWindowSpec, renderCapacityWindow, renderWeight, type WeightSpec } from "#spec.ts"

interface KeyData<R extends AnyRelation, Projection extends readonly string[]> {
	readonly kind: "key"
	readonly owner: R
	readonly projection: Projection
}

interface ContainmentData<Src extends FaceData = FaceData, Tgt extends FaceData = FaceData> {
	readonly kind: "containment"
	readonly source: Src
	readonly target: Tgt
}

interface MirrorsData<Src extends FaceData = FaceData, Tgt extends FaceData = FaceData> {
	readonly kind: "mirrors"
	readonly source: Src
	readonly target: Tgt
}

interface CapacityData<Tgt extends FaceData = FaceData, Src extends FaceData = FaceData> {
	readonly kind: "capacity"
	readonly target: Tgt
	readonly weight: WeightSpec
	readonly window: CapacityWindowSpec
	readonly source: Src
}

type StatementData = KeyData<AnyRelation, readonly string[]> | ContainmentData | MirrorsData | CapacityData

const admitted: unique symbol = Symbol("bumbledb.statement.admitted")

interface Statement {
	readonly data: StatementData
	readonly [admitted]: true
}

function isStatement(value: unknown): value is Statement {
	return typeof value === "object" && value !== null && admitted in value
}

interface ContainedStatement<Src extends FaceData, Tgt extends FaceData> extends Statement {
	readonly data: ContainmentData<Src, Tgt> | MirrorsData<Src, Tgt>
}

interface CapacityStatement<Tgt extends FaceData, Src extends FaceData> extends Statement {
	readonly data: CapacityData<Tgt, Src>
}

interface KeyStatement<R extends AnyRelation, Projection extends readonly string[]> extends Statement {
	readonly data: KeyData<R, Projection>
}

function renderRosterSide(roster: AnyClosedRoster | undefined): string {
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
		throw new AuthoringError({
			message: `${source.owner.name}(${source.projection.join(", ")}) and ${target.owner.name}(${target.projection.join(", ")}) project ${source.projection.length} vs ${target.projection.length} fields — positional pairing requires both faces to project equally many — ${renderStatement(statement)}`
		})
	}
}

function assertRosterAgreement(source: FaceData, target: FaceData, statement: Statement): void {
	source.projection.forEach(function agreeAt(fieldName, position) {
		const targetField = target.projection[position]
		if (targetField === undefined) {
			return
		}
		const sourceRoster = rosterOf(sealedFieldOf(source.owner, fieldName))
		const targetRoster = rosterOf(sealedFieldOf(target.owner, targetField))
		if (!rostersAgree(sourceRoster, targetRoster)) {
			throw new AuthoringError({
				message: `${source.owner.name}.${fieldName} is ${renderRosterSide(sourceRoster)} but ${target.owner.name}.${targetField} is ${renderRosterSide(targetRoster)} — closedness rides the descriptor: a closed reference is spelled with the vocabulary's own id descriptor (one meaning, one spelling), so faces pair closed-with-closed through one roster or bare-with-bare, never across — ${renderStatement(statement)}`
			})
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
 * through the owner's schema type. A DUPLICATE field in the projection is
 * refused here, at the mint (the engine's `FieldSet` refuses the same
 * duplicate at `Db.create`; canonical utterance says the twice-spelled
 * field is the once-spelled projection respelled) — without this wall a
 * `new Set`-collapsed duplicate could set-match a shorter target
 * projection the engine refuses.
 */
function key<
	R extends AnyRelation,
	const Projection extends readonly [keyof RelationFields<R> & string, ...(keyof RelationFields<R> & string)[]]
>(relation: R, fields: Projection): KeyStatement<R, Projection> {
	if (isClosedMember(relation)) {
		throw new AuthoringError({
			message: `key(${relation.name}, ...): closedness already materializes ${relation.name}(id) -> ${relation.name} — an explicit key on a closed relation is rejected as a duplicate`
		})
	}
	const seen = new Set<string>()
	for (const fieldName of fields) {
		if (seen.has(fieldName)) {
			throw new AuthoringError({
				message: `key(${relation.name}, ...): the projection spells ${fieldName} twice — write it once (the canonical-utterance law: one meaning, one spelling)`
			})
		}
		seen.add(fieldName)
	}
	const data: KeyData<R, Projection> = Object.freeze({
		kind: "key",
		owner: relation,
		projection: Object.freeze(fields)
	})
	return Object.freeze({ data, [admitted]: true as const })
}

function contained<A extends AnyFace, B extends AnyFace>(
	source: A,
	target: B & SameArity<A, B> & SameShapes<A, B>
): ContainedStatement<A["data"], B["data"]> {
	const data: ContainmentData<A["data"], B["data"]> = Object.freeze({
		kind: "containment",
		source: source.data,
		target: target.data
	})
	const statement = Object.freeze({ data, [admitted]: true as const })
	assertArityAgreement(data.source, data.target, statement)
	assertRosterAgreement(data.source, data.target, statement)
	return statement
}

function mirrors<A extends AnyFace, B extends AnyFace>(
	source: A,
	target: B & SameArity<A, B> & SameShapes<A, B>
): ContainedStatement<A["data"], B["data"]> {
	const data: MirrorsData<A["data"], B["data"]> = Object.freeze({
		kind: "mirrors",
		source: source.data,
		target: target.data
	})
	const statement = Object.freeze({ data, [admitted]: true as const })
	assertArityAgreement(data.source, data.target, statement)
	assertRosterAgreement(data.source, data.target, statement)
	return statement
}

function assertWeightOnSource(weight: WeightSpec, source: FaceData, statement: Statement): void {
	if (weight.kind === "unit") {
		return
	}
	const field = sealedFieldOf(source.owner, weight.field)
	if (field === undefined) {
		throw new AuthoringError({
			message: `${source.owner.name} has no field ${weight.field} — a weight names a field of the SOURCE's own row (the weight vocabulary is closed at the row) — ${renderStatement(statement)}`
		})
	}
	if (weight.kind === "field" && field.kind !== "u64") {
		throw new AuthoringError({
			message: `${source.owner.name}.${weight.field} is ${field.kind}, not u64 — a weight is u64-encoded (a signed weight would break the polarity scheduler: an insert could lower a sum) — ${renderStatement(statement)}`
		})
	}
	if (weight.kind === "durationField" && field.kind !== "interval") {
		throw new AuthoringError({
			message: `${source.owner.name}.${weight.field} is ${field.kind}, not an interval — Duration(...) weighs an interval field's measure — ${renderStatement(statement)}`
		})
	}
}

function assertBoundsOnTarget(window: CapacityWindowSpec, target: FaceData, statement: Statement): void {
	const bounds = window.kind === "range" ? [window.lo, window.hi] : [window.kind === "exact" ? window.n : window.lo]
	for (const bound of bounds) {
		if (bound.kind === "lit") {
			continue
		}
		const field = sealedFieldOf(target.owner, bound.field)
		if (field === undefined) {
			throw new AuthoringError({
				message: `${target.owner.name} has no field ${bound.field} — a dependent bound names a field of the TARGET's own row (bound names resolve against the target's full roster) — ${renderStatement(statement)}`
			})
		}
		if (bound.kind === "field" && field.kind !== "u64") {
			throw new AuthoringError({
				message: `${target.owner.name}.${bound.field} is ${field.kind}, not u64 — a dependent bound reads a u64 field of the TARGET row (Duration(...) is the interval-measure spelling) — ${renderStatement(statement)}`
			})
		}
		if (bound.kind === "durationField" && field.kind !== "interval") {
			throw new AuthoringError({
				message: `${target.owner.name}.${bound.field} is ${field.kind}, not an interval — Duration(...) bounds by an interval field's measure — ${renderStatement(statement)}`
			})
		}
	}
}

function capacity<B extends AnyFace, W extends CapacityWindow, A extends AnyFace>(
	target: B,
	window: W & UnitWindowBan<W> & UnitDimensionBan<W> & BoundsOnTarget<W, B>,
	source: A & SameArity<B, A> & SameShapes<B, A>
): CapacityStatement<B["data"], A["data"]>
function capacity<B extends AnyFace, M extends CapacityWeight, W extends CapacityWindow, A extends AnyFace>(
	target: B,
	weight: M & WeightOnSource<M, A>,
	window: W & BoundsOnTarget<W, B>,
	source: A & SameArity<B, A> & SameShapes<B, A>
): CapacityStatement<B["data"], A["data"]>
function capacity(
	target: AnyFace,
	second: unknown,
	third: unknown,
	fourth?: AnyFace
): CapacityStatement<FaceData, FaceData> {
	const weighted = fourth !== undefined
	const windowValue = weighted ? third : second
	const source = weighted ? fourth : (third as AnyFace)
	if (!isCapacityWindow(windowValue)) {
		throw new AuthoringError({
			message:
				"a capacity window is minted only by within() — a structural literal skips the ban table (the canonical-utterance law)"
		})
	}
	let weight: WeightSpec = unitWeight
	if (weighted) {
		if (!isCapacityWeight(second)) {
			throw new AuthoringError({
				message: "a capacity weight is minted only by weigh() — a structural literal skips the row-local weight wall"
			})
		}
		weight = second.weight
	}
	const window = windowValue.window
	if (weight.kind === "unit" && window.kind === "floor" && window.lo.kind === "lit" && window.lo.value === 1n) {
		throw new AuthoringError({
			message:
				"`{1..*}` on the unit instance says only what the bare containment says — drop the annotation and write the containment: contained(source, target)"
		})
	}
	if (weight.kind === "unit" && window.kind === "floor") {
		throw new AuthoringError({
			message:
				"`{N..*}` on the unit instance — a bare count floor is refused; weigh the source (`<=[w]{N..*}` stays legal) or drop the bound"
		})
	}

	// CapacityDimensionMixing twin — ruled 2026-07-24): a count of facts

	if (weight.kind === "unit" && window.kind === "range" && window.hi.kind === "durationField") {
		throw new AuthoringError({
			message: `a unit (count) window against the duration() bound on ${window.hi.field} mixes dimensions (C18) — weigh the source with weigh(duration(field)), or bound by a u64 field or literal`
		})
	}
	const data: CapacityData = Object.freeze({
		kind: "capacity",
		target: target.data,
		weight,
		window,
		source: source.data
	})
	const statement = Object.freeze({ data, [admitted]: true as const })
	assertArityAgreement(data.source, data.target, statement)
	assertRosterAgreement(data.source, data.target, statement)
	assertWeightOnSource(weight, data.source, statement)
	assertBoundsOnTarget(window, data.target, statement)
	return statement
}

function renderStatement(statement: Statement): string {
	const data = statement.data
	switch (data.kind) {
		case "key":
			return `${data.owner.name}(${data.projection.join(", ")}) -> ${data.owner.name}`
		case "containment":
			return `${renderFace(data.source)} <= ${renderFace(data.target)}`
		case "mirrors":
			return `${renderFace(data.source)} == ${renderFace(data.target)}`
		case "capacity":
			return `${renderFace(data.target)} <=${renderWeight(data.weight)}${renderCapacityWindow(data.window)} ${renderFace(data.source)}`
	}
}

export type {
	CapacityData,
	CapacityStatement,
	ContainedStatement,
	ContainmentData,
	KeyData,
	KeyStatement,
	Statement,
	StatementData
}
export { capacity, contained, isStatement, key, mirrors, renderStatement }
