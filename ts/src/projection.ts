/** `true` is contextual full Event syntax, never a stored field or value. */
import { type AnyClosed, sealedFieldOf } from "#closed.ts"
import { AuthoringError } from "#errors.ts"
import { type AnyField, event } from "#fields.ts"
import type { AnyRelation } from "#relation.ts"
import type { ProjectionSpec } from "#spec.ts"
import { arrayValue } from "#values.ts"

export type ProjectionTerm = string | true
export type NonemptyProjection<F extends string> = readonly [F, ...F[]] | readonly [...F[], true]
export function projectedField(owner: AnyRelation | AnyClosed, term: ProjectionTerm): AnyField | undefined {
	return term === true ? event : sealedFieldOf(owner, term)
}
export function storedProjection(projection: readonly ProjectionTerm[]): readonly string[] {
	return projection.filter((term): term is string => term !== true)
}
export function projectionDescriptor(
	owner: AnyRelation | AnyClosed,
	input: unknown,
	kind: "key" | "face" = "face"
): readonly ProjectionTerm[] {
	const terms = arrayValue("projection", input, (_, value) => {
		if (value !== true && typeof value !== "string")
			throw new AuthoringError({ message: "projection: expected a field name or trailing true" })
		return value
	})
	if (terms.length === 0) throw new AuthoringError({ message: "projection: expected a nonempty projection" })
	const seen = new Set<string>()
	const full = terms.at(-1) === true
	for (const [i, term] of terms.entries()) {
		if (term === true) {
			if (i !== terms.length - 1) throw new AuthoringError({ message: "projection: true must appear only at the end" })
			continue
		}
		const field = sealedFieldOf(owner, term)
		if (field === undefined) throw new AuthoringError({ message: `projection ${owner.name}: unknown field ${term}` })
		if (seen.has(term))
			throw new AuthoringError({
				message:
					kind === "key"
						? `key(${owner.name}, ...): the projection spells ${term} twice`
						: `face ${owner.name}: duplicate field ${term}`
			})
		seen.add(term)
		if (full && (field.kind === "event" || field.kind === "interval"))
			throw new AuthoringError({ message: "projection: true requires a scalar field prefix" })
	}
	return terms
}
export function lowerProjection(terms: readonly ProjectionTerm[]): ProjectionSpec {
	const fields = storedProjection(terms)
	return terms.at(-1) === true ? [...fields, { event: "full" }] : [...fields]
}
export function renderProjection(terms: readonly ProjectionTerm[]): string {
	return terms
		.map((term) => {
			if (term === true) return "true"
			if (term !== "true" && /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(term)) return term
			return JSON.stringify(term)
		})
		.join(", ")
}
