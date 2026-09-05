/**
 * Log-layer identity values: small owned immutable data, never handles.
 * `DatabaseId`/`IncarnationId`/`RequestId`/`OperationId` are Id128-backed
 * roles (canonical 32 lowercase hex); digests are full 32-byte roles
 * (64 lowercase hex). The distinct roles are nominal brands: an entity ID is
 * not a request ID, and neither is a history coordinate. Parsers here are
 * the bounded log boundary codecs for HTTP/session tokens — pure, `Result`-
 * returning, no I/O, no mint/generate; `SchemaId` and `Id128` remain
 * core-owned imports. Retry uses the caller-supplied bytes again.
 */
import type { Id128, SchemaId } from "@bjornpagen/bumbledb"
import { Result } from "effect"
import type { DbError } from "@bjornpagen/bumbledb"
import { invalidInput } from "#errors.ts"

declare const databaseIdBrand: unique symbol
export type DatabaseId = string & { readonly [databaseIdBrand]: typeof databaseIdBrand }
declare const incarnationIdBrand: unique symbol
export type IncarnationId = string & { readonly [incarnationIdBrand]: typeof incarnationIdBrand }
declare const requestIdBrand: unique symbol
export type RequestId = string & { readonly [requestIdBrand]: typeof requestIdBrand }
declare const operationIdBrand: unique symbol
export type OperationId = string & { readonly [operationIdBrand]: typeof operationIdBrand }
declare const commandDigestBrand: unique symbol
export type CommandDigest = string & { readonly [commandDigestBrand]: typeof commandDigestBrand }
declare const decisionDigestBrand: unique symbol
export type DecisionDigest = string & { readonly [decisionDigestBrand]: typeof decisionDigestBrand }
declare const rootIdBrand: unique symbol
/** A named restore-point/hold identity (chapter 21 NamedRoot). */
export type RootId = string & { readonly [rootIdBrand]: typeof rootIdBrand }
declare const planSetDigestBrand: unique symbol
/** The exact ordered pending-suffix commitment (chapter 33). */
export type PlanSetDigest = string & { readonly [planSetDigestBrand]: typeof planSetDigestBrand }

export interface DatabaseIdentity {
	readonly databaseId: DatabaseId
	readonly incarnationId: IncarnationId
	readonly schemaId: SchemaId
}

export interface DecisionStamp {
	readonly seq: bigint
	readonly hash: DecisionDigest
}

export interface StateStamp {
	readonly incarnation: IncarnationId
	readonly dataRevision: bigint
}

declare const receiptEpochBrand: unique symbol
/** Positive command-admission namespace; zero is no epoch and is invalid. */
export type ReceiptEpoch = bigint & { readonly [receiptEpochBrand]: typeof receiptEpochBrand }

export interface CommandId {
	readonly receiptEpoch: ReceiptEpoch
	readonly requestId: RequestId
}

/** Sufficient to resolve an uncertain submission without a live handle. */
export interface CommandRef {
	readonly identity: DatabaseIdentity
	readonly id: CommandId
	readonly digest: CommandDigest
}

/** One admin/migration operation's protocol identity, fixed before dispatch. */
export interface OperationRef {
	readonly identity: DatabaseIdentity
	readonly operation: OperationId
}

export type Freshness =
	| { readonly kind: "cached" }
	| { readonly kind: "latest" }
	| { readonly kind: "at-least"; readonly requested: DecisionStamp }

export type ReadConsistency =
	| { readonly kind: "cached" }
	| { readonly kind: "at-least"; readonly at: DecisionStamp }
	| { readonly kind: "latest" }

const U64_MAX = 0xffffffffffffffffn

function lowercaseHex(raw: string, length: number): boolean {
	if (raw.length !== length) {
		return false
	}
	for (let i = 0; i < raw.length; i++) {
		const code = raw.charCodeAt(i)
		const digit = code >= 0x30 && code <= 0x39
		const lower = code >= 0x61 && code <= 0x66
		if (!digit && !lower) {
			return false
		}
	}
	return true
}

function hex32(operation: string, raw: string): Result.Result<string, DbError> {
	return lowercaseHex(raw, 32) ? Result.succeed(raw) : Result.fail(invalidInput(operation))
}

function hex64(operation: string, raw: string): Result.Result<string, DbError> {
	return lowercaseHex(raw, 64) ? Result.succeed(raw) : Result.fail(invalidInput(operation))
}

function u64(operation: string, raw: string): Result.Result<bigint, DbError> {
	if (raw.length === 0 || raw.length > 20 || (raw.length > 1 && raw.startsWith("0"))) {
		return Result.fail(invalidInput(operation))
	}
	for (let i = 0; i < raw.length; i++) {
		const code = raw.charCodeAt(i)
		if (code < 0x30 || code > 0x39) {
			return Result.fail(invalidInput(operation))
		}
	}
	const value = BigInt(raw)
	return value <= U64_MAX ? Result.succeed(value) : Result.fail(invalidInput(operation))
}

