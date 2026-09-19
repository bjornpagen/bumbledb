/** Portable exact predicate derivations. Import copies the envelope; native
 * query admission checks its full mathematical replay before use. */
import { Result } from "effect"
import {
	encodedPredicate,
	isObservationPredicate,
	predicateBytes,
	type ObservationPredicate as Value
} from "#predicate-value.ts"
import { argumentError, type DbError } from "#runtime-errors.ts"

type ObservationPredicate = Value
function fromBytes(bytes: Uint8Array): Result.Result<ObservationPredicate, DbError> {
	return Result.try({
		try: () => encodedPredicate(bytes),
		catch: (cause) => argumentError("ObservationPredicate.fromBytes", cause)
	})
}
const ObservationPredicate = Object.freeze({ fromBytes, toBytes: predicateBytes, is: isObservationPredicate })

export { ObservationPredicate }
