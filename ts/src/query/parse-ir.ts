import { AuthoringError } from "#errors.ts"
import type {
	AtomIr,
	CmpOpIr,
	ConditionTreeIr,
	EventExprIr,
	EventTestIr,
	FindTermIr,
	HeadTermIr,
	ParsedQuery,
	QueryIr,
	RelationExprIr,
	RuleIr,
	ScalarExprIr,
	TermIr
} from "#native.ts"
import { parseNumberIr } from "#query/number-ir.ts"
import { payoffFromBytes } from "#query/payoff.ts"
import { roundingMode } from "#scalar.ts"
import { arrayValue as array, bytesValue, recordValue, valueDescriptor } from "#values.ts"

function fail(context: string, expected: string): never {
	throw new AuthoringError({
		message: `${context}: ${expected}`,
		diagnostic: { code: "InvalidQuery", context, expected }
	})
}

function tagged(context: string, input: unknown): Readonly<Record<string, unknown>> {
	if (typeof input !== "object" || input === null) return fail(context, "expected a descriptor record")
	return recordValue(context, input, Object.keys(input))
}

function ordinal(context: string, input: unknown, maximum = 0xffff): number {
	if (typeof input !== "number" || !Number.isInteger(input) || input < 0 || input > maximum)
		return fail(context, `expected an ordinal from 0 through ${maximum}`)
	return input
}

function term(context: string, input: unknown): TermIr {
	const raw = tagged(context, input)
	switch (raw.kind) {
		case "var":
			recordValue(context, raw, ["kind", "var"])
			return Object.freeze({ kind: raw.kind, var: ordinal(context, raw.var) })
		case "param":
		case "paramSet":
			recordValue(context, raw, ["kind", "param"])
			return Object.freeze({ kind: raw.kind, param: ordinal(context, raw.param) })
		case "literal":
			recordValue(context, raw, ["kind", "value"])
			return Object.freeze({ kind: raw.kind, value: valueDescriptor(`${context}.value`, raw.value) })
		default:
			return fail(context, "unknown term kind")
	}
}

function scalar(context: string, input: unknown, depth = 1): ScalarExprIr {
	if (depth > 128) return fail(context, "scalar expression deeper than 128")
	const raw = tagged(context, input)
	switch (raw.kind) {
		case "var":
			recordValue(context, raw, ["kind", "var"])
			return Object.freeze({ kind: raw.kind, var: ordinal(context, raw.var) })
		case "literal":
			recordValue(context, raw, ["kind", "value"])
			return Object.freeze({ kind: raw.kind, value: valueDescriptor(`${context}.value`, raw.value) })
		case "measure":
		case "negate":
		case "isNaN":
		case "isFinite":
			recordValue(context, raw, ["kind", "expr"])
			return Object.freeze({ kind: raw.kind, expr: scalar(`${context}.expr`, raw.expr, depth + 1) })
		case "add":
		case "subtract":
		case "multiply":
		case "divide":
			recordValue(context, raw, ["kind", "left", "right"])
			return Object.freeze({
				kind: raw.kind,
				left: scalar(`${context}.left`, raw.left, depth + 1),
				right: scalar(`${context}.right`, raw.right, depth + 1)
			})
		case "mulDiv":
			recordValue(context, raw, ["kind", "a", "b", "divisor", "rounding"])
			return Object.freeze({
				kind: raw.kind,
				a: scalar(`${context}.a`, raw.a, depth + 1),
				b: scalar(`${context}.b`, raw.b, depth + 1),
				divisor: scalar(`${context}.divisor`, raw.divisor, depth + 1),
				rounding: roundingMode(raw.rounding)
			})
		case "cast": {
			recordValue(context, raw, ["kind", "cast", "expr"])
			const cast = raw.cast
			if (cast !== "toF64" && cast !== "toF64Exact" && cast !== "toI64Exact" && cast !== "toU64Exact")
				return fail(context, "unknown numeric cast")
			return Object.freeze({ kind: raw.kind, cast, expr: scalar(`${context}.expr`, raw.expr, depth + 1) })
		}
		default:
			return fail(context, "unknown scalar expression kind")
	}
}

