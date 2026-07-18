/**
 * Stratified recursion — engine recursion as values, mirroring the IR's
 * cut exactly (`ir::Program`/`PredicateDef`/`AtomSource::Idb`;
 * `docs/architecture/20-query-ir.md` § engine recursion):
 *
 *   program(S, (p) => {
 *     const reach = p.rec("reach")
 *     reach.rule((r) => r.match(Node, { id: r.var("c") })
 *       .where(r.eq(r.var("c"), r.param("root"))).select("c"))
 *     reach.rule((r) => r.match(Parent, { child: r.var("c"), parent: r.var("m") })
 *       .idb(reach, r.var("m")).select("c"))
 *     return p.output((r) => r.match(Posting, { account: r.var("a"), minor: r.var("m") })
 *       .idb(reach, r.var("a")).select(r.sum("m")))
 *   })
 *
 * `p.rec(name)` declares one recursive predicate (declaration order = its
 * dense `PredId`); `rec.rule(...)` attaches one clause — its builder's
 * `idb` accepts ONLY the rec itself (the self-recursion cut as a
 * type-level boundary: mutual recursion is unwritable) and its head
 * projects bound variable NAMES only (aggregation/measure through a cycle
 * is unrepresentable — the strata judge's roster, made unwritable);
 * `p.output(...)` seals the recs and builds the output rules, whose `idb`
 * folds any FINISHED stratum (recipe 25's form). The rec value `.rule`
 * returns carries the params its rules used — thread it into the output's
 * `idb` and the program's inferred `Params` stays exactly the params the
 * rules use. Everything deeper — strata legality, signature sealing, the
 * three oracles — is the ENGINE's judge, surfacing typed at prepare.
 */

import * as errors from "@superbuilders/errors"
import type { RecData } from "#query/atom.ts"
import type {
	AnyRuleValue,
	OutputRuleScope,
	ParamsOf,
	ProgramState,
	Query,
	RawScope,
	RecRef,
	RecRuleScope,
	RowOf,
	RuleValue
} from "#query/lower.ts"
import { makeOutputRuleScope, makeQuery, makeRawScope } from "#query/lower.ts"
import type { Flatten, ParamsRecord, ShapeOf } from "#query/scope.ts"
import { inferred } from "#query/scope.ts"
import type { Schema, SchemaRelations } from "#schema.ts"

/**
 * One recursive predicate HANDLE: `rec.rule(...)` attaches a clause and
 * returns the SAME rec under a widened params type (the runtime data is
 * shared — either handle is the self-reference; the returned one carries
 * the rules' params for the output to thread).
 */
interface Rec<Rels extends SchemaRelations, Name extends string, P extends ParamsRecord> extends RecRef<Name, P> {
	rule<RV extends AnyRuleValue>(build: (r: RecRuleScope<Rels, Name>) => RV): Rec<Rels, Name, Flatten<P & ParamsOf<RV>>>
	readonly [inferred]?: { readonly params: P }
}

/** One output-rule builder function. */
type OutputBuild<Rels extends SchemaRelations> = (r: OutputRuleScope<Rels>) => AnyRuleValue

/** A build function's rule value. */
type BuiltRule<F> = F extends (r: never) => infer RV ? RV : never

/** The union row of a tuple of output builds. */
type OutputRow<Builds extends readonly OutputBuild<SchemaRelations>[]> = RowOf<BuiltRule<Builds[number]>>

/** The intersected params record of a tuple of output builds. */
type OutputParams<Builds extends readonly OutputBuild<SchemaRelations>[]> = ShapeOf<ParamsOf<BuiltRule<Builds[number]>>>

/**
 * The program scope: declare recs, attach their rules, then declare the
 * output — which seals the recs (a later `rec`/`rule` is a construction
 * error) and returns the program as an ordinary query value.
 */
interface ProgramScope<Rels extends SchemaRelations> {
	/** Declares one recursive predicate; declaration order = its dense `PredId`. */
	rec<const Name extends string>(name: Name): Rec<Rels, Name, Record<never, never>>
	/**
	 * Declares the output predicate (one rule per build; multiple rules =
	 * set union) and seals the program. Must be what the `program()`
	 * callback returns.
	 */
	output<const Builds extends readonly OutputBuild<Rels>[]>(
		...builds: Builds
	): Query<Rels, OutputRow<Builds>, OutputParams<Builds>>
}

/** The runtime rec-handle shape beneath the typed `Rec` face. */
interface RawRec<Name extends string> {
	readonly name: Name
	readonly data: RecData
	rule(build: (r: RawScope) => RuleValue<never, never>): RawRec<Name>
}

