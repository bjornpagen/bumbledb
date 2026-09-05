/**
 * The pure schema diff: previous canonical snapshot versus current canonical
 * snapshot, plus declared typed intent. Structural inference happens ONLY
 * where one safe interpretation exists (unchanged relations/fields carried by
 * identity projections, whole-relation additions, reorders); ambiguous
 * rename-versus-drop, data destruction, required backfills and type changes
 * REQUIRE typed intent or the diff returns the complete finite requirement
 * list — generation never guesses from matching shapes and never fabricates
 * a zero/null.
 *
 * Output is inert plan data with TOTAL ordinary-relation coverage (C11):
 * every ordinary source relation becomes exactly one `map-relation` or
 * `drop-relation`; every ordinary target relation exactly one
 * `map-relation`/`empty-relation`. A source field no expression references is
 * data loss and appears in `destructive` exactly once. Closed relations are
 * sealed schema axioms — unnameable by data operations and exempt from
 * coverage; their evolution is the schema change itself.
 */
import type { IntentRequirement } from "#migrations/fail.ts"
import { fieldExpression, planExpressionOf } from "#migrations/expr.ts"
import type { MigrationIntentEntry } from "#migrations/intent.ts"
import type { PlanFieldMap, PlanLoss, PlanOperation, TheoryRelation, TheorySnapshot } from "#migrations/types.ts"

export interface DiffResult {
	/** Maps, empties and drops in canonical order (no seeds, no validate). */
	readonly operations: readonly PlanOperation[]
	readonly destructive: readonly PlanLoss[]
	/** Ordinary target relations receiving declarative seed rows, target order. */
	readonly seedRelations: readonly string[]
	/** Deterministic tokens for the derived human label. */
	readonly labelTokens: readonly string[]
	/** Nonempty means: refuse generation with exactly these requirements. */
	readonly requirements: readonly IntentRequirement[]
	/**
	 * True only when every ordinary map is an identity projection.
	 * An explicit same-schema convert (units+1) is not identity.
	 */
	readonly identity: boolean
}

interface Consumable {
	readonly entry: MigrationIntentEntry
	consumed: boolean
}

function requirement(
	code: IntentRequirement["code"],
	relation: string,
	field: string | null,
	detail: string
): IntentRequirement {
	return { code, relation, field, detail }
}

function labelToken(text: string): string {
	let out = ""
	for (const ch of text.toLowerCase()) {
		out += (ch >= "a" && ch <= "z") || (ch >= "0" && ch <= "9") ? ch : "-"
	}
	return out.replaceAll(/-+/g, "-").replaceAll(/^-|-$/g, "")
}

