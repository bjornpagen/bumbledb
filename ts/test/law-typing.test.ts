/**
 * PRD-K4 probes: THE LAWS TYPE THE COLUMNS. `schema()` computes the
 * equivalence-class map over field slots FROM the statement list, at the
 * type level and at runtime, and this file pins the whole law set:
 * generator naming (fresh coordinates and closed synthetic ids); the
 * 3-hop containment chain landing one class end to end; the generator-less
 * class named by its least member in relation-declaration ×
 * field-declaration order at the VALUE tier (the wire authority) and
 * carried as its member-coordinate SET at the TYPE tier (deterministic —
 * a type-level least-member pick is unimplementable: union member order is
 * not observably stable); the bare field staying bare; the ψ-selected
 * face pairing; the selected-mirrors shape (the mirrors law types the
 * source column with the target's class — pinned exactly); the
 * one-generator wall at BOTH tiers (the named, self-locating `ClassWall`
 * compile verdict — generators and the offending paired slots pinned by
 * `Equal` — and the construction throw naming the statement), with the
 * re-homed cross-domain construction probes (contained/mirrors/window
 * unifying two mints, the closed-id generator included); the runtime/type
 * agreement diff (a GENERATED golden the compiler pins against the schema
 * type's `classes` while `deepStrictEqual` pins it against the runtime
 * map — one golden, two tiers); the wire lowering (spec `newtype` = class
 * name / omitted — `statements.test.ts` carries the full SchemaSpec pins);
 * the manifest golden (statements in == statements out: nothing is ever
 * synthesized, order preserved, count equal); and the declaration-order
 * refusal of integer-index names (the enumeration-order hazard closed at
 * construction). Every `Equal` probe is a value, so each compile-time
 * claim carries its own runtime assertion; every `@ts-expect-error` is
 * REAL (removing it breaks compilation).
 */

import assert from "node:assert/strict"
import { describe, test } from "node:test"

import { closed } from "#closed.ts"
import { atMost } from "#count.ts"
import { on } from "#face.ts"
import { str, u64 } from "#fields.ts"
import type { ClassWall, LawfulStatements } from "#law.ts"
import { lower } from "#lower.ts"
import { relation } from "#relation.ts"
import { schema } from "#schema.ts"
import { contained, mirrors, renderStatement, window } from "#statements.ts"

/** The identity-strength equality probe (the standard dual-function trick). */
type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

