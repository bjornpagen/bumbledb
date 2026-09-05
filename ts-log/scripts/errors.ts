import { Data } from "effect"

export class BuildInputError extends Data.TaggedError("BuildInputError")<{
	readonly message: string
}> {}

export class BuildOperationError extends Data.TaggedError("BuildOperationError")<{
	readonly message: string
	readonly cause: unknown
}> {}

export class DeclarationError extends Data.TaggedError("DeclarationError")<{
	readonly message: string
}> {}
