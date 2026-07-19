/**
 * Prepared-query marshaling seams, the two rides every execution takes —
 * the typed params object down to the bridge's positional `QueryParam[]`
 * (registry order = dense `ParamId`s, values tagged by each param's
 * ANCHORING use: the field position or comparison sibling that typed it,
 * op-aware exactly as comparison literals tag), and answer rows
 * (positional, head order) back up to plain objects of BARE structural
 * values — the marshal boundary is pure both ways: the engine computed
 * the answer under the prepared head, so a decoded row that carries every
 * select column IS a row (the trusted read seam), and nothing is asserted
 * on any value. Answers are SETS — no order or limit exists anywhere;
 * hosts sort. The `Prepared` VALUE itself (no lifecycle, GC-reclaimed
 * plan) lives in `#db.ts`.
 */

import * as errors from "@superbuilders/errors"
import type { FactValue, QueryParam, TaggedValue } from "#native.ts"
import type { SelectColumn } from "#query/atom.ts"
import { taggedCmpLiteral } from "#query/lower.ts"
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

/** Tags one supplied value-param cell by its anchoring use. */
function wireValue(entry: ParamEntry, context: string, value: unknown): TaggedValue {
	if (entry.anchor === undefined) {
		throw errors.new(
			`param ${entry.name} has no field-anchored use — bind it in an atom or compare it against a bound variable`
		)
	}
	return taggedCmpLiteral(context, entry.anchor, value, entry.op)
}

/**
 * Marshals the typed params object to the bridge's positional arguments,
 * in registry order (= the lowering's dense `ParamId`s). A missing entry
 * is a typed error naming the param; values tag by the anchoring use's
 * structural type; a set param takes a readonly array (the empty set is
 * legal and matches nothing — the engine's rule). A MEMBERSHIP-ARRAY
 * entry (`members` present — a literal set folded into the program) is
 * supplied by the SDK itself: each handle name rides the one
 * roster-verification point (`taggedHandleId`, through `wireValue`) and
 * crosses as the same `{ kind: "set", values }` a bound `r.inSet` param
 * crosses as — the host's params object is never consulted for it.
 */
function wireParams(entries: readonly ParamEntry[], supplied: Readonly<Record<string, unknown>>): QueryParam[] {
	return entries.map(function wireOne(entry): QueryParam {
		if (entry.members !== undefined) {
			return {
				kind: "set",
				values: entry.members.map(function wireMember(member, index) {
					return wireValue(entry, `membership array ${entry.name}[${index}]`, member)
				})
			}
		}
		const value = supplied[entry.name]
		if (value === undefined) {
			throw errors.new(`execute params object is missing param ${entry.name}`)
		}
		if (entry.shape === "mask") {
			return wireMask(entry.name, value)
		}
		if (entry.shape === "set") {
			if (!Array.isArray(value)) {
				throw errors.new(`param ${entry.name}: a set param binds a readonly array of values`)
			}
			return {
				kind: "set",
				values: value.map(function wireElement(element, index) {
					return wireValue(entry, `param ${entry.name}[${index}]`, element)
				})
			}
		}
		return wireValue(entry, `param ${entry.name}`, value)
	})
}

/**
 * The read-side trusted seam of answers: a decoded row carrying every
 * select column IS a `Row` — the engine computed it under the prepared
 * head, and the values are BARE structural values, so nothing is asserted
 * beyond presence (the store is the proof carrier; no brand exists to
 * re-derive).
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
 * = the select's written order) to named, frozen row objects of bare
 * structural values.
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
