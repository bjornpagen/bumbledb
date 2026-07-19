/**
 * The wire-tag golden — the TS half of the `wire_tags!` tripwire
 * (cleanup-0.5.0 U3 kill 10). The bridge's tag tables
 * (`ts/crate/src/tags.rs`) render `test/fixtures/tags.json` and a cargo
 * test verifies the file against them; THIS test closes the TS direction:
 * each const roster below is (a) compile-pinned EXACTLY equal to the
 * mirrored union in `native.ts`/`spec.ts` (identity-strength `Equal`
 * probes, both directions) and (b) runtime-asserted equal to the golden's
 * entry. A core-enum change therefore breaks the bridge compile
 * (exhaustive `tag()`), then the golden (cargo test), then this suite —
 * the three-place mirror can no longer drift silently in any direction.
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import { test } from "node:test"
import type {
	AggOpIr,
	AtomSourceIr,
	CmpOpIr,
	ConditionTreeIr,
	FindTermIr,
	HeadOpIr,
	HeadTermIr,
	MaskTermIr,
	QueryParam,
	StatementKindTag,
	TaggedValue,
	TermIr,
	Violation
} from "#native.ts"
import type { LiteralSetSpec, LiteralSpec, StatementSpec, ValueSpec, ValueTypeSpec, WindowSpec } from "#spec.ts"

/** Identity-strength type equality (the house probe). */
type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false
type Expect<T extends true> = T extends true ? true : never

const ROSTERS = {
	value: ["bool", "u64", "i64", "string", "fixedBytes", "intervalU64", "intervalI64", "allenMask"],
	valueType: ["bool", "u64", "i64", "string", "fixedBytes", "interval"],
	intervalElement: ["u64", "i64"],
	literal: ["handle", "value"],
	literalSet: ["one", "many"],
	window: ["exact", "range", "floor"],
	statement: ["fd", "containment", "cardinality"],
	statementKind: ["functionality", "containment", "cardinality"],
	term: ["var", "param", "paramSet", "literal", "measure"],
	aggregateOp: ["sum", "min", "max", "count", "countDistinct", "argMax", "argMin", "pack"],
	headTerm: ["var", "aggregate"],
	findTerm: ["var", "aggregate", "measure", "aggregateMeasure"],
	atomSource: ["edb", "idb"],
	cmpOp: ["eq", "ne", "lt", "le", "gt", "ge", "allen", "pointIn"],
	maskTerm: ["literal", "param"],
	condition: ["leaf", "and", "or"],
	direction: ["sourceUnsatisfied", "targetRequired"],
	param: ["set"]
} as const

/** The compile pins: each roster IS its mirrored union, exactly (both directions). */
type Pins = [
	Expect<Equal<(typeof ROSTERS.value)[number], TaggedValue["kind"]>>,
	Expect<Equal<(typeof ROSTERS.valueType)[number], ValueTypeSpec["kind"]>>,
	Expect<Equal<(typeof ROSTERS.intervalElement)[number], Extract<ValueTypeSpec, { element: unknown }>["element"]>>,
	Expect<Equal<(typeof ROSTERS.literal)[number], LiteralSpec["kind"]>>,
	Expect<Equal<(typeof ROSTERS.literalSet)[number], LiteralSetSpec["kind"]>>,
	Expect<Equal<(typeof ROSTERS.window)[number], WindowSpec["kind"]>>,
	Expect<Equal<(typeof ROSTERS.statement)[number], StatementSpec["kind"]>>,
	Expect<Equal<(typeof ROSTERS.statementKind)[number], StatementKindTag>>,
	Expect<Equal<(typeof ROSTERS.term)[number], TermIr["kind"]>>,
	Expect<Equal<(typeof ROSTERS.aggregateOp)[number], AggOpIr["kind"]>>,
	Expect<Equal<(typeof ROSTERS.aggregateOp)[number], HeadOpIr>>,
	Expect<Equal<(typeof ROSTERS.headTerm)[number], HeadTermIr["kind"]>>,
	Expect<Equal<(typeof ROSTERS.findTerm)[number], FindTermIr["kind"]>>,
	Expect<Equal<(typeof ROSTERS.atomSource)[number], AtomSourceIr["kind"]>>,
	Expect<Equal<(typeof ROSTERS.cmpOp)[number], CmpOpIr["kind"]>>,
	Expect<Equal<(typeof ROSTERS.maskTerm)[number], MaskTermIr["kind"]>>,
	Expect<Equal<(typeof ROSTERS.condition)[number], ConditionTreeIr["kind"]>>,
	Expect<Equal<(typeof ROSTERS.direction)[number], Exclude<Violation["direction"], undefined>>>,
	Expect<Equal<(typeof ROSTERS.param)[number], Exclude<QueryParam["kind"], TaggedValue["kind"]>>>,
	// The spec's ValueSpec is the tagged-value vocabulary minus the
	// bind-time-only Allen mask — pinned so the two lanes stay one family.
	Expect<Equal<Exclude<(typeof ROSTERS.value)[number], "allenMask">, ValueSpec["kind"]>>
]

test("the wire-tag rosters equal the tags.json golden, key for key", function goldenAgreement() {
	const pinned: Pins extends readonly true[] ? true : never = true
	assert.ok(pinned, "the compile pins hold (vacuous at runtime; the probes are the claim)")
	const golden: Record<string, readonly string[]> = JSON.parse(
		fs.readFileSync(new URL("./fixtures/tags.json", import.meta.url), "utf8")
	)
	assert.deepEqual(
		Object.keys(golden).toSorted(),
		Object.keys(ROSTERS).toSorted(),
		"the golden and the TS rosters cover the same tables"
	)
	for (const [key, roster] of Object.entries(ROSTERS)) {
		assert.deepEqual([...roster], golden[key], `table ${key} must equal the bridge's wire_tags! roster`)
	}
})
