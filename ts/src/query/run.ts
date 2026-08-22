import * as errors from "@superbuilders/errors"
import { handleOf } from "#marshal.ts"
import type { FactValue, QueryParam, TaggedValue } from "#native.ts"
import type { FindColumn } from "#query/atom.ts"
import { taggedCmpLiteral } from "#query/lower.ts"
import type { ParamEntry } from "#query/scope.ts"

function wireValue(entry: ParamEntry, context: string, value: unknown): TaggedValue {
	if (entry.anchor === undefined) {
		throw errors.new(
			`param ${entry.name} has no field-anchored use — bind it in an atom or compare it against a bound variable`
		)
	}
	return taggedCmpLiteral(context, entry.anchor, value, entry.op)
}

function wireParams(entries: readonly ParamEntry[], supplied: Readonly<Record<string, unknown>>): QueryParam[] {
	return entries.map(function wireOne(entry): QueryParam {
		if (entry.membership !== undefined) {
			return entry.membership
		}
		const value = supplied[entry.name]
		if (value === undefined) {
			throw errors.new(`execute params object is missing param ${entry.name}`)
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

function isAnswerRow<Row>(
	finds: readonly FindColumn[],
	decoded: Readonly<Record<string, FactValue>>
): decoded is Readonly<Record<string, FactValue>> & Row {
	return finds.every(function present(column) {
		return decoded[column.name] !== undefined
	})
}

function decodeAnswers<Row>(finds: readonly FindColumn[], rows: FactValue[][]): Row[] {
	return rows.map(function decodeRow(row) {
		if (row.length !== finds.length) {
			throw errors.new(`query answer arity ${row.length} does not match the ${finds.length} find columns`)
		}
		const decoded: Record<string, FactValue> = {}
		finds.forEach(function decodeCell(column, ordinal) {
			const cell = row[ordinal]
			if (cell === undefined) {
				throw errors.new(`query answer cell ${ordinal} (${column.name}) is absent`)
			}
			decoded[column.name] =
				column.closed === undefined ? cell : handleOf(`query answer column ${column.name}`, column.closed, cell)
		})
		Object.freeze(decoded)
		if (!isAnswerRow<Row>(finds, decoded)) {
			throw errors.new("query answer row is not a complete find record")
		}
		return decoded
	})
}

export { decodeAnswers, wireParams }
