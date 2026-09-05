import { Schema } from "effect"
import { runtimeErrorCodes } from "#runtime-codes.ts"

export { runtimeErrorCodes } from "#runtime-codes.ts"

const ResourceLimit = Schema.Struct({
	_tag: Schema.Literal("ResourceLimit"),
	dimension: Schema.String,
	used: Schema.BigInt,
	requested: Schema.BigInt,
	limit: Schema.BigInt
})
const PlainReason = Schema.Struct({
	_tag: Schema.Literals(
		runtimeErrorCodes.filter((code) => code !== "ResourceLimit" && code !== "Io" && code !== "Engine")
	)
})
const Io = Schema.Struct({
	_tag: Schema.Literal("Io"),
	kind: Schema.String,
	osCode: Schema.optional(Schema.Number)
})
/** A typed engine refusal crossing the executor: core family tag + message. */
const Engine = Schema.Struct({
	_tag: Schema.Literal("Engine"),
	kind: Schema.String,
	message: Schema.String
})
export const DbReason = Schema.Union([ResourceLimit, Io, Engine, PlainReason])
export class DbError extends Schema.TaggedError<DbError>()("DbError", {
	operation: Schema.String,
	reason: DbReason
}) {
	get code() {
		return this.reason._tag
	}
}

const decodeReason = Schema.decodeUnknownOption(DbReason)
export function dbError(operation: string, cause: unknown): DbError {
	const decoded = decodeReason(cause)
	return new DbError({ operation, reason: decoded._tag === "Some" ? decoded.value : { _tag: "Internal" } })
}

const Outstanding = Schema.Struct({
	phase: Schema.Literals(["open", "closing", "closed"]),
	queued: Schema.BigInt,
	active: Schema.BigInt,
	retained: Schema.BigInt,
	owners: Schema.BigInt,
	databases: Schema.BigInt,
	inputBytes: Schema.BigInt,
	workingBytes: Schema.BigInt,
	scratchBytes: Schema.BigInt,
	resultBytes: Schema.BigInt
})
export type OutstandingWork = typeof Outstanding.Type
const Close = Schema.Union([
	Schema.Struct({ kind: Schema.Literal("closed") }),
	Schema.Struct({ kind: Schema.Literal("incomplete"), outstanding: Outstanding }),
	Schema.Struct({ kind: Schema.Literal("failed"), error: DbError })
])
export type CloseReport = typeof Close.Type

export class CloseFailure extends Schema.TaggedError<CloseFailure>()("CloseFailure", {
	operation: Schema.String,
	report: Close
}) {}
