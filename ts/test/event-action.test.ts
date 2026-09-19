import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, Exit, Fiber, ManagedRuntime, Result } from "effect"
import { dbNative } from "#db-native.ts"
import { encodeAction } from "#event-action-data.ts"
import {
	ChangeSet,
	Db,
	Event,
	EventAction,
	type EventActionArenaDescription,
	type EventActionDescription,
	EventDescriptor,
	type EventFibreDescription,
	ExactPolynomial,
	ExactRational,
	event,
	NativeRuntime,
	ParameterDomain,
	ParameterRegion,
	PolynomialSigns,
	query,
	relation,
	schema,
	u64,
	v
} from "#index.ts"
import { nativeBindingIsLoaded } from "#native.ts"
import { nativeOperationWith, runtimeHandle } from "#runtime.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

const id = (n: number) => new Uint8Array(32).fill(n)
const bytes = (v: EventAction) => Buffer.from(EventAction.toBytes(v))
const ev = (v: Event) => Buffer.from(Event.toBytes(v))
async function run<A, E>(program: Effect.Effect<A, E, NativeRuntime>) {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		return await runtime.runPromise(program)
	} finally {
		await runtime.dispose()
	}
}
function at<T>(values: readonly T[], i: number): T {
	const value = values[i]
	assert.ok(value !== undefined)
	return value
}
const region = (source: Event, width: number, codes: readonly number[]) =>
	Effect.gen(function* () {
		let out = yield* Event.empty(source)
		for (const code of codes) {
			let value = source
			for (let bit = 0; bit < width; bit++) {
				const part = yield* Event.coordinate(source, BigInt(bit))
				value = yield* Event.and(value, code & (1 << bit) ? part : yield* Event.complement(part))
			}
			out = yield* Event.or(out, value)
		}
		return out
	})
const fixture = (edges: readonly number[] = [0, 5, 6, 7], domain?: ParameterDomain) =>
	Effect.gen(function* () {
		const source = (n: number, width: bigint) =>
			Effect.gen(function* () {
				const value = yield* Event.space(id(n), width)
				return domain === undefined ? value : yield* Event.withParameters(value, domain, [])
			})
		const states = yield* source(180, 1n)
		const choices = yield* source(181, 1n)
		const env = yield* source(182, 0n)
		const stateMap = { source: states, target: env, readouts: [] }
		const actions: EventFibreDescription = {
			identity: id(183),
			left: stateMap,
			right: { source: choices, target: env, readouts: [] },
			reversed: false
		}
		const sa = yield* EventDescriptor.inspect(yield* EventDescriptor.admit({ kind: "fibre", product: actions }))
		assert.equal(sa.kind, "fibre")
		assert.ok("space" in sa)
		const product: EventFibreDescription = {
			identity: id(184),
			left: { source: sa.space, target: env, readouts: [] },
			right: stateMap,
			reversed: false
		}
		const sat = yield* EventDescriptor.inspect(yield* EventDescriptor.admit({ kind: "fibre", product }))
		assert.equal(sat.kind, "fibre")
		assert.ok("space" in sat)
		const arena: EventActionArenaDescription = {
			actions,
			transition: { product, region: yield* region(sat.space, 3, edges) }
		}
		return {
			data: { kind: "arena", arena } satisfies EventActionDescription,
			states,
			pairs: sa.space,
			goal: yield* Event.coordinate(states, 0n)
		}
	})

test("action APIs are lazy and pure BEAC carriers own their bytes", () => {
	assert.equal(nativeBindingIsLoaded(), false)
	const input = Buffer.from("BEAC\x01")
	const value = Result.getOrThrow(EventAction.fromBytes(input))
	input.fill(0)
	const copy = EventAction.toBytes(value)
	copy.fill(0)
	assert.deepEqual(bytes(value), Buffer.from("BEAC\x01"))
	assert.equal(EventAction.isAction(value), true)
	assert.equal(EventAction.isAction({}), false)
	assert.equal(Result.isFailure(EventAction.fromBytes(Buffer.from("BEBA\x01"))), true)
	assert.equal([EventAction.admit({} as never), EventAction.describe(value), EventAction.inspect(value)].length, 3)
	assert.equal(nativeBindingIsLoaded(), false)
})

