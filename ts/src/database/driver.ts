/**
 * Drives one sans-IO log machine. Requests and store responses go through one mailbox, so the
 * machine steps strictly one input at a time; every `IoRequest` a step emits runs concurrently
 * (bounded) and comes back as another input. Each request carries a ticket that settles exactly
 * once, and closing the driver settles every ticket still open.
 */
import type { Scope } from "effect"
import { Deferred, Effect, Fiber, FiberSet, Queue, Semaphore } from "effect"
import { DbError } from "../runtime-errors.ts"
import type { ExecutorOptions, IoRequest, IoResponse, ObjectStore } from "./io.ts"
import { defaultExecutorOptions, execute } from "./io.ts"

interface Step<Settled> {
	readonly io: readonly IoRequest[]
	readonly done: ReadonlyArray<{ readonly ticket: bigint; readonly settled: Settled }>
}

/** The machine as the driver sees it. The driver never calls it concurrently. */
interface MachinePort<Request, Settled> {
	readonly request: (ticket: bigint, request: Request) => Effect.Effect<Step<Settled>, DbError>
	readonly respond: (response: IoResponse) => Effect.Effect<Step<Settled>, DbError>
	/** Stops the machine; its step settles every open ticket. */
	readonly close: Effect.Effect<Step<Settled>, DbError>
}

interface DriverOptions {
	/** Store calls in flight at once. */
	readonly concurrency: number
	readonly executor: ExecutorOptions
}

interface Driver<Request, Settled> {
	/** Submits one request and waits for its settlement. */
	readonly run: (request: Request) => Effect.Effect<Settled, DbError>
}

type Message<Request> =
	| { readonly _tag: "Request"; readonly ticket: bigint; readonly request: Request }
	| { readonly _tag: "Response"; readonly response: IoResponse }
	| { readonly _tag: "Close" }

const defaultDriverOptions: DriverOptions = { concurrency: 32, executor: defaultExecutorOptions }

/** Starts a driver owned by the current scope; closing the scope closes the machine. */
const make = Effect.fnUntraced(function* <Request, Settled>(
	port: MachinePort<Request, Settled>,
	store: ObjectStore,
	options: DriverOptions = defaultDriverOptions
): Effect.fn.Return<Driver<Request, Settled>, never, Scope.Scope> {
	const mailbox = yield* Queue.unbounded<Message<Request>>()
	const permits = yield* Semaphore.make(options.concurrency)
	const io = yield* FiberSet.make()
	const tickets = new Map<bigint, Deferred.Deferred<Settled, DbError>>()
	let nextTicket = 0n
	const closed = new DbError({ operation: "Database", reason: { _tag: "ClosedHandle" } })

	const apply = (step: Step<Settled>) =>
		Effect.gen(function* () {
			for (const { ticket, settled } of step.done) {
				const waiter = tickets.get(ticket)
				tickets.delete(ticket)
				if (waiter !== undefined) yield* Deferred.succeed(waiter, settled)
			}
			for (const request of step.io) {
				yield* FiberSet.run(
					io,
					Semaphore.withPermit(permits, execute(store, request, options.executor)).pipe(
						Effect.flatMap((response) => Queue.offer(mailbox, { _tag: "Response", response }))
					)
				)
			}
		})

	let stopped: DbError | undefined

	/** Ends the driver: later runs fail with `error`, and open tickets fail with it now. */
	const stop = (error: DbError) =>
		Effect.suspend(() => {
			stopped ??= error
			const open = [...tickets.values()]
			tickets.clear()
			return Effect.forEach(open, (waiter) => Deferred.fail(waiter, error), { discard: true })
		})

	const stepFor = (message: Message<Request>) => {
		switch (message._tag) {
			case "Request":
				return port.request(message.ticket, message.request)
			case "Response":
				return port.respond(message.response)
			case "Close":
				return port.close
		}
	}

	const loop: Effect.Effect<void> = Effect.gen(function* () {
		while (true) {
			const message = yield* Queue.take(mailbox)
			const step = stepFor(message)
			const failed = yield* step.pipe(
				Effect.flatMap(apply),
				Effect.as(false),
				Effect.catch((error) => Effect.as(stop(error), true))
			)
			if (failed || message._tag === "Close") return yield* stop(closed)
		}
	})

	const machine = yield* Effect.forkScoped(loop)
	yield* Effect.addFinalizer(() =>
		Queue.offer(mailbox, { _tag: "Close" }).pipe(
			Effect.andThen(Fiber.await(machine)),
			Effect.andThen(FiberSet.clear(io))
		)
	)

	return {
		run: (request: Request) =>
			Effect.suspend(() => {
				if (stopped !== undefined) return Effect.fail(stopped)
				const waiter = Deferred.makeUnsafe<Settled, DbError>()
				const ticket = nextTicket
				nextTicket += 1n
				tickets.set(ticket, waiter)
				Queue.offerUnsafe(mailbox, { _tag: "Request", ticket, request })
				return Deferred.await(waiter)
			})
	}
})

const Driver = Object.freeze({ make, defaultOptions: defaultDriverOptions })

export type { DriverOptions, MachinePort, Step }
export { Driver }
