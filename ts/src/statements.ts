import {
	type BoundsOnTarget,
	type CapacityWeight,
	type CapacityWindow,
	capacityWeight,
	capacityWindow,
	type UnitDimensionBan,
	unitWeight,
	type WeightOnSource
} from "#capacity.ts"
import { isClosedMember, memberDescriptor, sealedFieldOf } from "#closed.ts"
import { AuthoringError } from "#errors.ts"
import { type AnyFace, faceDescriptor, renderFace, type SameArity, type SameShapes } from "#face.ts"
import { type AnyClosedRoster, assertDeclarationRecord, rosterOf, rostersAgree, signaturesAgree } from "#fields.ts"
import { descriptorCache } from "#immutable.ts"
import type { AnyRelation, RelationFields } from "#relation.ts"
import { type CapacityWindowSpec, renderCapacityWindow, renderWeight, type WeightSpec } from "#spec.ts"
import { arrayValue, recordValue } from "#values.ts"

interface KeyStatement<R extends AnyRelation = AnyRelation, Projection extends readonly string[] = readonly string[]> {
	readonly kind: "key"
	readonly owner: R
	readonly projection: Projection
}
interface ContainmentStatement<Src extends AnyFace = AnyFace, Tgt extends AnyFace = AnyFace> {
	readonly kind: "containment"
	readonly source: Src
	readonly target: Tgt
}
interface MirrorsStatement<Src extends AnyFace = AnyFace, Tgt extends AnyFace = AnyFace> {
	readonly kind: "mirrors"
	readonly source: Src
	readonly target: Tgt
}
interface CapacityStatement<Tgt extends AnyFace = AnyFace, Src extends AnyFace = AnyFace> {
	readonly kind: "capacity"
	readonly target: Tgt
	readonly weight: CapacityWeight
	readonly window: CapacityWindow
	readonly source: Src
}
type Statement = KeyStatement | ContainmentStatement | MirrorsStatement | CapacityStatement

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
function assertArityAgreement(source: AnyFace, target: AnyFace, statement: Statement): void {
	if (source.projection.length !== target.projection.length) {
		throw new AuthoringError({
			message: `${source.owner.name}(${source.projection.join(", ")}) and ${target.owner.name}(${target.projection.join(", ")}) project ${source.projection.length} vs ${target.projection.length} fields — positional pairing requires both faces to project equally many — ${renderStatement(statement)}`
		})
	}
}