test("reopened reach and safety strategies retain checked choices, ranks and ordinary queryable Event regions", async () => {
	const retained = await run(
		Effect.gen(function* () {
			const data = yield* fixture()
			const arena = yield* EventAction.admit(data.data)
			const enabled = yield* EventDescriptor.inspect(yield* EventAction.enabled(arena))
			assert.ok("region" in enabled)
			assert.equal(yield* Event.isFull(enabled.region), true)
			const good = yield* EventDescriptor.inspect(yield* EventAction.good(arena, data.goal))
			assert.ok("region" in good)
			assert.equal(yield* Event.contains(good.region, 0n), false)
			assert.equal(yield* Event.isFull(yield* EventAction.predecessor(arena, data.goal)), true)
			const reach = yield* EventAction.reach(arena, data.goal)
			const result = yield* EventAction.inspect(reach)
			assert.equal(result.kind, "reach")
			assert.ok(result.kind === "reach")
			assert.equal(yield* Event.isFull(result.winning), true)
			assert.equal(result.ranks.length, 2)
			assert.deepEqual(ev(at(result.ranks, 0)), ev(data.goal))
			const selected = yield* region(data.pairs, 2, [2])
			const chosen = yield* EventAction.withPolicy(reach, selected)
			const description = yield* EventAction.describe(chosen)
			assert.ok(description.kind === "reach" && description.policy !== null)
			assert.deepEqual(ev(description.policy), ev(selected))
			assert.deepEqual(bytes(yield* EventAction.admit(description)), bytes(chosen))
			assert.ok(yield* Effect.flip(EventAction.withPolicy(reach, yield* region(data.pairs, 2, [0]))))
			assert.ok(yield* Effect.flip(EventAction.withPolicy(reach, yield* Event.empty(data.pairs))))
			assert.ok(yield* Effect.flip(EventAction.withPolicy(arena, selected)))
			const safe = yield* EventAction.safe(arena, data.states)
			const safeChosen = yield* EventAction.withPolicy(safe, yield* region(data.pairs, 2, [1, 2]))
			const safeDescription = yield* EventAction.describe(safeChosen)
			assert.ok(safeDescription.kind === "safe" && safeDescription.policy !== null)
			assert.equal(yield* Event.count(safeDescription.policy), 2n)
			assert.deepEqual(bytes(yield* EventAction.admit(safeDescription)), bytes(safeChosen))
			assert.ok(yield* Effect.flip(EventAction.withPolicy(safeChosen, data.pairs)))
			return { chosen, selected, result, safeChosen }
		})
	)
	await run(
		Effect.scoped(
			Effect.gen(function* () {
				const input = EventAction.toBytes(retained.chosen)
				const reopened = Result.getOrThrow(EventAction.fromBytes(input))
				input.fill(0)
				const result = yield* EventAction.inspect(reopened)
				assert.ok(result.kind === "reach")
				assert.deepEqual(result.ranks.map(ev), retained.result.ranks.map(ev))
				const described = yield* EventAction.describe(reopened)
				assert.ok(described.kind === "reach" && described.policy !== null)
				const safe = yield* EventAction.inspect(retained.safeChosen)
				assert.equal(safe.kind, "safe")
				const Policy = relation("Policy", { id: u64, when: event })
				const Expected = relation("Expected", { id: u64, when: event })
				const Theory = schema("ActionReplay", { Policy, Expected }, [])
				const db = yield* Db.create(storeDir("action-replay"), Theory)
				const draft = yield* ChangeSet.builder(Theory)
				yield* draft.insert(Policy, [{ id: 1n, when: described.policy }])
				yield* draft.insert(Expected, [{ id: 1n, when: retained.selected }])
				yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })
				const scan = query(Theory).rule((r) => {
					const row = v(Policy)
					return r.match(Policy, row).match(Expected, row).find(row)
				})
				const snapshot = yield* db.snapshot()
				assert.equal((yield* (yield* snapshot.execute(scan, {})).collect()).length, 1)
			})
		)
	)
})

