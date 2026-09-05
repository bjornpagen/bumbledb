import { Data } from "effect"

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

/** The legacy bridge's contextual failure; the exact thrown value is retained. */
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

export class ErrAsyncCallback extends Data.TaggedError("ErrAsyncCallback")<{
	readonly scope: "read" | "write"
	readonly message: string
}> {}

export class ErrSpentHandle extends Data.TaggedError("ErrSpentHandle")<{
	readonly handle: "ownedInstance" | "instanceBuilder" | "witness"
	readonly state: "disposed" | "spent" | "leasedForPublish" | "foreign"
	readonly message: string
	readonly cause?: unknown
}> {}

export class ErrUseAfterScope extends Data.TaggedError("ErrUseAfterScope")<{
	readonly handle: "readInstance" | "writeTransaction"
	readonly message: string
}> {}

export class ErrForeignPrepared extends Data.TaggedError("ErrForeignPrepared")<{
	readonly reason: "notPrepared" | "foreignStore"
	readonly message: string
}> {}

export class ErrForeignWitness extends Data.TaggedError("ErrForeignWitness")<{
	readonly reason: "notWitness" | "foreignStore"
	readonly message: string
}> {}

export class ErrNewtypeMismatch extends Data.TaggedError("ErrNewtypeMismatch")<{
	readonly operation: string
	readonly path: string
	readonly message: string
}> {}

export class ErrSchemaError extends Data.TaggedError("ErrSchemaError")<{
	readonly operation: string
	readonly path: string
	readonly message: string
}> {}

export class ErrFingerprintMismatch extends Data.TaggedError("ErrFingerprintMismatch")<{
	readonly operation: string
	readonly path: string
	readonly message: string
}> {}

export class ErrIrError extends Data.TaggedError("ErrIrError")<{
	readonly operation: "prepare"
	readonly message: string
}> {}
