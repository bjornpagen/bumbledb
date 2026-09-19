/** A bounded, shape-only parser for portable numerical query descriptions. */
import { AuthoringError } from "#errors.ts"
import { encodedRational } from "#exact-value.ts"
import type { NumberExprIr } from "#native.ts"
import { encodedNumber } from "#number-value.ts"
import { encodedParameter } from "#parameter-value.ts"
import { bytesValue, recordValue } from "#values.ts"

function fail(context: string): never {
	throw new AuthoringError({ message: `${context}: invalid numerical expression` })
}
export function parseNumberIr(
	context: string,
	input: unknown,
	depth = 1,
	budget = { nodes: 4096, bytes: 16 * 1024 * 1024 }
): NumberExprIr {
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
		const bytes = bytesValue(context, raw[key], budget.bytes)
		budget.bytes -= bytes.length
		return bytes
	}
	const child = (key: string) => parseNumberIr(`${context}.${key}`, raw[key], depth + 1, budget)
	switch (raw.kind) {
		case "var":
		case "integer":
			fields(["var"])
			return Object.freeze({ kind: raw.kind, var: ordinal(raw.var) })
		case "component": {
			fields(["observation", "component"])
			if (raw.component !== "value" && raw.component !== "numerator" && raw.component !== "evidenceMass")
				return fail(context)
			return Object.freeze({ kind: raw.kind, observation: ordinal(raw.observation), component: raw.component })
		}
		case "literal":
		case "imported": {
			fields(["bytes"])
			const value = bytes("bytes")
			if (raw.kind === "literal") encodedRational(value)
			else encodedNumber(value)
			return Object.freeze({ kind: raw.kind, bytes: value })
		}
		case "add":
		case "subtract":
		case "multiply":
		case "divide":
		case "min":
		case "max":
			fields(["left", "right"])
			return Object.freeze({ kind: raw.kind, left: child("left"), right: child("right") })
		case "negate":
		case "abs":
			fields(["value"])
			return Object.freeze({ kind: raw.kind, value: child("value") })
		case "pow":
			fields(["value", "exponent"])
			return Object.freeze({ kind: raw.kind, value: child("value"), exponent: ordinal(raw.exponent, 0xffffffff) })
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
