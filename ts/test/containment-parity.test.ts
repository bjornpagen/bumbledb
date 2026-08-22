import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, before, describe, test } from "node:test"

import { within } from "#capacity.ts"
import { closed } from "#closed.ts"
import { on } from "#face.ts"
import { i64, interval, str, u64 } from "#fields.ts"
import type { LawfulStatements, TargetKeyWall } from "#law.ts"
import { lower } from "#lower.ts"
import { type AnyRelation, relation } from "#relation.ts"
import { schema } from "#schema.ts"
import type { FieldSpec, SchemaSpec, SideSpec, StatementSpec, ValueTypeSpec } from "#spec.ts"
import { capacity, contained, key, mirrors, type Statement } from "#statements.ts"

type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-parity-"))

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const ReportSource = relation("Source", { value: str })
const ReportTarget = relation("Target", { target: u64.fresh, scope: u64, value: str })

const Coverage = relation("Coverage", { sourceAddress: str })
const AnalysisTargetEntry = relation("AnalysisTargetEntry", {
	entry: u64.fresh,
	policyPackage: u64,
	sourceAddress: str
})

describe("the target-key wall at schema() — the value tier, no native needed", function schemaHalf() {
	test("row 1: the report's minimal shape refuses, names throughout", function reportShape() {
		assert.throws(function minimal() {
			// @ts-expect-error — the TargetKeyWall verdict: Target(value) resolves no key of Target
			schema("Report", { Source: ReportSource, Target: ReportTarget }, [
				key(ReportTarget, ["scope", "value"]),
				contained(on(ReportSource, "value"), on(ReportTarget, "value"))
			])
		}, /^schema Report: Source\(value\) <= Target\(value\): target projection \(value\) matches no declared key of Target — available keys: \(target\); \(scope, value\)$/)
	})

	test("row 2: Primer's shape refuses with the spec's pinned diagnostic", function primerShape() {
		assert.throws(function primer() {
			// @ts-expect-error — the TargetKeyWall verdict: AnalysisTargetEntry(sourceAddress) resolves no key
			schema("Analysis", { Coverage, AnalysisTargetEntry }, [
				key(AnalysisTargetEntry, ["policyPackage", "sourceAddress"]),
				contained(on(Coverage, "sourceAddress"), on(AnalysisTargetEntry, "sourceAddress"))
			])
		}, /^schema Analysis: Coverage\(sourceAddress\) <= AnalysisTargetEntry\(sourceAddress\): target projection \(sourceAddress\) matches no declared key of AnalysisTargetEntry — available keys: \(entry\); \(policyPackage, sourceAddress\)$/)
	})

	test("row 3: projection = a declared key, same order — admitted", function sameOrder() {
		const Source = relation("Source", { scope: u64, value: str })
		const Target = relation("Target", { scope: u64, value: str })
		const admitted = schema("SameOrder", { Source, Target }, [
			key(Target, ["scope", "value"]),
			contained(on(Source, ["scope", "value"]), on(Target, ["scope", "value"]))
		])
		assert.equal(lower(admitted).statements.length, 2)
	})

	test("row 4: projection = a declared key, permuted order — admitted (set equality)", function permuted() {
		const Source = relation("Source", { scope: u64, value: str })
		const Target = relation("Target", { scope: u64, value: str })
		const admitted = schema("Permuted", { Source, Target }, [
			key(Target, ["scope", "value"]),
			contained(on(Source, ["value", "scope"]), on(Target, ["value", "scope"]))
		])
		assert.equal(lower(admitted).statements.length, 2)
	})

	test("row 5: strict subset and strict superset of a key both refuse", function subsetSuperset() {
		const Source = relation("Source", { scope: u64, value: str, extra: u64 })
		const Target = relation("Target", { scope: u64, value: str, extra: u64 })
		assert.throws(function subset() {
			// @ts-expect-error — the TargetKeyWall verdict: (scope) is a strict subset of (scope, value)
			schema("Subset", { Source, Target }, [
				key(Target, ["scope", "value"]),
				contained(on(Source, "scope"), on(Target, "scope"))
			])
		}, /target projection \(scope\) matches no declared key of Target — available keys: \(scope, value\)/)
		assert.throws(function superset() {
			// @ts-expect-error — the TargetKeyWall verdict: (scope, value, extra) is a strict superset of (scope, value)
			schema("Superset", { Source, Target }, [
				key(Target, ["scope", "value"]),
				contained(on(Source, ["scope", "value", "extra"]), on(Target, ["scope", "value", "extra"]))
			])
		}, /target projection \(scope, value, extra\) matches no declared key of Target — available keys: \(scope, value\)/)
	})

	test("row 6: a fresh-implied key target — admitted", function freshImplied() {
		const Owner = relation("Owner", { id: u64.fresh })
		const Ref = relation("Ref", { owner: u64 })
		const admitted = schema("Fresh", { Owner, Ref }, [contained(on(Ref, "owner"), on(Owner, "id"))])
		assert.equal(lower(admitted).statements.length, 1)
	})

	test("row 7: closed target, projection [id] — admitted", function closedHandle() {
		const Kind = closed("Kind", ["Checking", "Savings"])
		const Account = relation("Account", { kind: Kind.id })
		const admitted = schema("Closed", { Kind, Account }, [contained(on(Account, "kind"), on(Kind, "id"))])
		assert.equal(lower(admitted).statements.length, 1)
	})

	test("row 8: closed target, a payload projection — refused by CLOSEDNESS, its own message", function closedPayload() {
		const Sev = closed("Sev", { level: u64 }, { Info: { level: 1n }, Critical: { level: 5n } })
		const Task = relation("Task", { level: u64 })
		assert.throws(function payloadTarget() {
			// @ts-expect-error — the TargetKeyWall verdict: a closed target is addressed by its synthetic id only
			schema("Rubric", { Sev, Task }, [contained(on(Task, "level"), on(Sev, "level"))])
		}, /^schema Rubric: Task\(level\) <= Sev\(level\): closed target Sev is addressed by its synthetic id only — projection \(level\) must be exactly \(id\) \(rewrite the target side as on\(Sev, "id"\)\)$/)
	})

	test("row 8 sub-case: a declared payload key equal to the projection changes nothing — closedness judges first", function closedPayloadKeyed() {
		const Sev = closed("Sev", { level: u64 }, { Info: { level: 1n }, Critical: { level: 5n } })
		const Task = relation("Task", { level: u64 })

		// arm refuses BEFORE the key search, so a declared payload key
		// carrying exactly the refused field set changes nothing —

		assert.throws(function mintClosedKey() {
			// @ts-expect-error — key() takes an ordinary relation; a closed value lacks the relation shape
			key(Sev, ["level"])
		}, /closedness already materializes Sev\(id\) -> Sev/)
		assert.throws(function payloadTarget() {
			// @ts-expect-error — the TargetKeyWall verdict: a closed target is addressed by its synthetic id only
			schema("RubricKeyed", { Sev, Task }, [contained(on(Task, "level"), on(Sev, "level"))])
		}, /closed target Sev is addressed by its synthetic id only/)
	})

	test("row 9: mirrors with both faces keyed — admitted", function mirrorsBothKeyed() {
		const A = relation("A", { id: u64.fresh })
		const B = relation("B", { ref: u64 })
		const admitted = schema("Mirrored", { A, B }, [key(B, ["ref"]), mirrors(on(A, "id"), on(B, "ref"))])
		assert.equal(lower(admitted).statements.length, 2)
	})

	test("row 10: mirrors with exactly one face keyed — refused, naming the unkeyed orientation", function mirrorsOneKeyed() {
		const A = relation("A", { id: u64.fresh, peer: u64 })
		const B = relation("B", { ref: u64 })

		assert.throws(function reverseUnkeyed() {
			schema("Half", { A, B }, [key(B, ["ref"]), mirrors(on(A, "peer"), on(B, "ref"))])
		}, /^schema Half: A\(peer\) == B\(ref\): target projection \(peer\) matches no declared key of A — available keys: \(id\)$/)
	})

	test("row 11: capacity with a non-key target — refused, the same rule", function capacityNonKey() {
		const Parent = relation("Parent", { id: u64.fresh, group: u64 })
		const Child = relation("Child", { parent: u64 })
		assert.throws(function nonKeyTarget() {
			// @ts-expect-error — the TargetKeyWall verdict through the capacity target face
			schema("Grouped", { Parent, Child }, [capacity(on(Parent, "group"), within(0n, 3n), on(Child, "parent"))])
		}, /^schema Grouped: Parent\(group\) <=\{0\.\.3\} Child\(parent\): target projection \(group\) matches no declared key of Parent — available keys: \(id\)$/)
	})

	test("row 12: an interval-bearing projection with no pointwise key — refused with the pointwise hint", function pointwiseHint() {
		const Claim = relation("Claim", { who: u64, span: interval(i64) })
		const Roster = relation("Roster", { who: u64, span: interval(i64) })
		assert.throws(function noPointwiseKey() {
			// @ts-expect-error — the TargetKeyWall verdict: (who, span) set-matches no key of Roster
			schema("Shifts", { Claim, Roster }, [
				key(Roster, ["who"]),
				contained(on(Claim, ["who", "span"]), on(Roster, ["who", "span"]))
			])
		}, /target projection \(who, span\) matches no declared key of Roster — available keys: \(who\); hint: declare the exact pointwise key `R\(prefix…, interval\) -> R`$/)
	})

	test("the type tier's verdict is named and self-locating; a widened list degrades to the value tier", function typeTier() {
		const refused = [
			key(ReportTarget, ["scope", "value"]),
			contained(on(ReportSource, "value"), on(ReportTarget, "value"))
		] as const
		type Verdict = LawfulStatements<{ Source: typeof ReportSource; Target: typeof ReportTarget }, typeof refused>
		type Located = Verdict extends TargetKeyWall<infer T, infer P> ? readonly [T, P] : never
		const probeTarget: Equal<Located[0], "Target"> = true
		const probeProjection: Equal<Located[1], "value"> = true
		assert.ok(probeTarget && probeProjection)

		const widened: Statement[] = [
			key(ReportTarget, ["scope", "value"]),
			contained(on(ReportSource, "value"), on(ReportTarget, "value"))
		]
		assert.throws(function valueTierAuthoritative() {
			schema("Report", { Source: ReportSource, Target: ReportTarget }, widened)
		}, /target projection \(value\) matches no declared key of Target/)
	})

	test("a widened key OWNER degrades the whole type-tier wall — never a false wall on literal faces", function widenedKeyOwner() {
		const Source = relation("Source", { scope: u64, value: str })
		const Target = relation("Target", { scope: u64, value: str })

		const widenedOwner: AnyRelation = Target
		const admitted = schema("WidenedKeyOwner", { Source, Target }, [
			key(widenedOwner, ["scope", "value"]),
			contained(on(Source, ["scope", "value"]), on(Target, ["scope", "value"]))
		])
		assert.equal(lower(admitted).statements.length, 2)
	})

	test("a key widened WHOLE to bare Statement degrades the whole type-tier wall — never a false wall", function bareStatementKey() {
		const Source = relation("Source", { scope: u64, value: str })
		const Target = relation("Target", { scope: u64, value: str })

		const widenedStmt: Statement = key(Target, ["scope", "value"])
		const admitted = schema("BareStatementKey", { Source, Target }, [
			widenedStmt,
			contained(on(Source, ["scope", "value"]), on(Target, ["scope", "value"]))
		])
		assert.equal(lower(admitted).statements.length, 2)
	})

	test("a KeyStatement UNION degrades the whole type-tier wall — the roster cannot state which key the value carries", function keyStatementUnion() {
		const Source = relation("Source", { scope: u64, value: str })
		const Target = relation("Target", { scope: u64, value: str })
		const Other = relation("Other", { tag: u64 })

		function pickKey(flag: boolean) {
			return flag ? key(Target, ["scope", "value"]) : key(Other, ["tag"])
		}
		const unionKey = pickKey(true)
		const admitted = schema("KeyStatementUnion", { Source, Target, Other }, [
			unionKey,
			contained(on(Source, ["scope", "value"]), on(Target, ["scope", "value"]))
		])
		assert.equal(lower(admitted).statements.length, 2)
	})

	test("a statement-tuple UNION degrades the whole type-tier wall — the tier judges one singular tuple only", function statementTupleUnion() {
		const Source = relation("Source", { scope: u64, value: str })
		const Target = relation("Target", { scope: u64, value: str })

		const tupleA = [
			key(Target, ["scope", "value"]),
			contained(on(Source, ["scope", "value"]), on(Target, ["scope", "value"]))
		] as const
		const tupleB = [key(Target, ["value"]), contained(on(Source, "value"), on(Target, "value"))] as const
		function pick(flag: boolean) {
			return flag ? tupleA : tupleB
		}
		const admitted = schema("UnionTuple", { Source, Target }, pick(true))
		assert.equal(lower(admitted).statements.length, 2)
	})

	test("a projection UNION inside one key element degrades the whole type-tier wall — the projection is judged whole, never per-arm", function projectionUnion() {
		const Source = relation("Source", { scope: u64, value: str })
		const Target = relation("Target", { scope: u64, value: str })

		function pickProjection(flag: boolean) {
			return flag ? (["scope", "value"] as const) : (["value"] as const)
		}
		const unionProjectionKey = key(Target, pickProjection(false))
		const admitted = schema("ProjectionUnion", { Source, Target }, [
			unionProjectionKey,
			contained(on(Source, "value"), on(Target, "value"))
		])
		assert.equal(lower(admitted).statements.length, 2)
	})

	test("a REST-TAIL statement tuple degrades the whole type-tier wall — the roster cannot see past the tail", function restTail() {
		const Source = relation("Source", { scope: u64, value: str })
		const Target = relation("Target", { scope: u64, value: str })
		const head = contained(on(Source, "value"), on(Target, "value"))

		// after one step and judges the head against a roster blind to

		const restTailed: readonly [typeof head, ...Statement[]] = [head, key(Target, ["value"])]
		const admitted = schema("RestTail", { Source, Target }, restTailed)
		assert.equal(lower(admitted).statements.length, 2)
	})

	test("an INTERSECTION of statement tuples stays silent — one non-union type, no distributed judgment", function intersectionTuple() {
		const Source = relation("Source", { scope: u64, value: str })
		const Target = relation("Target", { scope: u64, value: str })
		const tupleA = [
			key(Target, ["scope", "value"]),
			contained(on(Source, ["scope", "value"]), on(Target, ["scope", "value"]))
		] as const
		const tupleB = [key(Target, ["value"]), contained(on(Source, "value"), on(Target, "value"))] as const

		type Verdict = LawfulStatements<{ Source: typeof Source; Target: typeof Target }, typeof tupleA & typeof tupleB>
		type Fired = Verdict extends TargetKeyWall<infer T, infer P> ? readonly [T, P] : "silent"
		const probeSilent: Equal<Fired, "silent"> = true
		assert.ok(probeSilent)
	})
})

