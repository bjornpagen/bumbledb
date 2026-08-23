/**
 * The pairwise intersection a CAS loser runs (15): subsumed, disjoint in
 * the strict sense (zero shared keys of any class, commute cells
 * included), a quantitative W test on parent keys shared child-to-child,
 * and conflict for everything else. The winner's footprint is always a
 * recomputation from its fetched ops — the carried-and-checked law —
 * so this function takes ops, never published sections.
 */

import type { LogTheory } from "#descriptor.ts"
import { descriptorOf } from "#descriptor.ts"
import type { BatchOp, FootprintRich } from "#footprint.ts"
import { computeFootprint, keyIdentity } from "#footprint.ts"

interface SharedKey {
	readonly class: "F" | "K" | "C" | "W"
	readonly statement: number | undefined
	readonly keyHex: string
}

interface DeltaInterval {
	readonly lo: bigint
	readonly hi: bigint
}

interface SharedCapacityParent {
	readonly statement: number
	readonly keyHex: string
	readonly loser: DeltaInterval
	readonly winner: DeltaInterval
}

type Intersection =
	| { readonly tag: "subsumed" }
	| { readonly tag: "disjoint" }
	| { readonly tag: "capacity"; readonly parents: readonly SharedCapacityParent[] }
	| { readonly tag: "conflict"; readonly shared: readonly SharedKey[] }

function intersectRich(loser: FootprintRich, winner: FootprintRich): Intersection {
	const winnerByIdentity = new Map<string, (typeof winner.entries)[number]>()
	const winnerKeys = new Map<string, (typeof winner.entries)[number][]>()
	for (const entry of winner.entries) {
		winnerByIdentity.set(`${keyIdentity(entry)}:${"mode" in entry ? entry.mode : ""}`, entry)
		const key = keyIdentity(entry)
		const bucket = winnerKeys.get(key)
		if (bucket === undefined) {
			winnerKeys.set(key, [entry])
		} else {
			bucket.push(entry)
		}
	}

	const loserFacts = loser.entries.filter(function facts(entry) {
		return entry.class === "F"
	})
	const subsumed =
		loserFacts.length > 0 &&
		loserFacts.every(function coveredSameMode(entry) {
			return winnerByIdentity.has(`${keyIdentity(entry)}:${"mode" in entry ? entry.mode : ""}`)
		})
	if (subsumed) {
		return { tag: "subsumed" }
	}

	const shared: SharedKey[] = []
	const capacityParents: SharedCapacityParent[] = []
	let hardConflict = false
	const seen = new Set<string>()
	for (const entry of loser.entries) {
		const key = keyIdentity(entry)
		if (seen.has(key)) {
			continue
		}
		seen.add(key)
		const winnerBucket = winnerKeys.get(key)
		if (winnerBucket === undefined) {
			continue
		}
		if (entry.class !== "W") {
			hardConflict = true
			shared.push({
				class: entry.class,
				statement: entry.class === "F" ? undefined : entry.statement,
				keyHex: key.split(":")[2] ?? ""
			})
			continue
		}
		const loserModes = loser.entries.filter(function sameKey(candidate) {
			return keyIdentity(candidate) === key
		})
		const anyParent =
			loserModes.some(function parentArm(candidate) {
				return "mode" in candidate && (candidate.mode === "parent+" || candidate.mode === "parent-")
			}) ||
			winnerBucket.some(function parentArm(candidate) {
				return "mode" in candidate && (candidate.mode === "parent+" || candidate.mode === "parent-")
			})
		if (anyParent) {
			hardConflict = true
			shared.push({ class: "W", statement: entry.statement, keyHex: key.split(":")[2] ?? "" })
			continue
		}
		const identity = `W:${entry.statement}:${key.split(":")[2] ?? ""}`
		const loserInterval = loser.intervals.get(identity)
		const winnerInterval = winner.intervals.get(identity)
		if (loserInterval === undefined || winnerInterval === undefined) {
			hardConflict = true
			shared.push({ class: "W", statement: entry.statement, keyHex: key.split(":")[2] ?? "" })
			continue
		}
		capacityParents.push({
			statement: entry.statement,
			keyHex: loserInterval.keyHex,
			loser: { lo: loserInterval.lo, hi: loserInterval.hi },
			winner: { lo: winnerInterval.lo, hi: winnerInterval.hi }
		})
	}

	if (hardConflict) {
		return { tag: "conflict", shared }
	}
	if (capacityParents.length > 0) {
		return { tag: "capacity", parents: capacityParents }
	}
	return { tag: "disjoint" }
}

/**
 * Intersects a loser's batch against a winner's, both as raw ops built
 * on the same base slot. Footprints are recomputed here, never trusted.
 */
function intersectionOf(theory: LogTheory, loserOps: readonly BatchOp[], winnerOps: readonly BatchOp[]): Intersection {
	const descriptor = descriptorOf(theory)
	return intersectRich(computeFootprint(descriptor, loserOps), computeFootprint(descriptor, winnerOps))
}

interface CapacitySlack {
	/** `ceiling − measure(base)`; null where unbounded. */
	readonly plus: bigint | null
	/** `measure(base) − floor`; null where unbounded. */
	readonly minus: bigint | null
}

/**
 * The W interval test at known slack: concurrent batches commute iff the
 * worst-case endpoints respect both bounds — `Σ max-endpoints ≤ slack⁺`
 * and `Σ min-endpoints ≥ −slack⁻` at every shared parent.
 */
function capacityCommutes(
	parents: readonly SharedCapacityParent[],
	slackOf: (statement: number, keyHex: string) => CapacitySlack
): boolean {
	return parents.every(function bounded(parent) {
		const slack = slackOf(parent.statement, parent.keyHex)
		const maxSum = parent.loser.hi + parent.winner.hi
		const minSum = parent.loser.lo + parent.winner.lo
		if (slack.plus !== null && maxSum > slack.plus) {
			return false
		}
		if (slack.minus !== null && minSum < -slack.minus) {
			return false
		}
		return true
	})
}

export type { CapacitySlack, DeltaInterval, Intersection, SharedCapacityParent, SharedKey }
export { capacityCommutes, intersectionOf, intersectRich }