function headTerm(context: string, input: unknown): HeadTermIr {
	const raw = tagged(context, input)
	if (raw.kind === "var" || raw.kind === "compute") {
		recordValue(context, raw, ["kind"])
		return Object.freeze({ kind: raw.kind })
	}
	recordValue(context, raw, ["kind", "op"])
	if (raw.kind !== "aggregate") return fail(context, "unknown head kind")
	const op = raw.op
	if (
		op !== "sum" &&
		op !== "mean" &&
		op !== "min" &&
		op !== "max" &&
		op !== "count" &&
		op !== "pack" &&
		op !== "expectation"
	)
		return fail(context, "unknown head aggregate")
	return Object.freeze({ kind: raw.kind, op })
}

function eventCount(context: string, input: unknown): bigint {
	if (typeof input !== "bigint" || input < 0n || input > 0xffffffffffffffffn)
		return fail(context, "expected a nonnegative u64 bigint")
	return input
}

type EventBudget = { remaining: number; bytes: number }

function eventImport(context: string, input: unknown, budget: EventBudget): Uint8Array {
	const bytes = bytesValue(context, input, budget.bytes)
	budget.bytes -= bytes.byteLength
	return bytes
}

function relationExpr(context: string, input: unknown, depth: number, budget: EventBudget): RelationExprIr {
	if (depth > 128 || budget.remaining-- <= 0) return fail(context, "Event expression exceeds shape budget")
	const raw = tagged(context, input)
	const child = (key: string) => relationExpr(`${context}.${key}`, raw[key], depth + 1, budget)
	const descriptor = () => eventImport(`${context}.descriptor`, raw.descriptor, budget)
	switch (raw.kind) {
		case "bind":
		case "test":
			recordValue(context, raw, ["kind", "descriptor", "expr"])
			return Object.freeze({
				kind: raw.kind,
				descriptor: descriptor(),
				expr: eventExpr(`${context}.expr`, raw.expr, depth + 1, budget)
			})
		case "identity":
			recordValue(context, raw, ["kind", "descriptor"])
			return Object.freeze({ kind: raw.kind, descriptor: descriptor() })
		case "not":
		case "converse":
			recordValue(context, raw, ["kind", "relation"])
			return Object.freeze({ kind: raw.kind, relation: child("relation") })
		case "apply":
			recordValue(context, raw, ["kind", "bits", "left", "right"])
			return Object.freeze({
				kind: raw.kind,
				bits: ordinal(context, raw.bits, 15),
				left: child("left"),
				right: child("right")
			})
		case "product": {
			recordValue(context, raw, ["kind", "op", "descriptor", "left", "right"])
			const op = raw.op
			if (op !== "compose" && op !== "leftResidual" && op !== "rightResidual")
				return fail(context, "unknown relation product operation")
			return Object.freeze({ kind: raw.kind, op, descriptor: descriptor(), left: child("left"), right: child("right") })
		}
		case "star":
			recordValue(context, raw, ["kind", "descriptor", "relation"])
			return Object.freeze({ kind: raw.kind, descriptor: descriptor(), relation: child("relation") })
		default:
			return fail(context, "unknown relation expression kind")
	}
}

