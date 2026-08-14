/**
 * Host shape parse for the wire `QueryIr`: rec/main nonempty, aggregate
 * finds split (Count has no `over`; folds require it), head/find
 * alignment. The engine validator remains the one roster authority —
 * this parse refuses only shape the host type can see.
 */

import * as errors from "@superbuilders/errors"
import type { FindTermIr, HeadTermIr, ParsedQuery, QueryIr, RuleIr } from "#native.ts"

/** Brands a shape-checked wire query so {@link Native.dbPrepare} will accept it. */
function parseQueryIr(ir: QueryIr): ParsedQuery {
	if (ir.rules.length === 0) {
		throw errors.new("parseQueryIr: main rules are empty")
	}
	align("query", ir.head, ir.rules)
	ir.interiors.forEach(function checkInterior(interior, index) {
		align(`interior ${index}`, interior.head, interior.rules)
	})
	if (ir.rec !== null) {
		if (ir.rec.base.length === 0) {
			throw errors.new("parseQueryIr: rec base is empty")
		}
		if (ir.rec.rec.length === 0) {
			throw errors.new("parseQueryIr: rec step is empty")
		}
		align("rec base", ir.rec.head, ir.rec.base)
		align("rec step", ir.rec.head, ir.rec.rec)
	}
	return ir as ParsedQuery
}

/** One rule list must share the head's width and var/aggregate family. */
function align(context: string, head: readonly HeadTermIr[], rules: readonly RuleIr[]): void {
	for (const [ruleIndex, rule] of rules.entries()) {
		if (rule.finds.length !== head.length) {
			throw errors.new(
				`${context}: rule ${ruleIndex} finds width ${rule.finds.length} does not match head width ${head.length}`
			)
		}
		rule.finds.forEach(function checkFind(find, position) {
			parseFind(`${context} rule ${ruleIndex} find ${position}`, find)
			const family = findFamily(find)
			if (family !== head[position]?.kind) {
				throw errors.new(
					`${context}: rule ${ruleIndex} find ${position} is ${family}, not head ${head[position]?.kind}`
				)
			}
		})
	}
}

/** Count forbids `over`; every other aggregate requires it. */
function parseFind(context: string, find: FindTermIr): void {
	if (find.kind !== "aggregate") {
		return
	}
	if (find.op.kind === "count") {
		if ("over" in find) {
			throw errors.new(`${context}: Count carries no over`)
		}
		return
	}
	if (!("over" in find)) {
		throw errors.new(`${context}: fold aggregate requires over`)
	}
}

/** Head family of one find term: measure is a var slot; measure-folds are aggregates. */
function findFamily(find: FindTermIr): "var" | "aggregate" {
	switch (find.kind) {
		case "var":
		case "measure":
			return "var"
		case "aggregate":
		case "aggregateMeasure":
			return "aggregate"
	}
}

export { parseQueryIr }
export type { ParsedQuery }
