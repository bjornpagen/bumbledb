/**
 * Little-endian byte primitives shared by the codec and the
 * fingerprint mirror. Every multi-byte integer on the batch wire
 * is little-endian; the fingerprint's canonical literal encoding is the
 * engine's big-endian order-preserving form — both live here so no third
 * spelling can appear.
 */

import * as errors from "@superbuilders/errors"

const U64_MAX = 0xffffffffffffffffn
const I64_MIN = -0x8000000000000000n
const I64_MAX = 0x7fffffffffffffffn
const I64_SIGN_BIT = 0x8000000000000000n

class ByteWriter {
	private buf: Uint8Array
	private len = 0

	constructor(capacity = 256) {
		this.buf = new Uint8Array(capacity)
	}

	private grow(need: number): void {
		if (this.len + need <= this.buf.length) {
			return
		}
		let capacity = this.buf.length * 2
		while (capacity < this.len + need) {
			capacity *= 2
		}
		const next = new Uint8Array(capacity)
		next.set(this.buf.subarray(0, this.len))
		this.buf = next
	}

	u8(value: number): void {
		this.grow(1)
		this.buf[this.len] = value
		this.len += 1
	}

	bytes(raw: Uint8Array): void {
		this.grow(raw.length)
		this.buf.set(raw, this.len)
		this.len += raw.length
	}

	u16le(value: number): void {
		this.grow(2)
		this.buf[this.len] = value & 0xff
		this.buf[this.len + 1] = (value >>> 8) & 0xff
		this.len += 2
	}

	u32le(value: number): void {
		this.grow(4)
		this.buf[this.len] = value & 0xff
		this.buf[this.len + 1] = (value >>> 8) & 0xff
		this.buf[this.len + 2] = (value >>> 16) & 0xff
		this.buf[this.len + 3] = (value >>> 24) & 0xff
		this.len += 4
	}

	u64le(value: bigint): void {
		if (value < 0n || value > U64_MAX) {
			throw errors.new(`u64 out of range: ${value}`)
		}
		this.grow(8)
		let v = value
		for (let i = 0; i < 8; i++) {
			this.buf[this.len + i] = Number(v & 0xffn)
			v >>= 8n
		}
		this.len += 8
	}

	i64le(value: bigint): void {
		if (value < I64_MIN || value > I64_MAX) {
			throw errors.new(`i64 out of range: ${value}`)
		}
		this.u64le(value & U64_MAX)
	}

	u64be(value: bigint): void {
		if (value < 0n || value > U64_MAX) {
			throw errors.new(`u64 out of range: ${value}`)
		}
		this.grow(8)
		let v = value
		for (let i = 7; i >= 0; i--) {
			this.buf[this.len + i] = Number(v & 0xffn)
			v >>= 8n
		}
		this.len += 8
	}

	/** The engine's sign-flipped big-endian i64 (lexicographic = numeric). */
	i64beFlipped(value: bigint): void {
		if (value < I64_MIN || value > I64_MAX) {
			throw errors.new(`i64 out of range: ${value}`)
		}
		this.u64be((value & U64_MAX) ^ I64_SIGN_BIT)
	}

	finish(): Uint8Array {
		return this.buf.slice(0, this.len)
	}
}

interface ReadFailure {
	fail(what: string): never
}

class ByteReader {
	private readonly buf: Uint8Array
	private pos = 0
	private readonly refusal: ReadFailure

	constructor(buf: Uint8Array, refusal: ReadFailure) {
		this.buf = buf
		this.refusal = refusal
	}

	remaining(): number {
		return this.buf.length - this.pos
	}

	private take(count: number, what: string): Uint8Array {
		if (this.pos + count > this.buf.length) {
			this.refusal.fail(what)
		}
		const out = this.buf.subarray(this.pos, this.pos + count)
		this.pos += count
		return out
	}