function eventExpr(
	context: string,
	input: unknown,
	depth = 1,
	budget = { remaining: 4096, bytes: 16 * 1024 * 1024 }
): EventExprIr {
	if (depth > 128 || budget.remaining-- <= 0) return fail(context, "Event expression exceeds shape budget")
	const raw = tagged(context, input)
	const child = (key: string) => eventExpr(`${context}.${key}`, raw[key], depth + 1, budget)
	switch (raw.kind) {
		case "bound":
			recordValue(context, raw, ["kind", "depth"])
			return Object.freeze({ kind: raw.kind, depth: ordinal(context, raw.depth, 65535) })
		case "fixed": {
			recordValue(context, raw, ["kind", "op", "scope", "expr"])
			const op = raw.op
			if (op !== "least" && op !== "greatest") return fail(context, "unknown fixed-point operation")
			const scope = eventImport(`${context}.scope`, raw.scope, budget)
			return Object.freeze({ kind: raw.kind, op, scope, expr: child("expr") })
		}
		case "relation": {
			recordValue(context, raw, ["kind", "op", "relation"])
			const op = raw.op
			if (op !== "region" && op !== "domain" && op !== "range") return fail(context, "unknown relation view operation")
			return Object.freeze({
				kind: raw.kind,
				op,
				relation: relationExpr(`${context}.relation`, raw.relation, depth + 1, budget)
			})
		}
		case "modal": {
			recordValue(context, raw, ["kind", "op", "relation", "expr"])
			const op = raw.op
			if (op !== "may" && op !== "all" && op !== "must" && op !== "post")
				return fail(context, "unknown modal operation")
			return Object.freeze({
				kind: raw.kind,
				op,
				relation: relationExpr(`${context}.relation`, raw.relation, depth + 1, budget),
				expr: child("expr")
			})
		}
		case "map": {
			recordValue(context, raw, ["kind", "op", "descriptor", "expr"])
			const op = raw.op
			if (
				op !== "pullback" &&
				op !== "image" &&
				op !== "universalImage" &&
				op !== "nonvacuousImage" &&
				op !== "possible" &&
				op !== "guaranteed"
			)
				return fail(context, "unknown Event readout operation")
			const descriptor = eventImport(`${context}.descriptor`, raw.descriptor, budget)
			return Object.freeze({ kind: raw.kind, op, descriptor, expr: child("expr") })
		}
		case "var":
		case "empty":
		case "full":
			recordValue(context, raw, ["kind", "var"])
			return Object.freeze({ kind: raw.kind, var: ordinal(context, raw.var) })
		case "not":
			recordValue(context, raw, ["kind", "expr"])
			return Object.freeze({ kind: raw.kind, expr: child("expr") })
		case "apply":
			recordValue(context, raw, ["kind", "bits", "left", "right"])
			return Object.freeze({
				kind: raw.kind,
				bits: ordinal(context, raw.bits, 15),
				left: child("left"),
				right: child("right")
			})
		case "ite":
			recordValue(context, raw, ["kind", "condition", "high", "low"])
			return Object.freeze({ kind: raw.kind, condition: child("condition"), high: child("high"), low: child("low") })
		case "cardinality": {
			recordValue(context, raw, ["kind", "minimum", "maximum", "events"])
			if (!Array.isArray(raw.events) || raw.events.length === 0 || raw.events.length > budget.remaining)
				return fail(context, "Event roster needs 1..4096 scope-bearing positions")
			return Object.freeze({
				kind: raw.kind,
				minimum: eventCount(context, raw.minimum),
				maximum: eventCount(context, raw.maximum),
				events: array(`${context}.events`, raw.events, (path, value) => eventExpr(path, value, depth + 1, budget))
			})
		}
		default:
			return fail(context, "unknown Event expression kind")
	}
}

function eventTest(context: string, input: unknown): EventTestIr {
	const raw = tagged(context, input)
	const budget = { remaining: 4096, bytes: 16 * 1024 * 1024 }
	const child = (key: string) => eventExpr(`${context}.${key}`, raw[key], 1, budget)
	switch (raw.kind) {
		case "isEmpty":
		case "isFull":
			recordValue(context, raw, ["kind", "expr"])
			return Object.freeze({ kind: raw.kind, expr: child("expr") })
		case "subset":
		case "equal":
		case "disjoint":
		case "covers":
			recordValue(context, raw, ["kind", "left", "right"])
			return Object.freeze({ kind: raw.kind, left: child("left"), right: child("right") })
		default:
			return fail(context, "unknown Event test kind")
	}
}

function payoffRatioIr(context: string, input: unknown) {
	const tag = tagged(context, input)
	if (tag.kind === "imported") {
		recordValue(context, tag, ["kind", "bytes"])
		const bytes = bytesValue(context, tag.bytes, 16 * 1024 * 1024)
		payoffFromBytes(bytes)
		return Object.freeze({ kind: "imported" as const, bytes })
	}
	const value = recordValue(context, input, ["kind", "numerator", "denominator"])
	if (value.kind !== "ratio") return fail(context, "unknown exact payoff expression")
	return Object.freeze({
		kind: "ratio" as const,
		numerator: ordinal(context, value.numerator),
		denominator: ordinal(context, value.denominator)
	})
}

