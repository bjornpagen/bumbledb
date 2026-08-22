import assert from "node:assert/strict"
import { test } from "node:test"

import { LawScale, pinBare, pinChain, pinClosed, pinGenerator, pinVocab } from "#test/fixtures/law-scale.ts"

test("the primer-scale fixture constructs and both tiers agree at scale", function scaleGate() {
	assert.ok(pinChain && pinVocab && pinGenerator && pinClosed && pinBare)

	assert.equal(Object.keys(LawScale.relations).length, 40, "40 relations")
	assert.equal(LawScale.statements.length, 155, "155 statements — statements in == statements out")
	let slots = 0
	for (const record of Object.values(LawScale.classes)) {
		slots += Object.keys(record).length
	}
	assert.equal(slots, 200, "200 field slots in the class map")

	assert.equal(LawScale.classes.R5?.ref, "R4.id")
	assert.equal(LawScale.classes.R9?.kind, "Vocab1.id")
	assert.equal(LawScale.classes.R31?.id, "R31.id")
	assert.equal(LawScale.classes.Vocab7?.id, "Vocab7.id")
	assert.equal(LawScale.classes.R3?.score, undefined)

	assert.equal(LawScale.classes.R0?.at, "R0.at")
	assert.equal(LawScale.classes.R20?.at, "R0.at")
	assert.equal(LawScale.classes.R21?.at, "R0.at", "the pointwise composite containment joins the same chain")
})
