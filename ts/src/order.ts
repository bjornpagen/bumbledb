/**
 * Host-side answer ordering — the census-fired convenience
 * (`docs/architecture/70-api.md` § the freeze ledger). Answers are SETS and
 * the ENGINE NEVER ORDERS; the language owns the sort (`rows.sort(...)`)
 * and the limit (`.slice(0, n)`) — the drizzle law. What JS lacks is a
 * number-returning comparator over the SDK's bigint-bearing cells, so the
 * SDK ships exactly that: sort keys are DATA — a bare column name is
 * ascending (the punning spelling; no `asc` wrapper exists — one spelling
 * per meaning) and `desc(name)` is the one descending spelling — folded by
 * `by(...)` into one row-typed comparator. Cross-type cells cannot arise
 * within one column (one column, one domain), and the cell order is TOTAL
 * anyway (the type-rank wall), so the comparator never throws.
 */

import type { FactValue } from "#native.ts"

/** One DESCENDING sort key, plain data — built by {@link desc}. */
interface Desc<K extends string> {
	readonly key: K
	readonly desc: true
}

/** One sort key: a bare column name (ascending — the punning spelling) or `desc(name)`. */
type SortKey<K extends string> = K | Desc<K>

/**
 * The type-rank wall: boolean 0, bigint 1, string 2, bytes 3, interval 4.
 * One column carries one domain, so a mixed pair never arises from decoded
 * answer rows — the wall exists to keep the cell order TOTAL (never a
 * throw), not to be reached.
 */
function cellRank(value: FactValue): number {
	if (typeof value === "boolean") {
		return 0
	}
	if (typeof value === "bigint") {
		return 1
	}
	if (typeof value === "string") {
		return 2
	}
	if (value instanceof Uint8Array) {
		return 3
	}
	return 4
}

/**
 * One cell against one cell. Same-type arms: boolean orders false < true;
 * bigint by `<`/`>`; string by the host language's own `<`/`>` (flavor,
 * recorded); bytes bytewise over the shared prefix, then by length;
 * intervals by start, then end. A mixed pair falls through to the
 * type-rank wall.
 */
function cellCmp(left: FactValue, right: FactValue): number {
	if (typeof left === "boolean" && typeof right === "boolean") {
		if (left === right) {
			return 0
		}
		if (left) {
			return 1
		}
		return -1
	}
	if (typeof left === "bigint" && typeof right === "bigint") {
		if (left < right) {
			return -1
		}
		if (left > right) {
			return 1
		}
		return 0
	}
	if (typeof left === "string" && typeof right === "string") {
		if (left < right) {
			return -1
		}
		if (left > right) {
			return 1
		}
		return 0
	}
	if (left instanceof Uint8Array && right instanceof Uint8Array) {
		const shared = Math.min(left.length, right.length)
		for (let index = 0; index < shared; index += 1) {
			const leftByte = left[index]
			const rightByte = right[index]
			// `index < shared` keeps both reads in bounds; the `undefined`
			// arms are the checker's indexed-access tax, never taken.
			if (leftByte !== undefined && rightByte !== undefined && leftByte !== rightByte) {
				return leftByte - rightByte
			}
		}
		return left.length - right.length
	}
	if (
		typeof left === "object" &&
		!(left instanceof Uint8Array) &&
		typeof right === "object" &&
		!(right instanceof Uint8Array)
	) {
		if (left.start < right.start) {
			return -1
		}
		if (left.start > right.start) {
			return 1
		}
		if (left.end < right.end) {
			return -1
		}
		if (left.end > right.end) {
			return 1
		}
		return 0
	}
	return cellRank(left) - cellRank(right)
}

/** Marks one sort key DESCENDING — the single descending spelling (a bare name is already ascending). */
function desc<const K extends string>(key: K): Desc<K> {
	const marker: Desc<K> = { key, desc: true }
	return Object.freeze(marker)
}

/**
 * Folds sort keys into ONE comparator typed against the row —
 * `Row extends Readonly<Record<K, FactValue>>` — so a key the row lacks, or
 * a column typed `number` (outside the cell domain), is a COMPILE error at
 * the `.sort` call site: the laws typed the columns, and the row type
 * carries that proof here (parse-don't-validate). The generic RETURN is the
 * load-bearing trick: `rows.sort(by("rank"))` instantiates `Row` from the
 * array's own element type and checks the key set right there.
 */
function by<const K extends string>(
	first: SortKey<K>,
	...rest: ReadonlyArray<SortKey<K>>
): <Row extends Readonly<Record<K, FactValue>>>(left: Row, right: Row) => number {
	const entries = [first, ...rest].map(function normalizeKey(sortKey): { readonly key: K; readonly factor: 1 | -1 } {
		if (typeof sortKey === "string") {
			return { key: sortKey, factor: 1 }
		}
		return { key: sortKey.key, factor: -1 }
	})
	return function compare<Row extends Readonly<Record<K, FactValue>>>(left: Row, right: Row): number {
		for (const entry of entries) {
			const order = cellCmp(left[entry.key], right[entry.key]) * entry.factor
			if (order !== 0) {
				return order
			}
		}
		return 0
	}
}

export type { Desc, SortKey }
export { by, desc }
