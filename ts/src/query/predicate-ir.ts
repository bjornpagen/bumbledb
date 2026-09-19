/** Bounded combined predicate/numerical description parsing. */
import { AuthoringError } from "#errors.ts"
import type { PredicateExprIr } from "#native.ts"
import { encodedParameter } from "#parameter-value.ts"
import { encodedPredicate } from "#predicate-value.ts"
import { parseNumberIr } from "#query/number-ir.ts"
import { bytesValue, recordValue } from "#values.ts"

function fail(context: string): never {
	throw new AuthoringError({ message: `${context}: invalid predicate expression` })
}
export function parsePredicateIr(
	context: string,
	input: unknown,
	depth = 1,
	budget = { nodes: 4096, bytes: 16 * 1024 * 1024 }
): PredicateExprIr {
	if (depth > 128 || budget.nodes === 0) return fail(`${context}: shape budget exceeded`)
	budget.nodes--
	if (typeof input !== "object" || input === null) return fail(context)
	const raw = recordValue(context, input, Object.keys(input))
	function fields(names: readonly string[]): void {
		recordValue(context, raw, ["kind", ...names])
	}
	function ordinal(value: unknown, max = 0xffff): number {
		if (typeof value !== "number" || !Number.isInteger(value) || value < 0 || value > max) return fail(context)
		return value
	}
	function bytes(key: string): Uint8Array {
		const b = bytesValue(context, raw[key], budget.bytes)
		budget.bytes -= b.length
		return b
	}
	const child = (key: string) => parsePredicateIr(`${context}.${key}`, raw[key], depth + 1, budget)
	switch (raw.kind) {
		case "var":
			fields(["var"])
			return Object.freeze({ kind: raw.kind, var: ordinal(raw.var) })
		case "sign":
			fields(["number", "signs"])
			return Object.freeze({
				kind: raw.kind,
				number: parseNumberIr(`${context}.number`, raw.number, depth + 1, budget),
				signs: ordinal(raw.signs, 7)
			})
		case "imported": {
			fields(["bytes"])
			const value = bytes("bytes")
			encodedPredicate(value)
			return Object.freeze({ kind: raw.kind, bytes: value })
		}
		case "negate":
			fields(["value"])
			return Object.freeze({ kind: raw.kind, value: child("value") })
		case "apply":
			fields(["op", "left", "right"])
			return Object.freeze({ kind: raw.kind, op: ordinal(raw.op, 15), left: child("left"), right: child("right") })
		case "onDomain": {
			fields(["value", "domain"])
			const domain = bytes("domain")
			encodedParameter("domain", domain)
			return Object.freeze({ kind: raw.kind, value: child("value"), domain })
		}
		default:
			return fail(context)
	}
}