function find(context: string, input: unknown): FindTermIr {
	const raw = tagged(context, input)
	if (raw.kind === "number") {
		recordValue(context, raw, ["kind", "expr"])
		return Object.freeze({ kind: "number", expr: parseNumberIr(`${context}.expr`, raw.expr) })
	}
	if (raw.kind === "segments") {
		recordValue(context, raw, ["kind", "op", "left", "right"])
		if (raw.op !== "intersection" && raw.op !== "difference") return fail(context, "unknown segment operator")
		return Object.freeze({
			kind: raw.kind,
			op: raw.op,
			left: ordinal(context, raw.left),
			right: ordinal(context, raw.right)
		})
	}
	switch (raw.kind) {
		case "var":
			recordValue(context, raw, ["kind", "var"])
			return Object.freeze({ kind: raw.kind, var: ordinal(context, raw.var) })
		case "count":
			if (Object.hasOwn(raw, "over")) return fail(context, "Count carries no over")
			recordValue(context, raw, ["kind"])
			return Object.freeze({ kind: raw.kind })
		case "event":
			recordValue(context, raw, ["kind", "expr"])
			return Object.freeze({ kind: raw.kind, expr: eventExpr(`${context}.expr`, raw.expr) })
		case "expectation":
			recordValue(context, raw, ["kind", "value", "when", "given"])
			return Object.freeze({
				kind: raw.kind,
				value: typeof raw.value === "number" ? ordinal(context, raw.value) : payoffRatioIr(context, raw.value),
				when: ordinal(context, raw.when),
				given: ordinal(context, raw.given)
			})
		case "probability": {
			recordValue(context, raw, ["kind", "event", "given"])
			const budget = { remaining: 4096, bytes: 16 * 1024 * 1024 }
			return Object.freeze({
				kind: raw.kind,
				event: eventExpr(`${context}.event`, raw.event, 1, budget),
				given: eventExpr(`${context}.given`, raw.given, 1, budget)
			})
		}
		case "test":
			recordValue(context, raw, ["kind", "expr"])
			return Object.freeze({ kind: raw.kind, expr: eventTest(`${context}.expr`, raw.expr) })
		case "compute":
			if (!Object.hasOwn(raw, "expr")) return fail(context, "compute requires expr")
			recordValue(context, raw, ["kind", "expr"])
			return Object.freeze({ kind: raw.kind, expr: scalar(`${context}.expr`, raw.expr) })
		case "pack":
			recordValue(context, raw, ["kind", "over"])
			return Object.freeze({ kind: raw.kind, over: ordinal(context, raw.over) })
		case "aggregate": {
			recordValue(context, raw, ["kind", "op", "over"])
			const op = recordValue(`${context}.op`, raw.op, ["kind"]).kind
			if (op !== "sum" && op !== "mean" && op !== "min" && op !== "max") return fail(context, "unknown fold operator")
			return Object.freeze({ kind: raw.kind, op: Object.freeze({ kind: op }), over: ordinal(context, raw.over) })
		}
		default:
			return fail(context, "unknown find kind")
	}
}

function atom(context: string, input: unknown): AtomIr {
	const raw = recordValue(context, input, ["source", "bindings"])
	const source = tagged(`${context}.source`, raw.source)
	let owned: AtomIr["source"]
	if (source.kind === "edb") {
		recordValue(context, source, ["kind", "relation"])
		owned = { kind: source.kind, relation: ordinal(`${context}.source.relation`, source.relation, 0xffffffff) }
	} else if (source.kind === "interior") {
		recordValue(context, source, ["kind", "interior"])
		owned = { kind: source.kind, interior: ordinal(`${context}.source.interior`, source.interior, 0xffffffff) }
	} else return fail(context, "unknown atom source")
	const seen = new Set<number>()
	const bindings = array(`${context}.bindings`, raw.bindings, (path, pair): readonly [number, TermIr] => {
		const cells = array(path, pair, (_, value) => value)
		if (cells.length !== 2) return fail(path, "a binding has exactly two elements")
		const field = ordinal(`${path}[0]`, cells[0])
		if (seen.has(field)) return fail(path, "duplicate field binding")
		seen.add(field)
		return Object.freeze([field, term(`${path}[1]`, cells[1])])
	})
	return Object.freeze({ source: Object.freeze(owned), bindings })
}

function comparisonOp(context: string, input: unknown): CmpOpIr {
	const raw = tagged(context, input)
	if (raw.kind === "allen") {
		recordValue(context, raw, ["kind", "mask"])
		return Object.freeze({ kind: raw.kind, mask: ordinal(`${context}.mask`, raw.mask, 0x1fff) })
	}
	recordValue(context, raw, ["kind"])
	const kind = raw.kind
	if (
		kind !== "eq" &&
		kind !== "ne" &&
		kind !== "lt" &&
		kind !== "le" &&
		kind !== "gt" &&
		kind !== "ge" &&
		kind !== "pointIn"
	)
		return fail(context, "unknown comparison operator")
	return Object.freeze({ kind })
}

