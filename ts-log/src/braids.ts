/**
 * Braid derivation as data (10): connected components of the statement
 * graph over ordinary relations, the braid id the smallest RelationId in
 * the component rendered `c{id:08x}`. Assignment is a pure function of
 * the descriptor, pinned cross-language by the codec goldens.
 */

import type { Braid, SerialStatement, Theory } from "#descriptor.ts"
import { braid, descriptorOf } from "#descriptor.ts"

/** The schema's own shard map: ordinary relation name → braid id. */
function braidsOf(theory: Theory): ReadonlyMap<string, Braid> {
	const descriptor = descriptorOf(theory)
	const out = new Map<string, Braid>()
	for (const relation of descriptor.relations) {
		const braid = descriptor.braidOfRelation.get(relation.id)
		if (braid !== undefined) {
			out.set(relation.name, braid)
		}
	}
	return out
}

/**
 * The degenerate-serial roster (15): key or capacity statements whose
 * determinant projection is empty name one global group, so their braid
 * serializes at that statement. Typed data beside the braid map, one
 * question per verb.
 */
function serialAtStatementsOf(theory: Theory): readonly SerialStatement[] {
	return descriptorOf(theory).serialAtStatements
}

export type { Braid }
export { braid, braidsOf, serialAtStatementsOf }