describe("the three class laws", function laws() {
	test("generators name their classes; a 3-hop chain lands the whole chain in the generator's class; bare stays bare", function chainGolden() {
		// Closedness rides the descriptor: every chain member referencing the
		// vocabulary is spelled with the vocabulary's OWN descriptor (the
		// roster-agreement wall — a plain u64 cannot alias Vocab.id).
		const Vocab = closed("Vocab", ["Alpha", "Beta"])
		const A = relation("A", { x: Vocab.id, note: str })
		const B = relation("B", { y: Vocab.id })
		const Chain = schema("Chain", { Vocab, A, B }, [
			contained(on(A, "x"), on(B, "y")),
			contained(on(B, "y"), on(Vocab, "id"))
		])
		// The type tier computed the same map the value tier carries.
		const probeAx: Equal<(typeof Chain)["classes"]["A"]["x"], "Vocab.id"> = true
		const probeBy: Equal<(typeof Chain)["classes"]["B"]["y"], "Vocab.id"> = true
		const probeVocab: Equal<(typeof Chain)["classes"]["Vocab"]["id"], "Vocab.id"> = true
		const probeBare: Equal<(typeof Chain)["classes"]["A"]["note"], undefined> = true
		assert.ok(probeAx && probeBy && probeVocab && probeBare)
		assert.deepStrictEqual(Chain.classes, {
			Vocab: { id: "Vocab.id" },
			A: { x: "Vocab.id", note: undefined },
			B: { y: "Vocab.id" }
		})
		assert.ok(Object.isFrozen(Chain.classes) && Object.isFrozen(Chain.classes.A))
	})

	test("a generator-less class is named by its least member in relation-declaration × field-declaration order", function leastMember() {
		const First = relation("First", { a: u64 })
		const Second = relation("Second", { b: u64 })
		// Second is DECLARED first in the record — the statement's written
		// orientation does not name the class; the declaration walk does.
		// The RUNTIME map (the wire authority) carries the ratified
		// least-member name; the TYPE tier carries the component's member
		// SET (deterministic — TypeScript's union order is not observable,
		// so a type-level least-member pick would drift), the runtime name a
		// member of it by construction, both slots the identical set (the
		// join judgment is the same at both tiers).
		const Pairing = schema("Pairing", { Second, First }, [mirrors(on(First, "a"), on(Second, "b"))])
		const probeFirst: Equal<(typeof Pairing)["classes"]["First"]["a"], "First.a" | "Second.b"> = true
		const probeSecond: Equal<(typeof Pairing)["classes"]["Second"]["b"], "First.a" | "Second.b"> = true
		const probeSameClass: Equal<(typeof Pairing)["classes"]["First"]["a"], (typeof Pairing)["classes"]["Second"]["b"]> =
			true
		assert.ok(probeFirst && probeSecond && probeSameClass)
		// The runtime name inhabits the type-level set — the property's type is honest.
		const runtimeName: (typeof Pairing)["classes"]["First"]["a"] = Pairing.classes.First.a
		assert.equal(runtimeName, "Second.b", "the ratified least-member name — Second declared first")
		assert.deepStrictEqual(Pairing.classes, {
			Second: { b: "Second.b" },
			First: { a: "Second.b" }
		})
	})

	test("a ψ-selected closed face pairs exactly like the bare face — the selection changes classes not at all", function psiPairing() {
		const Grade = closed("Grade", { mastered: str }, { Failed: { mastered: "no" }, DirectPass: { mastered: "yes" } })
		const Certificate = relation("Certificate", { id: u64.fresh, grade: Grade.id })
		const Mastery = schema("Mastery", { Grade, Certificate }, [
			contained(on(Certificate, "grade"), on(Grade.where({ mastered: "yes" }), "id"))
		])
		const probeGrade: Equal<(typeof Mastery)["classes"]["Certificate"]["grade"], "Grade.id"> = true
		const probeId: Equal<(typeof Mastery)["classes"]["Certificate"]["id"], "Certificate.id"> = true
		const probeColumn: Equal<(typeof Mastery)["classes"]["Grade"]["mastered"], undefined> = true
		assert.ok(probeGrade && probeId && probeColumn)
		assert.equal(Mastery.classes.Certificate?.grade, "Grade.id")
		assert.equal(Mastery.classes.Grade?.mastered, undefined, "a σ-selected column is not a paired slot")
	})

	test("the selected-mirrors shape (Calendar): the mirrors law types the source column with the target's class", function selectedMirrors() {
		const Booking = relation("Booking", { id: u64.fresh, room: u64 })
		const CalendarEntry = relation("CalendarEntry", { booking: u64, label: str })
		const Calendar = schema("Calendar", { Booking, CalendarEntry }, [
			mirrors(on(CalendarEntry.where({ label: "hold" }), "booking"), on(Booking, "id"))
		])
		// Pinned exactly: the σ-selected SOURCE column lands in the TARGET's
		// generator class — "Booking.id", never a least-member name.
		const probeSource: Equal<(typeof Calendar)["classes"]["CalendarEntry"]["booking"], "Booking.id"> = true
		assert.ok(probeSource)
		assert.deepStrictEqual(Calendar.classes, {
			Booking: { id: "Booking.id", room: undefined },
			CalendarEntry: { booking: "Booking.id", label: undefined }
		})
	})
})

