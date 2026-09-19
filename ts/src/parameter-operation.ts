import { Effect } from "effect"
import { dbNative } from "#db-native.ts"
import { AuthoringError, SdkInvariantError } from "#errors.ts"
import { nativeOperationWith, runtimeHandle } from "#runtime.ts"
import { argumentError } from "#runtime-errors.ts"
import type { OperationHandle } from "#runtime-native.ts"
import { bytesValue } from "#values.ts"

export function parameterName(input: unknown) {
	const bytes = bytesValue("Parameter identity", input, 32)
	if (bytes.length !== 32) throw new AuthoringError({ message: "Parameter identity: expected exactly 32 bytes" })
	return bytes
}
export const parameterOperation = Effect.fn("EventParameter.execute")(function* <A, W>(
	op: string,
	inputs: () => readonly Uint8Array[],
	argument: bigint,
	take: (operation: OperationHandle) => W,
	accept: (wire: W) => A
) {
	const runtime = yield* runtimeHandle()
	const owned = yield* Effect.try({
		try: () => {
			if (typeof argument !== "bigint" || argument < 0n || argument > 255n)
				throw new AuthoringError({ message: "Event parameter: expected a u8 mask" })
			return inputs()
		},
		catch: (cause) => argumentError(`EventParameter.${op}`, cause)
	})
	return yield* nativeOperationWith(
		`EventParameter.${op}`,
		(cb) => dbNative.runtimeEventParameter(runtime, op, owned, argument, cb),
		take,
		accept
	)
})
export function parameterResult<A>(
	op: string,
	inputs: () => readonly Uint8Array[],
	accept: (wire: Uint8Array) => A,
	argument = 0n
) {
	return parameterOperation(op, inputs, argument, (operation) => dbNative.runtimeBytesTake(operation), accept)
}
export function parameterData<A>(op: string, inputs: () => readonly Uint8Array[], accept: (wire: unknown) => A) {
	return parameterOperation(op, inputs, 0n, (operation) => dbNative.runtimeEventParameterTake(operation), accept)
}
export function parameterBoolean(op: string, inputs: () => readonly Uint8Array[]) {
	return parameterOperation(
		op,
		inputs,
		0n,
		(operation) => dbNative.runtimeRowsTake(operation),
		(rows) => {
			if (rows.length !== 1 || rows[0]?.length !== 1 || typeof rows[0][0] !== "boolean")
				throw new SdkInvariantError({ message: "Event parameter: expected a Boolean result" })
			return rows[0][0]
		}
	)
}
export function parameterOrdering(op: string, inputs: () => readonly Uint8Array[]) {
	return parameterOperation(
		op,
		inputs,
		0n,
		(operation) => dbNative.runtimeRowsTake(operation),
		(rows): -1 | 0 | 1 => {
			const value = rows[0]?.[0]
			if (rows.length !== 1 || rows[0]?.length !== 1 || (value !== -1n && value !== 0n && value !== 1n))
				throw new SdkInvariantError({ message: "Event parameter: expected an exact ordering" })
			return Number(value) as -1 | 0 | 1
		}
	)
}
