/**
 * The SDK's errors. Authoring refusals throw synchronously while a schema, query or value is
 * built; everything that runs as an Effect fails with `DbError { operation, reason }`; a native
 * close that does not drain cleanly is a `CloseFailure` defect.
 */
import { Schema } from "effect"

/** Where an authoring refusal points and what was expected there. */
export const AuthoringDiagnostic = Schema.Struct({
	code: Schema.Literals([
		"InvalidValue",
		"InvalidRecord",
		"UnknownField",
		"MissingField",
		"InvalidDeclaration",
		"InvalidQuery"
	]),
	context: Schema.String,
	expected: Schema.String
})
export type AuthoringDiagnostic = typeof AuthoringDiagnostic.Type

/** A schema, query, parameter or value refused while it is being authored. */
export class AuthoringError extends Schema.TaggedError<AuthoringError>()("AuthoringError", {
	message: Schema.String,
	diagnostic: Schema.optional(AuthoringDiagnostic)
}) {}

/** The native addon for this host could not be loaded. */
export class NativeLoadError extends Schema.TaggedError<NativeLoadError>()("NativeLoadError", {
	target: Schema.String,
	message: Schema.String,
	cause: Schema.Unknown
}) {}

const PlainReason = Schema.Struct({
	_tag: Schema.Literals([
		"RuntimeAlreadyLive",
		"ForeignRuntime",
		"ClosedHandle",
		"HandleBusy",
		"SpentHandle",
		"QueueFull",
		"Internal",
		"DirectoryBusy",
		"WriterBusy",
		"InvalidPath",
		"Cancelled",
		"OutOfMemory"
	])
})
const InvalidArgument = Schema.Struct({
	_tag: Schema.Literal("InvalidArgument"),
	detail: Schema.optional(
		Schema.Struct({
			message: Schema.String,
			diagnostic: Schema.optional(AuthoringDiagnostic)
		})
	)
})
const InvalidValue = Schema.Struct({ _tag: Schema.Literal("InvalidValue"), message: Schema.String })
const Io = Schema.Struct({ _tag: Schema.Literal("Io"), kind: Schema.String, osCode: Schema.optional(Schema.Number) })
const ResourceLimit = Schema.Struct({
	_tag: Schema.Literal("ResourceLimit"),
	dimension: Schema.String,
	used: Schema.BigInt,
	requested: Schema.BigInt,
	limit: Schema.BigInt
})
/** An engine refusal: the engine error's kind and message. */
const Engine = Schema.Struct({ _tag: Schema.Literal("Engine"), kind: Schema.String, message: Schema.String })
const Malformed = Schema.Struct({ _tag: Schema.Literal("Malformed"), path: Schema.String, message: Schema.String })

/** Why an operation failed: the addon's `RuntimeError`, plus authoring detail on `InvalidArgument`. */
export const DbReason = Schema.Union([PlainReason, InvalidArgument, InvalidValue, Io, ResourceLimit, Engine, Malformed])
export type DbReason = typeof DbReason.Type

/** The one runtime error: which operation failed, and why. */
export class DbError extends Schema.TaggedError<DbError>()("DbError", {
	operation: Schema.String,
	reason: DbReason
}) {
	get code() {
		return this.reason._tag
	}
	override get message(): string {
		return `${this.operation}: ${this.code}`
	}
}

const decodeReason = Schema.decodeUnknownOption(DbReason)

/** Reads a native refusal (or any thrown value) as a `DbError` for `operation`. */
export function dbError(operation: string, cause: unknown): DbError {
	if (cause instanceof DbError) return cause
	if (cause instanceof AuthoringError) return argumentError(operation, cause)
	const decoded = decodeReason(cause)
	return new DbError({ operation, reason: decoded._tag === "Some" ? decoded.value : { _tag: "Internal" } })
}

/** An input refusal, keeping an authoring error's message and diagnostic. */
export function argumentError(operation: string, cause: unknown): DbError {
	if (cause instanceof DbError) return cause
	if (cause instanceof AuthoringError) {
		const detail =
			cause.diagnostic === undefined
				? { message: cause.message }
				: { message: cause.message, diagnostic: cause.diagnostic }
		return new DbError({ operation, reason: { _tag: "InvalidArgument", detail } })
	}
	return new DbError({ operation, reason: { _tag: "InvalidArgument" } })
}

/** A contradiction between the SDK and the addon: a bug, reported as `Internal`. */
export function internalError(operation: string): DbError {
	return new DbError({ operation, reason: { _tag: "Internal" } })
}

const Outstanding = Schema.Struct({
	phase: Schema.Literals(["Open", "Closing", "Closed"]),
	queued: Schema.BigInt,
	active: Schema.BigInt,
	retained: Schema.BigInt,
	owners: Schema.BigInt,
	databases: Schema.BigInt,
	natives: Schema.BigInt
})
export type OutstandingWork = typeof Outstanding.Type
const Close = Schema.Union([
	Schema.Struct({ _tag: Schema.Literal("Closed") }),
	Schema.Struct({ _tag: Schema.Literal("Incomplete"), outstanding: Outstanding }),
	Schema.Struct({ _tag: Schema.Literal("Failed") })
])
export type CloseReport = typeof Close.Type

/** A native resource that did not drain cleanly when its scope closed. */
export class CloseFailure extends Schema.TaggedError<CloseFailure>()("CloseFailure", {
	operation: Schema.String,
	report: Close
}) {}
