import { Data } from "effect"

/** A tooling refusal or host-operation failure, with its original cause. */
export class ScriptError extends Data.TaggedError("ScriptError")<{
	readonly message: string
	readonly cause?: unknown
}> {}
