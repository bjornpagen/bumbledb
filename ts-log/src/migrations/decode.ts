/**
 * Bounded runtime decoding of untrusted generated data. The admin runner
 * consumes the static index import — plain JSON data — and never trusts it:
 * every plan and the manifest are structurally re-decoded here before
 * anything crosses the bridge, and the native codec re-judges them again
 * from canonical frames. A well-shaped object is still not a checked plan.
 */
import { Schema, Result } from "effect"
import {
	DatabaseId,
	DecisionDigest,
	IncarnationId,
	OperationId,
	parseDatabaseIdentity,
	parseSchemaId,
	PlanSetDigest
} from "#identity.ts"
import type { ActivationRef } from "#migrations/types.ts"
import { planExpressionOf, planValueOf } from "#migrations/expr.ts"
import type {
	GeneratedMigrations,
	ManifestEntry,
	MigrationManifest,
	MigrationPlan,
	PlanLoss,
	PlanOperation,
	PlanValue,
	RuntimeContract
} from "#migrations/types.ts"

const MAX_OPERATIONS = 65536
const MAX_ROWS = 1_000_000

const ManifestModel = Schema.Struct({
	manifestVersion: Schema.Literal(1),
	planVersion: Schema.Literal(1),
	baseSchemaId: Schema.String,
	basePrefixDigest: Schema.String,
	entries: Schema.Array(
		Schema.Struct({
			sequence: Schema.String,
			id: Schema.String,
			fromSchemaId: Schema.String,
			toSchemaId: Schema.String,
			planDigest: Schema.String,
			prefixDigest: Schema.String
		})
	)
})
const decodeManifestModel = Schema.decodeUnknownOption(ManifestModel)

export type DecodeResult<A> = { readonly ok: true; readonly value: A } | { readonly ok: false; readonly detail: string }

function bad<A>(detail: string): DecodeResult<A> {
	return { ok: false, detail }
}

export function decodeManifestData(value: unknown): DecodeResult<MigrationManifest> {
	const decoded = decodeManifestModel(value)
	if (decoded._tag === "None") {
		return bad("manifest shape")
	}
	const entries: readonly ManifestEntry[] = decoded.value.entries
	return { ok: true, value: { ...decoded.value, entries } }
}

function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
	return typeof value === "object" && value !== null && !Array.isArray(value)
}

function decodeOperation(value: unknown): PlanOperation | string {
	if (!isRecord(value) || typeof value.kind !== "string") {
		return "operation shape"
	}
	switch (value.kind) {
		case "map-relation": {
			if (typeof value.source !== "string" || typeof value.target !== "string" || !Array.isArray(value.fields)) {
				return "map-relation shape"
			}
			const fields = []
			for (const field of value.fields) {
				if (!isRecord(field) || typeof field.target !== "string") {
					return "map-relation field shape"
				}
				const expression = planExpressionOf(field.expression)
				if (!expression.ok) {
					return expression.detail
				}
				fields.push({ target: field.target, expression: expression.expression })
			}
			return { kind: "map-relation", source: value.source, target: value.target, fields }
		}
		case "empty-relation":
			return typeof value.target === "string" ? { kind: "empty-relation", target: value.target } : "empty-relation shape"
		case "drop-relation":
			return typeof value.source === "string" ? { kind: "drop-relation", source: value.source } : "drop-relation shape"
		case "seed": {
			if (typeof value.target !== "string" || !Array.isArray(value.rows) || value.rows.length > MAX_ROWS) {
				return "seed shape"
			}
			const rows: Array<readonly PlanValue[]> = []
			for (const row of value.rows) {
				if (!Array.isArray(row)) {
					return "seed row shape"
				}
				const cells: PlanValue[] = []
				for (const cell of row) {
					const decoded = planValueOf(cell)
					if (typeof decoded === "string") {
						return decoded
					}
					cells.push(decoded)
				}
				rows.push(cells)
			}
			return { kind: "seed", target: value.target, rows }
		}
		case "validate-schema":
			return typeof value.schemaId === "string"
				? { kind: "validate-schema", schemaId: value.schemaId }
				: "validate-schema shape"
		default:
			return `unknown operation kind ${value.kind}`
	}
}

export function decodePlanData(value: unknown): DecodeResult<MigrationPlan> {
	if (!isRecord(value)) {
		return bad("plan shape")
	}
	if (value.planVersion !== 1) {
		return bad("unsupported planVersion")
	}
	if (
		typeof value.sequence !== "string" ||
		typeof value.id !== "string" ||
		typeof value.fromSchemaId !== "string" ||
		typeof value.toSchemaId !== "string" ||
		!Array.isArray(value.operations) ||
		value.operations.length > MAX_OPERATIONS
	) {
		return bad("plan shape")
	}
	const operations: PlanOperation[] = []
	for (const operation of value.operations) {
		const decoded = decodeOperation(operation)
		if (typeof decoded === "string") {
			return bad(decoded)
		}
		operations.push(decoded)
	}
	const destructive: PlanLoss[] = []
	if (value.destructive !== undefined) {
		if (!Array.isArray(value.destructive)) {
			return bad("destructive shape")
		}
		for (const loss of value.destructive) {
			if (!isRecord(loss) || typeof loss.relation !== "string") {
				return bad("destructive entry shape")
			}
			if (loss.field === undefined || loss.field === null) {
				destructive.push({ relation: loss.relation })
			} else if (typeof loss.field === "string") {
				destructive.push({ relation: loss.relation, field: loss.field })
			} else {
				return bad("destructive field shape")
			}
		}
	}
	return {
		ok: true,
		value: {
			planVersion: 1,
			sequence: value.sequence,
			id: value.id,
			fromSchemaId: value.fromSchemaId,
			toSchemaId: value.toSchemaId,
			operations,
			destructive
		}
	}
}

