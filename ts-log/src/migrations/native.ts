/**
 * The production `MigrationCodec`: bounded JSON requests to and bounded
 * JSON responses from the shared native runtime. Schema validation, canonical
 * SchemaId + snapshot rendering (`schema_file::{schema_id, render}`), plan
 * parsing/validation/rendering/digesting (`migration::plan`) and manifest
 * verification/appending/plan-set digests (`migration::manifest`) are all
 * native. The durable status/migrate/activate/abort workflow lives in
 * `#migration-ops.ts`, reached through the `logAdmin` wire verb.
 *
 * The two imported core entrypoints accept bounded owned input and JSON
 * response bytes under one registered cancellable operation. The schema verb
 * takes the SDK `SchemaSpec` object because that wire already crosses the
 * bridge for open/create; it is never respelled as text here. Both are
 * read-only: they never open, initialize, freeze or migrate a database.
 */

import type { SchemaSpec } from "@bjornpagen/bumbledb"
import { internalMigrationRead, internalMigrationSchema } from "@bjornpagen/bumbledb/internal/log"
import { Effect, Schema } from "effect"
import type { LogError } from "#errors.ts"
import { logFailure } from "#errors.ts"
import type { JsonValue } from "#migrations/canonical.ts"
import { compactJson } from "#migrations/canonical.ts"
import type { ChainRequest, MigrationCodec, SchemaIdentity } from "#migrations/codec.ts"
import { boundedDetail, repository } from "#migrations/fail.ts"

const MAX_RESPONSE = 64 * 1024 * 1024

const encoder = new TextEncoder()
const decoder = new TextDecoder("utf-8", { fatal: true })

// ---------------------------------------------------------------------------
// Response boundary models (small external models — Effect Schema).
// ---------------------------------------------------------------------------

const RefusalWire = Schema.Struct({
	refused: Schema.Struct({
		code: Schema.String,
		detail: Schema.String
	})
})

const SchemaOk = Schema.Struct({
	schemaId: Schema.String,
	snapshot: Schema.String
})
const SchemaResponse = Schema.Union([SchemaOk, RefusalWire])

const EntryWire = Schema.Struct({
	sequence: Schema.String,
	id: Schema.String,
	fromSchemaId: Schema.String,
	toSchemaId: Schema.String,
	planDigest: Schema.String,
	prefixDigest: Schema.String
})

const ChainOk = Schema.Struct({
	headPrefixDigest: Schema.String,
	planSetDigest: Schema.NullOr(Schema.String),
	appended: Schema.NullOr(
		Schema.Struct({
			entry: EntryWire,
			planText: Schema.String,
			manifestText: Schema.String
		})
	)
})
const ChainResponse = Schema.Union([ChainOk, RefusalWire])

const decodeSchemaResponse = Schema.decodeUnknownOption(SchemaResponse)
const decodeChainResponse = Schema.decodeUnknownOption(ChainResponse)

/**
 * A native refusal code becomes the typed log error through the shared wire
 * decoder: known protocol codes become `ProtocolError`, anything else the
 * core `Internal` — never a fabricated success and never string matching.
 */
export function nativeRefusal(operation: string, code: string, detail: string): LogError {
	return logFailure(operation, {
		source: "protocol",
		reason: { _tag: code, detail: boundedDetail(detail) }
	})
}

// ---------------------------------------------------------------------------
// Request plumbing.
// ---------------------------------------------------------------------------

function requestBytes(body: JsonValue): Uint8Array {
	return encoder.encode(compactJson(body))
}

function decodeResponse(operation: string, bytes: Uint8Array): Effect.Effect<unknown, LogError> {
	if (bytes.byteLength > MAX_RESPONSE) {
		return Effect.fail(repository(operation, "<native response>", "response exceeds the bounded response cap"))
	}
	return Effect.try({
		try: () => JSON.parse(decoder.decode(bytes)) as unknown,
		catch: () => repository(operation, "<native response>", "response is not bounded JSON")
	})
}

// ---------------------------------------------------------------------------
// The production codec.
// ---------------------------------------------------------------------------

const schemaIdentity = Effect.fn("bumbledb-log.migrations.schemaIdentity")(function* (spec: SchemaSpec) {
	const operation = "migrations.schemaIdentity"
	const raw = yield* internalMigrationSchema(spec)
	const response = yield* decodeResponse(operation, raw)
	const decoded = decodeSchemaResponse(response)
	if (decoded._tag === "None") {
		return yield* Effect.fail(repository(operation, "<native response>", "unrecognized schema response"))
	}
	if ("refused" in decoded.value) {
		return yield* Effect.fail(nativeRefusal(operation, decoded.value.refused.code, decoded.value.refused.detail))
	}
	const identity: SchemaIdentity = { schemaId: decoded.value.schemaId, snapshot: decoded.value.snapshot }
	return identity
})

const verifyChain = Effect.fn("bumbledb-log.migrations.verifyChain")(function* (request: ChainRequest) {
	const operation = "migrations.verifyChain"
	const raw = yield* internalMigrationRead(
		requestBytes({
			kind: "chain",
			manifest: request.manifest,
			baseSchemaId: request.baseSchemaId,
			snapshots: request.snapshots,
			plans: request.plans,
			append: request.append,
			planSet: request.planSet === null ? null : { first: request.planSet.first, count: request.planSet.count }
		})
	)
	const response = yield* decodeResponse(operation, raw)
	const decoded = decodeChainResponse(response)
	if (decoded._tag === "None") {
		return yield* Effect.fail(repository(operation, "<native response>", "unrecognized chain response"))
	}
	if ("refused" in decoded.value) {
		return yield* Effect.fail(nativeRefusal(operation, decoded.value.refused.code, decoded.value.refused.detail))
	}
	return decoded.value
})

/** The one production binding of the generator's native-codec seam. */
export const productionCodec: MigrationCodec = { schemaIdentity, verifyChain }
