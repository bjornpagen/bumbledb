/** Owned source transport by native BESC role. Headers grant no certificates. */
import { AuthoringError } from "#errors.ts"
import { bytesValue } from "#values.ts"

const sourceTag: unique symbol = Symbol("bumbledb.Source")
export type SourceKind =
	| "function"
	| "kernel"
	| "revision"
	| "parameterFunction"
	| "familyFunction"
	| "parameterRefinement"
	| "familyKernel"
	| "parameterRestriction"
	| "familyRevision"
	| "betaSource"
export interface SourceValue<K extends SourceKind> {
	readonly [sourceTag]: K
}
const encodings = new WeakMap<SourceValue<SourceKind>, Uint8Array>()
const kinds = {
	function: [1, 0],
	kernel: [1, 1],
	revision: [1, 2],
	parameterFunction: [2, 0],
	familyFunction: [2, 1],
	parameterRefinement: [2, 3],
	familyKernel: [2, 2],
	parameterRestriction: [2, 4],
	familyRevision: [2, 5],
	betaSource: [2, 6]
} as const
export function isSource<K extends SourceKind>(kind: K, input: unknown): input is SourceValue<K> {
	if (typeof input !== "object" || input === null) return false
	const bytes = encodings.get(input as SourceValue<K>)
	return bytes?.[4] === kinds[kind][0] && bytes?.[5] === kinds[kind][1]
}
export function sourceBytes<K extends SourceKind>(kind: K, input: SourceValue<K>): Uint8Array {
	if (!isSource(kind, input)) throw new AuthoringError({ message: `Event source: expected an owned ${kind}` })
	const bytes = encodings.get(input)
	if (bytes === undefined) throw new AuthoringError({ message: "Event source: missing owned bytes" })
	return new Uint8Array(bytes)
}
export function encodedSource<K extends SourceKind>(kind: K, input: unknown): SourceValue<K> {
	const bytes = bytesValue("Event source", input, 16 * 1024 * 1024)
	if (
		bytes.length < 6 ||
		bytes[0] !== 66 ||
		bytes[1] !== 69 ||
		bytes[2] !== 83 ||
		bytes[3] !== 67 ||
		bytes[4] !== kinds[kind][0] ||
		bytes[5] !== kinds[kind][1]
	)
		throw new AuthoringError({ message: `Event source: expected a BESC v${kinds[kind][0]} ${kind} envelope` })
	const value = Object.freeze({ [sourceTag]: kind })
	encodings.set(value, bytes)
	return value
}