describe("the one-generator wall — two mints cannot share a carrier (the re-homed cross-domain probes)", function wall() {
	const Left = relation("Left", { id: u64.fresh, peer: u64 })
	const Right = relation("Right", { id: u64.fresh })
	const Vocab = closed("Vocab", ["Alpha", "Beta"])

	test("the compile verdict is named and self-locating: the colliding generators and the paired slots", function verdictShape() {
		const direct = [contained(on(Left, "id"), on(Right, "id"))] as const
		type Verdict = LawfulStatements<{ Left: typeof Left; Right: typeof Right }, typeof direct>
		type Located = Verdict extends ClassWall<infer G, infer Chain> ? readonly [G, Chain] : never
		const probeGenerators: Equal<Located[0], "Left.id" | "Right.id"> = true
		const probeChain: Equal<Located[1], readonly ["Left.id ~ Right.id"]> = true
		assert.ok(probeGenerators && probeChain)
		// A lawful list resolves to the no-op intersection instead.
		const lawful = [contained(on(Left, "peer"), on(Right, "id"))] as const
		type Lawful = LawfulStatements<{ Left: typeof Left; Right: typeof Right }, typeof lawful>
		const probeLawful: Equal<Lawful, unknown> = true
		assert.ok(probeLawful)
	})

	test("a containment unifying two fresh coordinates refuses at both tiers", function containedWall() {
		assert.throws(function runtimeTwin() {
			// @ts-expect-error — the ClassWall verdict: Left.id and Right.id are both generators
			schema("Broken", { Left, Right }, [contained(on(Left, "id"), on(Right, "id"))])
		}, /the statements unify two generators into one class — Left\.id and Right\.id \(two mints cannot share a carrier\) — Left\(id\) <= Right\(id\)/)
	})

	test("a mirrors bijection unifying two fresh coordinates refuses at both tiers", function mirrorsWall() {
		assert.throws(function runtimeTwin() {
			// @ts-expect-error — the ClassWall verdict through the == abbreviation
			schema("Broken", { Left, Right }, [mirrors(on(Left, "id"), on(Right, "id"))])
		}, /two mints cannot share a carrier.*Left\(id\) == Right\(id\)/)
	})

	test("a window's grouping join unifying two fresh coordinates refuses at both tiers", function windowWall() {
		assert.throws(function runtimeTwin() {
			// @ts-expect-error — the ClassWall verdict through the window's positionwise pairing
			schema("Broken", { Left, Right }, [window(on(Left, "id"), atMost(3n), on(Right, "id"))])
		}, /two mints cannot share a carrier.*Left\(id\) <=\{0\.\.3\} Right\(id\)/)
	})

	test("a closed relation's id is a generator too — unifying it with a fresh coordinate refuses (the roster wall fires first, at construction)", function closedWall() {
		// The two-mint collision through a closed id is now unreachable: the
		// statement constructors' roster-agreement wall refuses the pairing
		// EARLIER (a fresh u64 is a bare column; the closed [id] carries its
		// roster), so the generator wall behind it can never be reached
		// through a closed id — the refusal moved earlier and warmer.
		assert.throws(function runtimeTwin() {
			// @ts-expect-error — a fresh u64 mint never pairs the closed [id]: the roster rides the face shape
			schema("Broken", { Vocab, Left }, [contained(on(Left, "id"), on(Vocab, "id"))])
		}, /Left\.id is a bare column but Vocab\.id is a Vocab reference — closedness rides the descriptor/)
	})

	test("the wall fires through a TRANSITIVE chain, naming the statement that closed it", function transitiveWall() {
		const Bridge = relation("Bridge", { ref: u64 })
		assert.throws(function runtimeTwin() {
			// @ts-expect-error — the second containment merges the two generator components
			schema("Broken", { Left, Right, Bridge }, [
				contained(on(Bridge, "ref"), on(Left, "id")),
				contained(on(Bridge, "ref"), on(Right, "id"))
			])
		}, /two mints cannot share a carrier.*Bridge\(ref\) <= Right\(id\)/)
	})
})

