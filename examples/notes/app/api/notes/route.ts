/**
 * Notes collection routes: Node runtime, dynamic, authenticated before any open. The request
 * signal enters only at the runtime boundary, as fiber interruption.
 */
import { encodeBoundaryRows, Uuid } from "@bjornpagen/bumbledb"
import { Effect, Result } from "effect"
import { requirePrincipal } from "../../../src/auth.ts"
import { createNote } from "../../../src/db/commands.ts"
import { listNotes as collectNotes } from "../../../src/db/reads.ts"
import { Note } from "../../../src/db/schema.ts"
import { appRuntime, Databases } from "../../../src/db/server.ts"
import { exitResponse, respond, submitResponse } from "../../../src/http.ts"

export const runtime = "nodejs"
export const dynamic = "force-dynamic"

const listNotes = Effect.fn("routes.listNotes")(
	function* (request: Request) {
		const principal = yield* requirePrincipal(request)
		const db = yield* (yield* Databases).get(principal.tenantId)
		const rows = yield* collectNotes(yield* db.read("cached"))
		const body = yield* Effect.fromResult(encodeBoundaryRows(Note, rows))
		return Response.json(body, { headers: { "Cache-Control": "private, no-store" } })
	},
	Effect.scoped
)

/**
 * Create takes a client-supplied note id, generated once per intent and reused on retries: the
 * same id submits the identical command, decided once. Cross-origin writes are refused.
 */
const postNote = Effect.fn("routes.postNote")(
	function* (request: Request, body: { readonly id: string; readonly text: string }) {
		const principal = yield* requirePrincipal(request)
		const noteId = yield* Effect.fromResult(Uuid.parse(body.id))
		const db = yield* (yield* Databases).get(principal.tenantId)
		const outcome = yield* createNote(db, noteId, body.text)
		return submitResponse(outcome)
	},
	Effect.scoped
)

export async function GET(request: Request): Promise<Response> {
	const exit = await appRuntime.runPromiseExit(respond(listNotes(request)), { signal: request.signal })
	return exitResponse(exit)
}

export async function POST(request: Request): Promise<Response> {
	const allowed = process.env.APP_ORIGIN
	const origin = request.headers.get("origin")
	if (origin !== null && allowed !== undefined && origin !== allowed) {
		return Response.json({ error: "ForbiddenOrigin" }, { status: 403 })
	}
	const parsed = await parseBody(request)
	if (Result.isFailure(parsed)) {
		return Response.json({ error: "InvalidBody" }, { status: 400 })
	}
	const exit = await appRuntime.runPromiseExit(respond(postNote(request, parsed.success)), {
		signal: request.signal
	})
	return exitResponse(exit)
}

const TEXT_LIMIT = 16_384

async function parseBody(request: Request): Promise<Result.Result<{ id: string; text: string }, string>> {
	try {
		const raw: unknown = await request.json()
		if (typeof raw !== "object" || raw === null) {
			return Result.fail("not an object")
		}
		if (!("id" in raw) || !("text" in raw)) {
			return Result.fail("bad fields")
		}
		const id = raw.id
		const text = raw.text
		if (typeof id !== "string" || typeof text !== "string" || text.length === 0 || text.length > TEXT_LIMIT) {
			return Result.fail("bad fields")
		}
		return Result.succeed({ id, text })
	} catch {
		return Result.fail("not json")
	}
}
