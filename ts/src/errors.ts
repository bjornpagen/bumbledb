/**
 * The SDK's errors. Authoring refusals throw synchronously while a schema, query or value is
 * built; everything that runs as an Effect fails with `DbError { operation, reason }`; a native
 * close that does not drain cleanly is a `CloseFailure` defect.
 */
import { Schema } from "effect"
import { runtimeErrorCodes } from "./runtime-codes.ts"

export { runtimeErrorCodes } from "./runtime-codes.ts"

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

const ResourceLimit = Schema.Struct({
	_tag: Schema.Literal("ResourceLimit"),
	dimension: Schema.String,
	used: Schema.BigInt,
	requested: Schema.BigInt,
	limit: Schema.BigInt
})
const PlainReason = Schema.Struct({
	_tag: Schema.Literals(
		runtimeErrorCodes.filter(
			(code) => code !== "ResourceLimit" && code !== "Io" && code !== "Engine" && code !== "InvalidArgument"
		)
	)
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
const Io = Schema.Struct({
	_tag: Schema.Literal("Io"),
	kind: Schema.String,
	osCode: Schema.optional(Schema.Number)
})
const StatementDiagnostic = Schema.Struct({ id: Schema.Number, descriptor: Schema.String })
const SchemaDiagnostic = Schema.Struct({
	statement: StatementDiagnostic,
	conflict: Schema.optional(StatementDiagnostic)
})

/** An engine refusal, with the cited statement coordinates when the engine gives them. */
const Engine = Schema.Struct({
	_tag: Schema.Literal("Engine"),
	kind: Schema.String,
	message: Schema.String,
	diagnostic: Schema.optional(SchemaDiagnostic)
})
export const DbReason = Schema.Union([ResourceLimit, Io, Engine, InvalidArgument, PlainReason])

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
	if (decoded._tag === "Some") return new DbError({ operation, reason: decoded.value })
	if (
		typeof cause === "object" &&
		cause !== null &&
		"kind" in cause &&
		typeof cause.kind === "string" &&
		"message" in cause &&
		typeof cause.message === "string"
	) {
		const reason = decodeReason({
			_tag: "Engine",
			kind: cause.kind,
			message: cause.message,
			diagnostic: "diagnostic" in cause ? cause.diagnostic : undefined
		})
		if (reason._tag === "Some") return new DbError({ operation, reason: reason.value })
	}
	return new DbError({ operation, reason: { _tag: "Internal" } })
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
	phase: Schema.Literals(["open", "closing", "closed"]),
	queued: Schema.BigInt,
	active: Schema.BigInt,
	retained: Schema.BigInt,
	owners: Schema.BigInt,
	databases: Schema.BigInt,
	natives: Schema.BigInt
})
export type OutstandingWork = typeof Outstanding.Type
const Close = Schema.Union([
	Schema.Struct({ kind: Schema.Literal("closed") }),
	Schema.Struct({ kind: Schema.Literal("incomplete"), outstanding: Outstanding }),
	Schema.Struct({ kind: Schema.Literal("failed"), error: DbError })
])
export type CloseReport = typeof Close.Type

/** A native resource that did not drain cleanly when its scope closed. */
export class CloseFailure extends Schema.TaggedError<CloseFailure>()("CloseFailure", {
	operation: Schema.String,
	report: Close
}) {}
