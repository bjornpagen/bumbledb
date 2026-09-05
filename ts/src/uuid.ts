import { Result } from "effect"
import { DbError } from "#runtime-errors.ts"

/** Structural UUID spelling, compatible with host generators and ordinary literals.
 * Exact hexadecimal width and canonical case are checked at runtime boundaries.
 */
type Uuid = `${string}-${string}-${string}-${string}-${string}`

const SYNTAX = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/
const HEX = "0123456789abcdef"

function invalid(operation: string): DbError {
	return new DbError({ operation, reason: { _tag: "InvalidArgument" } })
}

/** Pure parsing: normalize hexadecimal case, never inspect UUID version or variant. */
function parse(text: string): Result.Result<Uuid, DbError> {
	if (typeof text !== "string" || text.length !== 36) {
		return Result.fail(invalid("Uuid.parse"))
	}
	const canonical = text.toLowerCase()
	return isUuid(canonical) ? Result.succeed(canonical) : Result.fail(invalid("Uuid.parse"))
}

/** Canonical wire validation. No normalization is permitted at the native boundary. */
function isUuid(value: unknown): value is Uuid {
	return typeof value === "string" && value.length === 36 && SYNTAX.test(value)
}

/** All sixteen-byte payloads are representable, including Nil, Max, and future versions. */
function fromBytes(bytes: Uint8Array): Result.Result<Uuid, DbError> {
	if (!(bytes instanceof Uint8Array) || bytes.length !== 16) {
		return Result.fail(invalid("Uuid.fromBytes"))
	}
	let text = ""
	for (const byte of bytes) {
		text += HEX.charAt(byte >>> 4)
		text += HEX.charAt(byte & 15)
	}
	return Result.succeed<Uuid>(
		`${text.slice(0, 8)}-${text.slice(8, 12)}-${text.slice(12, 16)}-${text.slice(16, 20)}-${text.slice(20)}`
	)
}

function digit(code: number): number {
	return code <= 57 ? code - 48 : code - 87
}

/** Validate the structural string before decoding fresh bytes in standard UUID order. */
function toBytes(value: Uuid): Result.Result<Uint8Array, DbError> {
	if (!isUuid(value)) {
		return Result.fail(invalid("Uuid.toBytes"))
	}
	const bytes = new Uint8Array(16)
	let offset = 0
	for (let index = 0; index < 16; index += 1) {
		if (value.charCodeAt(offset) === 45) {
			offset += 1
		}
		bytes[index] = (digit(value.charCodeAt(offset)) << 4) | digit(value.charCodeAt(offset + 1))
		offset += 2
	}
	return Result.succeed(bytes)
}

/** Generate UUIDs with a host library; preserve the result across database retries. */
const Uuid = Object.freeze({ parse, fromBytes, toBytes, isUuid })

export { Uuid }
