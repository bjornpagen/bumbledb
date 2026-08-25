/**
 * The per-braid generation map. Sum, domination, checkpoint order,
 * apply's increment, and the coordinate's one binary encoding live
 * here. Overflow is a refusal of `sum` and of nowhere else.
 */

import { ByteReader, ByteWriter, U64_MAX } from "#bytes.ts"
import type { Braid } from "#descriptor.ts"
import { braidHex } from "#descriptor.ts"
import type { Generation } from "#keys.ts"
import { generation } from "#keys.ts"

/** One braid id and its applied count on the wire: u32le + u64le. */
const PAIR_BYTES = 12n

const Overflow = { tag: "overflow" } as const
type Overflow = typeof Overflow

type CheckpointOrder = "before" | "equal" | "after"

type VectorParse =
	| { readonly tag: "ok"; readonly vector: Vector }
	| { readonly tag: "truncated" }
	| { readonly tag: "trailing"; readonly bytes: number }
	| { readonly tag: "malformed" }
	| { readonly tag: "overflow" }

function braidIdOf(id: Braid): number {
	return Number.parseInt(id.slice(1), 16)
}

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

	/** `u32le` count, then `(u32le braid, u64le g)` pairs in braid order. */
	encode(): Uint8Array {
		const braids = [...this.#counts.keys()].sort()
		const out = new ByteWriter(4 + braids.length * 12)
		out.u32le(braids.length)
		for (const id of braids) {
			out.u32le(braidIdOf(id))
			out.u64le(this.#counts.get(id) ?? 0n)
		}
		return out.finish()
	}

	/**
	 * Inverse of `encode`. A count the remaining bytes cannot open is
	 * truncated; entries must ascend; overflow is `sum`.
	 */
	static parse(bytes: Uint8Array): VectorParse {
		const short = { tag: "short" } as const
		const reader = new ByteReader(bytes, {
			fail(): never {
				throw short
			}
		})
		try {
			const count = BigInt(reader.u32le("count"))
			if (count > 0n && BigInt(reader.remaining()) / PAIR_BYTES < count) {
				return { tag: "truncated" }
			}
			const entries: Array<readonly [Braid, bigint]> = []
			let last: Braid | undefined
			for (let i = 0n; i < count; i++) {
				const name = braidHex(reader.u32le("braid"))
				const g = reader.u64le("g")
				if (last !== undefined && last >= name) {
					return { tag: "malformed" }
				}
				entries.push([name, g])
				last = name
			}
			if (reader.remaining() !== 0) {
				return { tag: "trailing", bytes: reader.remaining() }
			}
			const vector = new Vector(entries)
			if (typeof vector.sum() !== "bigint") {
				return { tag: "overflow" }
			}
			return { tag: "ok", vector }
		} catch (error) {
			if (error === short) {
				return { tag: "truncated" }
			}
			throw error
		}
	}
}

export type { CheckpointOrder, VectorParse }
export { Overflow, Vector }