// The engine half: the same matrix through native.dbCreate. Refused rows

function ordinary(name: string, fields: readonly FieldSpec[]): SchemaSpec["relations"][number] {
	return { name, fields, closed: undefined }
}

function fieldOf(name: string, valueType: ValueTypeSpec, fresh = false): FieldSpec {
	return { name, valueType, newtype: undefined, fresh }
}

function sideOf(relationName: string, projection: readonly string[]): SideSpec {
	return { relation: relationName, projection, selection: [] }
}

function fdOf(relationName: string, projection: readonly string[]): StatementSpec {
	return { kind: "fd", relation: relationName, projection }
}

function containmentOf(source: SideSpec, target: SideSpec, bidirectional = false): StatementSpec {
	return { kind: "containment", source, target, bidirectional }
}

describe("the target-key wall at the engine — parity through native.dbCreate", function engineHalf() {
	let bridge: typeof import("#native.ts")["native"]

	before(async function loadNative() {
		;({ native: bridge } = await import("#native.ts"))
	})

	let caseIndex = 0
	function caseDir(): string {
		caseIndex += 1
		return path.join(tmpRoot, `case-${caseIndex}`)
	}

	/** The engine's verdict on a refused spec: `schemaError`, message pinned exactly. */
	async function engineRefuses(spec: SchemaSpec, message: string): Promise<void> {
		const created = await bridge.dbCreate(caseDir(), spec)
		assert.equal(created.tag, "schemaError")
		if (created.tag === "schemaError") {
			assert.equal(created.message, message)
		}
	}

	async function engineAdmits(spec: SchemaSpec): Promise<void> {
		const created = await bridge.dbCreate(caseDir(), spec)
		assert.equal(created.tag, "accepted")
	}

	test("row 1 at the engine: the report's minimal shape — schemaError, names beside ids", async function reportShape() {
		await engineRefuses(
			{
				relations: [
					ordinary("Source", [fieldOf("value", { kind: "string" })]),
					ordinary("Target", [
						fieldOf("target", { kind: "u64" }, true),
						fieldOf("scope", { kind: "u64" }),
						fieldOf("value", { kind: "string" })
					])
				],
				statements: [
					fdOf("Target", ["scope", "value"]),
					containmentOf(sideOf("Source", ["value"]), sideOf("Target", ["value"]))
				]
			},
			"statement 2: target relation Target (1) projection {value (2)} matches no declared key; " +
				"available keys: key 0 {target (0)}; key 1 {scope (1), value (2)}"
		)
	})

	test("row 2 at the engine: Primer's shape — schemaError, names beside ids", async function primerShape() {
		await engineRefuses(
			{
				relations: [
					ordinary("Coverage", [fieldOf("sourceAddress", { kind: "string" })]),
					ordinary("AnalysisTargetEntry", [
						fieldOf("entry", { kind: "u64" }, true),
						fieldOf("policyPackage", { kind: "u64" }),
						fieldOf("sourceAddress", { kind: "string" })
					])
				],
				statements: [
					fdOf("AnalysisTargetEntry", ["policyPackage", "sourceAddress"]),
					containmentOf(sideOf("Coverage", ["sourceAddress"]), sideOf("AnalysisTargetEntry", ["sourceAddress"]))
				]
			},
			"statement 2: target relation AnalysisTargetEntry (1) projection {sourceAddress (2)} matches no declared key; " +
				"available keys: key 0 {entry (0)}; key 1 {policyPackage (1), sourceAddress (2)}"
		)
	})

	test("rows 3, 4, 6, 7, 9 at the engine: every admitted row creates", async function admittedRows() {
		const Source = relation("Source", { scope: u64, value: str })
		const Target = relation("Target", { scope: u64, value: str })
		await engineAdmits(
			lower(
				schema("SameOrder", { Source, Target }, [
					key(Target, ["scope", "value"]),
					contained(on(Source, ["scope", "value"]), on(Target, ["scope", "value"]))
				])
			)
		)
		await engineAdmits(
			lower(
				schema("Permuted", { Source, Target }, [
					key(Target, ["scope", "value"]),
					contained(on(Source, ["value", "scope"]), on(Target, ["value", "scope"]))
				])
			)
		)
		const Owner = relation("Owner", { id: u64.fresh })
		const Ref = relation("Ref", { owner: u64 })
		await engineAdmits(lower(schema("Fresh", { Owner, Ref }, [contained(on(Ref, "owner"), on(Owner, "id"))])))
		const Kind = closed("Kind", ["Checking", "Savings"])
		const Account = relation("Account", { kind: Kind.id })
		await engineAdmits(lower(schema("Closed", { Kind, Account }, [contained(on(Account, "kind"), on(Kind, "id"))])))
		const A = relation("A", { id: u64.fresh })
		const B = relation("B", { ref: u64 })
		await engineAdmits(lower(schema("Mirrored", { A, B }, [key(B, ["ref"]), mirrors(on(A, "id"), on(B, "ref"))])))
	})

	test("row 5 at the engine: subset and superset — schemaError", async function subsetSuperset() {
		const relations = [
			ordinary("Source", [
				fieldOf("scope", { kind: "u64" }),
				fieldOf("value", { kind: "string" }),
				fieldOf("extra", { kind: "u64" })
			]),
			ordinary("Target", [
				fieldOf("scope", { kind: "u64" }),
				fieldOf("value", { kind: "string" }),
				fieldOf("extra", { kind: "u64" })
			])
		]
		await engineRefuses(
			{
				relations,
				statements: [
					fdOf("Target", ["scope", "value"]),
					containmentOf(sideOf("Source", ["scope"]), sideOf("Target", ["scope"]))
				]
			},
			"statement 1: target relation Target (1) projection {scope (0)} matches no declared key; " +
				"available keys: key 0 {scope (0), value (1)}"
		)
		await engineRefuses(
			{
				relations,
				statements: [
					fdOf("Target", ["scope", "value"]),
					containmentOf(sideOf("Source", ["scope", "value", "extra"]), sideOf("Target", ["scope", "value", "extra"]))
				]
			},
			"statement 1: target relation Target (1) projection {scope (0), value (1), extra (2)} matches no declared key; " +
				"available keys: key 0 {scope (0), value (1)}"
		)
	})

	test("row 8 at the engine: a closed payload target — ClosedTargetNotHandle, names beside ids", async function closedPayload() {
		await engineRefuses(
			{
				relations: [
					{
						name: "Sev",
						fields: [fieldOf("level", { kind: "u64" })],
						closed: {
							newtype: "Sev.id",
							rows: [
								{ handle: "Info", values: [{ kind: "value", value: { kind: "u64", value: 1n } }] },
								{ handle: "Critical", values: [{ kind: "value", value: { kind: "u64", value: 5n } }] }
							]
						}
					},
					ordinary("Task", [fieldOf("level", { kind: "u64" })])
				],
				statements: [containmentOf(sideOf("Task", ["level"]), sideOf("Sev", ["level"]))]
			},
			"statement 1: closed target relation Sev (0) is addressed by its synthetic id only — " +
				"projection {level (1)} must be exactly {id (0)} (rewrite the target side as `R(id)`)"
		)
	})

	test("row 8 sub-case at the engine: a DECLARED payload key whose field set equals the projection — still ClosedTargetNotHandle", async function closedPayloadKeyed() {

		// exactly the refused field set, and the rule is CLOSEDNESS, not key

		// refusal below can only be the closed target's own.
		await engineRefuses(
			{
				relations: [
					{
						name: "Sev",
						fields: [fieldOf("level", { kind: "u64" })],
						closed: {
							newtype: "Sev.id",
							rows: [
								{ handle: "Info", values: [{ kind: "value", value: { kind: "u64", value: 1n } }] },
								{ handle: "Critical", values: [{ kind: "value", value: { kind: "u64", value: 5n } }] }
							]
						}
					},
					ordinary("Task", [fieldOf("level", { kind: "u64" })])
				],
				statements: [fdOf("Sev", ["level"]), containmentOf(sideOf("Task", ["level"]), sideOf("Sev", ["level"]))]
			},
			"statement 2: closed target relation Sev (0) is addressed by its synthetic id only — " +
				"projection {level (1)} must be exactly {id (0)} (rewrite the target side as `R(id)`)"
		)
	})

	test("row 10 at the engine: the mirrors' unkeyed reverse orientation — schemaError naming it", async function mirrorsOneKeyed() {
		const created = await bridge.dbCreate(caseDir(), {
			relations: [
				ordinary("A", [fieldOf("id", { kind: "u64" }, true), fieldOf("peer", { kind: "u64" })]),
				ordinary("B", [fieldOf("ref", { kind: "u64" })])
			],
			statements: [fdOf("B", ["ref"]), containmentOf(sideOf("A", ["peer"]), sideOf("B", ["ref"]), true)]
		})
		assert.equal(created.tag, "schemaError")
		if (created.tag === "schemaError") {
			assert.match(
				created.message,
				/target relation A \(0\) projection \{peer \(1\)\} matches no declared key; available keys: key 0 \{id \(0\)\}/
			)
		}
	})

	test("row 11 at the engine: a capacity's non-key target — schemaError, the same rule", async function capacityNonKey() {
		await engineRefuses(
			{
				relations: [
					ordinary("Parent", [fieldOf("id", { kind: "u64" }, true), fieldOf("group", { kind: "u64" })]),
					ordinary("Child", [fieldOf("parent", { kind: "u64" })])
				],
				statements: [
					{
						kind: "capacity",
						target: sideOf("Parent", ["group"]),
						weight: { kind: "unit" },
						window: { kind: "range", lo: { kind: "lit", value: 0n }, hi: { kind: "lit", value: 3n } },
						source: sideOf("Child", ["parent"])
					}
				]
			},
			"statement 1: target relation Parent (0) projection {group (1)} matches no declared key; " +
				"available keys: key 0 {id (0)}"
		)
	})

	test("row 12 at the engine: no pointwise key — schemaError with the pointwise hint", async function pointwiseHint() {
		const intervalType: ValueTypeSpec = { kind: "interval", element: "i64", width: undefined }
		await engineRefuses(
			{
				relations: [
					ordinary("Claim", [fieldOf("who", { kind: "u64" }), fieldOf("span", intervalType)]),
					ordinary("Roster", [fieldOf("who", { kind: "u64" }), fieldOf("span", intervalType)])
				],
				statements: [
					fdOf("Roster", ["who"]),
					containmentOf(sideOf("Claim", ["who", "span"]), sideOf("Roster", ["who", "span"]))
				]
			},
			"statement 1: target relation Roster (1) projection {who (0), span (1)} matches no declared key; " +
				"available keys: key 0 {who (0)}; hint: declare the exact pointwise key `R(prefix…, interval) -> R`"
		)
	})
})
