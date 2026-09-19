import { Result } from "effect"
import { AlgebraicRoot } from "#algebraic-root.ts"
import { SdkInvariantError } from "#errors.ts"
import { type ExactRational, encodedRational, rationalBytes } from "#exact-value.ts"
import { parameterBoolean, parameterData, parameterName, parameterResult } from "#parameter-operation.ts"
import { encodedParameter, isParameter, type ParameterValue, parameterBytes } from "#parameter-value.ts"
import { type ExactPolynomial, polynomialBytes } from "#polynomial-value.ts"
import { argumentError, type DbError } from "#runtime-errors.ts"
import { arrayValue, recordValue } from "#values.ts"

type ParameterRegion = ParameterValue<"region">
type ParameterDomain = ParameterValue<"domain">
/** Independent negative/zero/positive bits; all eight masks are supported. */
export const PolynomialSigns = Object.freeze({
	none: 0n,
	negative: 1n,
	zero: 2n,
	nonPositive: 3n,
	positive: 4n,
	nonZero: 5n,
	nonNegative: 6n,
	any: 7n
})
export type RealWitness =
	| { readonly kind: "rational"; readonly value: ExactRational }
	| { readonly kind: "algebraic"; readonly value: AlgebraicRoot }
export interface ParameterRegionDescription {
	readonly parameter: Uint8Array
	readonly boundaries: readonly AlgebraicRoot[]
	/** Alternating open sectors and boundary points, length 2 * boundaries + 1.
	 * First/last sectors extend to negative/positive infinity. */
	readonly membership: readonly boolean[]
}
const encode = (input: unknown) => encodedParameter("region", input)
const toBytes = (value: ParameterRegion) => parameterBytes("region", value)
const isParameterRegion = (input: unknown): input is ParameterRegion => isParameter("region", input)
const encodeDomain = (input: unknown) => encodedParameter("domain", input)
const domainBytes = (value: ParameterDomain) => parameterBytes("domain", value)
const isParameterDomain = (input: unknown): input is ParameterDomain => isParameter("domain", input)
function fromBytes(input: Uint8Array): Result.Result<ParameterRegion, DbError> {
	return Result.try({ try: () => encode(input), catch: (cause) => argumentError("ParameterRegion.fromBytes", cause) })
}
function domainFromBytes(input: Uint8Array): Result.Result<ParameterDomain, DbError> {
	return Result.try({
		try: () => encodeDomain(input),
		catch: (cause) => argumentError("ParameterDomain.fromBytes", cause)
	})
}
function full(parameter: Uint8Array) {
	return parameterResult("region.full", () => [parameterName(parameter)], encode)
}
function empty(parameter: Uint8Array) {
	return parameterResult("region.empty", () => [parameterName(parameter)], encode)
}
/** Solve over the whole real line. Other occurring parameter names refuse. */
function whereSign(parameter: Uint8Array, polynomial: ExactPolynomial, signs: bigint) {
	return parameterResult("region.sign", () => [parameterName(parameter), polynomialBytes(polynomial)], encode, signs)
}
function validate(value: ParameterRegion) {
	return parameterResult("region.validate", () => [toBytes(value)], encode)
}
/** Bit ((a << 1) | b), as with Event.apply. Both captured names must agree. */
function apply(mask: bigint, a: ParameterRegion, b: ParameterRegion) {
	return parameterResult("region.apply", () => [toBytes(a), toBytes(b)], encode, mask)
}
function and(a: ParameterRegion, b: ParameterRegion) {
	return apply(8n, a, b)
}
function or(a: ParameterRegion, b: ParameterRegion) {
	return apply(14n, a, b)
}
function xor(a: ParameterRegion, b: ParameterRegion) {
	return apply(6n, a, b)
}
function difference(a: ParameterRegion, b: ParameterRegion) {
	return apply(4n, a, b)
}
function complement(value: ParameterRegion) {
	return parameterResult("region.complement", () => [toBytes(value)], encode)
}
function equivalent(a: ParameterRegion, b: ParameterRegion) {
	return parameterBoolean("region.equivalent", () => [toBytes(a), toBytes(b)])
}
function included(a: ParameterRegion, b: ParameterRegion) {
	return parameterBoolean("region.included", () => [toBytes(a), toBytes(b)])
}
function isEmpty(value: ParameterRegion) {
	return parameterBoolean("region.isEmpty", () => [toBytes(value)])
}
function isFull(value: ParameterRegion) {
	return parameterBoolean("region.isFull", () => [toBytes(value)])
}
function containsRational(value: ParameterRegion, point: ExactRational) {
	return parameterBoolean("region.containsRational", () => [toBytes(value), rationalBytes(point)])
}
function containsRoot(value: ParameterRegion, point: AlgebraicRoot) {
	return parameterBoolean("region.containsRoot", () => [toBytes(value), AlgebraicRoot.toBytes(point)])
}
function witness(value: ParameterRegion) {
	return parameterData(
		"region.witness",
		() => [toBytes(value)],
		(input): RealWitness | null => {
			const data = recordValue("Parameter witness", input, ["witness"])
			if (data.witness === null) return null
			const witness = recordValue("RealWitness", data.witness, ["kind", "value"])
			if (witness.kind === "rational") return Object.freeze({ kind: "rational", value: encodedRational(witness.value) })
			if (witness.kind === "algebraic")
				return Object.freeze({ kind: "algebraic", value: encodedParameter("root", witness.value) })
			throw new SdkInvariantError({ message: "Parameter witness: unexpected kind" })
		}
	)
}
function describe(value: ParameterRegion) {
	return parameterData(
		"region.describe",
		() => [toBytes(value)],
		(input): ParameterRegionDescription => {
			const data = recordValue("ParameterRegion description", input, ["parameter", "boundaries", "membership"])
			const boundaries = arrayValue("Region boundaries", data.boundaries, (_, root) => encodedParameter("root", root))
			const membership = arrayValue("Region membership", data.membership, (_, member) => {
				if (typeof member !== "boolean") throw new SdkInvariantError({ message: "Region membership: expected Boolean" })
				return member
			})
			if (membership.length !== 2 * boundaries.length + 1)
				throw new SdkInvariantError({ message: "Region membership: invalid extent" })
			return Object.freeze({ parameter: parameterName(data.parameter), boundaries, membership })
		}
	)
}
/** Requires a mathematically nonempty region. It does not add a measure. */
function domain(region: ParameterRegion) {
	return parameterResult("region.domain", () => [toBytes(region)], encodeDomain)
}
function validateDomain(value: ParameterDomain) {
	return parameterResult("region.domain", () => [domainBytes(value)], encodeDomain)
}
/** Rechecks inhabitation, including for a pure imported envelope. */
function asRegion(value: ParameterDomain) {
	return parameterResult("region.domain", () => [domainBytes(value)], encode)
}
const ParameterRegion = Object.freeze({
	fromBytes,
	toBytes,
	isParameterRegion,
	full,
	empty,
	whereSign,
	validate,
	apply,
	and,
	or,
	xor,
	difference,
	complement,
	equivalent,
	included,
	isEmpty,
	isFull,
	containsRational,
	containsRoot,
	witness,
	describe
})
const ParameterDomain = Object.freeze({
	fromBytes: domainFromBytes,
	toBytes: domainBytes,
	isParameterDomain,
	new: domain,
	validate: validateDomain,
	asRegion
})

export { ParameterDomain, ParameterRegion }
