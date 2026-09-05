import { AuthoringError } from "#errors.ts"
import type { FindTermIr, HeadTermIr, ParsedQuery, QueryIr, RuleIr } from "#native.ts"

function parseQueryIr(ir: QueryIr): ParsedQuery {
	if (ir.rules.length === 0) {
		throw new AuthoringError({ message: "parseQueryIr: main rules are empty" })
	}
	align("query", ir.head, ir.rules)
	ir.interiors.forEach(function checkInterior(interior, index) {
		align(`interior ${index}`, interior.head, interior.rules)
	})
	if (ir.kind === "reach") {
		if (ir.rec.base.length === 0) {
			throw new AuthoringError({ message: "parseQueryIr: rec base is empty" })
		}
		if (ir.rec.rec.length === 0) {
			throw new AuthoringError({ message: "parseQueryIr: rec step is empty" })
		}
		align("rec base", ir.rec.head, ir.rec.base)
		align("rec step", ir.rec.head, ir.rec.rec)
	}
	return ir as ParsedQuery
}

function align(context: string, head: readonly HeadTermIr[], rules: readonly RuleIr[]): void {
	for (const [ruleIndex, rule] of rules.entries()) {
		if (rule.finds.length !== head.length) {
			throw new AuthoringError({
				message: `${context}: rule ${ruleIndex} finds width ${rule.finds.length} does not match head width ${head.length}`
			})
		}
		rule.finds.forEach(function checkFind(find, position) {
			parseFind(`${context} rule ${ruleIndex} find ${position}`, find)
			const family = findFamily(find)
			if (family !== head[position]?.kind) {
				throw new AuthoringError({
					message: `${context}: rule ${ruleIndex} find ${position} is ${family}, not head ${head[position]?.kind}`
				})
			}
		})
	}
}

function parseFind(context: string, find: FindTermIr): void {
	const raw = find as Record<string, unknown>
	if (find.kind === "count") {
		if ("over" in raw) {
			throw new AuthoringError({ message: `${context}: Count carries no over` })
		}
		return
	}
	if (find.kind === "pack" || find.kind === "aggregate") {
		if (!("over" in raw)) {
			throw new AuthoringError({ message: `${context}: ${find.kind} requires over` })
		}
	}
	if (find.kind === "compute" && !("expr" in raw)) {
		throw new AuthoringError({ message: `${context}: compute requires expr` })
	}
}

function findFamily(find: FindTermIr): "var" | "aggregate" | "compute" {
	switch (find.kind) {
		case "var":
			return "var"
		case "compute":
			return "compute"
		case "count":
		case "pack":
		case "aggregate":
			return "aggregate"
	}
}

export type { ParsedQuery }
export { parseQueryIr }
