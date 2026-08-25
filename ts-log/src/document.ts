/**
 * Canonical single-line document grammar shared by the manifest,
 * checkpoint, and sidecar. Every numeric field is a decimal u64
 * (bigint); every digest is 32 lowercase-hex bytes; pending bytes
 * are lowercase hex. A leading BOM, whitespace, leading zeros, or
 * a JSON number path are not in the language.
 */

import type { Digest32 } from "#bytes.ts"
import { digest32, fromHex, U64_MAX, utf8Encoder } from "#bytes.ts"

const DOC_VERSION = 3n

class Text {
	private readonly bytes: Uint8Array
	private at = 0

	constructor(bytes: Uint8Array) {
		this.bytes = bytes
	}

	offset(): number {
		return this.at
	}

	lit(expected: string): boolean {
		const want = utf8Encoder.encode(expected)
		const end = this.at + want.length
		if (end > this.bytes.length) {
			return false
		}
		for (let i = 0; i < want.length; i++) {
			if (this.bytes[this.at + i] !== want[i]) {
				return false
			}
		}
		this.at = end
		return true
	}

	peek(expected: string): boolean {
		const want = utf8Encoder.encode(expected)
		const end = this.at + want.length
		if (end > this.bytes.length) {
			return false
		}
		for (let i = 0; i < want.length; i++) {
			if (this.bytes[this.at + i] !== want[i]) {
				return false
			}
		}
		return true
	}

	private hexNibble(): number | undefined {
		const byte = this.bytes[this.at]
		if (byte === undefined) {
			return undefined
		}
		if (byte >= 0x30 && byte <= 0x39) {
			this.at += 1
			return byte - 0x30
		}
		if (byte >= 0x61 && byte <= 0x66) {
			this.at += 1
			return byte - 0x61 + 10
		}
		return undefined
	}

	hex32(): Digest32 | undefined {
		const out = new Uint8Array(32)
		for (let i = 0; i < 32; i++) {
			const hi = this.hexNibble()
			const lo = this.hexNibble()
			if (hi === undefined || lo === undefined) {
				return undefined
			}
			out[i] = (hi << 4) | lo
		}
		return digest32(out)
	}

	hexBytes(): Uint8Array | undefined {
		const out: number[] = []
		while (!this.peek('"')) {
			const hi = this.hexNibble()
			const lo = this.hexNibble()
			if (hi === undefined || lo === undefined) {
				return undefined
			}
			out.push((hi << 4) | lo)
		}
		return new Uint8Array(out)
	}

	/** Canonical JSON u64: decimal digits, no leading zero unless the value is zero. */
	u64(): bigint | undefined {
		const start = this.at
		let value = 0n
		while (this.at < this.bytes.length) {
			const byte = this.bytes[this.at]
			if (byte === undefined || byte < 0x30 || byte > 0x39) {
				break
			}
			const next = value * 10n + BigInt(byte - 0x30)
			if (next > U64_MAX) {
				return undefined
			}
			value = next
			this.at += 1
		}
		const len = this.at - start
		if (len === 0 || (len > 1 && this.bytes[start] === 0x30)) {
			return undefined
		}
		return value
	}

	/** A u64 that is not the `v` discriminator: a quoted decimal string. */
	quotedU64(): bigint | undefined {
		if (!this.lit('"')) {
			return undefined
		}
		const value = this.u64()
		if (value === undefined || !this.lit('"')) {
			return undefined
		}
		return value
	}

	hexU32(): number | undefined {
		let out = 0
		for (let i = 0; i < 8; i++) {
			const nibble = this.hexNibble()
			if (nibble === undefined) {
				return undefined
			}
			out = (out << 4) | nibble
		}
		return out >>> 0
	}

	finished(): boolean {
		return this.at === this.bytes.length
	}
}

function hexOf(bytes: Uint8Array): string {
	return [...bytes].map((byte) => byte.toString(16).padStart(2, "0")).join("")
}

function pendingHex(bytes: Uint8Array): string {
	return hexOf(bytes)
}

function pendingFromHex(hex: string): Uint8Array {
	return fromHex(hex)
}

export { DOC_VERSION, pendingFromHex, pendingHex, Text }
