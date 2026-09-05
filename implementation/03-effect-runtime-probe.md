# 03 — Version-matched Effect runtime probe

Date: 2026-09-04. All seven groups below passed with Node v26.4.0 and Effect 4.0.0-rc.112. This is review evidence, **not a new release framework or a Bumbledb implementation test**. It checks the library semantics that the proposed API depends on. It does not prove native late-success reclamation, bounded cleanup, event-loop fairness, memory accounting, performance or S3 publication certainty.

The exact executed source is preserved below. It ran as a temporary `effect-runtime-probe.mjs` outside the repository. To reproduce, save this block as an .mjs file and run it with Node; adjust only the absolute import path to an installation of the same Effect version. Promises are used for the test harness's outer boundary and controlled mock completion gates, not as a proposed database API. No Bumbledb modules, native addon or real database are loaded.

```js
// Effect 4 runtime probes only. No Bumbledb implementation is exercised.
import assert from "node:assert/strict"
import { setImmediate as nextTurn } from "node:timers/promises"
import { Cause, Context, Effect, Exit, Fiber, Layer, ManagedRuntime, Option, Stream } from "/Users/bjorn/Documents/edullm/node_modules/effect/dist/index.js"

let checks = 0
const passed = (name) => { checks++; console.log(`PASS ${name}`) }
const gate = () => Promise.withResolvers()

let executions = 0
const operation = Effect.fn("probe.operation")(function* () {
  return yield* Effect.sync(() => ++executions)
})
const lazy = operation()
assert.equal(executions, 0)
assert.equal(await Effect.runPromise(lazy), 1)
assert.equal(await Effect.runPromise(lazy), 2)
passed("Effect.fn is lazy and the same Effect reruns")

for (const shouldFail of [false, true]) {
  const released = []
  const exit = await Effect.runPromiseExit(Effect.scoped(Effect.gen(function* () {
    yield* Effect.acquireRelease(Effect.succeed("owner"), (_, exit) => Effect.sync(() => {
      released.push(exit._tag)
    }))
    if (shouldFail) return yield* Effect.fail("expected")
    return 42
  })))
  assert.equal(exit._tag, shouldFail ? "Failure" : "Success")
  assert.deepEqual(released, [shouldFail ? "Failure" : "Success"])
}
passed("Scope releases exactly once after success and typed failure")

for (const mode of ["take", "eof", "failure"]) {
  let pulls = 0
  let closes = 0
  const pages = Stream.unwrap(Effect.gen(function* () {
    yield* Effect.acquireRelease(Effect.void, () => Effect.sync(() => { closes++ }))
    return Stream.paginate(0, (position) => Effect.sync(() => {
      pulls++
      return [[[position, position + 10]], position < 2 ? Option.some(position + 1) : Option.none()]
    }))
  }))
  if (mode === "failure") {
    const exit = await Effect.runPromiseExit(pages.pipe(Stream.runForEach(() => Effect.fail("consumer"))))
    assert.equal(exit._tag, "Failure")
    assert.equal(pulls, 1)
  } else {
    const consumed = mode === "take" ? pages.pipe(Stream.take(1)) : pages
    const result = await Effect.runPromise(Stream.runCollect(consumed))
    assert.deepEqual(result, mode === "take" ? [[0, 10]] : [[0, 10], [1, 11], [2, 12]])
    assert.equal(pulls, mode === "take" ? 1 : 3)
  }
  assert.equal(closes, 1)
}
passed("page arrays, pull backpressure and cleanup on take/EOF/downstream failure")

const registered = gate()
const cleanupStarted = gate()
const drain = gate()
let cleanupFinished = false
let nativeSignal
const pending = Effect.callback((_resume, signal) => {
  nativeSignal = signal
  registered.resolve()
  return Effect.gen(function* () {
    cleanupStarted.resolve()
    yield* Effect.promise(() => drain.promise)
    cleanupFinished = true
  })
})
const pendingFiber = Effect.runFork(pending)
await registered.promise
let interrupted = false
const interruption = Effect.runPromise(Fiber.interrupt(pendingFiber)).then(() => { interrupted = true })
await cleanupStarted.promise
await nextTurn()
assert.equal(nativeSignal.aborted, true)
assert.equal(interrupted, false)
assert.equal(cleanupFinished, false)
drain.resolve()
await interruption
assert.equal(cleanupFinished, true)
assert.equal(Exit.hasInterrupts(await Effect.runPromise(Fiber.await(pendingFiber))), true)
passed("callback interruption aborts signal and joins asynchronous cleanup")

const acquiring = gate()
let cancelledAcquisition = false
let releasedUnacquired = false
const resourceFiber = Effect.runFork(Effect.scoped(Effect.acquireRelease(
  Effect.callback(() => {
    acquiring.resolve()
    return Effect.sync(() => { cancelledAcquisition = true })
  }),
  () => Effect.sync(() => { releasedUnacquired = true }),
  { interruptible: true }
)))
await acquiring.promise
await Effect.runPromise(Fiber.interrupt(resourceFiber))
assert.equal(cancelledAcquisition, true)
assert.equal(releasedUnacquired, false)
passed("interruptible acquire uses acquisition cleanup before any resource is acquired")

const closeFailure = { _tag: "CloseFailure", kind: "incomplete" }
const receipt = { kind: "decided", seq: 7 }
let observed
const finalizerExit = await Effect.runPromiseExit(Effect.scoped(Effect.gen(function* () {
  yield* Effect.acquireRelease(Effect.void, () => Effect.die(closeFailure))
  observed = receipt
  return receipt
})))
assert.equal(observed, receipt)
assert.equal(Exit.isFailure(finalizerExit), true)
assert.equal(finalizerExit.cause.reasons.some((reason) => Cause.isDieReason(reason) && reason.defect === closeFailure), true)
passed("a known result stays observed while finalizer defect fails the enclosing scope")

let opens = 0
let closes = 0
class Shared extends Context.Service()("probe/Shared") {}
class Consumer extends Context.Service()("probe/Consumer") {}
const sharedLayer = Layer.effect(Shared, Effect.acquireRelease(
  Effect.sync(() => ({ id: ++opens })),
  () => Effect.sync(() => { closes++ })
))
const consumerLayer = Layer.effect(Consumer, Effect.gen(function* () {
  return { shared: yield* Shared }
})).pipe(Layer.provideMerge(sharedLayer))
const runtime = ManagedRuntime.make(Layer.merge(consumerLayer, sharedLayer))
const readService = Effect.gen(function* () {
  const shared = yield* Shared
  const consumer = yield* Consumer
  assert.equal(shared, consumer.shared)
  return shared.id
})
assert.deepEqual(await Promise.all([runtime.runPromise(readService), runtime.runPromise(readService)]), [1, 1])
assert.equal(opens, 1)
assert.equal(closes, 0)
await Effect.runPromise(runtime.disposeEffect)
assert.equal(closes, 1)
passed("one reused Layer memoizes across a graph and concurrent ManagedRuntime calls")
console.log(`${checks} runtime probe groups passed; no native DB or performance qualification`)
```

The separate compile-only mock also passed with TypeScript 7.0.2 (strict, noEmit, NodeNext, skipLibCheck). It checked Effect.fn/scoped inference, Context.Service/Layer.effect/provideMerge, ManagedRuntime signal/disposeEffect, interruptible acquireRelease, callback cleanup, Result conversion, catchReason's retained parent error channel, and page-array Stream.unwrap/paginate types. Proposed Bumbledb declarations still need fresh packed-consumer qualification when implemented.
