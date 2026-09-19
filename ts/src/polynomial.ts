import { Effect, Result } from "effect"
import { dbNative } from "#db-native.ts"
import { AuthoringError, SdkInvariantError } from "#errors.ts"
import { type ExactRational, encodedRational, rationalBytes } from "#exact-value.ts"
import {
	encodedPolynomial,
	isExactPolynomial,
	polynomialBytes,
	type ExactPolynomial as Value
} from "#polynomial-value.ts"
import { nativeOperationWith, runtimeHandle } from "#runtime.ts"
import { argumentError, type DbError } from "#runtime-errors.ts"
import type { OperationHandle } from "#runtime-native.ts"
import { arrayValue, bytesValue, recordValue } from "#values.ts"

type ExactPolynomial = Value
export interface PolynomialPower {
	readonly parameter: Uint8Array
	readonly exponent: bigint
}
export interface PolynomialTerm {
	readonly coefficient: ExactRational
	readonly powers: readonly PolynomialPower[]
}
export interface ParameterBinding<A> {
	readonly parameter: Uint8Array
	readonly value: A
}
const MAX_TERMS = 65536
const MAX_BYTES = 16 * 1024 * 1024
function name(input: unknown): Uint8Array {
	const bytes = bytesValue("Parameter identity", input, 32)
	if (bytes.length !== 32) throw new AuthoringError({ message: "Parameter identity: expected exactly 32 bytes" })
	return bytes
}
function natural(input: unknown): bigint {
	if (typeof input !== "bigint" || input < 0n || input > 0xffffffffn)
		throw new AuthoringError({ message: "Polynomial exponent: expected a u32 bigint" })
	return input
}
function roster(input: unknown, maximum: number): asserts input is readonly unknown[] {
	if (!Array.isArray(input) || input.length > maximum)
		throw new AuthoringError({ message: "ExactPolynomial: exceeds input roster" })
}
function operands() {
	const values: Uint8Array[] = []
	let remaining = MAX_BYTES
	return {
		values,
		add(bytes: Uint8Array) {
			if (bytes.length > remaining) throw new AuthoringError({ message: "ExactPolynomial: exceeds 16 MiB input" })
			remaining -= bytes.length
			values.push(bytes)
		}
	}
}
const execute = Effect.fn("ExactPolynomial.execute")(function* <A, W>(
	op: string,
	inputs: () => readonly Uint8Array[],
	argument: bigint,
	take: (operation: OperationHandle) => W,
	accept: (wire: W) => A
) {
	const runtime = yield* runtimeHandle()
	const owned = yield* Effect.try({
		try: () => {
			natural(argument)
			return inputs()
		},
		catch: (cause) => argumentError(`ExactPolynomial.${op}`, cause)
	})
	return yield* nativeOperationWith(
		`ExactPolynomial.${op}`,
		(cb) => dbNative.runtimeExactPolynomial(runtime, op, owned, argument, cb),
		take,
		accept
	)
})
function encoded<A>(op: string, inputs: () => readonly Uint8Array[], accept: (bytes: Uint8Array) => A, argument = 0n) {
	return execute(op, inputs, argument, (operation) => dbNative.runtimeBytesTake(operation), accept)
}
function polynomial(op: string, inputs: () => readonly Uint8Array[], argument = 0n) {
	return encoded(op, inputs, encodedPolynomial, argument)
}
function boolean(op: string, inputs: () => readonly Uint8Array[]) {
	return execute(
		op,
		inputs,
		0n,
		(operation) => dbNative.runtimeRowsTake(operation),
		(rows) => {
			if (rows.length !== 1 || rows[0]?.length !== 1 || typeof rows[0][0] !== "boolean")
				throw new SdkInvariantError({ message: "ExactPolynomial: expected a Boolean result" })
			return rows[0][0]
		}
	)
}
function fromBytes(input: Uint8Array): Result.Result<ExactPolynomial, DbError> {
	return Result.try({
		try: () => encodedPolynomial(input),
		catch: (cause) => argumentError("ExactPolynomial.fromBytes", cause)
	})
}
/** Parameter identity is application supplied. Reusing it reuses the unknown;
 * different names do not establish independent random variables or a prior. */
function parameter(identity: Uint8Array) {
	return polynomial("parameter", () => [name(identity)])
}
function constant(value: ExactRational) {
	return polynomial("constant", () => [rationalBytes(value)])
}
/** Normalize unordered/repeated monomials and powers. Every input is checked,
 * including zero coefficients and zero powers. */