describe("the runtime/type agreement and the wire", function agreement() {
	/** The one fixture both tiers are diffed over (every law exercised). */
	function buildFixture() {
		const Vocab = closed("Vocab", ["Alpha", "Beta"])
		const Holder = relation("Holder", { id: u64.fresh, name: str })
		const Account = relation("Account", { id: u64.fresh, holder: u64, kind: Vocab.id, note: str })
		const Terms = relation("Terms", { account: u64 })
		return schema("Agreement", { Vocab, Holder, Account, Terms }, [
			contained(on(Account, "holder"), on(Holder, "id")),
			contained(on(Account, "kind"), on(Vocab, "id")),
			mirrors(on(Account, "id"), on(Terms, "account")),
			window(on(Holder, "id"), atMost(3n), on(Account, "holder"))
		])
	}

	/**
	 * GENERATED GOLDEN (scripts/generate-law-fixtures.ts — regenerate, never
	 * hand-edit): the compiler pins it against the schema TYPE's class map
	 * (`Equal`), the assertion pins it against the runtime map — one golden
	 * proving the two tiers computed the same classes.
	 */
	const GOLDEN = {
		Vocab: { id: "Vocab.id" },
		Holder: { id: "Holder.id", name: undefined },
		Account: { id: "Account.id", holder: "Holder.id", kind: "Vocab.id", note: undefined },
		Terms: { account: "Account.id" }
	} as const

	test("the runtime class map and the type-level class map are the same map", function diff() {
		const fixture = buildFixture()
		const probe: Equal<(typeof fixture)["classes"], typeof GOLDEN> = true
		assert.ok(probe)
		assert.deepStrictEqual(fixture.classes, GOLDEN)
	})

	test("the manifest golden: statements in == statements out — nothing synthesized, order preserved, count equal", function manifestGolden() {
		const fixture = buildFixture()
		const written = [
			"Account(holder) <= Holder(id)",
			"Account(kind) <= Vocab(id)",
			"Account(id) == Terms(account)",
			"Holder(id) <={0..3} Account(holder)"
		]
		assert.deepStrictEqual(fixture.statements.map(renderStatement), written)
		const spec = lower(fixture)
		assert.equal(
			spec.statements.length,
			fixture.statements.length,
			"the lowered list is the declared list — count equal"
		)
		assert.equal(spec.statements.length, written.length)
	})

	test("the wire: spec newtype = class name for classed slots, omitted for bare (fingerprint-neutral by the engine's drop)", function wire() {
		const spec = lower(buildFixture())
		const account = spec.relations.find(function byName(candidate) {
			return candidate.name === "Account"
		})
		assert.ok(account)
		assert.deepStrictEqual(
			account.fields.map(function newtypeOf(field) {
				return [field.name, field.newtype]
			}),
			[
				["id", "Account.id"],
				["holder", "Holder.id"],
				["kind", "Vocab.id"],
				["note", undefined]
			]
		)
		const vocab = spec.relations.find(function byName(candidate) {
			return candidate.name === "Vocab"
		})
		assert.ok(vocab)
		assert.equal(vocab.closed?.newtype, "Vocab.id", "a closed relation's handle newtype IS its id's generator class")
	})

	test("integer-index names are refused at construction — the declaration-order law's enumeration hazard", function integerNames() {
		assert.throws(function integerField() {
			relation("R", { "0": u64 })
		}, /integer index — JavaScript object keys re-order integer indices/)
	})

	test("dotted names are refused at construction — the coordinate encoding stays injective at both tiers", function dottedNames() {
		assert.throws(function dottedRelation() {
			relation("A.B", { x: u64 })
		}, /contains a dot — the law classes key on the `relation\.field` coordinate/)
		assert.throws(function dottedField() {
			relation("A", { "B.x": u64 })
		}, /contains a dot/)
		assert.throws(function dottedClosed() {
			closed("A.B", ["Yes", "No"])
		}, /contains a dot/)
	})

	test("a plain __proto__ declaration entry is refused — the Annex B setter would silently drop the key", function protoEntries() {
		const litHandles = { __proto__: { pages: 1n }, Warn: { pages: 2n } }
		assert.throws(function protoLiteralHandle() {
			closed("Sev", { pages: u64 }, litHandles)
		}, /prototype was replaced/)
		// The computed spelling creates an own data property and is admitted: no name is reserved.
		const Sev = closed("Sev", { pages: u64 }, { ["__proto__"]: { pages: 1n }, Warn: { pages: 2n } })
		assert.deepStrictEqual([...Sev.data.handles], ["__proto__", "Warn"])
		assert.ok(Object.hasOwn(Sev.axioms, "__proto__"))
		assert.deepStrictEqual(Object.getOwnPropertyDescriptor(Sev.axioms, "__proto__")?.value, { pages: 1n })
	})
})
