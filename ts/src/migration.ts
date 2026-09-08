/**
 * Private log-migration integration entrypoints: the two read-only
 * native migration-codec verbs the `@bjornpagen/bumbledb-log/migrations`
 * generator imports, wired over the shared executor onto the native
 * `schema_file`, `migration::plan` and `migration::manifest` modules.
 *
 * Both follow `hashChunk`'s shape: bounded owned input, bounded owned JSON
 * response bytes, ONE registered cancellable operation under the acquired
 * `NativeRuntime`. The schema verb takes the SDK `SchemaSpec` object — the
 * same wire that already crosses the bridge for open/create — never a
 * respelled text form. Neither verb opens, initializes, freezes or migrates
 * a database; the durable migrate/activate/abort workflow is the log's
 * admin surface, not this seam.
 */
import { Effect } from "effect"
import { dbNative } from "#db-native.ts"
import { nativeOperationWith, runtimeHandle } from "#runtime.ts"
import { DbError, dbError } from "#runtime-errors.ts"
import type { SchemaSpec } from "#spec.ts"

function owned(operation: string, value: Uint8Array | null): Uint8Array {
	if (value === null) {
		throw dbError(operation, { _tag: "Internal" })
	}
	return value
}

/**
 * Canonical schema admission for the migration generator: validates the
 * spec natively and yields bounded JSON bytes carrying the canonical
 * `schemaId` and the rendered schema snapshot.
 */
const internalMigrationSchema = Effect.fn("internalMigrationSchema")(function* (spec: SchemaSpec) {
	const runtime = yield* runtimeHandle()
	return yield* nativeOperationWith(
		"internalMigrationSchema",
		(callback) => dbNative.runtimeMigrationSchema(runtime, spec, callback),
		dbNative.runtimeBytesTake,
		(bytes) => owned("internalMigrationSchema", bytes)
	)
})

/**
 * One bounded read-only migration-codec request (plan parse/validate/
 * render/digest, manifest verify/append preview, plan-set digests): owned
 * JSON request bytes in, owned JSON response bytes out.
 */
const internalMigrationRead = Effect.fn("internalMigrationRead")(function* (request: Uint8Array) {
	if (!(request instanceof Uint8Array) || !(request.buffer instanceof ArrayBuffer)) {
		return yield* Effect.fail(new DbError({ operation: "internalMigrationRead", reason: { _tag: "InvalidArgument" } }))
	}
	const runtime = yield* runtimeHandle()
	return yield* nativeOperationWith(
		"internalMigrationRead",
		(callback) => dbNative.runtimeMigrationRead(runtime, request, callback),
		dbNative.runtimeBytesTake,
		(bytes) => owned("internalMigrationRead", bytes)
	)
})

export { internalMigrationRead, internalMigrationSchema }
