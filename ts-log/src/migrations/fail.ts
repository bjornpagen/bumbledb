/**
 * Migration-lane failure construction over the one log error vocabulary
 * (`LogError = DbError | ProtocolError`, P08's `#errors.ts`). Refusals are
 * typed E failures with bounded structured detail; there is no third error
 * class and no side-channel message matching. Resource-budget refusals use
 * the core `DbError` `ResourceLimit` reason.
 *
 * Cross-lane note (recorded in implementation/packets/P10.md): the structured
 * reasons `MigrationIntentRequired`/`MigrationUnsupported`/
 * `MigrationRepository` and the structured `MigrationDrift` payload are
 * requested additions to P08's `#codes.ts`/`#errors.ts` roster, mirroring the
 * native migration codec's refusal codes (C11).
 */
import { DbError } from "@bjornpagen/bumbledb"
import { ProtocolError } from "#errors.ts"

/** One actionable generation refusal: what is required, about which subject. */
export interface IntentRequirement {
	readonly code:
		| "ambiguous"
		| "destructive"
		| "missing-backfill"
		| "type-change"
		| "unsupported"
		| "stale-intent"
		| "conflicting-intent"
	readonly relation: string
	readonly field: string | null
	readonly detail: string
}

const MAX_REQUIREMENTS = 32
const MAX_DETAIL = 512

export function boundedDetail(detail: string): string {
	return detail.length > MAX_DETAIL ? `${detail.slice(0, MAX_DETAIL - 1)}…` : detail
}

function boundedRequirement(requirement: IntentRequirement): IntentRequirement {
	return {
		code: requirement.code,
		relation: requirement.relation,
		field: requirement.field,
		detail: boundedDetail(requirement.detail)
	}
}

/** Generation refuses with the complete finite list of required typed intent. */
export function intentRequired(operation: string, requirements: readonly IntentRequirement[]): ProtocolError {
	const truncated = requirements.length > MAX_REQUIREMENTS
	return new ProtocolError({
		operation,
		reason: {
			_tag: "MigrationIntentRequired",
			requirements: requirements.slice(0, MAX_REQUIREMENTS).map(boundedRequirement),
			truncated
		}
	})
}

/** A recorded plan/manifest/snapshot no longer matches its recorded identity. */
export function drift(operation: string, detail: string): ProtocolError {
	return new ProtocolError({
		operation,
		reason: { _tag: "MigrationDrift", detail: boundedDetail(detail) }
	})
}

/** The finite supported grammar cannot express the authored input. */
export function unsupported(operation: string, detail: string): ProtocolError {
	return new ProtocolError({
		operation,
		reason: { _tag: "MigrationUnsupported", detail: boundedDetail(detail) }
	})
}

/** A malformed or unreadable repository artifact/layout. */
export function repository(operation: string, path: string, detail: string): ProtocolError {
	return new ProtocolError({
		operation,
		reason: { _tag: "MigrationRepository", path: boundedDetail(path), detail: boundedDetail(detail) }
	})
}

/** A generation/seed budget refusal on the core resource reason. */
export function budget(
	operation: string,
	dimension: string,
	used: bigint,
	requested: bigint,
	limit: bigint
): DbError {
	return new DbError({
		operation,
		reason: { _tag: "ResourceLimit", dimension, used, requested, limit }
	})
}
