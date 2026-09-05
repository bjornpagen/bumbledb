/**
 * A deterministic scripted wire double for the thin Effect layer. It is a
 * test harness for the LANGUAGE layer only: every response is enqueued by
 * the test, nothing is decided here, and no protocol logic exists to drift
 * from the one native machine. Real cross-boundary behavior runs in the F3
 * lanes against the packaged addon (P05/P12); these tests pin the layer's
 * certainty preservation, capability bookkeeping and scope policy.
 */
import { Effect } from "effect"
import type { CloseWire, OperationHandle, RuntimeHandle } from "@bjornpagen/bumbledb"
import { NativeRuntime } from "@bjornpagen/bumbledb"
import type { CoreChangesView, CoreIntegration } from "#machine.ts"
import type { LogNative } from "#native.ts"

/** The fake shared-runtime service: the double never dereferences it. */
export function provideRuntime<A, E>(effect: Effect.Effect<A, E, NativeRuntime>): Effect.Effect<A, E> {
	return Effect.provideService(effect, NativeRuntime, {
		close: () => Effect.succeed({ kind: "closed" as const }),
		inspect: () => Effect.die(new Error("wire double: no runtime inspection"))
	})
}

export interface PlannedOperation {
	/** Value returned by the matching take function. */
	readonly result?: unknown
	/** Thrown by the take function instead (post-dispatch failure). */
	readonly failure?: unknown
	/** Thrown synchronously at registration (pre-dispatch refusal). */
	readonly refuse?: unknown
	/** Hold the completion callback until `releaseHeld()`. */
	readonly hold?: boolean
}

interface OpState {
	readonly verb: string
	readonly planned: PlannedOperation
	readonly callback: () => void
	cancelled: boolean
}

export interface RecordedCall {
	readonly verb: string
	readonly request: unknown
}

export interface WireDouble {
	readonly wire: LogNative
	readonly calls: RecordedCall[]
	readonly held: OpState[]
	cancelCount(): number
	plan(verb: string, planned: PlannedOperation): void
	planClose(verb: string, report: CloseWire): void
	releaseHeld(): void
}

const CLOSED: CloseWire = { kind: "closed" }

export function makeWireDouble(): WireDouble {
	const calls: RecordedCall[] = []
	const held: OpState[] = []
	const plans = new Map<string, PlannedOperation[]>()
	const closePlans = new Map<string, CloseWire[]>()
	const counters = { cancels: 0 }

	function start(verb: string, request: unknown, callback: () => void): OperationHandle {
		calls.push({ verb, request })
		const planned = plans.get(verb)?.shift()
		if (planned === undefined) {
			throw new Error(`wire double: unplanned operation ${verb}`)
		}
		if (planned.refuse !== undefined) {
			throw planned.refuse
		}
		const op: OpState = { verb, planned, callback, cancelled: false }
		if (planned.hold === true) {
			held.push(op)
		} else {
			queueMicrotask(() => {
				if (!op.cancelled) {
					op.callback()
				}
			})
		}
		return op as unknown as OperationHandle
	}

	function take(operation: OperationHandle): unknown {
		const op = operation as unknown as OpState
		if (op.planned.failure !== undefined) {
			throw op.planned.failure
		}
		return op.planned.result
	}

	function close(verb: string, request: unknown, callback: (report: CloseWire) => void): void {
		calls.push({ verb, request })
		const report = closePlans.get(verb)?.shift() ?? CLOSED
		queueMicrotask(() => callback(report))
	}

	const wire = {
		runtimeCancel(operation: OperationHandle, callback: (report: CloseWire) => void) {
			const op = operation as unknown as OpState
			op.cancelled = true
			counters.cancels += 1
			queueMicrotask(() => callback(CLOSED))
		},
		logErrorCodes: () => [],
		logHistoryOpen: (_runtime: unknown, _policy: unknown, request: unknown, callback: () => void) =>
			start("logHistoryOpen", request, callback),
		logHistoryTake: take,
		logHistoryCall: (capability: unknown, _policy: unknown, request: unknown, callback: () => void) =>
			start("logHistoryCall", { capability, request }, callback),
		logHistoryResult: take,
		logHistoryClose: (capability: unknown, callback: (report: CloseWire) => void) =>
			close("logHistoryClose", capability, callback),
		logSnapshotClose: (snapshot: unknown, callback: (report: CloseWire) => void) =>
			close("logSnapshotClose", snapshot, callback),
		logCommandSeal: (change: unknown, _policy: unknown, request: unknown, callback: () => void) =>
			start("logCommandSeal", { change, request }, callback),
		logCommandDecode: (_runtime: unknown, _policy: unknown, bytes: unknown, schema: unknown, callback: () => void) =>
			start("logCommandDecode", { bytes, schema }, callback),
		logCommandTake: take,
		logCommandEncode: (command: unknown, _policy: unknown, callback: () => void) =>
			start("logCommandEncode", command, callback),
		logBytesTake: take,
		logCommandClose: (command: unknown, callback: (report: CloseWire) => void) =>
			close("logCommandClose", command, callback),
		logCacheMake: (_runtime: unknown, _policy: unknown, request: unknown, callback: () => void) =>
			start("logCacheMake", request, callback),
		logCacheTake: take,
		logCacheAcquire: (cache: unknown, _policy: unknown, request: unknown, callback: () => void) =>
			start("logCacheAcquire", { cache, request }, callback),
		logBorrowTake: take,
		logCacheInspect: (cache: unknown, _policy: unknown, callback: () => void) =>
			start("logCacheInspect", cache, callback),
		logCacheInspectTake: take,
		logCacheEvict: (cache: unknown, _policy: unknown, request: unknown, callback: () => void) =>
			start("logCacheEvict", { cache, request }, callback),
		logCacheEvictTake: take,
		logBorrowRelease: (borrow: unknown, callback: (report: CloseWire) => void) =>
			close("logBorrowRelease", borrow, callback),
		logCacheClose: (cache: unknown, callback: (report: CloseWire) => void) => close("logCacheClose", cache, callback),
		logAdmin: (_runtime: unknown, _policy: unknown, request: unknown, callback: () => void) =>
			start("logAdmin", request, callback),
		logAdminTake: take
	}

	return {
		wire: wire as unknown as LogNative,
		calls,
		held,
		cancelCount: () => counters.cancels,
		plan(verb, planned) {
			const queue = plans.get(verb)
			if (queue === undefined) {
				plans.set(verb, [planned])
			} else {
				queue.push(planned)
			}
		},
		planClose(verb, report) {
			const queue = closePlans.get(verb)
			if (queue === undefined) {
				closePlans.set(verb, [report])
			} else {
				queue.push(report)
			}
		},
		releaseHeld() {
			for (const op of held.splice(0, held.length)) {
				if (!op.cancelled) {
					op.callback()
				}
			}
		}
	}
}

