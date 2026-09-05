/**
 * Admin/migration certainty wrappers: mutating operations return
 * `AdminOutcome` in A with E = never and a stable operation reference
 * derived BEFORE dispatch; `not-started` proves this invocation performed
 * no authoritative mutation; interruption after dispatch is
 * `outcome-unknown` (or `completed` if the receipt already decoded) under
 * the original operationId — never a new ID. Read-only status/verification
 * has typed E.
 * `completed(paused)` is a known report, not permission to cut over.
 * Maps to OPS-006 (primary audit row), OPS-TEST-01 (status fixtures,
 * layer side), ERASE-03 reporting shape, TS-MIG-09/10 (wrapper side).
 */
import assert from "node:assert/strict"
import { describe, test } from "node:test"
import type { AnySchema } from "@bjornpagen/bumbledb"
import { Effect, Exit, Fiber } from "effect"
import type { OperationId, PlanSetDigest } from "#identity.ts"
import { makeLogMachine } from "#machine.ts"
import type { GeneratedMigrations } from "#outcome.ts"
import {
	identityWire,
	localBinding,
	makeIntegration,
	makeWireDouble,
	provideRuntime,
	stampWire,
	stateWire,
	work
} from "#test/double.ts"

const OPERATION = "4b".repeat(16) as OperationId
const adminOptions = { ...work, operationId: OPERATION }

const plans: GeneratedMigrations = {
	manifest: {
		manifestVersion: 1,
		planVersion: 1,
		baseSchemaId: "0".repeat(64),
		basePrefixDigest: "1".repeat(64),
		entries: [
			{
				sequence: "0",
				id: "0000-initialize",
				fromSchemaId: "0".repeat(64),
				toSchemaId: "2d".repeat(32),
				planDigest: "6e".repeat(32),
				prefixDigest: "7d".repeat(32)
			}
		]
	},
	plans: [
		{
			planVersion: 1,
			sequence: "0",
			id: "0000-initialize",
			fromSchemaId: "0".repeat(64),
			toSchemaId: "2d".repeat(32),
			operations: [{ kind: "validate-schema", schemaId: "2d".repeat(32) }],
			destructive: []
		}
	],
	snapshots: ["base-schema-render", "target-schema-render"]
}

type Double = ReturnType<typeof makeWireDouble>
type Machine = ReturnType<typeof makeLogMachine>

function make(): { double: Double; machine: Machine } {
	const double = makeWireDouble()
	return { double, machine: makeLogMachine(double.wire, makeIntegration()) }
}