	u8(what: string): number {
		const raw = this.take(1, what)
		const byte = raw[0]
		if (byte === undefined) {
			this.refusal.fail(what)
		}
		return byte
	}

	bytes(count: number, what: string): Uint8Array {
		return new Uint8Array(this.take(count, what))
	}

	u16le(what: string): number {
		const raw = this.take(2, what)
		return (raw[0] ?? 0) | ((raw[1] ?? 0) << 8)
	}

	u32le(what: string): number {
		const raw = this.take(4, what)
		return (((raw[0] ?? 0) | ((raw[1] ?? 0) << 8) | ((raw[2] ?? 0) << 16)) + (raw[3] ?? 0) * 0x1000000) >>> 0
	}

	u64le(what: string): bigint {
		const raw = this.take(8, what)
		let value = 0n
		for (let i = 7; i >= 0; i--) {
			value = (value << 8n) | BigInt(raw[i] ?? 0)
		}
		return value
	}

	i64le(what: string): bigint {
		const unsigned = this.u64le(what)
		return unsigned > I64_MAX ? unsigned - (U64_MAX + 1n) : unsigned
	}
}

function bytesEqual(a: Uint8Array, b: Uint8Array): boolean {
	if (a.length !== b.length) {
		return false
	}
	for (let i = 0; i < a.length; i++) {
		if (a[i] !== b[i]) {
			return false
		}
	}
	return true
}

function bytesCompare(a: Uint8Array, b: Uint8Array): number {
	const shared = Math.min(a.length, b.length)
	for (let i = 0; i < shared; i++) {
		const delta = (a[i] ?? 0) - (b[i] ?? 0)
		if (delta !== 0) {
			return delta
		}
	}
	return a.length - b.length
}

const HEX_DIGITS = "0123456789abcdef"

function toHex(bytes: Uint8Array): string {
	let out = ""
	for (const byte of bytes) {
		out += HEX_DIGITS[byte >>> 4]
		out += HEX_DIGITS[byte & 0xf]
	}
	return out
}

function fromHex(hex: string): Uint8Array {
	if (hex.length % 2 !== 0 || /[^0-9a-f]/.test(hex)) {
		throw errors.new(`not lowercase hex: ${hex}`)
	}
	const out = new Uint8Array(hex.length / 2)
	for (let i = 0; i < out.length; i++) {
		const hi = HEX_DIGITS.indexOf(hex[i * 2] ?? "")
		const lo = HEX_DIGITS.indexOf(hex[i * 2 + 1] ?? "")
		if (hi < 0 || lo < 0) {
			throw errors.new(`not lowercase hex: ${hex}`)
		}
		out[i] = (hi << 4) | lo
	}
	return out
}

declare const digest32Brand: unique symbol
type Digest32 = Uint8Array & { readonly [digest32Brand]: typeof digest32Brand }

function digest32(bytes: Uint8Array): Digest32 {
	if (bytes.length !== 32) {
		throw errors.new(`digest is not 32 bytes: ${bytes.length}`)
	}
	return bytes as Digest32
}

function digest32FromHex(hex: string): Digest32 {
	return digest32(fromHex(hex))
}

function hex32(bytes: Digest32): string {
	return toHex(bytes)
}

function saturatingAddU64(a: bigint, b: bigint): bigint {
	const sum = a + b
	return sum > U64_MAX ? U64_MAX : sum
}

function checkedAddU64(a: bigint, b: bigint): bigint | undefined {
	const sum = a + b
	return sum > U64_MAX ? undefined : sum
}

const utf8Encoder = new TextEncoder()
const utf8StrictDecoder = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true })

export type { Digest32 }
export {
	ByteReader,
	ByteWriter,
	bytesCompare,
	bytesEqual,
	checkedAddU64,
	digest32,
	digest32FromHex,
	fromHex,
	hex32,
	I64_MAX,
	I64_MIN,
	saturatingAddU64,
	toHex,
	U64_MAX,
	utf8Encoder,
	utf8StrictDecoder
}