function condition(context: string, input: unknown, depth = 1): ConditionTreeIr {
	if (depth > 64) return fail(context, "condition tree deeper than 64")
	const raw = tagged(context, input)
	if (raw.kind === "leaf") {
		recordValue(context, raw, ["kind", "cmp"])
		const cmp = recordValue(`${context}.cmp`, raw.cmp, ["op", "lhs", "rhs"])
		return Object.freeze({
			kind: raw.kind,
			cmp: Object.freeze({
				op: comparisonOp(`${context}.cmp.op`, cmp.op),
				lhs: term(`${context}.cmp.lhs`, cmp.lhs),
				rhs: term(`${context}.cmp.rhs`, cmp.rhs)
			})
		})
	}
	if (raw.kind !== "and" && raw.kind !== "or") return fail(context, "unknown condition kind")
	recordValue(context, raw, ["kind", "children"])
	return Object.freeze({
		kind: raw.kind,
		children: array(`${context}.children`, raw.children, (path, value) => condition(path, value, depth + 1))
	})
}

function rule(context: string, input: unknown): RuleIr {
	const raw = recordValue(context, input, ["finds", "atoms", "negated", "conditions"])
	return Object.freeze({
		finds: array(`${context}.finds`, raw.finds, find),
		atoms: array(`${context}.atoms`, raw.atoms, atom),
		negated: array(`${context}.negated`, raw.negated, atom),
		conditions: array(`${context}.conditions`, raw.conditions, condition)
	})
}

function align(context: string, head: readonly HeadTermIr[], rules: readonly RuleIr[]): void {
	for (const [index, rule] of rules.entries()) {
		if (rule.finds.length !== head.length) fail(`${context}.rules[${index}]`, "finds width does not match head width")
		for (const [position, find] of rule.finds.entries()) {
			const term = head[position]
			const projects =
				find.kind === "var" ||
				find.kind === "compute" ||
				find.kind === "segments" ||
				find.kind === "event" ||
				find.kind === "test" ||
				find.kind === "probability" ||
				find.kind === "number"
			if (term === undefined || (term.kind === "aggregate") === projects)
				fail(`${context}.rules[${index}].finds[${position}]`, "find family does not match head")
			if (term.kind === "aggregate") {
				const op = find.kind === "aggregate" ? find.op.kind : find.kind
				if (op !== term.op)
					fail(`${context}.rules[${index}].finds[${position}]`, "aggregate operator does not match head")
			}
		}
	}
}

/**
 * Own and validate the complete wire grammar. The native compiler then
 * checks schema membership, binding types, safety and recursion semantics;
 * parsing alone does not assert that a query is admitted by a schema.
 */
function parseQueryIr(input: unknown): ParsedQuery {
	const raw = tagged("query", input)
	if (raw.kind !== "cq" && raw.kind !== "reach") return fail("query", "unknown query kind")
	recordValue(
		"query",
		raw,
		raw.kind === "cq" ? ["kind", "interiors", "head", "rules"] : ["kind", "interiors", "rec", "head", "rules"]
	)
	const rules = array("query.rules", raw.rules, rule)
	if (rules.length === 0) return fail("parseQueryIr", "main rules are empty")
	const head = array("query.head", raw.head, headTerm)
	align("query", head, rules)
	const interiors = array("query.interiors", raw.interiors, (path, input) => {
		const raw = recordValue(path, input, ["head", "rules"])
		const head = array(`${path}.head`, raw.head, headTerm)
		const rules = array(`${path}.rules`, raw.rules, rule)
		if (rules.length === 0) return fail(path, "interior rules are empty")
		align(path, head, rules)
		return Object.freeze({ head, rules })
	})
	let owned: QueryIr
	if (raw.kind === "cq") owned = { kind: raw.kind, interiors, head, rules }
	else {
		const rawRec = recordValue("query.rec", raw.rec, ["head", "base", "rec"])
		const headRec = array("query.rec.head", rawRec.head, headTerm)
		const base = array("query.rec.base", rawRec.base, rule)
		if (base.length === 0) return fail("parseQueryIr", "rec base is empty")
		const rec = array("query.rec.rec", rawRec.rec, rule)
		if (rec.length === 0) return fail("parseQueryIr", "rec step is empty")
		align("rec base", headRec, base)
		align("rec step", headRec, rec)
		if (
			headRec.some((term) => term.kind !== "var") ||
			[...base, ...rec].some((rule) => rule.negated.length !== 0 || rule.finds.some((find) => find.kind !== "var"))
		)
			return fail("query.rec", "recursion requires projection-only heads and no negation")
		owned = { kind: raw.kind, interiors, head, rules, rec: Object.freeze({ head: headRec, base, rec }) }
	}
	return Object.freeze(owned) as ParsedQuery
}

export type { ParsedQuery }
export { parseQueryIr }
