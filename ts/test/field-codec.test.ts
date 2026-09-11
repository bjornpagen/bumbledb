import assert from "node:assert/strict"
import { test } from "node:test"
import { Schema as EffectSchema, Result } from "effect"
import {
	type AnyField,
	bool,
	bytes,
	closed,
	closedId,
	decodeBoundaryField,
	decodeBoundaryRows,
	encodeBoundaryField,
	encodeBoundaryRows,
	f64,
	fieldSchema,
	type Infer,
	i64,
	interval,
	relation,
	str,
	u64,
	uuid
} from "#index.ts"

function inputField<F extends AnyField>(field: F): EffectSchema.Codec<Infer<F>, unknown> {
	return fieldSchema(field)
}

const Kind = closed("Kind", ["Imported", "Electronic", "Postal"])
const fields = {
	unsigned: u64,
	signed: i64,
	number: f64,
	flag: bool,
	text: str,
	id: uuid,
	bytes: bytes(2),
	kind: closedId(Kind),
	unsignedSpan: interval(u64),
	signedSpan: interval(i64),
	fixedSpan: interval(i64, 5n),
	floatSpan: interval(f64)
}
const Row = relation("Values", fields)
const value = {
	unsigned: 0xffffffffffffffffn,
	signed: -0x8000000000000000n,
	number: -0,
	flag: true,
	text: "bee 🐝",
	id: "00112233-4455-6677-8899-aabbccddeeff" as const,
	bytes: new Uint8Array([0, 255]),
	kind: "Imported" as const,
	unsignedSpan: { start: 0n, end: 0xffffffffffffffffn },
	signedSpan: { start: -0x8000000000000000n, end: 0x7fffffffffffffffn },
	fixedSpan: { start: -5n, end: 0n },
	floatSpan: { start: Number.NEGATIVE_INFINITY, end: Number.POSITIVE_INFINITY }
}

test("public field codecs agree with rows for every field kind and preserve generic inference", () => {
	const encoded = Result.getOrThrow(encodeBoundaryRows(Row, [value]))[0]
	assert.ok(encoded)
	const canonical = Result.getOrThrow(decodeBoundaryRows(Row, [encoded]))[0]
	assert.ok(canonical)
	for (const name of Object.keys(fields) as Array<keyof typeof fields>) {
		const field = fields[name]
		assert.ok(EffectSchema.is(inputField(field))(value[name]), name)
		assert.deepEqual(Result.getOrThrow(encodeBoundaryField(field, value[name])), encoded[name], name)
		assert.deepEqual(Result.getOrThrow(decodeBoundaryField(field, encoded[name])), canonical[name], name)
	}
	const decoded: Result.Result<Infer<typeof fields.kind>, unknown> = decodeBoundaryField(fields.kind, "Postal")
	assert.equal(Result.getOrThrow(decoded), "Postal")
	assert.ok(Object.is(Result.getOrThrow(decodeBoundaryField(f64, { $f64: "0000000000000000" })), 0))
	assert.ok(Number.isNaN(Result.getOrThrow(decodeBoundaryField(f64, { $f64: "7ff8000000000000" }))))
	assert.deepEqual(Result.getOrThrow(encodeBoundaryField(f64, Number.NaN)), { $f64: "7ff8000000000000" })
})

test("strict field decoding rejects malformed representations and host schemas reject invalid values", () => {
	const badHost: ReadonlyArray<readonly [AnyField, unknown]> = [
		[u64, -1n],
		[u64, 1n << 64n],
		[i64, 1n << 63n],
		[i64, -(1n << 63n) - 1n],
		[u64, 1],
		[bool, 1],
		[str, "\ud800"],
		[uuid, value.id.toUpperCase()],
		[bytes(2), new Uint8Array(1)],
		[bytes(2), new Uint8Array(new SharedArrayBuffer(2))],
		[fields.kind, "Unlisted"],
		[interval(i64), { start: 1n, end: 1n }],
		[interval(u64), { start: -1n, end: 1n }],
		[fields.fixedSpan, { start: 0n, end: 4n }],
		[fields.fixedSpan, { start: 0x7ffffffffffffffan, end: 0x7fffffffffffffffn }],
		[interval(f64), { start: Number.NaN, end: 1 }],
		[interval(f64), { start: 0, end: 1, extra: true }]
	]
	for (const [field, bad] of badHost) {
		assert.equal(EffectSchema.is(fieldSchema(field))(bad), false)
		assert.ok(Result.isFailure(encodeBoundaryField(field, bad as Infer<AnyField>)))
	}
	for (const [field, bad] of [
		[u64, "01"],
		[i64, "-0"],
		[i64, "9223372036854775808"],
		[u64, 1n],
		[f64, 1],
		[f64, { $f64: "8000000000000000" }],
		[f64, { $f64: "7ff8000000000001" }],
		[f64, { $f64: "7ff0000000000000", extra: true }],
		[bytes(2), "00FF"],
		[bytes(2), "00"],
		[fields.kind, "Unlisted"],
		[interval(i64), { start: "0", end: "0" }],
		[interval(f64), { start: { $f64: "7ff8000000000000" }, end: { $f64: "7ff0000000000000" } }]
	] as const) {
		assert.ok(Result.isFailure(decodeBoundaryField(field, bad)))
	}
})

test("field descriptors are owned; accessors never execute while decoding values", () => {
	const handles: [string, ...string[]] = ["A", "B"]
	const descriptor = { kind: "u64", closed: { name: "Owned", handles } } as const
	const check = EffectSchema.is(fieldSchema(descriptor))
	handles[0] = "changed"
	assert.ok(check("A"))
	assert.equal(check("changed"), false)
	let touched = false
	const bad = {
		get start() {
			touched = true
			return "0"
		},
		end: "1"
	}
	assert.ok(Result.isFailure(decodeBoundaryField(interval(i64), bad)))
	assert.equal(touched, false)
})

function fieldTypes() {
	// @ts-expect-error A host integer is a bigint, not its JSON representation.
	encodeBoundaryField(i64, "1")
	// @ts-expect-error Closed references retain their precise roster.
	encodeBoundaryField(fields.kind, "Unlisted")
	const selected = EffectSchema.Struct({ amount: inputField(i64), method: inputField(fields.kind) })
	const check: EffectSchema.Codec<{ readonly amount: bigint; readonly method: Infer<typeof fields.kind> }, unknown> =
		selected
	return check
}
void fieldTypes
