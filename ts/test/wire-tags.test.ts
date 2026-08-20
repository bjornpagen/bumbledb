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
	AdmissionTag,
	AggOpIr,
	AtomSourceIr,
	CmpOpIr,
	ConditionTreeIr,
	ErrorFamilyKind,
	FindTermIr,
	HeadOpIr,
	HeadTermIr,
	OpenKind,
	PrepareKind,
	QueryIr,
	QueryParam,
	StatementKindTag,
	TaggedValue,
	TermIr,
	Violation,
	WriteTag
} from "#native.ts"
import type {
	CapacityBoundSpec,
	CapacityWindowSpec,
	LiteralSetSpec,
	LiteralSpec,
	StatementSpec,
	ValueSpec,
	ValueTypeSpec,
	WeightSpec
} from "#spec.ts"

/** Identity-strength type equality (the house probe). */
type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false
type Expect<T extends true> = T extends true ? true : never

const ROSTERS = {
	value: ["bool", "u64", "i64", "string", "fixedBytes", "intervalU64", "intervalI64"],
	valueType: ["bool", "u64", "i64", "string", "fixedBytes", "interval"],
	intervalElement: ["u64", "i64"],
	literal: ["handle", "value"],
	literalSet: ["one", "many"],
	capacityWindow: ["exact", "range", "floor"],
	capacityBound: ["lit", "field", "durationField"],
	weight: ["unit", "field", "durationField"],
	statement: ["fd", "containment", "capacity"],
	statementKind: ["functionality", "containment", "capacity"],
	term: ["var", "param", "paramSet", "literal", "measure"],
	aggregateOp: ["sum", "min", "max", "count", "pack"],
	headTerm: ["var", "aggregate"],
	findTerm: ["var", "count", "aggregate", "pack", "measure", "aggregateMeasure"],
	atomSource: ["edb", "interior"],
	cmpOp: ["eq", "ne", "lt", "le", "gt", "ge", "allen", "pointIn"],
	condition: ["leaf", "and", "or"],
	query: ["cq", "reach"],
	direction: ["sourceUnsatisfied", "targetRequired"],
	param: ["set"],
	errorFamily: [
		"formatMismatch",
		"schemaMismatch",
		"alreadyInitialized",
		"destinationExists",
		"publishedButUnsynced",
		"environmentLocked",
		"io",
		"lmdb",
		"readersFull",
		"schema",
		"validation",
		"factShape",
		"freshExhausted",
		"closedRelationWrite",
		"commitSync",
		"transactionPoisoned",
		"foreignPrepared",
		"foreignWitness",
		"param",
		"measureOfRay",
		"capacityRayMeasure",
		"derivedBudgetExceeded",
		"overflow",
		"resultBytesOverflow",
		"corruption"
	],
	admissionTag: ["accepted", "rejected"],
	writeTag: ["accepted", "rejected", "abandoned", "moved"],
	openKind: ["schemaError", "newtypeMismatch", "fingerprintMismatch"],
	prepareKind: ["irError"]
} as const

/** The compile pins: each roster IS its mirrored union, exactly (both directions). */
type Pins = [
	Expect<Equal<(typeof ROSTERS.value)[number], TaggedValue["kind"]>>,
	Expect<Equal<(typeof ROSTERS.valueType)[number], ValueTypeSpec["kind"]>>,
	Expect<Equal<(typeof ROSTERS.intervalElement)[number], Extract<ValueTypeSpec, { element: unknown }>["element"]>>,
	Expect<Equal<(typeof ROSTERS.literal)[number], LiteralSpec["kind"]>>,
	Expect<Equal<(typeof ROSTERS.literalSet)[number], LiteralSetSpec["kind"]>>,
	Expect<Equal<(typeof ROSTERS.capacityWindow)[number], CapacityWindowSpec["kind"]>>,
	Expect<Equal<(typeof ROSTERS.capacityBound)[number], CapacityBoundSpec["kind"]>>,
	Expect<Equal<(typeof ROSTERS.weight)[number], WeightSpec["kind"]>>,
	Expect<Equal<(typeof ROSTERS.statement)[number], StatementSpec["kind"]>>,
	Expect<Equal<(typeof ROSTERS.statementKind)[number], StatementKindTag>>,
	Expect<Equal<(typeof ROSTERS.term)[number], TermIr["kind"]>>,
	Expect<Equal<(typeof ROSTERS.aggregateOp)[number], AggOpIr["kind"]>>,
	Expect<Equal<(typeof ROSTERS.aggregateOp)[number], HeadOpIr>>,
	Expect<Equal<(typeof ROSTERS.headTerm)[number], HeadTermIr["kind"]>>,
	Expect<Equal<(typeof ROSTERS.findTerm)[number], FindTermIr["kind"]>>,
	Expect<Equal<(typeof ROSTERS.atomSource)[number], AtomSourceIr["kind"]>>,
	Expect<Equal<(typeof ROSTERS.cmpOp)[number], CmpOpIr["kind"]>>,
	Expect<Equal<(typeof ROSTERS.condition)[number], ConditionTreeIr["kind"]>>,
	Expect<Equal<(typeof ROSTERS.query)[number], QueryIr["kind"]>>,
	Expect<Equal<(typeof ROSTERS.direction)[number], Extract<Violation, { readonly kind: "containment" }>["direction"]>>,
	Expect<Equal<(typeof ROSTERS.param)[number], Exclude<QueryParam["kind"], TaggedValue["kind"]>>>,
	Expect<Equal<(typeof ROSTERS.value)[number], ValueSpec["kind"]>>,
	Expect<Equal<(typeof ROSTERS.errorFamily)[number], ErrorFamilyKind>>,
	Expect<Equal<(typeof ROSTERS.admissionTag)[number], AdmissionTag>>,
	Expect<Equal<(typeof ROSTERS.writeTag)[number], WriteTag>>,
	Expect<Equal<(typeof ROSTERS.openKind)[number], OpenKind>>,
	Expect<Equal<(typeof ROSTERS.prepareKind)[number], PrepareKind>>
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
