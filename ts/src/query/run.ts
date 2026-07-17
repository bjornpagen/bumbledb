/**
 * Prepared-query marshaling seams (PRD-08), the two rides every execution
 * takes — the typed params object down to the bridge's positional
 * `QueryParam[]` (declaration order = dense `ParamId`s, values tagged by
 * each param's declaring field), and answer rows (positional, head order)
 * back up to plain objects with branded values (the store is the proof
 * carrier: the engine computed the answer, so readback asserts the brand,
 * the same trust direction as fact readback in `#marshal.ts`). Answers are
 * SETS — no order or limit exists anywhere; hosts sort. The `Prepared`
 * VALUE itself (no lifecycle, GC-reclaimed plan) lives in `#db.ts`.
 */

import * as errors from "@superbuilders/errors"
import type { FactValue, QueryParam, TaggedValue } from "#native.ts"
import type { SelectColumn } from "#query/lower.ts"
import { taggedLiteral } from "#query/lower.ts"
import type { ParamEntry } from "#query/scope.ts"

/** The 13-bit Allen mask ceiling (`bumbledb/crates/bumbledb/src/allen.rs`: bits above the low 13 are unrepresentable). */
const ALLEN_ALL_BITS = (1 << 13) - 1

/** Tags one supplied mask-param value. */
function wireMask(name: string, value: unknown): TaggedValue {
	if (typeof value !== "number" || !Number.isInteger(value) || value < 0 || value > ALLEN_ALL_BITS) {
		throw errors.new(`param ${name}: an Allen-mask param binds a 13-bit mask number built from the ALLEN constants`)
	}
	return { kind: "allenMask", mask: value }
}

/**
 * Marshals the typed params object to the bridge's positional arguments,
 * in param declaration order (= the lowering's dense `ParamId`s). A
 * missing entry is a typed error naming the param; values tag by the
 * declaring field's structural type; a set param takes a readonly array
 * (the empty set is legal and matches nothing — the engine's rule).
 */
function wireParams(entries: readonly ParamEntry[], supplied: Readonly<Record<string, unknown>>): QueryParam[] {
	return entries.map(function wireOne(entry): QueryParam {
		const value = supplied[entry.name]
		if (value === undefined) {
			throw errors.new(`execute params object is missing param ${entry.name}`)
		}
		if (entry.shape === "mask") {
			return wireMask(entry.name, value)
		}
		if (entry.data === undefined) {
			throw errors.new(`param ${entry.name}: registry entry carries no declaring field`)
		}
		if (entry.shape === "set") {
			if (!Array.isArray(value)) {
				throw errors.new(`param ${entry.name}: a set param binds a readonly array of values`)
			}
			const data = entry.data
			return {
				kind: "set",
				values: value.map(function wireElement(element, index) {
					return taggedLiteral(`param ${entry.name}[${index}]`, data, element)
				})
			}
		}
		return taggedLiteral(`param ${entry.name}`, entry.data, value)
	})
}

/**
 * The read-side trusted seam of answers: a decoded row carrying every
 * select column IS a `Row` — the engine computed it under the prepared
 * head, so the brands are asserted, not re-derived (the `#marshal.ts`
 * trust direction).
 */
function isAnswerRow<Row>(
	select: readonly SelectColumn[],
	decoded: Readonly<Record<string, FactValue>>
): decoded is Readonly<Record<string, FactValue>> & Row {
	return select.every(function present(column) {
		return decoded[column.name] !== undefined
	})
}

/**
 * Decodes positional answer rows (column order = the program's head order
 * = the select record's written order) to named, branded, frozen row
 * objects.
 */
function decodeAnswers<Row>(select: readonly SelectColumn[], rows: FactValue[][]): Row[] {
	return rows.map(function decodeRow(row) {
		if (row.length !== select.length) {
			throw errors.new(`query answer arity ${row.length} does not match the ${select.length} select columns`)
		}
		const decoded: Record<string, FactValue> = {}
		select.forEach(function decodeCell(column, ordinal) {
			const cell = row[ordinal]
			if (cell === undefined) {
				throw errors.new(`query answer cell ${ordinal} (${column.name}) is absent`)
			}
			decoded[column.name] = cell
		})
		Object.freeze(decoded)
		if (!isAnswerRow<Row>(select, decoded)) {
			throw errors.new("query answer row is not a complete select record")
		}
		return decoded
	})
}

export { decodeAnswers, wireParams }
