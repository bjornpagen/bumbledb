import { AuthoringError, SdkInvariantError } from "#errors.ts"
import type { QueryParam, TaggedValue } from "#native.ts"
import type { FindColumn } from "#query/atom.ts"
import { taggedCmpLiteral } from "#query/lower.ts"
import type { ParamEntry } from "#query/scope.ts"
import type { CellValue } from "#rows.ts"
import { decodeCell, handleOf, setOwnField } from "#rows.ts"
import { arrayValue, recordValue } from "#values.ts"

function wireValue(entry: ParamEntry, context: string, value: unknown): TaggedValue {
	if (entry.anchor === undefined) {
		throw new AuthoringError({
			message: `param ${entry.name} has no field-anchored use — bind it in an atom or compare it against a bound variable`
		})
	}
	return taggedCmpLiteral(context, entry.anchor, value, entry.op)
}

function wireParams(entries: readonly ParamEntry[], supplied: Readonly<Record<string, unknown>>): QueryParam[] {
	const input = recordValue(
		"query parameters",
		supplied,
		entries.filter((entry) => entry.membership === undefined).map((entry) => entry.name)
	)
	return entries.map(function wireOne(entry): QueryParam {
		if (entry.membership !== undefined) {
			return entry.membership
		}
		const value = input[entry.name]
		if (value === undefined) {
			throw new AuthoringError({ message: `execute params object is missing param ${entry.name}` })
		}
		if (entry.shape === "set") {
			return {
				kind: "set",
				values: arrayValue(`param ${entry.name}`, value, function wireElement(context, element) {
					return wireValue(entry, context, element)
				})
			}
		}
		return wireValue(entry, `param ${entry.name}`, value)
	})
}

function isAnswerRow<Row>(
	finds: readonly FindColumn[],
	decoded: Readonly<Record<string, unknown>>
): decoded is Readonly<Record<string, unknown>> & Row {
	return finds.every(function present(column) {
		return decoded[column.name] !== undefined
	})
}

/**
 * The one answer-row decoder: owned positional cells into a plain frozen
 * record keyed by the find columns — the SAME fields and shapes across
 * every row and page (stable row shape). A column carrying its mint slot
 * decodes through the full value roster (uuid bytes lift to canonical
 * hex, closed ids lift to handle names, float intervals stay owned plain
 * objects); an aggregate column without a slot passes the engine's owned
 * scalar through, with the closed lift when the column is closed-typed.
 */
function decodeAnswers<Row>(finds: readonly FindColumn[], rows: readonly (readonly CellValue[])[]): Row[] {
	return rows.map(function decodeRow(row) {
		if (row.length !== finds.length) {
			throw new SdkInvariantError({
				message: `query answer arity ${row.length} does not match the ${finds.length} find columns`
			})
		}
		const decoded: Record<string, unknown> = {}
		finds.forEach(function decodeColumn(column, ordinal) {
			const cell = row[ordinal]
			if (cell === undefined) {
				throw new SdkInvariantError({ message: `query answer cell ${ordinal} (${column.name}) is absent` })
			}
			if (column.slot !== undefined) {
				setOwnField(decoded, column.name, decodeCell(`query answer column ${column.name}`, column.slot.field, cell))
				return
			}
			setOwnField(
				decoded,
				column.name,
				column.closed === undefined ? cell : handleOf(`query answer column ${column.name}`, column.closed, cell)
			)
		})
		Object.freeze(decoded)
		if (!isAnswerRow<Row>(finds, decoded)) {
			throw new SdkInvariantError({ message: "query answer row is not a complete find record" })
		}
		return decoded
	})
}

export { decodeAnswers, wireParams }