describe("admin certainty", function suite() {
	test("checkpoint completes with its report and the pre-dispatch ref", async function checkpoint() {
		const { double, machine } = make()
		double.plan("logAdmin", {
			result: {
				certainty: "completed",
				publicationPhase: "confirmed",
				value: { verb: "checkpoint", at: stampWire, state: stateWire, root: "root-1" }
			}
		})
		const outcome = await Effect.runPromise(provideRuntime(machine.admin.checkpoint(localBinding, adminOptions)))
		assert.equal(outcome.kind, "completed")
		assert.equal(outcome.ref.operation, OPERATION)
		assert.equal(outcome.ref.identity.databaseId, identityWire.databaseId)
		if (outcome.kind === "completed") {
			assert.equal(outcome.value.root, "root-1")
			assert.equal(outcome.value.at.seq, 7n)
			assert.equal(outcome.phase, "confirmed")
		}
		// The operation id crossed the wire with the request (fixed before dispatch).
		const request = double.calls[0]?.request as { operationId: string }
		assert.equal(request.operationId, OPERATION)
	})

	test("registration refusal is not-started and E stays never", async function notStarted() {
		const { double, machine } = make()
		double.plan("logAdmin", { refuse: { source: "core", reason: { _tag: "QueueFull" } } })
		const exit = await Effect.runPromiseExit(provideRuntime(machine.admin.collectGarbage(localBinding, adminOptions)))
		assert.ok(Exit.isSuccess(exit))
		const outcome = Exit.getSuccess(exit)
		assert.ok(outcome._tag === "Some")
		assert.equal(outcome.value.kind, "not-started")
		if (outcome.value.kind === "not-started") {
			assert.equal(outcome.value.ref.operation, OPERATION)
			assert.equal(outcome.value.error.code, "QueueFull")
		}
	})

	test("a lost completion is outcome-unknown with the retained ref", async function unknown() {
		const { double, machine } = make()
		double.plan("logAdmin", { failure: { source: "protocol", reason: { _tag: "Backend" } } })
		const outcome = await Effect.runPromise(
			provideRuntime(
				machine.admin.backup(localBinding, {
					...adminOptions,
					destination: { kind: "filesystem", directory: "/tmp/bumbledb-backup" }
				})
			)
		)
		assert.equal(outcome.kind, "outcome-unknown")
		if (outcome.kind === "outcome-unknown") {
			assert.equal(outcome.ref.operation, OPERATION)
			assert.equal(outcome.error.code, "Backend")
			assert.equal(outcome.phase, "dispatchedUnresolved")
		}
	})

	test("erase reports residual copies honestly", async function erase() {
		const { double, machine } = make()
		double.plan("logAdmin", {
			result: {
				certainty: "completed",
				publicationPhase: "confirmed",
				value: {
					verb: "erase",
					tombstoned: true,
					retainedRoots: ["root-legal-hold"],
					residual: [{ kind: "backup", location: "s3://backups/t1" }]
				}
			}
		})
		const outcome = await Effect.runPromise(
			provideRuntime(machine.admin.erase(localBinding, { ...adminOptions, retainRoots: [] }))
		)
		assert.equal(outcome.kind, "completed")
		if (outcome.kind === "completed") {
			assert.equal(outcome.value.tombstoned, true)
			assert.equal(outcome.value.residual[0]?.kind, "backup")
		}
	})

	test("verifyBackup is read-only with typed E", async function verify() {
		const { double, machine } = make()
		double.plan("logAdmin", { failure: { source: "protocol", reason: { _tag: "UnsupportedArtifact" } } })
		const exit = await Effect.runPromiseExit(
			provideRuntime(machine.admin.verifyBackup({ kind: "filesystem", directory: "/tmp/x" }, work))
		)
		assert.ok(Exit.isFailure(exit))
		const error = Exit.findErrorOption(exit)
		assert.ok(error._tag === "Some")
		assert.equal(error.value.code, "UnsupportedArtifact")
	})

	test("interrupting a mutating operation is outcome-unknown under the original operationId", async function interrupted() {
		const { double, machine } = make()
		double.plan("logAdmin", {
			hold: true,
			result: {
				certainty: "completed",
				publicationPhase: "confirmed",
				value: { verb: "checkpoint", at: stampWire, state: stateWire, root: "root-2" }
			}
		})
		// The ref exists BEFORE dispatch: the app persisted operationId already.
		const fiber = Effect.runFork(provideRuntime(machine.admin.checkpoint(localBinding, adminOptions)))
		await new Promise((resolve) => setImmediate(resolve))
		await Effect.runPromise(Fiber.interrupt(fiber))
		const exit = await Effect.runPromise(Fiber.await(fiber))
		assert.ok(Exit.isSuccess(exit))
		const outcome = Exit.getSuccess(exit)
		assert.ok(outcome._tag === "Some")
		assert.equal(outcome.value.kind, "outcome-unknown")
		if (outcome.value.kind === "outcome-unknown") {
			assert.equal(outcome.value.ref.operation, OPERATION)
			assert.equal(outcome.value.phase, "dispatchedUnresolved")
			assert.equal(outcome.value.error.code, "Cancelled")
		}
		assert.ok(double.cancelCount() >= 1)
	})
})

