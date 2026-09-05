import { LogInputError } from "#errors.ts"

const U64_MAX = 0xffffffffffffffffn
const utf8Encoder = new TextEncoder()
/** Fatal UTF-8. ignoreBOM is true: a leading U+FEFF is a character, not a stripped BOM. */
const utf8StrictDecoder = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true })
declare const digest32Brand: unique symbol
type Digest32 = Uint8Array & {
	readonly [digest32Brand]: typeof digest32Brand
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
const HEX_DIGITS = "0123456789abcdef"
function hexNibble(byte: number): number | undefined {
	if (byte >= 0x30 && byte <= 0x39) {
		return byte - 0x30
	}
	if (byte >= 0x61 && byte <= 0x66) {
		return byte - 0x61 + 10
	}
	return undefined
}
function toHex(bytes: Uint8Array): string {
	let out = ""
	for (const byte of bytes) {
		out += HEX_DIGITS[byte >>> 4]
		out += HEX_DIGITS[byte & 0xf]
	}
	return out
}
function fromHex(hex: string): Uint8Array {
	const raw = utf8Encoder.encode(hex)
	if (raw.length % 2 !== 0) {
		throw new LogInputError({ message: `not lowercase hex: ${hex}` })
	}
	const out = new Uint8Array(raw.length / 2)
	for (let i = 0, j = 0; i < raw.length; i += 2, j++) {
		const hiByte = raw[i]
		const loByte = raw[i + 1]
		if (hiByte === undefined || loByte === undefined) {
			throw new LogInputError({ message: `not lowercase hex: ${hex}` })
		}
		const hi = hexNibble(hiByte)
		const lo = hexNibble(loByte)
		if (hi === undefined || lo === undefined) {
			throw new LogInputError({ message: `not lowercase hex: ${hex}` })
		}
		out[j] = (hi << 4) | lo
	}
	return out
}
function digest32(bytes: Uint8Array): Digest32 {
	if (bytes.length !== 32) {
		throw new LogInputError({ message: `digest is not 32 bytes: ${bytes.length}` })
	}
	const out = new Uint8Array(32)
	out.set(bytes)
	return out as Digest32
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

export type { Digest32 }
export {
	bytesEqual,
	checkedAddU64,
	digest32,
	digest32FromHex,
	fromHex,
	hex32,
	saturatingAddU64,
	toHex,
	U64_MAX,
	utf8Encoder,
	utf8StrictDecoder
}