test("strict descriptions and native admission reject forged roles, objectives, policies and envelope contents", async () => {
	await run(
		Effect.gen(function* () {
			const data = yield* fixture([4, 5, 6, 7])
			const arena = yield* EventAction.admit(data.data)
			const reach = yield* EventAction.reach(arena, data.goal)
			const chosen = yield* EventAction.withPolicy(reach, yield* region(data.pairs, 2, [0]))
			const selected = yield* EventAction.describe(chosen)
			assert.ok(selected.kind === "reach")
			assert.deepEqual(bytes(yield* EventAction.admit(selected)), bytes(chosen))
			const desc = yield* EventAction.describe(reach)
			assert.ok(desc.kind === "reach")
			assert.deepEqual(bytes(yield* EventAction.admit({ ...desc, policy: null })), bytes(reach))
			const foreign = yield* Event.space(id(189), 1n)
			for (const bad of [
				{ ...data.data, winner: data.states },
				{ ...data.data, arena: { ...data.data.arena, actions: { ...data.data.arena.actions, identity: id(190) } } },
				{ ...desc, goal: foreign },
				{ ...desc, goal: data.states },
				{ ...desc, policy: yield* Event.empty(data.pairs) },
				{ ...desc, policy: undefined }
			])
				assert.ok(yield* Effect.flip(EventAction.admit(bad as EventActionDescription)))
			for (const op of [EventAction.good, EventAction.predecessor, EventAction.reach, EventAction.safe]) {
				const attempt: Effect.Effect<unknown, unknown, NativeRuntime> = op(arena, foreign)
				assert.ok(yield* Effect.flip(attempt))
			}
			const encoded = EventAction.toBytes(chosen)
			for (const bad of [
				encoded.subarray(0, 5),
				encoded.subarray(0, encoded.length - 1),
				new Uint8Array([...encoded, 0])
			]) {
				assert.ok(yield* Effect.flip(EventAction.inspect(Result.getOrThrow(EventAction.fromBytes(bad)))))
			}
			const runtime = yield* runtimeHandle()
			for (const [op, input, operands] of [
				["inspect", encoded, [Event.toBytes(data.goal)]],
				["reach", encoded, []],
				["admit", { kind: "arena", arena: {}, forged: true }, []],
				["admit", { ...encodeAction(desc), policy: undefined }, []],
				["fromMemory", encoded, []]
			] as const)
				assert.ok(
					yield* Effect.flip(
						nativeOperationWith(
							"raw action refusal",
							(cb) => dbNative.runtimeEventAction(runtime, op, input as never, operands, cb),
							dbNative.runtimeBytesTake,
							(v) => v
						)
					)
				)
		})
	)
})

test("independently authored BEAC and raw worker submission retain copied data and exact policy semantics", async () => {
	await run(
		Effect.gen(function* () {
			const data = yield* fixture()
			const value = yield* EventAction.admit(data.data)
			const integer = (n: number) => {
				const b = Buffer.alloc(8)
				b.writeBigUInt64LE(BigInt(n))
				return b
			}
			const blob = (v: Event) => Buffer.concat([integer(Event.toBytes(v).length), Event.toBytes(v)])
			const map = (v: EventFibreDescription["left"]) =>
				Buffer.concat([blob(v.source), blob(v.target), integer(v.readouts.length), ...v.readouts.map(blob)])
			const fibre = (v: EventFibreDescription) =>
				Buffer.concat([v.identity, Buffer.from([Number(v.reversed)]), map(v.left), map(v.right)])
			const arena = data.data.arena
			const common = Buffer.concat([
				fibre(arena.actions),
				fibre(arena.transition.product),
				blob(arena.transition.region)
			])
			const authored = Buffer.concat([Buffer.from("BEAC\x01\0"), common])
			assert.deepEqual(bytes(value), authored)
			const reach = yield* EventAction.reach(value, data.goal)
			const recipe = Buffer.concat([Buffer.from("BEAC\x01\x01"), common, blob(data.goal), Buffer.from([0])])
			const replay = yield* EventAction.describe(Result.getOrThrow(EventAction.fromBytes(recipe)))
			assert.deepEqual(bytes(yield* EventAction.admit(replay)), bytes(reach))
			const runtime = yield* runtimeHandle()
			const wire = encodeAction(data.data)
			const copied = yield* nativeOperationWith(
				"raw action copy",
				(cb) => {
					const operation = dbNative.runtimeEventAction(runtime, "admit", wire, [], cb)
					wire.arena.actions.identity.fill(0)
					wire.arena.transition.region.fill(0)
					wire.arena.actions.left.source.fill(0)
					return operation
				},
				dbNative.runtimeBytesTake,
				(bytes) => Result.getOrThrow(EventAction.fromBytes(bytes))
			)
			assert.deepEqual(bytes(copied), bytes(value))
			const encoded = EventAction.toBytes(value)
			const goal = Event.toBytes(data.goal)
			const copiedReach = yield* nativeOperationWith(
				"raw objective copy",
				(cb) => {
					const operation = dbNative.runtimeEventAction(runtime, "reach", encoded, [goal], cb)
					encoded.fill(0)
					goal.fill(0)
					return operation
				},
				dbNative.runtimeBytesTake,
				(bytes) => Result.getOrThrow(EventAction.fromBytes(bytes))
			)
			assert.deepEqual(bytes(copiedReach), bytes(reach))
			const fiber = yield* Effect.forkChild(Effect.forever(EventAction.inspect(reach)))
			yield* Effect.yieldNow
			yield* Fiber.interrupt(fiber)
			assert.equal(Exit.isFailure(yield* Fiber.await(fiber)), true)
			assert.equal((yield* EventAction.inspect(reach)).kind, "reach")
		})
	)
})