/**
 * `Id128`'s canonical TypeScript value is its 32-lowercase-hex string
 * (chapter 35 V8 rules). This defensive re-check keeps the log's nominal
 * conversions total even for a hostile structural forgery.
 */
function id128Hex(operation: string, id: Id128): Result.Result<string, DbError> {
	return typeof id === "string" ? hex32(operation, id) : Result.fail(invalidInput(operation))
}

export const DatabaseId = {
	fromHex(raw: string): Result.Result<DatabaseId, DbError> {
		return Result.map(hex32("DatabaseId.fromHex", raw), (hex) => hex as DatabaseId)
	},
	from(id: Id128): Result.Result<DatabaseId, DbError> {
		return Result.map(id128Hex("DatabaseId.from", id), (hex) => hex as DatabaseId)
	}
} as const

export const IncarnationId = {
	fromHex(raw: string): Result.Result<IncarnationId, DbError> {
		return Result.map(hex32("IncarnationId.fromHex", raw), (hex) => hex as IncarnationId)
	},
	from(id: Id128): Result.Result<IncarnationId, DbError> {
		return Result.map(id128Hex("IncarnationId.from", id), (hex) => hex as IncarnationId)
	}
} as const

export const RequestId = {
	/** The explicit pure nominal conversion over the core's canonical bytes. */
	from(id: Id128): Result.Result<RequestId, DbError> {
		return Result.map(id128Hex("RequestId.from", id), (hex) => hex as RequestId)
	},
	fromHex(raw: string): Result.Result<RequestId, DbError> {
		return Result.map(hex32("RequestId.fromHex", raw), (hex) => hex as RequestId)
	}
} as const

export const OperationId = {
	from(id: Id128): Result.Result<OperationId, DbError> {
		return Result.map(id128Hex("OperationId.from", id), (hex) => hex as OperationId)
	},
	fromHex(raw: string): Result.Result<OperationId, DbError> {
		return Result.map(hex32("OperationId.fromHex", raw), (hex) => hex as OperationId)
	}
} as const

export const ReceiptEpoch = {
	from(raw: bigint): Result.Result<ReceiptEpoch, DbError> {
		if (typeof raw !== "bigint" || raw <= 0n || raw > U64_MAX) {
			return Result.fail(invalidInput("ReceiptEpoch.from"))
		}
		return Result.succeed(raw as ReceiptEpoch)
	}
} as const

export const CommandDigest = {
	fromHex(raw: string): Result.Result<CommandDigest, DbError> {
		return Result.map(hex64("CommandDigest.fromHex", raw), (hex) => hex as CommandDigest)
	}
} as const

export const DecisionDigest = {
	fromHex(raw: string): Result.Result<DecisionDigest, DbError> {
		return Result.map(hex64("DecisionDigest.fromHex", raw), (hex) => hex as DecisionDigest)
	}
} as const

export const RootId = {
	fromString(raw: string): Result.Result<RootId, DbError> {
		const operation = "RootId.fromString"
		if (raw.length === 0 || raw.length > 128) {
			return Result.fail(invalidInput(operation))
		}
		for (let i = 0; i < raw.length; i++) {
			const code = raw.charCodeAt(i)
			const digit = code >= 0x30 && code <= 0x39
			const lower = code >= 0x61 && code <= 0x7a
			const dash = code === 0x2d
			if (!digit && !lower && !dash) {
				return Result.fail(invalidInput(operation))
			}
		}
		return Result.succeed(raw as RootId)
	}
} as const

export const PlanSetDigest = {
	fromHex(raw: string): Result.Result<PlanSetDigest, DbError> {
		return Result.map(hex64("PlanSetDigest.fromHex", raw), (hex) => hex as PlanSetDigest)
	}
} as const

/**
 * The schema identity is core-owned; the log validates only the canonical
 * 64-lowercase-hex spelling when a token crosses its own HTTP boundary.
 */
export function parseSchemaId(raw: string): Result.Result<SchemaId, DbError> {
	return Result.map(hex64("parseSchemaId", raw), (hex) => hex as unknown as SchemaId)
}

// Canonical bounded token spellings. `:`-joined fixed-width fields; every
// parser refuses malformed widths, uppercase, and noncanonical integers.

export function renderDecisionStamp(stamp: DecisionStamp): string {
	return `${stamp.seq.toString(10)}:${stamp.hash}`
}

