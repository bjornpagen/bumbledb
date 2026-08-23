/**
 * The driver's raw-value vocabulary and its two canonical encodings:
 * the command codec's tagged little-endian form (20's tag table, the one
 * shared tagged-value encoding the footprint keys also hash), and the
 * engine's big-endian order-preserving literal form the fingerprint
 * mirror reproduces. Values here are always RAW — strings as UTF-8,
 * never intern ids — which is what makes footprint keys state-independent.
 */

import type { ValueTypeSpec } from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import type { ByteReader, ByteWriter } from "#bytes.ts"
import { bytesEqual, I64_MAX, I64_MIN, U64_MAX, utf8Encoder, utf8StrictDecoder } from "#bytes.ts"

interface LogInterval {
	readonly start: bigint
	readonly end: bigint
}

type LogValue = boolean | bigint | string | Uint8Array | LogInterval

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

function isInterval(value: LogValue): value is LogInterval {
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

function checkAgainst(context: string, type: ValueTypeSpec, value: LogValue): void {
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
			if (typeof value !== "string" || !value.isWellFormed()) {
				throw errors.new(`${context}: expected well-formed string`)
			}
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
			if (type.width !== undefined && value.end - value.start !== type.width) {
				throw errors.new(`${context}: interval width must be ${type.width}`)
			}
			return
		}
	}
}

/** The codec's tagged form: `tag u8` + payload, at the field's layout. */
function writeTagged(out: ByteWriter, type: ValueTypeSpec, value: LogValue): void {
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
			const raw = utf8Encoder.encode(value as string)
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
			const interval = value as LogInterval
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
function readTagged(reader: ByteReader, type: ValueTypeSpec, refusal: TaggedRefusal): LogValue {
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
			const decoded = errors.trySync(function decodeUtf8() {
				return utf8StrictDecoder.decode(raw)
			})
			if (decoded.error) {
				refusal.invalidUtf8()
			}
			return decoded.data
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
			const end = start + type.width
			if (type.element === "u64" ? end > U64_MAX : end > I64_MAX) {
				refusal.intervalOverflow()
			}
			return { start, end }
		}
	}
}

function valuesEqual(a: LogValue, b: LogValue): boolean {
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

/**
 * The engine's canonical big-endian literal form (`encode_literal`) —
 * the fingerprint mirror's alphabet. Strings never reach this encoder:
 * the fingerprint's `put_literal` length-prefixes them separately, and
 * closed ground axioms with string columns are the mirror's recorded gap.
 */
function writeCanonicalLiteral(out: ByteWriter, type: ValueTypeSpec, value: LogValue): void {
	switch (type.kind) {
		case "bool": {
			out.u8(value === true ? 1 : 0)
			return
		}
		case "u64": {
			out.u64be(value as bigint)
			return
		}
		case "i64": {
			out.i64beFlipped(value as bigint)
			return
		}
		case "string":
			throw errors.new("canonical literal: strings are length-prefixed by the caller, never encoded here")
		case "fixedBytes": {
			const raw = value as Uint8Array
			const padded = Math.ceil(type.len / 8) * 8
			out.bytes(raw)
			for (let i = raw.length; i < padded; i++) {
				out.u8(0)
			}
			return
		}
		case "interval": {
			const interval = value as LogInterval
			if (type.width !== undefined) {
				if (type.element === "u64") {
					out.u64be(interval.start)
				} else {
					out.i64beFlipped(interval.start)
				}
				return
			}
			if (type.element === "u64") {
				out.u64be(interval.start)
				out.u64be(interval.end)
			} else {
				out.i64beFlipped(interval.start)
				out.i64beFlipped(interval.end)
			}
			return
		}
	}
}

export type { LogInterval, LogValue, TaggedRefusal }
export { checkAgainst, isInterval, readTagged, TAG, valuesEqual, wireTagOf, writeCanonicalLiteral, writeTagged }
