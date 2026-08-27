/**
 * The per-braid generation map. Sum, domination, checkpoint order, and
 * apply's increment live here. Overflow is a refusal of `sum` and of
 * nowhere else.
 */

import { U64_MAX } from "#bytes.ts"
import type { Braid } from "#descriptor.ts"
import type { Generation } from "#keys.ts"
import { generation } from "#keys.ts"

const Overflow = { tag: "overflow" } as const
type Overflow = typeof Overflow

type CheckpointOrder = "before" | "equal" | "after"

/** Applied counts keyed by braid. Any vector is a legal restore point. */
class Vector {
	readonly #counts: Map<Braid, bigint>

	constructor(entries: Iterable<readonly [Braid, bigint]> = []) {
		const counts = new Map<Braid, bigint>()
		for (const [braid, g] of [...entries].sort(function byBraid(left, right) {
			if (left[0] < right[0]) {
				return -1
			}
			if (left[0] > right[0]) {
				return 1
			}
			return 0
		})) {
			counts.set(braid, g)
		}
		this.#counts = counts
	}

	static from(map: ReadonlyMap<Braid, bigint>): Vector {
		return new Vector(map)
	}

	/** The wholeness arithmetic. The one overflow site. */
	sum(): bigint | Overflow {
		let acc = 0n
		for (const g of this.#counts.values()) {
			const next = acc + g
			if (next > U64_MAX) {
				return Overflow
			}
			acc = next
		}
		return acc
	}

	/** Pointwise `this[braid] >= other[braid]`; an absent braid is zero. */
	dominates(other: Vector): boolean {
		for (const [braid, g] of other.#counts) {
			if (this.at(braid) < g) {
				return false
			}
		}
		return true
	}

	/** The total order the manifest CAS installs. */
	order(other: Vector): CheckpointOrder {
		const left = this.sum()
		const right = other.sum()
		const leftOverflow = typeof left !== "bigint"
		const rightOverflow = typeof right !== "bigint"
		if (leftOverflow && rightOverflow) {
			return "equal"
		}
		if (leftOverflow) {
			return "after"
		}
		if (rightOverflow) {
			return "before"
		}
		if (left < right) {
			return "before"
		}
		if (left > right) {
			return "after"
		}
		return "equal"
	}

	/** The applied count for `braid`; absent is zero. */
	at(braid: Braid): Generation {
		return generation(this.#counts.get(braid) ?? 0n)
	}

	/** Apply's one mutation: this braid's count advances by one. */
	advance(braid: Braid): Vector {
		const next = new Map(this.#counts)
		const current = next.get(braid) ?? 0n
		next.set(braid, current === U64_MAX ? U64_MAX : current + 1n)
		return new Vector(next)
	}
}

export type { CheckpointOrder }
export { Overflow, Vector }
