/**
 * The one production machine instance: the real addon wire plus the core's
 * internal integration seam (C10). `internalPublishedReader` wraps a
 * published core snapshot handle in the exact core `QueryReader`;
 * `internalChanges` is the core's landed private ChangeSet registry
 * accessor (the retained native change — no re-marshaled rows); `lower` is
 * the same core schema lowering `Db.open` admits; `runtimeHandle` captures
 * the already-acquired shared `NativeRuntime`. All four are core-owned
 * (P07/P06) and imported literally — the log mirrors none of them.
 */
import {
	internalChanges,
	internalMigrationRead,
	internalMigrationSchema,
	internalPublishedReader,
	lower,
	runtimeHandle
} from "@bjornpagen/bumbledb/internal/log"
import type { Capability, ChangeSet, QueryReader } from "@bjornpagen/bumbledb/internal/log"
import type { AnySchema } from "@bjornpagen/bumbledb"
import type { CoreIntegration, PublishedReadCapability } from "#machine.ts"
import { makeLogMachine } from "#machine.ts"
import type { CoreSnapshotHandle } from "#native.ts"
import { logNative } from "#native.ts"

const coreIntegration: CoreIntegration = {
	reader<S extends AnySchema>(
		core: CoreSnapshotHandle | Capability,
		schema: S
	): QueryReader<S> & PublishedReadCapability<S> {
		return internalPublishedReader(core, schema)
	},
	changes(value: ChangeSet<AnySchema> | object) {
		return internalChanges(value)
	},
	schemaSpec(schema: AnySchema) {
		return lower(schema)
	},
	runtime() {
		return runtimeHandle()
	}
}

export const log = makeLogMachine(logNative, coreIntegration)
