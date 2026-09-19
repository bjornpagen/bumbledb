import { Result } from "effect"
import { dbNative } from "#db-native.ts"
import { EventDescriptor, encodedDescriptor } from "#event-descriptor.ts"
import { type Event, encodedEvent, eventBytes } from "#event-value.ts"
import { FiniteFunction } from "#finite-function.ts"
import { argumentError, type DbError } from "#runtime-errors.ts"
import { sourceBoolean, sourceOperation, sourceResult } from "#source-operation.ts"
import { encodedSource, isSource, type SourceValue, sourceBytes } from "#source-value.ts"
import { recordValue } from "#values.ts"

type FiniteKernel = SourceValue<"kernel">
export interface FiniteKernelDescription {
	/** Checked onto map from the extension to the parent. */
	readonly parent: EventDescriptor
	readonly density: FiniteFunction
}
export interface SourceExtension {
	/** Full measured joint space. */
	readonly space: Event
	/** Pullback translates old Events; it does not sample them again. */
	readonly parent: EventDescriptor
}
const encode = (input: unknown) => encodedSource("kernel", input)
const toBytes = (input: FiniteKernel) => sourceBytes("kernel", input)
const isFiniteKernel = (input: unknown): input is FiniteKernel => isSource("kernel", input)
function fromBytes(input: Uint8Array): Result.Result<FiniteKernel, DbError> {
	return Result.try({ try: () => encode(input), catch: (cause) => argumentError("FiniteKernel.fromBytes", cause) })
}
/** Nonnegative conditional mass summing to one over every parent fibre,
 * including structurally possible parents assigned zero mass by a prior. */
function create(parent: EventDescriptor, density: FiniteFunction) {
	return sourceResult("kernel.new", () => [EventDescriptor.toBytes(parent), FiniteFunction.toBytes(density)], encode)
}
function validate(value: FiniteKernel) {
	return sourceResult("kernel.validate", () => [toBytes(value)], encode)
}
function describe(value: FiniteKernel) {
	return sourceOperation(
		"kernel.describe",
		() => [toBytes(value)],
		0n,
		(operation) => dbNative.runtimeEventSourceTake(operation),
		(input): FiniteKernelDescription => {
			const data = recordValue("FiniteKernel description", input, ["parent", "density"])
			return Object.freeze({ parent: encodedDescriptor(data.parent), density: encodedSource("function", data.density) })
		}
	)
}
/** Reuse a fixed channel under an explicit full prior with the same named
 * structural parent. The resulting joint law preserves that prior marginal. */
function close(value: FiniteKernel, prior: Event) {
	return sourceOperation(
		"kernel.close",
		() => [toBytes(value), eventBytes(prior)],
		0n,
		(operation) => dbNative.runtimeEventSourceTake(operation),
		(input): SourceExtension => {
			const data = recordValue("SourceExtension", input, ["space", "parent"])
			return Object.freeze({ space: encodedEvent(data.space), parent: encodedDescriptor(data.parent) })
		}
	)
}
/** Check that the whole density is determined by an onto readout of the
 * extension (e.g. visible state plus outcome). No visibility policy is inferred. */
function factorsThrough(value: FiniteKernel, readout: EventDescriptor) {
	return sourceBoolean("kernel.factorsThrough", () => [toBytes(value), EventDescriptor.toBytes(readout)])
}
const FiniteKernel = Object.freeze({
	fromBytes,
	toBytes,
	isFiniteKernel,
	new: create,
	validate,
	describe,
	close,
	factorsThrough
})

export { FiniteKernel }