function assertRosterAgreement(source: AnyFace, target: AnyFace, statement: Statement): void {
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

function assertWeightOnSource(weight: WeightSpec, source: AnyFace, statement: Statement): void {
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

function assertBoundsOnTarget(window: CapacityWindowSpec, target: AnyFace, statement: Statement): void {
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

function assertShapes(source: AnyFace, target: AnyFace): void {
	for (let i = 0; i < source.projection.length; i++) {
		const a = sealedFieldOf(source.owner, source.projection[i] ?? "")
		const b = sealedFieldOf(target.owner, target.projection[i] ?? "")
		if (a === undefined || b === undefined) throw new AuthoringError({ message: "statement: unknown projected field" })
		const left = a.kind === "interval" ? { ...a, width: undefined } : a
		const right = b.kind === "interval" ? { ...b, width: undefined } : b
		if (!signaturesAgree(left, right))
			throw new AuthoringError({ message: "statement: projected field shapes do not agree" })
	}
}

/** Every authoring path checks and owns the same plain descriptor. */
function statementDescriptor<S extends Statement>(input: S): S
function statementDescriptor(input: unknown): Statement
function statementDescriptor(input: unknown): Statement {
	return checkedStatement(input)
}

const checkedStatement = descriptorCache((raw): Statement => {
	const input = recordValue("statement", raw, typeof raw === "object" && raw !== null ? Object.keys(raw) : [])
	assertDeclarationRecord("statement", input)
	if (input?.kind === "key") {
		recordValue("key", input, ["kind", "owner", "projection"])
		const owner = memberDescriptor(input.owner as unknown)
		if (isClosedMember(owner))
			throw new AuthoringError({
				message: `key(${owner.name}, ...): closedness already materializes its key; explicit keys on closed relations are duplicates`
			})
		const projection = arrayValue("key projection", input.projection, (_, value) => {
			if (typeof value !== "string") throw new AuthoringError({ message: "key: expected a field name" })
			return value
		})
		if (projection.length === 0) throw new AuthoringError({ message: "key: expected a nonempty field projection" })
		const seen = new Set<string>()
		for (const field of projection) {
			if (typeof field !== "string" || sealedFieldOf(owner, field) === undefined)
				throw new AuthoringError({ message: `key(${owner.name}, ...): unknown field ${String(field)}` })
			if (seen.has(field))
				throw new AuthoringError({ message: `key(${owner.name}, ...): the projection spells ${field} twice` })
			seen.add(field)
		}
		return Object.freeze({ kind: "key", owner, projection })
	}
	if (input?.kind !== "containment" && input?.kind !== "mirrors" && input?.kind !== "capacity") {
		throw new AuthoringError({ message: "statement: unknown kind" })
	}
	recordValue(
		"statement",
		input,
		input.kind === "capacity" ? ["kind", "target", "weight", "window", "source"] : ["kind", "source", "target"]
	)
	const source = faceDescriptor(input.source)
	const target = faceDescriptor(input.target)
	const statement =
		input.kind === "capacity"
			? Object.freeze({
					kind: input.kind,
					source,
					target,
					weight: capacityWeight(input.weight),
					window: capacityWindow(input.window)
				})
			: Object.freeze({ kind: input.kind, source, target })
	assertArityAgreement(source, target, statement)
	assertRosterAgreement(source, target, statement)
	assertShapes(source, target)
	if (statement.kind === "capacity") {
		const { weight, window } = statement
		if (weight.kind === "unit" && window.kind === "range" && window.hi.kind === "durationField") {
			throw new AuthoringError({ message: "a unit (count) window against a duration bound mixes dimensions (C18)" })
		}
		assertWeightOnSource(weight, source, statement)
		assertBoundsOnTarget(window, target, statement)
	}
	return statement
})

function key<
	R extends AnyRelation,
	const Projection extends readonly [keyof RelationFields<R> & string, ...(keyof RelationFields<R> & string)[]]
>(relation: R, fields: Projection): KeyStatement<R, Projection> {
	return statementDescriptor({ kind: "key", owner: relation, projection: fields })
}

function contained<A extends AnyFace, B extends AnyFace>(
	source: A,
	target: B & SameArity<A, B> & SameShapes<A, B>
): ContainmentStatement<A, B> {
	return statementDescriptor({ kind: "containment", source, target })
}

function mirrors<A extends AnyFace, B extends AnyFace>(
	source: A,
	target: B & SameArity<A, B> & SameShapes<A, B>
): MirrorsStatement<A, B> {
	return statementDescriptor({ kind: "mirrors", source, target })
}

function capacity<B extends AnyFace, W extends CapacityWindow, A extends AnyFace>(
	target: B,
	options: {
		readonly from: A & SameArity<B, A> & SameShapes<B, A>
		readonly within: W & UnitDimensionBan<W> & BoundsOnTarget<W, B>
	}
): CapacityStatement<B, A>
function capacity<B extends AnyFace, M extends CapacityWeight, W extends CapacityWindow, A extends AnyFace>(
	target: B,
	options: {
		readonly from: A & SameArity<B, A> & SameShapes<B, A>
		readonly weight: M & WeightOnSource<M, A>
		readonly within: W & BoundsOnTarget<W, B>
	}
): CapacityStatement<B, A>
function capacity(
	target: AnyFace,
	options: { readonly from: AnyFace; readonly weight?: unknown; readonly within: unknown }
): CapacityStatement {
	recordValue(
		"capacity options",
		options,
		Object.hasOwn(options, "weight") ? ["from", "weight", "within"] : ["from", "within"]
	)
	return statementDescriptor({
		kind: "capacity",
		target,
		source: options.from,
		weight: options.weight === undefined ? unitWeight : capacityWeight(options.weight),
		window: capacityWindow(options.within)
	})
}

function renderStatement(statement: Statement): string {
	switch (statement.kind) {
		case "key":
			return `${statement.owner.name}(${statement.projection.join(", ")}) -> ${statement.owner.name}`
		case "containment":
			return `${renderFace(statement.source)} <= ${renderFace(statement.target)}`
		case "mirrors":
			return `${renderFace(statement.source)} == ${renderFace(statement.target)}`
		case "capacity":
			return `${renderFace(statement.target)} <=${renderWeight(statement.weight)}${renderCapacityWindow(statement.window)} ${renderFace(statement.source)}`
	}
}

export type { CapacityStatement, ContainmentStatement, KeyStatement, MirrorsStatement, Statement }
export { capacity, contained, key, mirrors, renderStatement, statementDescriptor }
