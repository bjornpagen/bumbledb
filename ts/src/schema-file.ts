import { Effect } from "effect"
import { dbNative } from "#db-native.ts"
import { nativeOperationWith, runtimeHandle } from "#runtime.ts"
import { DbError, dbError } from "#runtime-errors.ts"
import type { SchemaSpec } from "#spec.ts"

const decoder = new TextDecoder("utf-8", { fatal: true })

function text(operation: string, bytes: Uint8Array | null): string {
	if (bytes === null) throw dbError(operation, { _tag: "Internal" })
	return decoder.decode(bytes)
}

const internalSchemaSnapshot = Effect.fn("internalSchemaSnapshot")(function* (spec: SchemaSpec) {
	const runtime = yield* runtimeHandle()
	return yield* nativeOperationWith(
		"internalSchemaSnapshot",
		(callback) => dbNative.runtimeSchemaSnapshot(runtime, spec, callback),
		dbNative.runtimeBytesTake,
		(bytes) => text("internalSchemaSnapshot", bytes)
	)
})

const internalSchemaBindings = Effect.fn("internalSchemaBindings")(function* (snapshot: string) {
	if (typeof snapshot !== "string" || !snapshot.isWellFormed()) {
		return yield* Effect.fail(new DbError({ operation: "internalSchemaBindings", reason: { _tag: "InvalidArgument" } }))
	}
	const runtime = yield* runtimeHandle()
	const bytes = new TextEncoder().encode(snapshot)
	return yield* nativeOperationWith(
		"internalSchemaBindings",
		(callback) => dbNative.runtimeSchemaBindings(runtime, bytes, callback),
		dbNative.runtimeBytesTake,
		(bytes) => text("internalSchemaBindings", bytes)
	)
})

export { internalSchemaBindings, internalSchemaSnapshot }
