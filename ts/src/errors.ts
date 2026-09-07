import { Data } from "effect"

/**
 * Pure authoring failures, SDK-invariant defects and internal native-boundary
 * diagnostics. AST misuse can throw synchronously without I/O. Runtime
 * adapters translate operational failures into the public `DbError` class
 * in `#runtime-errors.ts`; contradictions in SDK/native state remain defects.
 */

/** A pure schema, query, parameter, or value-authoring refusal. */
export class AuthoringError extends Data.TaggedError("AuthoringError")<{
	readonly message: string
}> {}

/** A contradiction in an SDK/native result, not an application refusal. */
export class SdkInvariantError extends Data.TaggedError("SdkInvariantError")<{
	readonly message: string
}> {}

export class NativeLoadError extends Data.TaggedError("NativeLoadError")<{
	readonly package: string
	readonly operation: "resolve" | "load"
	readonly message: string
	readonly cause: unknown
}> {}

/** A native bridge call's contextual failure; the exact thrown value is retained. */
export class NativeOperationError extends Data.TaggedError("NativeOperationError")<{
	readonly operation: string
	readonly cause: unknown
}> {
	override get message(): string {
		const cause = this.cause
		const detail =
			typeof cause === "object" && cause !== null && "message" in cause && typeof cause.message === "string"
				? cause.message
				: String(cause)
		return `${this.operation}: ${detail}`
	}
}

export class NativeReportedError extends Data.TaggedError("NativeReportedError")<{
	readonly kind: string
	readonly message: string
	readonly cause: unknown
}> {}
