import type { AnySchema } from "@bjornpagen/bumbledb"
import { internalSchemaBindings, internalSchemaSnapshot, lower } from "@bjornpagen/bumbledb/internal/log"
import { Effect } from "effect"

/** Admit an authored schema and render its canonical native snapshot. */
export const schemaSnapshot = Effect.fn("bumbledb-log.schemaSnapshot")(function* (schema: AnySchema) {
	return yield* internalSchemaSnapshot(lower(schema))
})

/** Emit ordinary, typed SDK declarations from one independently retained snapshot. */
export const schemaBindings = internalSchemaBindings