export function decodeGeneratedMigrations(value: unknown): DecodeResult<GeneratedMigrations> {
	if (!isRecord(value)) {
		return bad("generated migrations shape")
	}
	const manifest = decodeManifestData(value.manifest)
	if (!manifest.ok) {
		return bad(manifest.detail)
	}
	if (!Array.isArray(value.plans)) {
		return bad("plans shape")
	}
	if (value.plans.length !== manifest.value.entries.length) {
		return bad("plan count does not match the manifest")
	}
	if (!Array.isArray(value.snapshots) || value.snapshots.length !== manifest.value.entries.length + 1) {
		return bad("snapshots must be the empty-base schema plus one target per entry")
	}
	const snapshots: string[] = []
	for (const [index, raw] of value.snapshots.entries()) {
		if (typeof raw !== "string" || raw.length === 0) {
			return bad(`snapshot ${index} must be nonempty canonical schema-file text`)
		}
		snapshots.push(raw)
	}
	const plans: MigrationPlan[] = []
	for (const [index, raw] of value.plans.entries()) {
		const plan = decodePlanData(raw)
		if (!plan.ok) {
			return bad(`plan ${index}: ${plan.detail}`)
		}
		const entry = manifest.value.entries[index]
		if (
			entry === undefined ||
			plan.value.sequence !== entry.sequence ||
			plan.value.id !== entry.id ||
			plan.value.fromSchemaId !== entry.fromSchemaId ||
			plan.value.toSchemaId !== entry.toSchemaId
		) {
			return bad(`plan ${index} disagrees with its manifest entry`)
		}
		plans.push(plan.value)
	}
	return { ok: true, value: { manifest: manifest.value, plans, snapshots } }
}

export function decodeRuntimeContract(value: unknown): DecodeResult<RuntimeContract> {
	if (!isRecord(value)) {
		return bad("runtime contract shape")
	}
	if (value.contractVersion !== 1) {
		return bad("unsupported contractVersion")
	}
	if (typeof value.schemaId !== "string" || typeof value.appliedPrefixDigest !== "string") {
		return bad("runtime contract shape")
	}
	const steps = value.steps === undefined ? "0" : String(value.steps)
	return {
		ok: true,
		value: {
			contractVersion: 1,
			schemaId: value.schemaId,
			appliedPrefixDigest: value.appliedPrefixDigest,
			steps
		}
	}
}

export function decodeActivationRef(value: unknown): DecodeResult<ActivationRef> {
	if (!isRecord(value)) {
		return bad("activation shape")
	}
	if (typeof value.operation !== "string") {
		return bad("activation operation")
	}
	if (typeof value.planSetDigest !== "string") {
		return bad("activation planSetDigest")
	}
	if (typeof value.targetGenesis !== "string") {
		return bad("activation targetGenesis")
	}
	const operation = OperationId.fromHex(value.operation)
	if (Result.isFailure(operation)) {
		return bad("activation operation")
	}
	const planSetDigest = PlanSetDigest.fromHex(value.planSetDigest)
	if (Result.isFailure(planSetDigest)) {
		return bad("activation planSetDigest")
	}
	const targetGenesis = DecisionDigest.fromHex(value.targetGenesis)
	if (Result.isFailure(targetGenesis)) {
		return bad("activation targetGenesis")
	}
	if (typeof value.target === "string") {
		const target = parseDatabaseIdentity(value.target)
		if (Result.isFailure(target)) {
			return bad("activation target")
		}
		return {
			ok: true,
			value: {
				operation: operation.success,
				planSetDigest: planSetDigest.success,
				target: target.success,
				targetGenesis: targetGenesis.success
			}
		}
	}
	if (!isRecord(value.target)) {
		return bad("activation target")
	}
	if (typeof value.target.databaseId !== "string" || typeof value.target.incarnationId !== "string" || typeof value.target.schemaId !== "string") {
		return bad("activation target")
	}
	const databaseId = DatabaseId.fromHex(value.target.databaseId)
	if (Result.isFailure(databaseId)) {
		return bad("activation target databaseId")
	}
	const incarnationId = IncarnationId.fromHex(value.target.incarnationId)
	if (Result.isFailure(incarnationId)) {
		return bad("activation target incarnationId")
	}
	const schemaId = parseSchemaId(value.target.schemaId)
	if (Result.isFailure(schemaId)) {
		return bad("activation target schemaId")
	}
	return {
		ok: true,
		value: {
			operation: operation.success,
			planSetDigest: planSetDigest.success,
			target: {
				databaseId: databaseId.success,
				incarnationId: incarnationId.success,
				schemaId: schemaId.success
			},
			targetGenesis: targetGenesis.success
		}
	}
}

/** Decode a persisted ready-to-switch migrate outcome's activation field. */
export function decodeReadyToSwitchActivation(value: unknown): DecodeResult<ActivationRef> {
	if (!isRecord(value)) {
		return bad("migrate outcome shape")
	}
	if (value.kind !== "completed" || !isRecord(value.value) || value.value.kind !== "ready-to-switch") {
		return bad("migrate outcome is not ready-to-switch")
	}
	return decodeActivationRef(value.value.activation)
}