export const fakeRuntime = { __fake: "runtime" } as unknown as RuntimeHandle

/**
 * Registered change views for the seal path, mirroring the core's landed
 * `internalChanges` accessor exactly: `undefined` for a foreign object,
 * `{ handle, schemaId, closed }` for a registered ChangeSet. The stored
 * record is mutable so a test can flip `closed` (the spent capability
 * state); the machine reads the view through the readonly seam type.
 */
interface ChangeViewFixture {
	readonly handle: unknown
	readonly schemaId: string
	closed: boolean
}

const changeViews = new WeakMap<object, ChangeViewFixture>()

export function registerChange(
	changes: object,
	handle: unknown,
	options: { readonly schemaId?: string; readonly closed?: boolean } = {}
): void {
	changeViews.set(changes, {
		handle,
		schemaId: options.schemaId ?? identityWire.schemaId,
		closed: options.closed ?? false
	})
}

/** Marks a registered ChangeSet double closed (the spent capability state). */
export function closeRegisteredChange(changes: object): void {
	const view = changeViews.get(changes)
	if (view !== undefined) {
		view.closed = true
	}
}

export function makeIntegration(): CoreIntegration {
	const die = () => Effect.die(new Error("wire double: core reader is not exercised here"))
	// The reader is a deliberately inert stub: these language-layer tests
	// never execute core reads; the seam shape is what is typed.
	const reader = (() => ({ get: die, execute: die, session: die })) as unknown as CoreIntegration["reader"]
	return {
		reader,
		changes(value) {
			// The fixture's plain-string schemaId stands in for the branded
			// core SchemaId, exactly as the other wire fixtures do.
			return changeViews.get(value) as CoreChangesView | undefined
		},
		schemaSpec: (schema) => schema,
		runtime: () => Effect.succeed(fakeRuntime)
	}
}

// ── Shared fixtures ────────────────────────────────────────────────────────

export const work = {
	inputBytes: 1024n,
	workingBytes: 1024n,
	scratchBytes: 1024n,
	resultBytes: 1024n,
	rows: 64n,
	workUnits: 64n,
	timeout: 1000
}

export const submitOptions = {
	...work,
	attempts: 3,
	backoff: { baseMillis: 5, capMillis: 50 }
}

export const identityWire = {
	databaseId: "0f".repeat(16),
	incarnationId: "1e".repeat(16),
	schemaId: "2d".repeat(32)
}

export const otherIdentityWire = {
	databaseId: "aa".repeat(16),
	incarnationId: "bb".repeat(16),
	schemaId: "2d".repeat(32)
}

export function handleWire(identity: typeof identityWire = identityWire) {
	return {
		history: { __double: "history-capability" },
		meta: { identity, receiptEpoch: 1n }
	}
}

export const stampWire = { seq: 7n, hash: "3c".repeat(32) }
export const stateWire = { incarnation: identityWire.incarnationId, dataRevision: 4n }

export const refWire = {
	identity: identityWire,
	receiptEpoch: 1n,
	requestId: "4b".repeat(16),
	digest: "5a".repeat(32)
}

export const receiptWire = {
	command: refWire,
	decisionAt: stampWire,
	stateAt: stateWire,
	outcome: { kind: "no-change", result: { attempt: "6f".repeat(16) } }
}

export const localBinding = {
	kind: "local" as const,
	directory: "/tmp/bumbledb-double",
	identity: {
		databaseId: identityWire.databaseId,
		incarnationId: identityWire.incarnationId,
		schemaId: identityWire.schemaId
	}
} as unknown as import("#options.ts").LocalBinding

export const otherLocalBinding = {
	kind: "local" as const,
	directory: "/tmp/bumbledb-double-other",
	identity: {
		databaseId: otherIdentityWire.databaseId,
		incarnationId: otherIdentityWire.incarnationId,
		schemaId: otherIdentityWire.schemaId
	}
} as unknown as import("#options.ts").LocalBinding