export function parseDecisionStamp(raw: string): Result.Result<DecisionStamp, DbError> {
	const operation = "parseDecisionStamp"
	if (typeof raw !== "string" || raw.length > 96) {
		return Result.fail(invalidInput(operation))
	}
	const parts = raw.split(":")
	const seqRaw = parts[0]
	const hashRaw = parts[1]
	if (parts.length !== 2 || seqRaw === undefined || hashRaw === undefined) {
		return Result.fail(invalidInput(operation))
	}
	return Result.flatMap(u64(operation, seqRaw), (seq) =>
		Result.map(hex64(operation, hashRaw), (hash) => ({ seq, hash: hash as DecisionDigest }))
	)
}

export function renderStateStamp(stamp: StateStamp): string {
	return `${stamp.incarnation}:${stamp.dataRevision.toString(10)}`
}

export function parseStateStamp(raw: string): Result.Result<StateStamp, DbError> {
	const operation = "parseStateStamp"
	if (typeof raw !== "string" || raw.length > 64) {
		return Result.fail(invalidInput(operation))
	}
	const parts = raw.split(":")
	const incarnationRaw = parts[0]
	const revisionRaw = parts[1]
	if (parts.length !== 2 || incarnationRaw === undefined || revisionRaw === undefined) {
		return Result.fail(invalidInput(operation))
	}
	return Result.flatMap(hex32(operation, incarnationRaw), (incarnation) =>
		Result.map(u64(operation, revisionRaw), (dataRevision) => ({
			incarnation: incarnation as IncarnationId,
			dataRevision
		}))
	)
}

export function renderDatabaseIdentity(identity: DatabaseIdentity): string {
	return `${identity.databaseId}:${identity.incarnationId}:${identity.schemaId}`
}

export function parseDatabaseIdentity(raw: string): Result.Result<DatabaseIdentity, DbError> {
	const operation = "parseDatabaseIdentity"
	if (typeof raw !== "string" || raw.length > 144) {
		return Result.fail(invalidInput(operation))
	}
	const parts = raw.split(":")
	const databaseRaw = parts[0]
	const incarnationRaw = parts[1]
	const schemaRaw = parts[2]
	if (parts.length !== 3 || databaseRaw === undefined || incarnationRaw === undefined || schemaRaw === undefined) {
		return Result.fail(invalidInput(operation))
	}
	return Result.flatMap(hex32(operation, databaseRaw), (databaseId) =>
		Result.flatMap(hex32(operation, incarnationRaw), (incarnationId) =>
			Result.map(parseSchemaId(schemaRaw), (schemaId) => ({
				databaseId: databaseId as DatabaseId,
				incarnationId: incarnationId as IncarnationId,
				schemaId
			}))
		)
	)
}

export function renderCommandRef(ref: CommandRef): string {
	return `${renderDatabaseIdentity(ref.identity)}:${ref.id.receiptEpoch.toString(10)}:${ref.id.requestId}:${ref.digest}`
}

export function parseCommandRef(raw: string): Result.Result<CommandRef, DbError> {
	const operation = "parseCommandRef"
	if (typeof raw !== "string" || raw.length > 288) {
		return Result.fail(invalidInput(operation))
	}
	const parts = raw.split(":")
	if (parts.length !== 6) {
		return Result.fail(invalidInput(operation))
	}
	const [databaseRaw, incarnationRaw, schemaRaw, epochRaw, requestRaw, digestRaw] = parts
	if (
		databaseRaw === undefined ||
		incarnationRaw === undefined ||
		schemaRaw === undefined ||
		epochRaw === undefined ||
		requestRaw === undefined ||
		digestRaw === undefined
	) {
		return Result.fail(invalidInput(operation))
	}
	return Result.flatMap(parseDatabaseIdentity(`${databaseRaw}:${incarnationRaw}:${schemaRaw}`), (identity) =>
		Result.flatMap(u64(operation, epochRaw), (epochValue) =>
			Result.flatMap(ReceiptEpoch.from(epochValue), (receiptEpoch) =>
				Result.flatMap(RequestId.fromHex(requestRaw), (requestId) =>
					Result.map(CommandDigest.fromHex(digestRaw), (digest) => ({
						identity,
						id: { receiptEpoch, requestId },
						digest
					}))
				)
			)
		)
	)
}

export function sameIdentity(a: DatabaseIdentity, b: DatabaseIdentity): boolean {
	return a.databaseId === b.databaseId && a.incarnationId === b.incarnationId && a.schemaId === b.schemaId
}

export function sameCommandRef(a: CommandRef, b: CommandRef): boolean {
	return (
		sameIdentity(a.identity, b.identity) &&
		a.id.receiptEpoch === b.id.receiptEpoch &&
		a.id.requestId === b.id.requestId &&
		a.digest === b.digest
	)
}
