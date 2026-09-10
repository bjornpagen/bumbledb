import { type AnyClosed, memberDescriptor, sealedFieldOf, sealedFieldsOf } from "#closed.ts"
import { AuthoringError } from "#errors.ts"
import { type AnyField, assertDeclarationRecord, type ClosedIdField, type Infer, literalOf } from "#fields.ts"
import type { AnyRelation, RelationFields } from "#relation.ts"
import { type LiteralSetSpec, type LiteralSpec, renderLiteral } from "#spec.ts"
import { arrayValue, recordValue } from "#values.ts"

type Selectable = AnyRelation | AnyClosed
type FieldsOf<R extends Selectable> = R extends AnyRelation
	? RelationFields<R>
	: R extends AnyClosed
		? { readonly id: ClosedIdField<R["name"], R["handles"]> } & R["columns"]
		: never
type SelectionInput<R extends Selectable> = {
	readonly [K in keyof FieldsOf<R>]?: Infer<FieldsOf<R>[K]> | readonly Infer<FieldsOf<R>[K]>[]
}
interface SelectionBinding {
	readonly field: string
	readonly set: LiteralSetSpec
}
interface Selected<R extends Selectable = Selectable> {
	readonly relation: R
	readonly selection: readonly SelectionBinding[]
}
type AnySelected = Selected

/** Checked logical literals use the same field interpreter as fact values. */
function selectionLiteral(field: AnyField, input: unknown): LiteralSpec {
	const raw = recordValue("selection literal", input, "closed" in field ? ["kind", "handle"] : ["kind", "value"])
	if ("closed" in field) {
		if (raw.kind !== "handle") throw new AuthoringError({ message: "closed selection requires a handle literal" })
		return Object.freeze(literalOf(field, raw.handle))
	}
	if (raw.kind !== "value") throw new AuthoringError({ message: "selection requires a value literal" })
	const value = recordValue(
		"selection value",
		raw.value,
		field.kind === "interval" ? ["kind", "start", "end"] : ["kind", "value"]
	)
	let kind: string = field.kind
	if (field.kind === "str") kind = "string"
	if (field.kind === "bytes") kind = "fixedBytes"
	if (field.kind === "interval") kind = { u64: "intervalU64", i64: "intervalI64", f64: "intervalF64" }[field.element]
	if (value.kind !== kind) throw new AuthoringError({ message: `selection requires the ${kind} value tag` })
	const literal = literalOf(field, field.kind === "interval" ? { start: value.start, end: value.end } : value.value)
	if (literal.kind === "value") Object.freeze(literal.value)
	return Object.freeze(literal)
}

function selectionBindings(owner: Selectable, input: unknown): readonly SelectionBinding[] {
	if (!Array.isArray(input)) throw new AuthoringError({ message: "selection: expected bindings" })
	const selected = new Set<string>()
	return Object.freeze(
		arrayValue("selection", input, (_, raw) => {
			const binding = recordValue("selection binding", raw, ["field", "set"])
			if (typeof binding.field !== "string") throw new AuthoringError({ message: "selection: expected a field name" })
			const field = sealedFieldOf(owner, binding.field)
			if (field === undefined)
				throw new AuthoringError({ message: `relation ${owner.name} has no field ${binding.field}` })
			if (selected.has(binding.field)) throw new AuthoringError({ message: "selection: duplicate field" })
			selected.add(binding.field)
			const set = binding.set
			if (typeof set !== "object" || set === null || !("kind" in set))
				throw new AuthoringError({ message: "literal set: expected a descriptor" })
			assertDeclarationRecord("literal set", set)
			if (set.kind === "one") {
				const rawSet = recordValue("literal set", set, ["kind", "literal"])
				return Object.freeze({
					field: binding.field,
					set: Object.freeze({ kind: "one" as const, literal: selectionLiteral(field, rawSet.literal) })
				})
			}
			if (set.kind !== "many") throw new AuthoringError({ message: "literal set: unknown kind" })
			const rawSet = recordValue("literal set", set, ["kind", "literals"])
			if (!Array.isArray(rawSet.literals) || rawSet.literals.length < 2)
				throw new AuthoringError({ message: "literal set: expected at least two distinct literals" })
			const literals = arrayValue("selection literals", rawSet.literals, (_, literal) =>
				selectionLiteral(field, literal)
			)
			if (new Set(literals.map(renderLiteral)).size !== literals.length)
				throw new AuthoringError({ message: "literal set: duplicate literal" })
			return Object.freeze({
				field: binding.field,
				set: Object.freeze({ kind: "many" as const, literals: Object.freeze(literals) })
			})
		})
	)
}

function select<R extends Selectable>(relation: R, input: SelectionInput<R>): Selected<R> {
	const owner = memberDescriptor(relation)
	assertDeclarationRecord(`relation ${owner.name} selection`, input)
	const fields = sealedFieldsOf(owner)
	const selection: SelectionBinding[] = Object.entries(input).map(([name, entry]) => {
		const field = fields.find((field) => field.name === name)?.field
		if (field === undefined) throw new AuthoringError({ message: `relation ${owner.name} has no field ${name}` })
		if (!Array.isArray(entry)) return { field: name, set: { kind: "one", literal: literalOf(field, entry) } }
		const context = `relation ${owner.name}.${name}`
		if (entry.length < 2)
			throw new AuthoringError({
				message:
					entry.length === 0
						? `${context}: an empty literal set selects nothing — write the selection you mean`
						: `${context}: a one-element literal set is the bare literal respelled — write the literal`
			})
		const literals = arrayValue(context, entry, (_, value) => literalOf(field, value))
		const seen = new Set<string>()
		for (const literal of literals) {
			const rendered = renderLiteral(literal)
			if (seen.has(rendered))
				throw new AuthoringError({ message: `${context}: the literal set spells ${rendered} twice — write it once` })
			seen.add(rendered)
		}
		return { field: name, set: { kind: "many", literals } }
	})
	if (selection.length === 0)
		throw new AuthoringError({
			message: `relation ${owner.name}: an empty selection is the bare relation respelled — pass the relation itself`
		})
	return Object.freeze({ relation: owner, selection: selectionBindings(owner, selection) })
}

export type { AnySelected, FieldsOf, Selectable, Selected, SelectionBinding, SelectionInput }
export { select, selectionBindings, selectionLiteral }
