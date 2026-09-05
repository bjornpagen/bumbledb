/**
 * F3 review-fix regressions (findings E and F) through the REAL public SDK
 * over the packaged addon — no wire double anywhere in this file. Finding F:
 * an invariant-rejected submission exposes the COMPLETE decoded violation
 * set (statement identities, canonical spellings, bounded example facts,
 * truncation labels) through `History.submit` and `History.resolve`, before
 * and after reopen — never an apparently-valid empty rejection. Finding E:
 * `snapshot({ consistency: at-least })` validates exact same-lineage
 * ancestry — a lower sequence with a wrong hash refuses `WrongLineage`, a
 * future stamp refuses structured `NotYetAvailable`, and a valid retained
 * ancestor accepts with at-least freshness.
 *
 * These lanes REQUIRE the rebuilt addon (`pnpm --dir ts run build` stages
 * it); a stale addon fails loudly here rather than skipping.
 */
import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { test } from "node:test"
import type { ExecutionPolicy, NativeRuntimeOptions, Violation } from "@bjornpagen/bumbledb"
import { ChangeSet, lower, NativeRuntime, relation, schema, u64 } from "@bjornpagen/bumbledb"
import { key } from "@bjornpagen/bumbledb"
import { Effect, ManagedRuntime } from "effect"
import { ProtocolError } from "#errors.ts"
import type { DatabaseIdentity, DecisionStamp, OperationId, ReceiptEpoch, RequestId } from "#identity.ts"
import { LocalHistory } from "#history.ts"
import { Command } from "#command.ts"
import { productionCodec } from "#migrations/native.ts"
import type { SubmitOptions } from "#options.ts"
import type { SubmitOutcome, TerminalReceipt } from "#outcome.ts"

const Item = relation("Item", { a: u64, b: u64 })
const GateMini = schema("GateMini", { Item }, [key(Item, ["a"])])

const runtimeOptions: NativeRuntimeOptions = {
	workers: 2,
	queueCapacity: 16,
	cleanupCapacity: 16,
	ownerCapacity: 16,
	nativeHandleCapacity: 64,
	inputBytes: 16_000_000n,
	workingBytes: 64_000_000n,
	scratchBytes: 64_000_000n,
	resultBytes: 16_000_000n,
	chunkBytes: 1_000_000n,
	cleanupTimeout: "2 seconds"
}

const work: ExecutionPolicy = {
	inputBytes: 4_000_000n,
	workingBytes: 16_000_000n,
	scratchBytes: 16_000_000n,
	resultBytes: 4_000_000n,
	rows: 100_000n,
	workUnits: 10_000_000n,
	timeout: "10 seconds"
}

const submitOptions: SubmitOptions = {
	...work,
	attempts: 4,
	backoff: { baseMillis: 0, capMillis: 0 }
}

function runtime() {
	return ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
}

let dirSeq = 0
function storeDir(tag: string): string {
	dirSeq += 1
	const dir = path.join(os.tmpdir(), `bumbledb-gate-evidence-${tag}-${process.pid}-${dirSeq}`)
	fs.rmSync(dir, { recursive: true, force: true })
	return path.join(dir, "tenant")
}

const hex = (byte: number, width: number) => byte.toString(16).padStart(2, "0").repeat(width)

function identityFor(seed: number, schemaId: string): DatabaseIdentity {
	return {
		databaseId: hex(seed, 16) as DatabaseIdentity["databaseId"],
		incarnationId: hex(seed ^ 0xff, 16) as DatabaseIdentity["incarnationId"],
		schemaId: schemaId as DatabaseIdentity["schemaId"]
	}
}

function creationFor(seed: number, snapshot: string) {
	return {
		operationId: hex(seed + 1, 16) as OperationId,
		artifact: new TextEncoder().encode(snapshot)
	}
}

function commandInput(scope: DatabaseIdentity, request: number, changes: ChangeSet<typeof GateMini>) {
	return {
		scope,
		id: {
			receiptEpoch: 1n as ReceiptEpoch,
			requestId: hex(request, 16) as RequestId
		},
		changes,
		precondition: { kind: "blind" } as const,
		result: {}
	}
}

