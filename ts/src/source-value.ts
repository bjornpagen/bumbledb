/** Owned source transport by native BESC role. Headers grant no certificates. */
import { AuthoringError } from "#errors.ts"
import { bytesValue } from "#values.ts"

const sourceTag: unique symbol = Symbol("bumbledb.Source")
export type SourceKind = "function" | "kernel" | "revision"
export interface SourceValue<K extends SourceKind> {
	readonly [sourceTag]: K
}
const encodings = new WeakMap<SourceValue<SourceKind>, Uint8Array>()
const kinds = { function: 0, kernel: 1, revision: 2 } as const
export function isSource<K extends SourceKind>(kind: K, input: unknown): input is SourceValue<K> {
	return typeof input === "object" && input !== null && encodings.get(input as SourceValue<K>)?.[5] === kinds[kind]
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
		bytes[4] !== 1 ||
		bytes[5] !== kinds[kind]
	)
		throw new AuthoringError({ message: `Event source: expected a BESC v1 ${kind} envelope` })
	const value = Object.freeze({ [sourceTag]: kind })
	encodings.set(value, bytes)
	return value
}
