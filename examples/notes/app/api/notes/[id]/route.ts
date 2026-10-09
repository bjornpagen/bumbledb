/**
 * One note. GET reads the latest state; PATCH sets the pin at the revision it read, so an
 * intervening change is a failed precondition, never a silent overwrite. A retry reuses its
 * request key; a new decision after a conflict uses a new one.
 */
import { encodeBoundaryRows, Uuid } from "@bjornpagen/bumbledb"
import { Effect, Option, Result } from "effect"
import { requirePrincipal } from "../../../../src/auth.ts"
import { setPinned } from "../../../../src/db/commands.ts"
import { getNote } from "../../../../src/db/reads.ts"
import { Note } from "../../../../src/db/schema.ts"
import { appRuntime, Databases } from "../../../../src/db/server.ts"
import { exitResponse, respond, submitResponse } from "../../../../src/http.ts"

export const runtime = "nodejs"
export const dynamic = "force-dynamic"

const readNote = Effect.fn("routes.readNote")(
	function* (request: Request, rawId: string) {
		const principal = yield* requirePrincipal(request)
		const id = yield* Effect.fromResult(Uuid.parse(rawId))
		const db = yield* (yield* Databases).get(principal.tenantId)
		const found = yield* getNote(yield* db.read("latest"), id)
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
		const id = yield* Effect.fromResult(Uuid.parse(rawId))
		const requestKey = yield* Effect.fromResult(Uuid.parse(body.requestKey))
		const db = yield* (yield* Databases).get(principal.tenantId)
		const result = yield* setPinned(db, requestKey, id, body.pinned)
		if (result._tag === "Missing") {
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
