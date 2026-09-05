/**
 * The schema-tagged boundary VALUE codec (chapter 30's HTTP/export form;
 * API-02/API-09 pure half): every f64 — finite included — crosses as
 * `{"$f64":"<16 lowercase hex>"}` of canonical binary64 bits, integers as
 * canonical decimal strings, Id128 as 32 lowercase hex, bytes as one strict
 * lowercase-hex encoding, intervals as `{start,end}` in their element
 * encoding, closed references as handle names. Decoders reject malformed
 * widths, unknown tags, noncanonical representations and wrong-schema
 * values. These are pure bounded per-row functions — no native work.
 */
import assert from "node:assert/strict"
import { test } from "node:test"
import { Result } from "effect"
import { decodeBoundaryRows, encodeBoundaryRows } from "#codec.ts"
import { Id128 } from "#id128.ts"
import { Attempt, Student } from "#test/fixtures/learning.ts"

function mustId(hex: string): Id128 {
	const parsed = Id128.fromHex(hex)
	if (!Result.isSuccess(parsed)) {
		throw new Error("the fixture hex is canonical")
	}
	return parsed.success
}

const student = mustId("00112233445566778899aabbccddeeff")
const attempt = mustId("ffeeddccbbaa99887766554433221100")

test("finite, infinite, NaN and negative-zero floats all cross as canonical $f64 bit images", function floatImages() {
	const rows = [
		{ id: attempt, student, score: 0.9, units: 1n, active: { start: 0n, end: 60n } },
		{ id: attempt, student, score: Number.POSITIVE_INFINITY, units: 0n, active: { start: 0n, end: 1n } },
		{ id: attempt, student, score: Number.NaN, units: 0n, active: { start: 0n, end: 1n } },
		{ id: attempt, student, score: -0, units: 0n, active: { start: 0n, end: 1n } }
	]
	const encoded = encodeBoundaryRows(Attempt, rows)
	assert.ok(Result.isSuccess(encoded))
	const images = encoded.success.map((row) => row.score)
	assert.deepEqual(images, [
		{ $f64: "3feccccccccccccd" },
		{ $f64: "7ff0000000000000" },
		// EVERY NaN payload canonicalizes to the one quiet NaN image.
		{ $f64: "7ff8000000000000" },
		// -0 canonicalizes to +0 (chapter 11).
		{ $f64: "0000000000000000" }
	])
	// Integers are canonical decimal strings — never JSON numbers.
	assert.equal(encoded.success[0]?.units, "1")
	// Id128 is the canonical 32-lowercase-hex value.
	assert.equal(encoded.success[0]?.id, attempt)
	// The whole encode/decode round-trip is exact (bit-for-bit after
	// canonicalization; NaN equals NaN as a database value).
	const decoded = decodeBoundaryRows(Attempt, structuredClone(encoded.success))
	assert.ok(Result.isSuccess(decoded))
	assert.equal(decoded.success[0]?.score, 0.9)
	assert.equal(decoded.success[1]?.score, Number.POSITIVE_INFINITY)
	assert.ok(Number.isNaN(decoded.success[2]?.score))
	assert.ok(Object.is(decoded.success[3]?.score, 0), "-0 came back as +0")
})

test("decoders reject malformed widths, noncanonical images, unknown tags and wrong-schema values", function decoderRefusals() {
	const good = {
		id: attempt,
		student,
		score: { $f64: "3feccccccccccccd" },
		units: "1",
		active: { start: "0", end: "60" }
	}
	assert.ok(Result.isSuccess(decodeBoundaryRows(Attempt, [good])))

	const cases: ReadonlyArray<[string, unknown]> = [
		["noncanonical NaN image", { ...good, score: { $f64: "7ff8000000000001" } }],
		["negative-zero image", { ...good, score: { $f64: "8000000000000000" } }],
		["short bit image", { ...good, score: { $f64: "3fecccccccccccc" } }],
		["uppercase bit image", { ...good, score: { $f64: "3FECCCCCCCCCCCCD" } }],
		["raw JSON number for f64", { ...good, score: 0.9 }],
		["extra tag beside $f64", { ...good, score: { $f64: "3feccccccccccccd", unit: "score" } }],
		["JSON number for u64", { ...good, units: 1 }],
		["leading-zero decimal", { ...good, units: "01" }],
		["negative u64", { ...good, units: "-1" }],
		["u64 overflow", { ...good, units: (1n << 64n).toString() }],
		["uppercase Id128", { ...good, id: attempt.toUpperCase() }],
		["empty interval", { ...good, active: { start: "60", end: "60" } }],
		["inverted interval", { ...good, active: { start: "60", end: "0" } }],
		["unknown extra field", { ...good, extra: 1 }],
		["missing field", { id: attempt, student, score: good.score, units: "1" }]
	]
	for (const [label, row] of cases) {
		const decoded = decodeBoundaryRows(Attempt, [row])
		assert.ok(Result.isFailure(decoded), `${label} must refuse`)
	}
})

test("typed and dynamic boundary input share one accepted domain — the encoder refuses what the decoder refuses", function closedDomain() {
	// A malformed HOST value never encodes: the boundary is symmetric.
	const bad = [{ id: attempt, student, score: 0.5, units: 1, active: { start: 0n, end: 60n } }]
	const encoded = encodeBoundaryRows(Attempt, bad as never)
	assert.ok(Result.isFailure(encoded), "a JS number where bigint is declared refuses at encode")
})

test("student rows carry budget as exact decimal; a float budget image refuses", function integerFidelity() {
	const encoded = encodeBoundaryRows(Student, [{ id: student, name: "Ada", budget: 10n }])
	assert.ok(Result.isSuccess(encoded))
	assert.equal(encoded.success[0]?.budget, "10")
	const decoded = decodeBoundaryRows(Student, [{ id: student, name: "Ada", budget: 10 }])
	assert.ok(Result.isFailure(decoded), "a JSON number is not a canonical integer")
})