function violatingChanges(rows: ReadonlyArray<{ a: bigint; b: bigint }>) {
	return Effect.gen(function* () {
		const draft = yield* ChangeSet.builder(GateMini, work)
		yield* draft.insert(Item, rows)
		return yield* draft.finish()
	})
}

function decidedRejection(outcome: SubmitOutcome): {
	receipt: TerminalReceipt
	violations: readonly Violation[]
} {
	assert.equal(outcome.kind, "decided", "a violating command still DECIDES (durable rejection)")
	assert.ok(outcome.kind === "decided")
	const rejected = outcome.receipt.outcome
	assert.equal(rejected.kind, "invariant-rejected")
	assert.ok(rejected.kind === "invariant-rejected")
	return { receipt: outcome.receipt, violations: rejected.violations }
}

function assertCompleteViolations(violations: readonly Violation[]) {
	assert.ok(violations.length >= 1, "THE DEFECT-F REPRO: the violations array must NEVER be empty")
	const violation = violations[0]
	assert.ok(violation !== undefined)
	assert.equal(violation.statementId, 0)
	assert.equal(violation.kind, "functionality")
	assert.ok(violation.canonical.length > 0, "the canonical statement spelling is preserved")
	assert.ok(violation.facts.length >= 1, "bounded example facts are preserved")
	const fact = violation.facts[0]
	assert.ok(fact !== undefined)
	assert.equal(fact.relation, "Item")
	assert.ok(fact.fields.some((field) => field.name === "a"))
	// The bounded-example truncation label rides on each decoded row.
	const labeled = violation as unknown as { factsTruncated?: boolean }
	assert.equal(typeof labeled.factsTruncated, "boolean", "the truncation label crosses the bridge")
}

