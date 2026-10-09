/**
 * The embedded engine: one `Db` over a local LMDB directory, for single-process use, tests and
 * benches. Applications that want the durable log use `Database` from the package root.
 */
export type {
	ApplyOutcome,
	CoreWitness,
	DbInspection,
	JudgeOutcome,
	PreparedQuery,
	QueryReader,
	Snapshot,
	StorageInspection,
	WriteExpected,
	WriteOptions
} from "./db.ts"
export { Db } from "./db.ts"
