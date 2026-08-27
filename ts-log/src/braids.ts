/**
 * Braid derivation as data (10/20): connected components of the
 * statement graph over ordinary relations, the braid id the smallest
 * RelationId in the component rendered `c{id:08x}`. The one derivation
 * lives in `crates/bumbledb-log` and reaches the descriptor parse
 * through the engine bridge (`internalLogBraidsOf`); this façade names
 * that derivation in the driver's vocabulary.
 */

import type { Braid, SerialStatement, Theory } from "#descriptor.ts"
import { braid, braidHex, descriptorOf } from "#descriptor.ts"

const U32_MAX = 0xffffffff

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
 * serializes at that statement. The statement ids are the log core's
 * own roster, read off the descriptor's derivation.
 */
function serialAtStatementsOf(theory: Theory): readonly SerialStatement[] {
	return descriptorOf(theory).serialAtStatements
}

/**
 * Parses a wire u32 into a braid id: valid only when the relation it
 * names is the smallest in its own component. An unknown, closed, or
 * non-head id is not a braid — the caller refuses, it is not ignored.
 */
function parse(theory: Theory, raw: number): Braid | undefined {
	if (!Number.isInteger(raw) || raw < 0 || raw > U32_MAX) {
		return undefined
	}
	const name = braidHex(raw)
	return descriptorOf(theory).braidMembers.has(name) ? name : undefined
}

export type { Braid }
export { braid, braidsOf, parse, serialAtStatementsOf }