export function diffSchemas(
	prev: TheorySnapshot,
	next: TheorySnapshot,
	entries: readonly MigrationIntentEntry[]
): DiffResult {
	const requirements: IntentRequirement[] = []
	const intents: Consumable[] = entries.map((entry) => ({ entry, consumed: false }))
	const prevByName = new Map(prev.relations.map((relation) => [relation.name, relation]))
	const nextByName = new Map(next.relations.map((relation) => [relation.name, relation]))

	// --- Relation renames -----------------------------------------------------
	const relationRename = new Map<string, string>() // prev name -> next name
	const renameTargets = new Set<string>()
	for (const intent of intents) {
		if (intent.entry.kind !== "rename-relation") {
			continue
		}
		const { from, to } = intent.entry
		intent.consumed = true
		if (relationRename.has(from) || renameTargets.has(to)) {
			requirements.push(
				requirement("conflicting-intent", from, null, `two renameRelation intents touch ${from} or ${to}`)
			)
			continue
		}
		if (!prevByName.has(from)) {
			requirements.push(requirement("stale-intent", from, null, `renameRelation: no previous relation named ${from}`))
			continue
		}
		if (!nextByName.has(to)) {
			requirements.push(requirement("stale-intent", to, null, `renameRelation: no current relation named ${to}`))
			continue
		}
		if (nextByName.has(from)) {
			requirements.push(
				requirement("conflicting-intent", from, null, `renameRelation: ${from} still exists in the current schema`)
			)
			continue
		}
		if (prevByName.has(to)) {
			requirements.push(
				requirement("conflicting-intent", to, null, `renameRelation: ${to} already existed in the previous schema`)
			)
			continue
		}
		relationRename.set(from, to)
		renameTargets.add(to)
	}

	// --- Field renames, keyed by the current (next) relation name -------------
	const fieldRename = new Map<string, Map<string, string>>()
	for (const intent of intents) {
		if (intent.entry.kind !== "rename-field") {
			continue
		}
		intent.consumed = true
		const { relation, from, to } = intent.entry
		let table = fieldRename.get(relation)
		if (table === undefined) {
			table = new Map()
			fieldRename.set(relation, table)
		}
		if (table.has(from) || [...table.values()].includes(to)) {
			requirements.push(
				requirement("conflicting-intent", relation, from, `two renameField intents touch ${relation}.${from} or .${to}`)
			)
			continue
		}
		table.set(from, to)
	}

	// --- Pair relations ---------------------------------------------------------
	const pairs: Array<{ readonly source: TheoryRelation; readonly target: TheoryRelation }> = []
	const pairedNext = new Set<string>()
	const droppedSource: string[] = []

	for (const source of prev.relations) {
		const targetName = relationRename.get(source.name) ?? source.name
		const target = nextByName.get(targetName)
		if (target !== undefined) {
			if (source.closed !== target.closed) {
				requirements.push(
					requirement(
						"unsupported",
						target.name,
						null,
						`changing ${target.name} between closed and ordinary is not a supported transform; drop the old relation and declare the new one with explicit intent`
					)
				)
				continue
			}
			pairs.push({ source, target })
			pairedNext.add(target.name)
			continue
		}
		if (source.closed) {
			// A removed closed relation is pure schema change: its extension is
			// declared data, unnameable by plan operations, and nothing user-owned
			// is discarded by data machinery.
			continue
		}
		const drop = intents.find(
			(intent) => !intent.consumed && intent.entry.kind === "drop-relation" && intent.entry.relation === source.name
		)
		if (drop !== undefined) {
			drop.consumed = true
			droppedSource.push(source.name)
			continue
		}
		const candidates = next.relations
			.filter((candidate) => !candidate.closed && !prevByName.has(candidate.name) && !renameTargets.has(candidate.name))
			.map((candidate) => candidate.name)
		const hint =
			candidates.length > 0
				? ` If this is a rename, declare renameRelation("${source.name}", ${candidates.join(" | ")}) instead.`
				: ""
		requirements.push(
			requirement(
				"destructive",
				source.name,
				null,
				`relation ${source.name} was removed; discarding its data requires dropRelation("${source.name}").${hint}`
			)
		)
	}

	// --- Walk ordinary pairs ------------------------------------------------------
	const mapOperations: Array<{ readonly op: PlanOperation; readonly identity: boolean }> = []
	const destructive: PlanLoss[] = []

	for (const { source, target } of pairs) {
		if (source.closed) {
			continue
		}
		const renames = fieldRename.get(target.name) ?? new Map<string, string>()
		const targetFields = new Map(target.fields.map((field) => [field.name, field]))
		const sourceFields = new Map(source.fields.map((field) => [field.name, field]))

		for (const [from, to] of renames) {
			// Staleness preempts conflict: a rename whose `from` no longer exists
			// (typically an intent already consumed by a recorded migration — after
			// it applied, `to` exists on BOTH sides) matches no change between
			// these schemas. It must classify stale with its remediation, never as
			// a contradiction with a live intent.
			if (!sourceFields.has(from) || !targetFields.has(to)) {
				if (!sourceFields.has(from)) {
					requirements.push(
						requirement(
							"stale-intent",
							target.name,
							from,
							`renameField: no previous field ${from}; an intent already recorded by a generated migration must be removed`
						)
					)
				}
				if (!targetFields.has(to)) {
					requirements.push(
						requirement(
							"stale-intent",
							target.name,
							to,
							`renameField: no current field ${to}; an intent already recorded by a generated migration must be removed`
						)
					)
				}
				continue
			}
			if (targetFields.has(from)) {
				requirements.push(
					requirement("conflicting-intent", target.name, from, `renameField: ${from} still exists in the current schema`)
				)
			}
			if (sourceFields.has(to)) {
				requirements.push(
					requirement("conflicting-intent", target.name, to, `renameField: ${to} already existed in the previous schema`)
				)
			}
		}

		// Pair fields: renamed or same-named survivors.
		const sourceOfTarget = new Map<string, string>() // target field -> source field
		const acknowledged = new Set<string>() // source fields with explicit replace/drop intent
		for (const field of source.fields) {
			const survivor = renames.get(field.name) ?? field.name
			if (targetFields.has(survivor)) {
				sourceOfTarget.set(survivor, field.name)
				continue
			}
			const drop = intents.find(
				(intent) =>
					!intent.consumed &&
					intent.entry.kind === "drop-field" &&
					intent.entry.relation === target.name &&
					intent.entry.field === field.name
			)
			if (drop !== undefined) {
				drop.consumed = true
				acknowledged.add(field.name)
				continue
			}
			const added = target.fields.filter((candidate) => !sourceFields.has(candidate.name)).map((candidate) => candidate.name)
			const hint =
				added.length > 0
					? ` If this is a rename, declare renameField(${target.name}, "${field.name}", "${added.join('" | "')}").`
					: ""
			requirements.push(
				requirement(
					"destructive",
					target.name,
					field.name,
					`field ${target.name}.${field.name} was removed; discarding its values requires dropField(${target.name}, "${field.name}").${hint}`
				)
			)
		}

		// Build the complete target projection in target declaration order.
		const projections: PlanFieldMap[] = []
		const referenced = new Set<string>()
		let identity =
			source.name === target.name &&
			source.fields.length === target.fields.length &&
			source.fields.every((field, ordinal) => target.fields[ordinal]?.name === field.name)

		for (const field of target.fields) {
			const sourceName = sourceOfTarget.get(field.name)
			const convert = intents.find(
				(intent) =>
					!intent.consumed &&
					intent.entry.kind === "convert" &&
					intent.entry.relation === target.name &&
					intent.entry.field === field.name
			)
			const fill = intents.find(
				(intent) =>
					!intent.consumed &&
					intent.entry.kind === "backfill" &&
					intent.entry.relation === target.name &&
					intent.entry.field === field.name
			)
			if (sourceName !== undefined) {
				const sourceField = sourceFields.get(sourceName)
				// Staleness preempts conflict: an intent targeting a field that is
				// UNCHANGED between these schemas (same name, same type, no rename in
				// play) matches no change — the signature of an intent already
				// consumed by a recorded migration. `conflicting-intent` is reserved
				// for intents contradicting a live change on the field.
				const fieldUnchanged =
					sourceName === field.name && sourceField !== undefined && sourceField.type === field.type
				// Backfill is only for new fields. On an unchanged existing field it
				// is stale (already recorded, or the wrong constructor). Convert on
				// the same field is an explicit meaning change — record it even when
				// the schema types match (L18 units+1), never drop as identity.
				if (fieldUnchanged && fill !== undefined) {
					fill.consumed = true
					requirements.push(
						requirement(
							"stale-intent",
							target.name,
							field.name,
							`backfill targets ${target.name}.${field.name}, which is unchanged between these schemas; an intent already recorded by a generated migration must be removed`
						)
					)
					referenced.add(sourceName)
					projections.push({ target: field.name, expression: fieldExpression(sourceName) })
					continue
				}
				if (fill !== undefined) {
					fill.consumed = true
					requirements.push(
						requirement(
							"conflicting-intent",
							target.name,
							field.name,
							`backfill targets the existing field ${target.name}.${field.name}; use convert for existing fields`
						)
					)
					continue
				}
				if (convert !== undefined) {
					convert.consumed = true
					if (convert.entry.kind !== "convert") {
						continue
					}
					const serialized = planExpressionOf(convert.entry.expression)
					if (!serialized.ok) {
						requirements.push(requirement("unsupported", target.name, field.name, serialized.detail))
						continue
					}
					const unknown = serialized.fields.filter((name) => !sourceFields.has(name))
					if (unknown.length > 0) {
						requirements.push(
							requirement(
								"unsupported",
								target.name,
								field.name,
								`convert references unknown source fields ${unknown.join(", ")} of ${source.name}`
							)
						)
						continue
					}
					for (const name of serialized.fields) {
						referenced.add(name)
					}
					acknowledged.add(sourceName)
					projections.push({ target: field.name, expression: serialized.expression })
					identity = false
					continue
				}
				if (sourceField !== undefined && sourceField.type !== field.type) {
					requirements.push(
						requirement(
							"type-change",
							target.name,
							field.name,
							`field ${target.name}.${field.name} changed type from ${sourceField.type} to ${field.type}; declare convert(${target.name}, "${field.name}", <checked cast expression>)`
						)
					)
					continue
				}
				referenced.add(sourceName)
				projections.push({ target: field.name, expression: fieldExpression(sourceName) })
				if (sourceName !== field.name) {
					identity = false
				}
				continue
			}
			// A new field on an existing relation.
			if (convert !== undefined) {
				convert.consumed = true
				requirements.push(
					requirement(
						"conflicting-intent",
						target.name,
						field.name,
						`convert targets the new field ${target.name}.${field.name}; use backfill for new fields`
					)
				)
				continue
			}
			if (fill === undefined) {
				requirements.push(
					requirement(
						"missing-backfill",
						target.name,
						field.name,
						`new required field ${target.name}.${field.name} needs backfill(${target.name}, "${field.name}", <typed expression over the old row>); no zero/null is fabricated`
					)
				)
				continue
			}
			fill.consumed = true
			if (fill.entry.kind !== "backfill") {
				continue
			}
			const serialized = planExpressionOf(fill.entry.expression)
			if (!serialized.ok) {
				requirements.push(requirement("unsupported", target.name, field.name, serialized.detail))
				continue
			}
			const unknown = serialized.fields.filter((name) => !sourceFields.has(name))
			if (unknown.length > 0) {
				requirements.push(
					requirement(
						"unsupported",
						target.name,
						field.name,
						`backfill references unknown source fields ${unknown.join(", ")} of ${source.name}`
					)
				)
				continue
			}
			for (const name of serialized.fields) {
				referenced.add(name)
			}
			projections.push({ target: field.name, expression: serialized.expression })
			identity = false
		}

		// Losses: source fields no expression references. Each needs the
		// explicit intent gathered above; the plan records each exactly once.
		for (const field of source.fields) {
			if (referenced.has(field.name)) {
				continue
			}
			if (!acknowledged.has(field.name)) {
				// Reachable only when the pairing above already refused; keep the
				// requirement complete rather than emitting an unacknowledged loss.
				continue
			}
			destructive.push({ relation: source.name, field: field.name })
		}

		mapOperations.push({
			op: { kind: "map-relation", source: source.name, target: target.name, fields: projections },
			identity
		})
	}

	// --- New relations ------------------------------------------------------------
	const emptyOperations: PlanOperation[] = []
	for (const relation of next.relations) {
		if (relation.closed || pairedNext.has(relation.name)) {
			continue
		}
		emptyOperations.push({ kind: "empty-relation", target: relation.name })
	}

	// --- Drops ----------------------------------------------------------------------
	const dropOperations: PlanOperation[] = []
	for (const name of droppedSource) {
		dropOperations.push({ kind: "drop-relation", source: name })
		destructive.push({ relation: name })
	}

	// --- Seeds -----------------------------------------------------------------------
	const seedRelations: string[] = []
	for (const intent of intents) {
		if (intent.entry.kind !== "seed") {
			continue
		}
		intent.consumed = true
		const target = nextByName.get(intent.entry.relation)
		if (target === undefined) {
			requirements.push(
				requirement("stale-intent", intent.entry.relation, null, `seed: no current relation named ${intent.entry.relation}`)
			)
			continue
		}
		if (target.closed) {
			requirements.push(
				requirement(
					"unsupported",
					intent.entry.relation,
					null,
					`seed targets the closed relation ${intent.entry.relation}; closed extensions are declared in the schema`
				)
			)
			continue
		}
		if (!seedRelations.includes(target.name)) {
			seedRelations.push(target.name)
		}
	}
	const nextOrder = next.relations.map((relation) => relation.name)
	seedRelations.sort((a, b) => nextOrder.indexOf(a) - nextOrder.indexOf(b))

	// --- Leftover (stale) intent --------------------------------------------------------
	for (const intent of intents) {
		if (intent.consumed) {
			continue
		}
		const entry = intent.entry
		const subject = "relation" in entry ? entry.relation : entry.from
		const field = "field" in entry ? entry.field : null
		requirements.push(
			requirement("stale-intent", subject, field, `${entry.kind} intent matches no change between these schemas`)
		)
	}

	// --- Deterministic label tokens --------------------------------------------------------
	const labelTokens: string[] = []
	if (prev.relations.length === 0) {
		labelTokens.push("initialize")
	} else {
		for (const op of emptyOperations) {
			if (op.kind === "empty-relation") {
				labelTokens.push(`create-${labelToken(op.target)}`)
			}
		}
		for (const entry of mapOperations) {
			if (!entry.identity && entry.op.kind === "map-relation") {
				labelTokens.push(labelToken(entry.op.target))
			}
		}
		for (const name of seedRelations) {
			labelTokens.push(`seed-${labelToken(name)}`)
		}
		for (const op of dropOperations) {
			if (op.kind === "drop-relation") {
				labelTokens.push(`drop-${labelToken(op.source)}`)
			}
		}
		if (labelTokens.length === 0) {
			labelTokens.push("laws")
		}
	}

	return {
		operations: [...mapOperations.map((entry) => entry.op), ...emptyOperations, ...dropOperations],
		destructive,
		seedRelations,
		labelTokens,
		requirements,
		identity:
			mapOperations.every((entry) => entry.identity) &&
			emptyOperations.length === 0 &&
			dropOperations.length === 0
	}
}
