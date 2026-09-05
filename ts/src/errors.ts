import { Data } from "effect"

/**
 * Pure authoring refusals and SDK-invariant defects. These are the ONLY
 * non-`DbError` throw families in the core package: chapter 35 permits
 * programmer-facing AST misuse to throw synchronously (no I/O ever), and a
 * contradiction between SDK and native state is a defect, not a domain
 * outcome. Every operational failure of effectful work is the single
 * `DbError` tagged-reason class in `#runtime-errors.ts` — the old
 * per-surface `Err*` wrapper family is deleted (duplicate error wrappers
 * are banned by C02/C10).
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