test("rejected submissions expose the complete violation set through submit, resolve and reopen", async function evidence() {
	const rt = runtime()
	try {
		const program = Effect.gen(function* () {
			const identityInfo = yield* productionCodec.schemaIdentity(lower(GateMini), work)
			const scope = identityFor(0x47, identityInfo.schemaId)
			const directory = storeDir("f-evidence")
			const binding = { kind: "local", directory, identity: scope } as const

			const first = yield* Effect.scoped(
				Effect.gen(function* () {
					const history = yield* LocalHistory.create(binding, GateMini, {
						...work,
						creation: creationFor(0x47, identityInfo.snapshot)
					})
					// Two rows sharing key `a` violate `Item(a) -> Item`.
					const changes = yield* violatingChanges([
						{ a: 1n, b: 10n },
						{ a: 1n, b: 20n }
					])
					const command = yield* Command.seal(commandInput(scope, 0x09, changes), work)
					const submitted = yield* history.submit(command, submitOptions)
					const { receipt, violations } = decidedRejection(submitted)
					assertCompleteViolations(violations)

					// Resolve returns the SAME retained evidence.
					const resolved = yield* history.resolve(receipt.command, work)
					assert.equal(resolved.kind, "found")
					assert.ok(resolved.kind === "found")
					const kept = resolved.receipt.outcome
					assert.ok(kept.kind === "invariant-rejected")
					assertCompleteViolations(kept.violations)
					assert.deepEqual(
						kept.violations.map((violation) => violation.canonical),
						violations.map((violation) => violation.canonical)
					)
					return { ref: receipt.command, canonical: violations.map((violation) => violation.canonical) }
				})
			)

			// Resolve AFTER REOPEN: a fresh open of the same materialization
			// still decodes the durable canonical evidence.
			return yield* Effect.scoped(
				Effect.gen(function* () {
					const history = yield* LocalHistory.open(binding, GateMini, work)
					const resolved = yield* history.resolve(first.ref, work)
					assert.equal(resolved.kind, "found")
					assert.ok(resolved.kind === "found")
					const kept = resolved.receipt.outcome
					assert.ok(kept.kind === "invariant-rejected")
					assertCompleteViolations(kept.violations)
					assert.deepEqual(
						kept.violations.map((violation) => violation.canonical),
						first.canonical
					)
					return kept.violations.length
				})
			)
		})
		const count = await rt.runPromise(program)
		assert.ok(count >= 1)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})

test("at-least snapshots validate exact ancestry, never a sequence floor", async function ancestry() {
	const rt = runtime()
	try {
		const program = Effect.gen(function* () {
			const identityInfo = yield* productionCodec.schemaIdentity(lower(GateMini), work)
			const scope = identityFor(0x51, identityInfo.schemaId)
			const directory = storeDir("e-ancestry")
			const binding = { kind: "local", directory, identity: scope } as const
			return yield* Effect.scoped(
				Effect.gen(function* () {
					const history = yield* LocalHistory.create(binding, GateMini, {
						...work,
						creation: creationFor(0x51, identityInfo.snapshot)
					})
					// Three committed decisions (distinct keys).
					const stamps: DecisionStamp[] = []
					for (const request of [1, 2, 3]) {
						const changes = yield* violatingChanges([{ a: BigInt(request), b: 0n }])
						const command = yield* Command.seal(commandInput(scope, request, changes), work)
						const outcome = yield* history.submit(command, submitOptions)
						assert.equal(outcome.kind, "decided")
						assert.ok(outcome.kind === "decided")
						assert.equal(outcome.receipt.outcome.kind, "committed")
						stamps.push(outcome.receipt.decisionAt)
					}
					const first = stamps[0]
					const tip = stamps[2]
					assert.ok(first !== undefined && tip !== undefined)
					assert.equal(first.seq, 1n)

					// A valid RETAINED ancestor accepts with at-least freshness.
					yield* Effect.scoped(
						Effect.gen(function* () {
							const snapshot = yield* history.snapshot({
								...work,
								consistency: { kind: "at-least", at: first }
							})
							assert.equal(snapshot.freshness.kind, "at-least")
							assert.equal(snapshot.decisionStamp.seq, tip.seq, "the served frame is the tip")
						})
					)

					// THE DEFECT-E REPRO: an older sequence with a WRONG hash was
					// silently accepted as a floor; it must refuse WrongLineage.
					const forged = { seq: 1n, hash: "ee".repeat(32) as DecisionStamp["hash"] }
					const wrongLineage = yield* Effect.flip(
						Effect.scoped(history.snapshot({ ...work, consistency: { kind: "at-least", at: forged } }))
					)
					assert.ok(wrongLineage instanceof ProtocolError, "an older wrong-hash stamp must refuse typed")
					assert.equal(wrongLineage.code, "WrongLineage")

					// A same-sequence-as-tip wrong hash refuses too.
					const forgedTip = { seq: tip.seq, hash: "dd".repeat(32) as DecisionStamp["hash"] }
					const tipLineage = yield* Effect.flip(
						Effect.scoped(history.snapshot({ ...work, consistency: { kind: "at-least", at: forgedTip } }))
					)
					assert.ok(tipLineage instanceof ProtocolError)
					assert.equal(tipLineage.code, "WrongLineage")

					// A future stamp refuses the structured NotYetAvailable.
					const future = { seq: 99n, hash: "09".repeat(32) as DecisionStamp["hash"] }
					const notYet = yield* Effect.flip(
						Effect.scoped(history.snapshot({ ...work, consistency: { kind: "at-least", at: future } }))
					)
					assert.ok(notYet instanceof ProtocolError)
					assert.equal(notYet.code, "NotYetAvailable")
					const reason = notYet.reason
					// `in` narrowing: the roster type of the plain-reason arm keeps
					// every literal, so the structured fields discriminate.
					assert.ok(reason._tag === "NotYetAvailable" && "requestedSeq" in reason)
					assert.equal(reason.requestedSeq, 99n)
					assert.equal(reason.capturedSeq, 3n)
					return stamps.length
				})
			)
		})
		const decided = await rt.runPromise(program)
		assert.equal(decided, 3)
	} finally {
		await Effect.runPromise(rt.disposeEffect)
	}
})
