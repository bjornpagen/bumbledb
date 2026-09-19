import { SdkInvariantError } from "#errors.ts"
import { type EventDescriptor, encodedDescriptor } from "#event-descriptor.ts"
import { type Event, encodedEvent } from "#event-value.ts"
import type { FamilyFunction } from "#family-function.ts"
import type { ParameterFunction } from "#parameter-function.ts"
import type { ParameterRegion } from "#parameter-region.ts"
import type { ParameterRestriction } from "#parameter-restriction.ts"
import { encodedParameter } from "#parameter-value.ts"
import { encodedSource } from "#source-value.ts"
import { arrayValue, bytesValue, recordValue } from "#values.ts"

export type FamilyRevisionReceipt =
	| { readonly kind: "condition"; readonly evidence: Event; readonly mass: ParameterFunction }
	| { readonly kind: "likelihood"; readonly likelihood: FamilyFunction; readonly normalizer: ParameterFunction }
	| {
			readonly kind: "jeffrey"
			readonly cells: readonly Event[]
			readonly targets: readonly ParameterFunction[]
			readonly oldMasses: readonly ParameterFunction[]
			/** One exact target > 0 and old mass = 0 region per ordered cell. */
			readonly unsupportedRegions: readonly ParameterRegion[]
	  }
export type FamilyRevisionOutcome =
	| { readonly kind: "impossible" }
	| {
			readonly kind: "revised"
			readonly refinedPrior: Event
			readonly posterior: Event
			readonly restriction: ParameterRestriction
			/** Posterior → refinedPrior, generally not onto. Use FamilyRevision.pullback
			 * for an Event from the original prior; refinement must be composed first. */
			readonly translation: EventDescriptor
	  }
export interface FamilyRevisionInspection {
	readonly identity: Uint8Array
	readonly prior: Event
	readonly defined: ParameterRegion
	readonly receipt: FamilyRevisionReceipt
	readonly outcome: FamilyRevisionOutcome
}
function invalid(): never {
	throw new SdkInvariantError({ message: "Family revision: malformed inspection" })
}
function tag(input: unknown) {
	if (typeof input !== "object" || input === null) invalid()
	return recordValue("Family revision record", input, Object.keys(input)).kind
}
/** Check owned transport shape; the native worker has replayed the mathematics. */
export function decodeFamilyRevision(input: unknown): FamilyRevisionInspection {
	let remaining = 16 * 1024 * 1024
	let items = 4096 - 3
	const charge = () => {
		if (--items < 0) invalid()
	}
	const bytes = (input: unknown) => {
		charge()
		const value = bytesValue("Family revision bytes", input, remaining)
		remaining -= value.length
		return value
	}
	const list = <A>(input: unknown, accept: (input: unknown) => A) => {
		charge()
		if (!Array.isArray(input) || input.length > items) invalid()
		return arrayValue("Family revision list", input, (_, item) => accept(item))
	}
	const event = (input: unknown) => encodedEvent(bytes(input))
	const fn = (input: unknown) => encodedSource("parameterFunction", bytes(input))
	const region = (input: unknown) => encodedParameter("region", bytes(input))
	const data = recordValue("Family revision", input, ["identity", "prior", "defined", "receipt", "outcome"])
	const identity = bytes(data.identity)
	if (identity.length !== 32) invalid()
	const prior = event(data.prior)
	const defined = region(data.defined)
	const receipt = (): FamilyRevisionReceipt => {
		switch (tag(data.receipt)) {
			case "condition": {
				const value = recordValue("Family condition", data.receipt, ["kind", "evidence", "mass"])
				return Object.freeze({ kind: "condition", evidence: event(value.evidence), mass: fn(value.mass) })
			}
			case "likelihood": {
				const value = recordValue("Family likelihood", data.receipt, ["kind", "likelihood", "normalizer"])
				return Object.freeze({
					kind: "likelihood",
					likelihood: encodedSource("familyFunction", bytes(value.likelihood)),
					normalizer: fn(value.normalizer)
				})
			}
			case "jeffrey": {
				const value = recordValue("Family Jeffrey", data.receipt, [
					"kind",
					"cells",
					"targets",
					"oldMasses",
					"unsupportedRegions"
				])
				const cells = list(value.cells, event)
				const targets = list(value.targets, fn)
				const oldMasses = list(value.oldMasses, fn)
				const unsupportedRegions = list(value.unsupportedRegions, region)
				if ([targets.length, oldMasses.length, unsupportedRegions.length].some((n) => n !== cells.length)) invalid()
				return Object.freeze({ kind: "jeffrey", cells, targets, oldMasses, unsupportedRegions })
			}
			default:
				return invalid()
		}
	}
	const outcome = (): FamilyRevisionOutcome => {
		if (tag(data.outcome) === "impossible") {
			recordValue("Impossible family revision", data.outcome, ["kind"])
			return Object.freeze({ kind: "impossible" })
		}
		if (tag(data.outcome) !== "revised") invalid()
		const value = recordValue("Revised family", data.outcome, [
			"kind",
			"refinedPrior",
			"posterior",
			"restriction",
			"translation"
		])
		return Object.freeze({
			kind: "revised",
			refinedPrior: event(value.refinedPrior),
			posterior: event(value.posterior),
			restriction: encodedSource("parameterRestriction", bytes(value.restriction)),
			translation: encodedDescriptor(bytes(value.translation))
		})
	}
	return Object.freeze({ identity, prior, defined, receipt: receipt(), outcome: outcome() })
}
