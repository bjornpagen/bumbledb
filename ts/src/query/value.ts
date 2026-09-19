/** Query domains include owned observations; stored schema fields do not. */
import { AuthoringError } from "#errors.ts"
import type { AnyField, Infer, SignatureOf } from "#fields.ts"
import { rosterOf, signaturesAgree } from "#fields.ts"
import type { ExpectationAnswer, ExpectationResult } from "#query/expectation.ts"
import type { ProbabilityAnswer, ProbabilityResult } from "#query/probability.ts"

type ObservationResult = ProbabilityResult | ExpectationResult
type QueryValue = AnyField | ObservationResult
type QueryInfer<F extends QueryValue> = F extends ProbabilityResult
	? ProbabilityAnswer
	: F extends ExpectationResult
		? ExpectationAnswer
		: F extends AnyField
			? Infer<F>
			: never
type QuerySignature<F extends QueryValue> = F extends AnyField ? SignatureOf<F> : readonly [F["kind"]]

function isObservation(field: QueryValue): field is ObservationResult {
	return field.kind === "probability" || field.kind === "expectation"
}

function queryValuesAgree(a: QueryValue, b: QueryValue): boolean {
	if (isObservation(a) || isObservation(b)) return a.kind === b.kind
	return signaturesAgree(a, b)
}

function queryRosterOf(field: QueryValue | undefined) {
	return field === undefined || isObservation(field) ? undefined : rosterOf(field)
}

/** Literals, parameters and stored codecs must never receive observation identities. */
function storedValue(context: string, field: QueryValue): AnyField {
	if (isObservation(field))
		throw new AuthoringError({
			message: `${context}: ${field.kind} observations require same-kind variable identity comparisons`
		})
	return field
}

export type { ObservationResult, QueryInfer, QuerySignature, QueryValue }
export { isObservation, queryRosterOf, queryValuesAgree, storedValue }
