/** Owned BEPR/BEAR envelopes. Pure imports confer no mathematical certificate. */
import { AuthoringError } from "#errors.ts"
import { bytesValue } from "#values.ts"

const parameterTag: unique symbol = Symbol("bumbledb.Parameter")
type Kind = "region" | "domain" | "root"
export interface ParameterValue<K extends Kind> {
	readonly [parameterTag]: K
}
const encodings = new WeakMap<ParameterValue<Kind>, { kind: Kind; bytes: Uint8Array }>()
export function isParameter<K extends Kind>(kind: K, input: unknown): input is ParameterValue<K> {
	return typeof input === "object" && input !== null && encodings.get(input as ParameterValue<K>)?.kind === kind
}
export function parameterBytes<K extends Kind>(kind: K, value: ParameterValue<K>): Uint8Array {
	if (!isParameter(kind, value)) throw new AuthoringError({ message: `Event parameter: expected an owned ${kind}` })
	const data = encodings.get(value)
	if (data === undefined) throw new AuthoringError({ message: "Event parameter: missing owned bytes" })
	return new Uint8Array(data.bytes)
}
export function encodedParameter<K extends Kind>(kind: K, input: unknown): ParameterValue<K> {
	const bytes = bytesValue("Event parameter", input, 16 * 1024 * 1024)
	const magic = kind === "root" ? [66, 69, 65, 82] : [66, 69, 80, 82]
	if (bytes.length < 5 || magic.some((byte, i) => bytes[i] !== byte) || bytes[4] !== 1)
		throw new AuthoringError({
			message: `Event parameter: expected a ${kind === "root" ? "BEAR" : "BEPR"} v1 envelope`
		})
	const value = Object.freeze({ [parameterTag]: kind })
	encodings.set(value, { kind, bytes })
	return value
}