/** Builds the runtime rec handle over shared rec data. */
function makeRawRec<Name extends string>(state: ProgramState, name: Name, data: RecData): RawRec<Name> {
	const rec: RawRec<Name> = {
		name,
		data,
		rule(build) {
			if (state.sealed) {
				throw errors.new(
					`rec ${name}: the program's output is already declared — recursive rules attach before p.output`
				)
			}
			const built = build(makeRawScope({ kind: "rec", self: data }))
			const head = data.rules[0]
			if (head !== undefined) {
				const declared = head.select.map(function columnName(column) {
					return column.name
				})
				const candidate = built.rule.select.map(function columnName(column) {
					return column.name
				})
				if (declared.join(", ") !== candidate.join(", ")) {
					throw errors.new(
						`rec ${name}: every rule derives the same head — rule 0 projects (${declared.join(", ")}), this rule projects (${candidate.join(", ")})`
					)
				}
			}
			data.rules.push(built.rule)
			return makeRawRec<Name>(state, name, data)
		}
	}
	Object.freeze(rec)
	return rec
}

/**
 * The rec handles' trusted admission seam (the `refsComplete` pattern):
 * the checkable fact — the handle owns exactly the rec data it names — is
 * verified before the raw handle is admitted at its typed face.
 */
function isRecHandle<Rels extends SchemaRelations, Name extends string, P extends ParamsRecord>(
	data: RecData,
	rec: RawRec<Name>
): rec is RawRec<Name> & Rec<Rels, Name, P> {
	return rec.data === data
}

/** Builds one typed rec handle over shared rec data. */
function makeRec<Rels extends SchemaRelations, Name extends string, P extends ParamsRecord>(
	state: ProgramState,
	name: Name,
	data: RecData
): Rec<Rels, Name, P> {
	const raw = makeRawRec<Name>(state, name, data)
	if (!isRecHandle<Rels, Name, P>(data, raw)) {
		throw errors.new(`rec ${name}: handle construction incomplete`)
	}
	return raw
}

/**
 * Builds a stratified program over a schema. The callback declares recs
 * and their rules through the scope and MUST return `p.output(...)` — the
 * sealed program is an ordinary query value: `db.prepare` lowers it to
 * the one `ProgramIr` shape the engine executes under the per-stratum
 * fixpoint driver.
 */
function program<Rels extends SchemaRelations, Q extends Query<Rels, unknown, ParamsRecord>>(
	theory: Schema<Rels>,
	build: (p: ProgramScope<Rels>) => Q
): Q {
	const state: ProgramState = { recs: [], sealed: false }
	const names = new Set<string>()
	const made: { query: unknown } = { query: undefined }
	const scope: ProgramScope<Rels> = {
		rec<const Name extends string>(name: Name): Rec<Rels, Name, Record<never, never>> {
			if (state.sealed) {
				throw errors.new(`program: the output is already declared — rec ${name} would be unreachable`)
			}
			if (names.has(name)) {
				throw errors.new(
					`program: a rec named ${name} is already declared — rec names are the self-recursion cut's identity`
				)
			}
			names.add(name)
			const data: RecData = { name, rules: [] }
			state.recs.push(data)
			return makeRec<Rels, Name, Record<never, never>>(state, name, data)
		},
		output<const Builds extends readonly OutputBuild<Rels>[]>(
			...builds: Builds
		): Query<Rels, OutputRow<Builds>, OutputParams<Builds>> {
			if (state.sealed) {
				throw errors.new("program: output is declared once — multiple rules are multiple builds of the one output")
			}
			state.sealed = true
			for (const rec of state.recs) {
				if (rec.rules.length === 0) {
					throw errors.new(
						`program: rec ${rec.name} has no rules — a predicate with no defining clause seals no signature`
					)
				}
				Object.freeze(rec.rules)
			}
			if (builds.length === 0) {
				throw errors.new("program: the output needs at least one rule")
			}
			const rules = builds.map(function buildRule(buildOne) {
				return buildOne(makeOutputRuleScope<Rels>(state)).rule
			})
			const q = makeQuery<Rels, OutputRow<Builds>, OutputParams<Builds>>(theory, state.recs, rules)
			made.query = q
			return q
		}
	}
	Object.freeze(scope)
	const result = build(scope)
	if (made.query !== result) {
		throw errors.new("program: the build callback must return p.output(...) — the sealed program IS the query value")
	}
	return result
}

export type { OutputBuild, ProgramScope, Rec }
export { program }
