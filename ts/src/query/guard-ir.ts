import { AuthoringError } from "#errors.ts"
import { encodedEvent } from "#event-value.ts"
import type { GuardExprIr, GuardPlanIr } from "#native.ts"
import { parsePredicateIr } from "#query/predicate-ir.ts"
import { arrayValue, bytesValue, recordValue } from "#values.ts"

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
	const common = Object.hasOwn(raw, "resolve")
	const peers = Object.hasOwn(raw, "sources")
	recordValue(context, raw, [
		"kind",
		"predicate",
		"plan",
		...(transport ? ["input"] : []),
		...(common ? ["resolve"] : []),
		...(peers ? ["sources"] : [])
	])
	if (typeof raw.plan !== "object" || raw.plan === null || !("kind" in raw.plan)) return fail(context)
	const p = recordValue(context, raw.plan, [
		"kind",
		"source",
		...(raw.plan.kind === "refine" || raw.plan.kind === "boundRefine" ? ["identity"] : [])
	])
	const budget = { nodes: 4095, bytes: 16 * 1024 * 1024 }
	function variable(value: unknown): number {
		if (typeof value !== "number" || !Number.isInteger(value) || value < 0 || value > 0xffff) return fail(context)
		return value
	}
	let plan: GuardPlanIr
	if (p.kind === "boundExisting" || p.kind === "boundRefine") {
		const source = variable(p.source)
		plan =
			p.kind === "boundExisting"
				? Object.freeze({ kind: p.kind, source })
				: Object.freeze({ kind: p.kind, source, identity: variable(p.identity) })
	} else {
		if (p.kind !== "existing" && p.kind !== "refine") return fail(context)
		const source = bytesValue(context, p.source, budget.bytes)
		budget.bytes -= source.length
		encodedEvent(source)
		if (p.kind === "refine") {
			const identity = bytesValue(context, p.identity, Math.min(32, budget.bytes))
			if (identity.length !== 32) return fail(context)
			budget.bytes -= identity.length
			plan = Object.freeze({ kind: p.kind, source, identity })
		} else plan = Object.freeze({ kind: p.kind, source })
	}
	const predicate = parsePredicateIr(`${context}.predicate`, raw.predicate, 2, budget)
	if (common && (!Array.isArray(raw.resolve) || raw.resolve.length === 0 || raw.resolve.length > budget.nodes))
		return fail(`${context}: invalid common roster or shape budget`)
	const resolve = common
		? { resolve: arrayValue(`${context}.resolve`, raw.resolve, (label, p) => parsePredicateIr(label, p, 2, budget)) }
		: {}
	if (peers && (!Array.isArray(raw.sources) || raw.sources.length === 0 || raw.sources.length > budget.nodes))
		return fail(`${context}: invalid common source roster or shape budget`)
	const sources = peers ? { sources: arrayValue(`${context}.sources`, raw.sources, (_, v) => variable(v)) } : {}
	budget.nodes -= sources.sources?.length ?? 0
	if (raw.kind === "lift" || raw.kind === "descend") {
		if (typeof raw.input !== "number" || !Number.isInteger(raw.input) || raw.input < 0 || raw.input > 0xffff)
			return fail(context)
		return Object.freeze({ kind: raw.kind, predicate, plan, ...resolve, ...sources, input: raw.input })
	}
	return Object.freeze({ kind: raw.kind, predicate, plan, ...resolve, ...sources })
}