describe("migration wrappers", function suite() {
	test("migrationStatus is read-only and decodes the status sum", async function status() {
		const { double, machine } = make()
		double.plan("logAdmin", {
			result: {
				certainty: "report",
				value: {
					verb: "migration-status",
					status: { kind: "pending", pending: ["0000-initialize"] }
				}
			}
		})
		const status = await Effect.runPromise(provideRuntime(machine.migrations.migrationStatus(localBinding, plans, work)))
		assert.equal(status.kind, "pending")
		if (status.kind === "pending") {
			assert.deepEqual(status.pending, ["0000-initialize"])
		}
		const request = double.calls[0]?.request as { verb: string; plans: { entries: unknown[]; plans: string[] } }
		assert.equal(request.verb, "migration-status")
		assert.equal(request.plans.entries.length, 1)
		// The plan bodies crossed as inert canonical data, not functions.
		assert.equal(typeof request.plans.plans[0], "string")
	})

	test("migrate completes as ready-to-switch with a bound activation ref; it does not activate", async function readyToSwitch() {
		const { double, machine } = make()
		const activation = {
			operationId: OPERATION,
			planSetDigest: "8c".repeat(32),
			target: { ...identityWire, incarnationId: "9b".repeat(16) },
			targetGenesis: "7a".repeat(32)
		}
		double.plan("logAdmin", {
			result: {
				certainty: "completed",
				publicationPhase: "confirmed",
				value: {
					verb: "migration-migrate",
					value: {
						kind: "ready-to-switch",
						deploymentBinding: {
							kind: "local",
							directory: "/tmp/bumbledb-target",
							identity: activation.target
						},
						activation
					}
				}
			}
		})
		const outcome = await Effect.runPromise(
			provideRuntime(machine.migrations.migrate(localBinding, plans, adminOptions))
		)
		assert.equal(outcome.kind, "completed")
		if (outcome.kind === "completed") {
			assert.equal(outcome.value.kind, "ready-to-switch")
			if (outcome.value.kind === "ready-to-switch") {
				assert.equal(outcome.value.activation.targetGenesis, "7a".repeat(32))
				assert.equal(outcome.value.deploymentBinding.kind, "local")
			}
		}
		// Exactly one wire verb ran: migrate never dispatched an activation.
		assert.equal(double.calls.length, 1)
		assert.equal((double.calls[0]?.request as { verb: string }).verb, "migration-migrate")
	})

	test("migrate completed(paused) reports the frozen source, not a success claim", async function paused() {
		const { double, machine } = make()
		double.plan("logAdmin", {
			result: {
				certainty: "completed",
				publicationPhase: "confirmed",
				value: {
					verb: "migration-migrate",
					value: {
						kind: "paused",
						error: { source: "protocol", reason: { _tag: "InsufficientLocalDisk", requiredBytes: 10n, availableBytes: 1n } },
						sourceState: { access: "frozen", operationId: OPERATION }
					}
				}
			}
		})
		const outcome = await Effect.runPromise(
			provideRuntime(machine.migrations.migrate(localBinding, plans, adminOptions))
		)
		assert.equal(outcome.kind, "completed")
		if (outcome.kind === "completed") {
			assert.equal(outcome.value.kind, "paused")
			if (outcome.value.kind === "paused") {
				assert.equal(outcome.value.sourceState.access, "frozen")
				assert.equal(outcome.value.error.code, "InsufficientLocalDisk")
			}
		}
	})

	test("activate and abort take their bound refs and report verified access", async function activateAbort() {
		const { double, machine } = make()
		const target = {
			databaseId: identityWire.databaseId,
			incarnationId: "9b".repeat(16),
			schemaId: identityWire.schemaId
		}
		const activationRef = {
			operation: OPERATION,
			planSetDigest: "8c".repeat(32) as PlanSetDigest,
			target,
			targetGenesis: "7a".repeat(32)
		} as unknown as Parameters<typeof machine.migrations.activateMigration>[0]
		const abortRef = {
			operation: {
				identity: {
					databaseId: identityWire.databaseId,
					incarnationId: identityWire.incarnationId,
					schemaId: identityWire.schemaId
				},
				operation: OPERATION
			},
			planSetDigest: "8c".repeat(32) as PlanSetDigest,
			target
		}
		double.plan("logAdmin", {
			result: {
				certainty: "completed",
				publicationPhase: "confirmed",
				value: {
					verb: "migration-activate",
					target: { ...identityWire, incarnationId: "9b".repeat(16) },
					accessMode: "active",
					operationId: OPERATION,
					activatedNow: true
				}
			}
		})
		const activated = await Effect.runPromise(provideRuntime(machine.migrations.activateMigration(activationRef, work)))
		assert.equal(activated.kind, "completed")
		if (activated.kind === "completed") {
			assert.equal(activated.value.accessMode, "active")
			assert.equal(activated.value.activatedNow, true)
		}

		double.plan("logAdmin", {
			result: {
				certainty: "completed",
				publicationPhase: "confirmed",
				value: {
					verb: "migration-abort",
					target: { ...identityWire, incarnationId: "9b".repeat(16) },
					targetFenced: true,
					sourceAccess: "active"
				}
			}
		})
		const aborted = await Effect.runPromise(
			provideRuntime(
				machine.migrations.abortMigration(
					abortRef as unknown as Parameters<typeof machine.migrations.abortMigration>[0],
					work
				)
			)
		)
		assert.equal(aborted.kind, "completed")
		if (aborted.kind === "completed") {
			// The target fence is durable BEFORE the source thawed.
			assert.equal(aborted.value.targetFenced, true)
			assert.equal(aborted.value.sourceAccess, "active")
		}
	})

	test("plans carry the ordered schema snapshots as inert transport", async function snapshots() {
		const { double, machine } = make()
		double.plan("logAdmin", {
			result: {
				certainty: "completed",
				publicationPhase: "confirmed",
				value: { verb: "migration-initialize", binding: { kind: "local", directory: "/tmp/t", identity: identityWire }, genesis: "7a".repeat(32) }
			}
		})
		const outcome = await Effect.runPromise(
			provideRuntime(machine.migrations.initialize(localBinding, plans, adminOptions))
		)
		assert.equal(outcome.kind, "completed")
		const request = double.calls[0]?.request as { plans: { snapshots: string[]; entries: unknown[] } }
		assert.deepEqual(request.plans.snapshots, ["base-schema-render", "target-schema-render"])
		assert.equal(request.plans.snapshots.length, request.plans.entries.length + 1)
	})

	test("a snapshot row-count mismatch refuses before dispatch as not-started", async function snapshotMismatch() {
		const { double, machine } = make()
		// One entry needs TWO snapshot rows (base + target); one row is malformed
		// caller input and must never reach the native machine.
		const short = { ...plans, snapshots: ["base-schema-render"] }
		double.plan("logAdmin", {
			result: {
				certainty: "completed",
				publicationPhase: "confirmed",
				value: { verb: "migration-initialize" }
			}
		})
		const outcome = await Effect.runPromise(
			provideRuntime(machine.migrations.initialize(localBinding, short, adminOptions))
		)
		assert.equal(outcome.kind, "not-started")
		if (outcome.kind === "not-started") {
			assert.equal(outcome.error.code, "InvalidArgument")
		}
		assert.equal(double.calls.length, 0)
	})

	test("absent snapshots refuse before dispatch", async function noSnapshots() {
		const { double, machine } = make()
		const missing = { ...plans, snapshots: undefined as unknown as string[] }
		const outcome = await Effect.runPromise(
			provideRuntime(machine.migrations.migrate(localBinding, missing, adminOptions))
		)
		assert.equal(outcome.kind, "not-started")
		if (outcome.kind === "not-started") {
			assert.equal(outcome.error.code, "InvalidArgument")
		}
		assert.equal(double.calls.length, 0)
	})

	test("binding-carrying admin verbs thread the lowered schema when supplied", async function adminSchema() {
		const { double, machine } = make()
		const schema = { __schema: "lowered-by-double-passthrough" } as unknown as AnySchema
		double.plan("logAdmin", {
			result: {
				certainty: "completed",
				publicationPhase: "confirmed",
				value: { verb: "checkpoint", at: stampWire, state: stateWire, root: "root-3" }
			}
		})
		const outcome = await Effect.runPromise(
			provideRuntime(machine.admin.checkpoint(localBinding, { ...adminOptions, schema }))
		)
		assert.equal(outcome.kind, "completed")
		// The double's schemaSpec is identity: the request carries the lowering.
		const request = double.calls[0]?.request as { schema?: unknown }
		assert.deepEqual(request.schema, schema)

		// Absent schema stays absent on the wire (native decides the refusal).
		double.plan("logAdmin", {
			result: {
				certainty: "completed",
				publicationPhase: "confirmed",
				value: { verb: "checkpoint", at: stampWire, state: stateWire, root: "root-4" }
			}
		})
		await Effect.runPromise(provideRuntime(machine.admin.checkpoint(localBinding, adminOptions)))
		const bare = double.calls[1]?.request as Record<string, unknown>
		assert.ok(!("schema" in bare))
	})

	test("restore and verifyBackup thread the backup operation id", async function backupId() {
		const { double, machine } = make()
		const backup = "0a".repeat(16) as OperationId
		const schema = { __schema: "restore-target" } as unknown as AnySchema
		double.plan("logAdmin", {
			result: {
				certainty: "completed",
				publicationPhase: "confirmed",
				value: {
					verb: "restore",
					identity: identityWire,
					genesis: "7a".repeat(32),
					binding: { kind: "local", directory: "/tmp/restored", identity: identityWire }
				}
			}
		})
		const restored = await Effect.runPromise(
			provideRuntime(
				machine.admin.restore({ kind: "filesystem", directory: "/tmp/backup" }, localBinding, {
					...adminOptions,
					backup,
					schema
				})
			)
		)
		assert.equal(restored.kind, "completed")
		const request = double.calls[0]?.request as { backup: string; schema: unknown }
		assert.equal(request.backup, backup)
		assert.deepEqual(request.schema, schema)

		double.plan("logAdmin", {
			result: {
				certainty: "report",
				value: {
					verb: "verify-backup",
					identity: identityWire,
					at: stampWire,
					state: stateWire,
					objects: 3n,
					bytes: 12n,
					manifestDigest: "5a".repeat(32)
				}
			}
		})
		const verified = await Effect.runPromise(
			provideRuntime(machine.admin.verifyBackup({ kind: "filesystem", directory: "/tmp/backup" }, { ...work, backup }))
		)
		assert.equal(verified.manifestDigest, "5a".repeat(32))
		const verify = double.calls[1]?.request as { backup: string }
		assert.equal(verify.backup, backup)
	})

	test("activate/abort thread the optional source binding and target schema", async function activateLocation() {
		const { double, machine } = make()
		const schema = { __schema: "target" } as unknown as AnySchema
		const activationRef = {
			operation: OPERATION,
			planSetDigest: "8c".repeat(32) as PlanSetDigest,
			target: {
				databaseId: identityWire.databaseId,
				incarnationId: "9b".repeat(16),
				schemaId: identityWire.schemaId
			},
			targetGenesis: "7a".repeat(32)
		} as unknown as Parameters<typeof machine.migrations.activateMigration>[0]
		double.plan("logAdmin", {
			result: {
				certainty: "completed",
				publicationPhase: "confirmed",
				value: {
					verb: "migration-activate",
					target: { ...identityWire, incarnationId: "9b".repeat(16) },
					accessMode: "active",
					operationId: OPERATION,
					activatedNow: false
				}
			}
		})
		const outcome = await Effect.runPromise(
			provideRuntime(machine.migrations.activateMigration(activationRef, { ...work, binding: localBinding, schema }))
		)
		assert.equal(outcome.kind, "completed")
		const request = double.calls[0]?.request as { binding: { kind: string; directory: string }; schema: unknown }
		assert.equal(request.binding.kind, "local")
		assert.equal(request.binding.directory, "/tmp/bumbledb-double")
		assert.deepEqual(request.schema, schema)
	})

	test("uncertain abort stays outcome-unknown: never an implicit thaw claim", async function uncertainAbort() {
		const { double, machine } = make()
		double.plan("logAdmin", { failure: { source: "protocol", reason: { _tag: "Backend" } } })
		const outcome = await Effect.runPromise(
			provideRuntime(
				machine.migrations.abortMigration(
					{
						operation: {
							identity: {
								databaseId: identityWire.databaseId,
								incarnationId: identityWire.incarnationId,
								schemaId: identityWire.schemaId
							},
							operation: OPERATION
						},
						planSetDigest: "8c".repeat(32),
						target: {
							databaseId: identityWire.databaseId,
							incarnationId: "9b".repeat(16),
							schemaId: identityWire.schemaId
						}
					} as unknown as Parameters<typeof machine.migrations.abortMigration>[0],
					work
				)
			)
		)
		assert.equal(outcome.kind, "outcome-unknown")
	})
})
