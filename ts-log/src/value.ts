/**
 * The driver's raw-value vocabulary and its one wire encoding: the
 * command codec's tagged little-endian form (the tag table the batch
 * wire already declared). Values here are always RAW — strings as
 * UTF-8, never intern ids — so a key hashed from a raw value cannot
 * depend on a catalog.
 */

import type { ValueTypeSpec } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import type { ByteReader, ByteWriter } from "#bytes.ts"
import { bytesEqual, I64_MAX, I64_MIN, U64_MAX, utf8Encoder, utf8StrictDecoder } from "#bytes.ts"

interface Interval {
	readonly start: bigint
	readonly end: bigint
}

type Value = boolean | bigint | string | Uint8Array | Interval

/** A string cell: well-formed UTF-8 by construction. A lone surrogate cannot enter. */
declare const wellFormedUtf8Brand: unique symbol
type WellFormedUtf8 = string & { readonly [wellFormedUtf8Brand]: typeof wellFormedUtf8Brand }

/** Fatal encoder: refuses lone surrogates rather than emitting U+FFFD. */
function wellFormedUtf8(text: string): WellFormedUtf8 {
	if (!text.isWellFormed()) {
		throw errors.new("string cell is not well-formed UTF-8")
	}
	return text as WellFormedUtf8
}

function parseWellFormedUtf8(raw: Uint8Array): WellFormedUtf8 | undefined {
	const decoded = errors.trySync(function decodeUtf8() {
		return utf8StrictDecoder.decode(raw)
	})
	if (decoded.error) {
		return undefined
	}
	return decoded.data as WellFormedUtf8
}

function domainCeiling(element: "u64" | "i64"): bigint {
	return element === "u64" ? U64_MAX : I64_MAX
}

/** Fixed-width `[start, start + width)`; the domain ceiling is not a value. */
function fixedInterval(start: bigint, width: bigint, element: "u64" | "i64"): Interval | undefined {
	const end = start + width
	if (end <= start || end >= domainCeiling(element)) {
		return undefined
	}
	return { start, end }
}

/** 20's tag table: the codec's own numbering, normative to the byte. */
const TAG = {
	bool: 0,
	u64: 1,
	i64: 2,
	string: 3,
	fixedBytes: 4,
	interval: 5,
	fixedInterval: 6
} as const

function isInterval(value: Value): value is Interval {
	return typeof value === "object" && !(value instanceof Uint8Array)
}

function wireTagOf(type: ValueTypeSpec): number {
	switch (type.kind) {
		case "bool":
			return TAG.bool
		case "u64":
			return TAG.u64
		case "i64":
			return TAG.i64
		case "string":
			return TAG.string
		case "fixedBytes":
			return TAG.fixedBytes
		case "interval":
			return type.width === undefined ? TAG.interval : TAG.fixedInterval
	}
}

function checkAgainst(context: string, type: ValueTypeSpec, value: Value): void {
	switch (type.kind) {
		case "bool": {
			if (typeof value !== "boolean") {
				throw errors.new(`${context}: expected boolean`)
			}
			return
		}
		case "u64": {
			if (typeof value !== "bigint" || value < 0n || value > U64_MAX) {
				throw errors.new(`${context}: expected u64 bigint`)
			}
			return
		}
		case "i64": {
			if (typeof value !== "bigint" || value < I64_MIN || value > I64_MAX) {
				throw errors.new(`${context}: expected i64 bigint`)
			}
			return
		}
		case "string": {
			if (typeof value !== "string") {
				throw errors.new(`${context}: expected well-formed string`)
			}
			wellFormedUtf8(value)
			return
		}
		case "fixedBytes": {
			if (!(value instanceof Uint8Array) || value.length !== type.len) {
				throw errors.new(`${context}: expected ${type.len}-byte Uint8Array`)
			}
			return
		}
		case "interval": {
			if (!(typeof value === "object") || value instanceof Uint8Array) {
				throw errors.new(`${context}: expected interval value`)
			}
			const lo = type.element === "u64" ? 0n : I64_MIN
			const hi = type.element === "u64" ? U64_MAX : I64_MAX
			if (value.start < lo || value.end > hi || value.start >= value.end) {
				throw errors.new(`${context}: interval bounds out of range or empty`)
			}
			if (type.width !== undefined) {
				const parsed = fixedInterval(value.start, type.width, type.element)
				if (parsed === undefined) {
					throw errors.new(`${context}: fixed interval end is the domain ceiling`)
				}
				if (parsed.end !== value.end) {
					throw errors.new(`${context}: interval width must be ${type.width}`)
				}
			}
			return
		}
	}
}

