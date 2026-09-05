/**
 * One note: GET reads through the shared QueryReader capability; PATCH is
 * the witnessed pin toggle (exact-state precondition — an intervening net
 * change is a durable precondition-failed receipt, never a silent
 * overwrite). A retry of the SAME revision decision reuses the SAME
 * request key; a NEW decision after a conflict mints a new one.
 */
import { encodeBoundaryRows, Id128 } from "@bjornpagen/bumbledb"
import { Effect, Option, Result } from "effect"
import { requirePrincipal } from "../../../../src/auth.ts"
import { bindingFor } from "../../../../src/db/bindings.ts"
import { setPinned } from "../../../../src/db/commands.ts"
import { getNote } from "../../../../src/db/reads.ts"
import { Note } from "../../../../src/db/schema.ts"
import { requestPolicy } from "../../../../src/db/runtime-policy.ts"
import { appRuntime, Databases } from "../../../../src/db/server.ts"
import { exitResponse, respond, submitResponse } from "../../../../src/http.ts"

export const runtime = "nodejs"
export const dynamic = "force-dynamic"

const readNote = Effect.fn("routes.readNote")(
	function* (request: Request, rawId: string) {
		const principal = yield* requirePrincipal(request)
		const binding = yield* bindingFor(principal.tenantId)
		const id = yield* Effect.fromResult(Id128.fromHex(rawId))
		const work = requestPolicy(request)
		const databases = yield* Databases
		const db = yield* databases.acquire(binding, work)
		const snapshot = yield* db.snapshot({ ...work, consistency: { kind: "latest" } })
		const found = yield* getNote(snapshot, id, work)
		if (Option.isNone(found)) {
			return Response.json({ error: "NotFound" }, { status: 404 })
		}
		const body = yield* Effect.fromResult(encodeBoundaryRows(Note, [found.value]))
		return Response.json(body[0], { headers: { "Cache-Control": "private, no-store" } })
	},
	Effect.scoped
)

const patchNote = Effect.fn("routes.patchNote")(
	function* (request: Request, rawId: string, body: { readonly requestKey: string; readonly pinned: boolean }) {
		const principal = yield* requirePrincipal(request)
		const binding = yield* bindingFor(principal.tenantId)
		const id = yield* Effect.fromResult(Id128.fromHex(rawId))
		const requestKey = yield* Effect.fromResult(Id128.fromHex(body.requestKey))
		const work = requestPolicy(request)
		const databases = yield* Databases
		const db = yield* databases.acquire(binding, work)
		const result = yield* setPinned(db, principal.tenantId, requestKey, id, body.pinned, work)
		if (result.kind === "missing") {
			return Response.json({ error: "NotFound" }, { status: 404 })
		}
		return submitResponse(result.outcome)
	},
	Effect.scoped
)

export async function GET(request: Request, context: { params: Promise<{ id: string }> }): Promise<Response> {
	const { id } = await context.params
	const exit = await appRuntime.runPromiseExit(respond(readNote(request, id)), { signal: request.signal })
	return exitResponse(exit)
}

export async function PATCH(request: Request, context: { params: Promise<{ id: string }> }): Promise<Response> {
	const { id } = await context.params
	const parsed = await parseBody(request)
	if (Result.isFailure(parsed)) {
		return Response.json({ error: "InvalidBody" }, { status: 400 })
	}
	const exit = await appRuntime.runPromiseExit(respond(patchNote(request, id, parsed.success)), {
		signal: request.signal
	})
	return exitResponse(exit)
}

async function parseBody(request: Request): Promise<Result.Result<{ requestKey: string; pinned: boolean }, string>> {
	try {
		const raw: unknown = await request.json()
		if (typeof raw !== "object" || raw === null) {
			return Result.fail("not an object")
		}
		if (!("requestKey" in raw) || !("pinned" in raw)) {
			return Result.fail("bad fields")
		}
		const requestKey = raw.requestKey
		const pinned = raw.pinned
		if (typeof requestKey !== "string" || typeof pinned !== "boolean") {
			return Result.fail("bad fields")
		}
		return Result.succeed({ requestKey, pinned })
	} catch {
		return Result.fail("not json")
	}
}