test("general action replay retains one unknown parameter without inventing a prior or eliminating worlds", async () => {
	await run(
		Effect.gen(function* () {
			const name = id(195)
			const p = yield* ExactPolynomial.parameter(name)
			const one = yield* ExactPolynomial.constant(yield* ExactRational.fraction(1n))
			const domain = yield* ParameterDomain.new(
				yield* ParameterRegion.and(
					yield* ParameterRegion.whereSign(name, p, PolynomialSigns.nonNegative),
					yield* ParameterRegion.whereSign(name, yield* ExactPolynomial.subtract(one, p), PolynomialSigns.nonNegative)
				)
			)
			const data = yield* fixture(undefined, domain)
			const arena = yield* EventAction.admit(data.data)
			const result = yield* EventAction.inspect(yield* EventAction.reach(arena, data.goal))
			assert.ok(result.kind === "reach")
			assert.deepEqual(ev(result.states), ev(data.states))
			assert.equal(yield* Event.isFull(result.winning), true)
			const desc = yield* EventAction.describe(yield* EventAction.safe(arena, data.states))
			assert.ok(desc.kind === "safe" && desc.policy !== null)
			assert.deepEqual(ev(desc.arena.actions.left.source), ev(data.states))
			assert.deepEqual(bytes(yield* EventAction.admit(desc)), bytes(yield* EventAction.safe(arena, data.states)))
		})
	)
})

test("one-step action queries retain distinct outcome roles while iterative solvers refuse them", async () => {
	await run(
		Effect.gen(function* () {
			const input = yield* fixture()
			const outcomes = yield* Event.space(id(196), 1n)
			const original = input.data.arena
			const product: EventFibreDescription = {
				identity: id(197),
				left: original.transition.product.left,
				right: { ...original.transition.product.right, source: outcomes },
				reversed: false
			}
			const pairs = yield* EventDescriptor.inspect(yield* EventDescriptor.admit({ kind: "fibre", product }))
			assert.ok("space" in pairs)
			const value = yield* EventAction.admit({
				kind: "arena",
				arena: { actions: original.actions, transition: { product, region: pairs.space } }
			})
			const described = yield* EventAction.inspect(value)
			assert.deepEqual(ev(described.outcomes), ev(outcomes))
			assert.equal(yield* Event.isFull(yield* EventAction.predecessor(value, outcomes)), true)
			assert.ok(yield* Effect.flip(EventAction.reach(value, input.states)))
			assert.ok(yield* Effect.flip(EventAction.safe(value, input.states)))
			assert.ok(yield* Effect.flip(EventAction.good(value, input.states)))
		})
	)
})