/** The codec's tagged form: `tag u8` + payload, at the field's layout. */
function writeTagged(out: ByteWriter, type: ValueTypeSpec, value: Value): void {
	switch (type.kind) {
		case "bool": {
			out.u8(TAG.bool)
			out.u8(value === true ? 1 : 0)
			return
		}
		case "u64": {
			out.u8(TAG.u64)
			out.u64le(value as bigint)
			return
		}
		case "i64": {
			out.u8(TAG.i64)
			out.i64le(value as bigint)
			return
		}
		case "string": {
			const text = wellFormedUtf8(value as string)
			const raw = utf8Encoder.encode(text)
			out.u8(TAG.string)
			out.u32le(raw.length)
			out.bytes(raw)
			return
		}
		case "fixedBytes": {
			out.u8(TAG.fixedBytes)
			out.bytes(value as Uint8Array)
			return
		}
		case "interval": {
			const interval = value as Interval
			if (type.width === undefined) {
				out.u8(TAG.interval)
				if (type.element === "u64") {
					out.u64le(interval.start)
					out.u64le(interval.end)
				} else {
					out.i64le(interval.start)
					out.i64le(interval.end)
				}
				return
			}
			out.u8(TAG.fixedInterval)
			if (type.element === "u64") {
				out.u64le(interval.start)
			} else {
				out.i64le(interval.start)
			}
			return
		}
	}
}

/** The value parser's refusal channel: one method per proved cause,
 *  matching the Rust decoder's cross-implementation identities. */
interface TaggedRefusal {
	badTag(expected: number, actual: number): never
	boolByte(byte: number): never
	invalidUtf8(): never
	emptyInterval(): never
	intervalOverflow(): never
}

/** Full parse at the layout's type; every illegal byte is a typed refusal. */
function readTagged(reader: ByteReader, type: ValueTypeSpec, refusal: TaggedRefusal): Value {
	const expected = wireTagOf(type)
	const tag = reader.u8("value tag")
	if (tag !== expected) {
		refusal.badTag(expected, tag)
	}
	switch (type.kind) {
		case "bool": {
			const byte = reader.u8("bool payload")
			if (byte > 1) {
				refusal.boolByte(byte)
			}
			return byte === 1
		}
		case "u64":
			return reader.u64le("u64 payload")
		case "i64":
			return reader.i64le("i64 payload")
		case "string": {
			const len = reader.u32le("string length")
			const raw = reader.bytes(len, "string payload")
			const text = parseWellFormedUtf8(raw)
			if (text === undefined) {
				refusal.invalidUtf8()
			}
			return text
		}
		case "fixedBytes":
			return reader.bytes(type.len, "fixedBytes payload")
		case "interval": {
			if (type.width === undefined) {
				const start = type.element === "u64" ? reader.u64le("interval start") : reader.i64le("interval start")
				const end = type.element === "u64" ? reader.u64le("interval end") : reader.i64le("interval end")
				if (start >= end) {
					refusal.emptyInterval()
				}
				return { start, end }
			}
			const start = type.element === "u64" ? reader.u64le("interval start") : reader.i64le("interval start")
			const parsed = fixedInterval(start, type.width, type.element)
			if (parsed === undefined) {
				refusal.intervalOverflow()
			}
			return parsed
		}
	}
}

function valuesEqual(a: Value, b: Value): boolean {
	if (typeof a === "boolean" || typeof a === "bigint" || typeof a === "string") {
		return a === b
	}
	if (a instanceof Uint8Array) {
		return b instanceof Uint8Array && bytesEqual(a, b)
	}
	if (typeof b !== "object" || b instanceof Uint8Array) {
		return false
	}
	return a.start === b.start && a.end === b.end
}

export type { Interval, TaggedRefusal, Value, WellFormedUtf8 }
export { checkAgainst, isInterval, readTagged, TAG, valuesEqual, wellFormedUtf8, wireTagOf, writeTagged }
