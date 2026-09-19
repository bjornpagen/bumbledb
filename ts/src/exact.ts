import { Effect, Result } from "effect"
import { dbNative } from "#db-native.ts"
import { AuthoringError, SdkInvariantError } from "#errors.ts"
import { encodedRational, isExactRational, rationalBytes, type ExactRational as Value } from "#exact-value.ts"
import { nativeOperationWith, runtimeHandle } from "#runtime.ts"
import { argumentError, type DbError } from "#runtime-errors.ts"
import type { OperationHandle } from "#runtime-native.ts"

type ExactRational = Value
function fromBytes(input: Uint8Array): Result.Result<ExactRational, DbError> {
	return Result.try({
		try: () => encodedRational(input),
		catch: (cause) => argumentError("ExactRational.fromBytes", cause)
	})
}
function text(input: unknown): Uint8Array {
	if (typeof input !== "string" || input.length > 65536 || !input.isWellFormed())
		throw new AuthoringError({ message: "ExactRational: expected bounded well-formed numeric text" })
	return new TextEncoder().encode(input)
}
function integerText(value: bigint | string): Uint8Array {
	return text(typeof value === "bigint" ? value.toString() : value)
}
const execute = Effect.fn("ExactRational.execute")(function* <A, W>(
	op: string,
	inputs: () => readonly Uint8Array[],
	argument: number,
	take: (operation: OperationHandle) => W,
	accept: (wire: W) => A
) {
	const runtime = yield* runtimeHandle()
	const owned = yield* Effect.try({ try: inputs, catch: (cause) => argumentError(`ExactRational.${op}`, cause) })
	return yield* nativeOperationWith(
		`ExactRational.${op}`,
		(cb) => dbNative.runtimeExactRational(runtime, op, owned, argument, cb),
		take,
		accept
	)
})
function encoded(op: string, inputs: () => readonly Uint8Array[], argument = 0) {
	return execute(op, inputs, argument, (operation) => dbNative.runtimeBytesTake(operation), encodedRational)
}
function scalar(op: string, inputs: readonly ExactRational[]) {
	return execute(
		op,
		() => inputs.map(rationalBytes),
		0,
		(operation) => dbNative.runtimeRowsTake(operation),
		(rows) => {
			if (rows.length !== 1 || rows[0]?.length !== 1)
				throw new SdkInvariantError({ message: "ExactRational: invalid scalar result" })
			return rows[0][0]
		}
	)
}
function bool(op: string, inputs: readonly ExactRational[]) {
	return Effect.map(scalar(op, inputs), (value) => {
		if (typeof value !== "boolean") throw new SdkInvariantError({ message: "ExactRational: expected Boolean result" })
		return value
	})
}
function fraction(numerator: bigint | string, denominator: bigint | string = 1n) {
	return encoded("fraction", () => [integerText(numerator), integerText(denominator)])
}
function decimal(value: string) {
	return encoded("decimal", () => [text(value)])
}
/** Exact IEEE binary64 value. Use decimal for an authored decimal instead. */
function binary64(value: number) {
	return encoded(
		"binary64",
		() => {
			if (typeof value !== "number") throw new AuthoringError({ message: "ExactRational.binary64: expected a number" })
			return []
		},
		value
	)
}
function validate(value: ExactRational) {
	return encoded("validate", () => [rationalBytes(value)])
}
function textResult(value: ExactRational) {
	return Effect.map(scalar("text", [value]), (text) => {
		if (typeof text !== "string") throw new SdkInvariantError({ message: "ExactRational: expected text result" })
		return text
	})
}
function add(a: ExactRational, b: ExactRational) {
	return encoded("add", () => [rationalBytes(a), rationalBytes(b)])
}
function subtract(a: ExactRational, b: ExactRational) {
	return encoded("subtract", () => [rationalBytes(a), rationalBytes(b)])
}
function multiply(a: ExactRational, b: ExactRational) {
	return encoded("multiply", () => [rationalBytes(a), rationalBytes(b)])
}
function divide(a: ExactRational, b: ExactRational) {
	return encoded("divide", () => [rationalBytes(a), rationalBytes(b)])
}
function equal(a: ExactRational, b: ExactRational) {
	return bool("equal", [a, b])
}
function compare(a: ExactRational, b: ExactRational) {
	return Effect.map(scalar("compare", [a, b]), (value): -1 | 0 | 1 => {
		if (value === -1n) return -1
		if (value === 0n) return 0
		if (value === 1n) return 1
		throw new SdkInvariantError({ message: "ExactRational: invalid comparison" })
	})
}
function isZero(value: ExactRational) {
	return bool("isZero", [value])
}
function isNegative(value: ExactRational) {
	return bool("isNegative", [value])
}
function isProbability(value: ExactRational) {
	return bool("isProbability", [value])
}
const ExactRational = Object.freeze({
	fromBytes,
	toBytes: rationalBytes,
	isExactRational,
	fraction,
	decimal,
	binary64,
	validate,
	toString: textResult,
	add,
	subtract,
	multiply,
	divide,
	equal,
	compare,
	isZero,
	isNegative,
	isProbability
})

export { ExactRational }
