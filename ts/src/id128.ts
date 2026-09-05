import { randomBytes } from "node:crypto"
import { Effect, Result } from "effect"
import { DbError } from "#runtime-errors.ts"

/**
 * `Id128` — the ordinary application-owned 128-bit identifier: sixteen
 * native bytes, spelled in TypeScript as the canonical 32-character
 * lowercase hexadecimal string (chapter 30's value vocabulary). It is a
 * value, not a capability: it carries no incarnation, decision counter,
 * issuer lease or birth provenance, and the database issues none. There is
 * no `FreshRef`, allocator reservation, fresh map, ID-burn transaction or
 * generated-ID receipt anywhere in the SDK.
 *
 * The host type is a branded string so a random `string` does not typecheck
 * where an `Id128` is required, while the runtime representation stays the
 * idiomatic canonical hex (chapter 35: "canonical hex for Id128"). The
 * checked native boundary revalidates every crossing; the brand is a typing
 * aid, never a trusted proof.
 */
declare const id128Brand: unique symbol

type Id128 = string & { readonly [id128Brand]: "bumbledb.Id128" }

const HEX = "0123456789abcdef"

function isCanonicalHex(text: string): boolean {
	if (text.length !== 32) {
		return false
	}
	for (let index = 0; index < 32; index += 1) {
		const code = text.charCodeAt(index)
		const digit = code >= 48 && code <= 57
		const lower = code >= 97 && code <= 102
		if (!digit && !lower) {
			return false
		}
	}
	return true
}

function invalid(operation: string): DbError {
	return new DbError({ operation, reason: { _tag: "InvalidArgument" } })
}

/**
 * Admits an already-validated canonical hex string at the branded type.
 * Private trusted seam: every caller has just verified `isCanonicalHex`.
 */
function admit(text: string): Id128 {
	return text as Id128
}

function bytesToHex(bytes: Uint8Array): string {
	let out = ""
	for (const byte of bytes) {
		out += HEX[byte >> 4]
		out += HEX[byte & 0x0f]
	}
	return out
}

/**
 * Pure fixed-size parse of the canonical 32-lowercase-hex spelling.
 * Genuinely fallible small parsing uses `Result` (chapter 35); no I/O, no
 * native work, no normalization of alternate spellings — uppercase, dashes
 * (UUID punctuation), `0x` prefixes and wrong widths all refuse.
 */
function fromHex(text: string): Result.Result<Id128, DbError> {
	if (typeof text !== "string" || !isCanonicalHex(text)) {
		return Result.fail(invalid("Id128.fromHex"))
	}
	return Result.succeed(admit(text))
}

/** Pure conversion of sixteen owned bytes to the canonical hex value. */
function fromBytes(bytes: Uint8Array): Result.Result<Id128, DbError> {
	if (!(bytes instanceof Uint8Array) || bytes.length !== 16) {
		return Result.fail(invalid("Id128.fromBytes"))
	}
	return Result.succeed(admit(bytesToHex(bytes)))
}

/** The sixteen owned bytes of a canonical value (a fresh copy each call). */
function toBytes(id: Id128): Uint8Array {
	const out = new Uint8Array(16)
	for (let index = 0; index < 16; index += 1) {
		out[index] = Number.parseInt(id.slice(index * 2, index * 2 + 2), 16)
	}
	return out
}

/**
 * Sixteen cryptographically random bytes as an Effect — effectful entropy,
 * never a module-import side effect and never Effect's noncryptographic
 * test/random service (chapter 35). Generate once for an original intent,
 * persist alongside the request, and never regenerate inside a database
 * retry: a timeout retries the identical sealed command, not a new ID.
 */
const random: () => Effect.Effect<Id128, DbError> = () =>
	Effect.try({
		try: () => admit(bytesToHex(randomBytes(16))),
		catch: (cause) => (cause instanceof DbError ? cause : new DbError({ operation: "Id128.random", reason: { _tag: "Internal" } }))
	})

/** Runtime brand test over the canonical spelling (no allocation). */
function isId128(value: unknown): value is Id128 {
	return typeof value === "string" && isCanonicalHex(value)
}

const Id128 = Object.freeze({
	fromHex,
	fromBytes,
	toBytes,
	random,
	isId128
})

export { Id128 }
