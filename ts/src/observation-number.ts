/** Portable exact numerical derivations. Import copies the envelope; native
 * query admission checks its full mathematical replay before use. */
import { Result } from "effect"
import { encodedNumber, isObservationNumber, numberBytes, type ObservationNumber as Value } from "#number-value.ts"
import { argumentError, type DbError } from "#runtime-errors.ts"

type ObservationNumber = Value
function fromBytes(bytes: Uint8Array): Result.Result<ObservationNumber, DbError> {
	return Result.try({
		try: () => encodedNumber(bytes),
		catch: (cause) => argumentError("ObservationNumber.fromBytes", cause)
	})
}
const ObservationNumber = Object.freeze({ fromBytes, toBytes: numberBytes, is: isObservationNumber })

export { ObservationNumber }
