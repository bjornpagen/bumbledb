import { Result } from "effect"
import { EventDescriptor, encodedDescriptor } from "#event-descriptor.ts"
import { type Event, encodedEvent, eventBytes } from "#event-value.ts"
import { FamilyFunction } from "#family-function.ts"
import type { SourceExtension } from "#finite-kernel.ts"
import { parameterBoolean, parameterData, parameterResult } from "#parameter-operation.ts"
import { argumentError, type DbError } from "#runtime-errors.ts"
import { encodedSource, isSource, type SourceValue, sourceBytes } from "#source-value.ts"
import { recordValue } from "#values.ts"

type FamilyKernel = SourceValue<"familyKernel">
export interface FamilyKernelDescription {
	readonly parent: EventDescriptor
	readonly density: FamilyFunction
}
const encode = (input: unknown) => encodedSource("familyKernel", input)
const toBytes = (value: FamilyKernel) => sourceBytes("familyKernel", value)
const isFamilyKernel = (input: unknown): input is FamilyKernel => isSource("familyKernel", input)
function fromBytes(input: Uint8Array): Result.Result<FamilyKernel, DbError> {
	return Result.try({ try: () => encode(input), catch: (cause) => argumentError("FamilyKernel.fromBytes", cause) })
}
/** Nonnegative channel normalized over every parent fibre at every admitted
 * parameter, including parent worlds assigned zero mass by a prior. */
function create(parent: EventDescriptor, density: FamilyFunction) {
	return parameterResult("kernel.new", () => [EventDescriptor.toBytes(parent), FamilyFunction.toBytes(density)], encode)
}
function validate(value: FamilyKernel) {
	return parameterResult("kernel.validate", () => [toBytes(value)], encode)
}
function describe(value: FamilyKernel) {
	return parameterData(
		"kernel.describe",
		() => [toBytes(value)],
		(input): FamilyKernelDescription => {
			const data = recordValue("FamilyKernel", input, ["parent", "density"])
			return Object.freeze({
				parent: encodedDescriptor(data.parent),
				density: encodedSource("familyFunction", data.density)
			})
		}
	)
}
/** Preserve this explicitly designated prior's marginal and all old worlds.
 * This introduces no distribution over the shared parameter. */
function close(value: FamilyKernel, prior: Event) {
	return parameterData(
		"kernel.close",
		() => [toBytes(value), eventBytes(prior)],
		(input): SourceExtension => {
			const data = recordValue("Family extension", input, ["space", "parent"])
			return Object.freeze({ space: encodedEvent(data.space), parent: encodedDescriptor(data.parent) })
		}
	)
}
/** Numerical dependency on an onto readout of the extension. Visibility must
 * be declared by the caller; it is not inferred from a successful factor. */
function factorsThrough(value: FamilyKernel, readout: EventDescriptor) {
	return parameterBoolean("kernel.factorsThrough", () => [toBytes(value), EventDescriptor.toBytes(readout)])
}
const FamilyKernel = Object.freeze({
	fromBytes,
	toBytes,
	isFamilyKernel,
	new: create,
	validate,
	describe,
	close,
	factorsThrough
})

export { FamilyKernel }