function fromTerms(terms: readonly PolynomialTerm[]) {
	return polynomial("new", () => {
		roster(terms, MAX_TERMS)
		const out = operands()
		arrayValue("Polynomial terms", terms, (_, term) => {
			const data = recordValue("PolynomialTerm", term, ["coefficient", "powers"])
			roster(data.powers, 256)
			out.add(rationalBytes(data.coefficient as ExactRational))
			const packed = new Uint8Array(data.powers.length * 36)
			const view = new DataView(packed.buffer)
			let offset = 0
			arrayValue("Polynomial powers", data.powers, (_, power) => {
				const data = recordValue("PolynomialPower", power, ["parameter", "exponent"])
				packed.set(name(data.parameter), offset)
				view.setUint32(offset + 32, Number(natural(data.exponent)), true)
				offset += 36
			})
			out.add(packed)
		})
		return out.values
	})
}
function zero() {
	return fromTerms([])
}
function validate(value: ExactPolynomial) {
	return polynomial("validate", () => [polynomialBytes(value)])
}
function describe(value: ExactPolynomial) {
	return execute(
		"describe",
		() => [polynomialBytes(value)],
		0n,
		(operation) => dbNative.runtimeExactPolynomialTake(operation),
		(input): readonly PolynomialTerm[] =>
			arrayValue("Polynomial terms", input, (_, term) => {
				const data = recordValue("PolynomialTerm", term, ["coefficient", "powers"])
				return Object.freeze({
					coefficient: encodedRational(data.coefficient),
					powers: arrayValue("Polynomial powers", data.powers, (_, power) => {
						const data = recordValue("PolynomialPower", power, ["parameter", "exponent"])
						return Object.freeze({ parameter: name(data.parameter), exponent: natural(data.exponent) })
					})
				})
			})
	)
}
function add(a: ExactPolynomial, b: ExactPolynomial) {
	return polynomial("add", () => [polynomialBytes(a), polynomialBytes(b)])
}
function subtract(a: ExactPolynomial, b: ExactPolynomial) {
	return polynomial("subtract", () => [polynomialBytes(a), polynomialBytes(b)])
}
function multiply(a: ExactPolynomial, b: ExactPolynomial) {
	return polynomial("multiply", () => [polynomialBytes(a), polynomialBytes(b)])
}
function pow(value: ExactPolynomial, exponent: bigint) {
	return polynomial("pow", () => [polynomialBytes(value)], exponent)
}
function bindings<A>(value: ExactPolynomial, entries: readonly ParameterBinding<A>[], bytes: (value: A) => Uint8Array) {
	roster(entries, MAX_TERMS)
	const out = operands()
	out.add(polynomialBytes(value))
	arrayValue("Parameter bindings", entries, (_, entry) => {
		const data = recordValue("ParameterBinding", entry, ["parameter", "value"])
		out.add(name(data.parameter))
		out.add(bytes(data.value as A))
	})
	return out.values
}
/** Simultaneous substitution: replacement expressions are not substituted again. */
function substitute(value: ExactPolynomial, entries: readonly ParameterBinding<ExactPolynomial>[]) {
	return polynomial("substitute", () => bindings(value, entries, polynomialBytes))
}
/** Exact rational evaluation. No source-domain membership is implied. */
function evaluate(value: ExactPolynomial, entries: readonly ParameterBinding<ExactRational>[]) {
	return encoded("evaluate", () => bindings(value, entries, rationalBytes), encodedRational)
}
/** Algebraic moments under an explicit Beta(alpha,beta) prior on full [0,1].
 * This does not integrate a constrained source or invent a prior for its Events.
 * Integrate unnormalized evidence first and divide afterwards. */
function integrateBeta(value: ExactPolynomial, identity: Uint8Array, alpha: ExactRational, beta: ExactRational) {
	return polynomial("integrateBeta", () => [
		polynomialBytes(value),
		name(identity),
		rationalBytes(alpha),
		rationalBytes(beta)
	])
}
/** Polynomial identity, independent of constraints on a parameter domain. */
function equal(a: ExactPolynomial, b: ExactPolynomial) {
	return boolean("equal", () => [polynomialBytes(a), polynomialBytes(b)])
}
function isZero(value: ExactPolynomial) {
	return boolean("isZero", () => [polynomialBytes(value)])
}
const ExactPolynomial = Object.freeze({
	fromBytes,
	toBytes: polynomialBytes,
	isExactPolynomial,
	parameter,
	constant,
	fromTerms,
	zero,
	validate,
	describe,
	add,
	subtract,
	multiply,
	pow,
	substitute,
	evaluate,
	integrateBeta,
	equal,
	isZero
})

export { ExactPolynomial }
