import { AuthoringError } from "#errors.ts"
import { encodedEvent } from "#event-value.ts"
import type { GuardExprIr, GuardPlanIr } from "#native.ts"
import { parsePredicateIr } from "#query/predicate-ir.ts"
import { bytesValue, recordValue } from "#values.ts"

function fail(context: string): never {
	throw new AuthoringError({ message: `${context}: invalid guard expression` })
}
export function parseGuardIr(context: string, input: unknown): GuardExprIr {
	if (typeof input !== "object" || input === null) return fail(context)
	const raw = recordValue(context, input, Object.keys(input))
	if (
		raw.kind !== "holds" &&
		raw.kind !== "fails" &&
		raw.kind !== "undefined" &&
		raw.kind !== "lift" &&
		raw.kind !== "descend"
	)
		return fail(context)
	const transport = raw.kind === "lift" || raw.kind === "descend"
	recordValue(context, raw, transport ? ["kind", "predicate", "plan", "input"] : ["kind", "predicate", "plan"])
	const p = recordValue(context, raw.plan, [
		"kind",
		"source",
		...(typeof raw.plan === "object" && raw.plan !== null && "kind" in raw.plan && raw.plan.kind === "refine"
			? ["identity"]
			: [])
	])
	if (p.kind !== "existing" && p.kind !== "refine") return fail(context)
	const budget = { nodes: 4095, bytes: 16 * 1024 * 1024 }
	const source = bytesValue(context, p.source, budget.bytes)
	budget.bytes -= source.length
	encodedEvent(source)
	let plan: GuardPlanIr
	if (p.kind === "refine") {
		const identity = bytesValue(context, p.identity, Math.min(32, budget.bytes))
		if (identity.length !== 32) return fail(context)
		budget.bytes -= identity.length
		plan = Object.freeze({ kind: p.kind, source, identity })
	} else plan = Object.freeze({ kind: p.kind, source })
	const predicate = parsePredicateIr(`${context}.predicate`, raw.predicate, 2, budget)
	if (raw.kind === "lift" || raw.kind === "descend") {
		if (typeof raw.input !== "number" || !Number.isInteger(raw.input) || raw.input < 0 || raw.input > 0xffff)
			return fail(context)
		return Object.freeze({ kind: raw.kind, predicate, plan, input: raw.input })
	}
	return Object.freeze({ kind: raw.kind, predicate, plan })
}
