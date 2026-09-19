import { type AnyClosed, closedDescriptor, closedId, isClosedMember, sealedFieldOf } from "#closed.ts"
import { AuthoringError } from "#errors.ts"
import type { Face, SameArity, SameShapes } from "#face.ts"
import { type ClosedIdField, signaturesAgree } from "#fields.ts"
import type { AnyRelation, RelationFields } from "#relation.ts"
import {
	type ContainmentStatement,
	type KeyStatement,
	type MirrorsStatement,
	statementDescriptor
} from "#statements.ts"
import { recordValue } from "#values.ts"

type IdentityKey = KeyStatement<AnyRelation>
type KeyFace<K extends IdentityKey> = Face<K["owner"], K["projection"]>
type ScalarKey<K extends IdentityKey> =
	Extract<
		RelationFields<K["owner"]>[Extract<K["projection"][number], string>],
		{ readonly kind: "interval" | "event" }
	> extends never
		? unknown
		: { readonly "alternative identity keys must be scalar": never }
type Discriminator<P extends IdentityKey, C extends AnyClosed> = {
	[F in keyof RelationFields<P["owner"]>]: RelationFields<P["owner"]>[F] extends ClosedIdField<C["name"], C["handles"]>
		? F
		: never
}[keyof RelationFields<P["owner"]>] &
	string

function scalarKey(input: IdentityKey): IdentityKey {
	const checked = statementDescriptor(input)
	if (checked.kind !== "key") throw new AuthoringError({ message: "alternatives: expected a key statement" })
	if (isClosedMember(checked.owner))
		throw new AuthoringError({ message: "alternatives: identity keys must belong to ordinary relations" })
	for (const name of checked.projection) {
		if (name === true) continue
		if (["interval", "event"].includes(sealedFieldOf(checked.owner, name)?.kind ?? ""))
			throw new AuthoringError({
				message: "alternatives: identity keys must be scalar; region keys prove point coverage"
			})
	}
	return checked
}

/**
 * Exhaustive closed alternatives, expanded into ordinary laws. Declare the
 * supplied keys in the schema once; this helper emits only the discriminator
 * containment followed by one mirrors statement per handle in roster order.
 * No new statement kind or assembled payload representation is introduced.
 */
function alternatives<
	P extends IdentityKey,
	C extends AnyClosed,
	const Arms extends Record<C["handles"][number], IdentityKey>
>(
	parent: P & ScalarKey<P>,
	discriminator: Discriminator<NoInfer<P>, NoInfer<C>>,
	roster: C,
	arms: Arms &
		Record<Exclude<keyof Arms, C["handles"][number]>, never> & {
			readonly [H in C["handles"][number]]: ScalarKey<Arms[H]> &
				SameArity<KeyFace<P>, KeyFace<Arms[H]>> &
				SameShapes<KeyFace<P>, KeyFace<Arms[H]>>
		}
): readonly (ContainmentStatement | MirrorsStatement)[] {
	const primary = scalarKey(parent)
	const closed = closedDescriptor(roster)
	const field = sealedFieldOf(primary.owner, discriminator)
	if (field === undefined || !signaturesAgree(field, closedId(closed)))
		throw new AuthoringError({ message: "alternatives: discriminator must reference the supplied closed roster" })
	const payloads = recordValue("alternative arms", arms, closed.handles)
	const statements: (ContainmentStatement | MirrorsStatement)[] = [
		statementDescriptor({
			kind: "containment",
			source: { owner: primary.owner, projection: [discriminator], selection: [] },
			target: { owner: closed, projection: ["id"], selection: [] }
		})
	]
	const children = new Set<string>([primary.owner.name])
	for (const handle of closed.handles) {
		const child = scalarKey(payloads[handle] as IdentityKey)
		if (children.has(child.owner.name))
			throw new AuthoringError({
				message: `alternatives: each arm needs its own payload relation (${child.owner.name})`
			})
		children.add(child.owner.name)
		statements.push(
			statementDescriptor({
				kind: "mirrors",
				source: {
					owner: primary.owner,
					projection: primary.projection,
					selection: [{ field: discriminator, set: { kind: "one", literal: { kind: "handle", handle } } }]
				},
				target: { owner: child.owner, projection: child.projection, selection: [] }
			})
		)
	}
	return Object.freeze(statements)
}

export { alternatives }
