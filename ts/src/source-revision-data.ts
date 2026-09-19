import { SdkInvariantError } from "#errors.ts"
import { type EventDescriptor, encodedDescriptor } from "#event-descriptor.ts"
import { type Event, encodedEvent } from "#event-value.ts"
import { type ExactRational, encodedRational } from "#exact-value.ts"
import type { FiniteFunction } from "#finite-function.ts"
import { encodedSource } from "#source-value.ts"
import { arrayValue, bytesValue, recordValue } from "#values.ts"

export type RevisionReceipt =
	| { readonly kind: "condition"; readonly evidence: Event; readonly mass: ExactRational }
	| { readonly kind: "likelihood"; readonly likelihood: FiniteFunction; readonly normalizer: ExactRational }
	| {
			readonly kind: "jeffrey"
			readonly cells: readonly Event[]
			readonly oldMasses: readonly ExactRational[]
			readonly targets: readonly ExactRational[]
	  }
export type RevisionOutcome =
	| {
			readonly kind: "revised"
			readonly posterior: Event
			/** Onto map posterior → prior. Pullback translates old Events.
			 * This does not assert preservation of the prior measure. */
			readonly translation: EventDescriptor
	  }
	| { readonly kind: "impossible"; readonly cause: "zeroEvidence" | "zeroLikelihood" }
	| { readonly kind: "impossible"; readonly cause: "unsupportedTargets"; readonly cells: readonly number[] }
export interface SourceRevisionInspection {
	readonly prior: Event
	readonly receipt: RevisionReceipt
	readonly outcome: RevisionOutcome
}
function invalid(): never {
	throw new SdkInvariantError({ message: "Source revision: malformed inspection" })
}
function tag(input: unknown): unknown {
	if (typeof input !== "object" || input === null) invalid()
	return recordValue("Revision record", input, Object.keys(input)).kind
}
/** Transport checks only. The worker has replayed all mathematical claims. */
export function decodeRevision(input: unknown): SourceRevisionInspection {
	let remaining = 16 * 1024 * 1024
	let items = 4096 - 3 // result, receipt and outcome records
	const charge = (n = 1) => {
		items -= n
		if (items < 0) invalid()
	}
	const bytes = (input: unknown) => {
		charge()
		const value = bytesValue("Revision bytes", input, remaining)
		remaining -= value.length
		return value
	}
	const list = <A>(input: unknown, accept: (input: unknown) => A) => {
		charge()
		if (!Array.isArray(input) || input.length > items) invalid()
		return arrayValue("Revision list", input, (_, item) => accept(item))
	}
	const event = (input: unknown) => encodedEvent(bytes(input))
	const rational = (input: unknown) => encodedRational(bytes(input))
	const data = recordValue("Source revision", input, ["prior", "receipt", "outcome"])
	const prior = event(data.prior)
	const receipt = (): RevisionReceipt => {
		switch (tag(data.receipt)) {
			case "condition": {
				const value = recordValue("Condition receipt", data.receipt, ["kind", "evidence", "mass"])
				return Object.freeze({ kind: "condition", evidence: event(value.evidence), mass: rational(value.mass) })
			}
			case "likelihood": {
				const value = recordValue("Likelihood receipt", data.receipt, ["kind", "likelihood", "normalizer"])
				return Object.freeze({
					kind: "likelihood",
					likelihood: encodedSource("function", bytes(value.likelihood)),
					normalizer: rational(value.normalizer)
				})
			}
			case "jeffrey": {
				const value = recordValue("Jeffrey receipt", data.receipt, ["kind", "cells", "oldMasses", "targets"])
				const cells = list(value.cells, event)
				const oldMasses = list(value.oldMasses, rational)
				const targets = list(value.targets, rational)
				if (cells.length !== oldMasses.length || cells.length !== targets.length) invalid()
				return Object.freeze({ kind: "jeffrey", cells, oldMasses, targets })
			}
			default:
				return invalid()
		}
	}
	const outcome = (): RevisionOutcome => {
		if (tag(data.outcome) === "revised") {
			const value = recordValue("Revised source", data.outcome, ["kind", "posterior", "translation"])
			return Object.freeze({
				kind: "revised",
				posterior: event(value.posterior),
				translation: encodedDescriptor(bytes(value.translation))
			})
		}
		if (tag(data.outcome) !== "impossible") invalid()
		const value = recordValue("Impossible revision", data.outcome, ["kind", "cause", "cells"])
		const cells = list(value.cells, (input) => {
			charge()
			if (typeof input !== "number" || !Number.isInteger(input) || input < 0 || input > 0xffffffff) invalid()
			return input
		})
		if (value.cause === "unsupportedTargets") return Object.freeze({ kind: "impossible", cause: value.cause, cells })
		if (cells.length !== 0 || (value.cause !== "zeroEvidence" && value.cause !== "zeroLikelihood")) invalid()
		return Object.freeze({ kind: "impossible", cause: value.cause })
	}
	return Object.freeze({ prior, receipt: receipt(), outcome: outcome() })
}
