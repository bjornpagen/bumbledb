/**
 * Notes collection routes — server-only Node runtime, dynamic (never
 * cached across identities), authenticated before any open. The request
 * abort signal enters ONLY at the ManagedRuntime run boundary as fiber
 * interruption. All database work is Effect; the Promise below is the
 * framework boundary, not a database API.
 */
import { encodeBoundaryRows, Id128 } from "@bjornpagen/bumbledb"
import { Effect, Result } from "effect"
import { requirePrincipal } from "../../../src/auth.ts"
import { bindingFor } from "../../../src/db/bindings.ts"
import { createNote } from "../../../src/db/commands.ts"
import { listNotes as collectNotes } from "../../../src/db/reads.ts"
import { Note } from "../../../src/db/schema.ts"
import { requestPolicy } from "../../../src/db/runtime-policy.ts"
import { appRuntime, Databases } from "../../../src/db/server.ts"
import { exitResponse, respond, submitResponse } from "../../../src/http.ts"

export const runtime = "nodejs"
export const dynamic = "force-dynamic"

const listNotes = Effect.fn("routes.listNotes")(
	function* (request: Request) {
		const principal = yield* requirePrincipal(request)
		const binding = yield* bindingFor(principal.tenantId)
		const work = requestPolicy(request)
		const databases = yield* Databases
		const db = yield* databases.acquire(binding, work)
		const snapshot = yield* db.snapshot({ ...work, consistency: { kind: "cached" } })
		const rows = yield* collectNotes(snapshot, work)
		const body = yield* Effect.fromResult(encodeBoundaryRows(Note, rows))
		return Response.json(body, { headers: { "Cache-Control": "private, no-store" } })
	},
	Effect.scoped
)

/**
 * Create requires a client-supplied note id (32 lowercase hex): the id is
 * generated ONCE by the caller for the original intent and reused on
 * retries — the same id builds the identical command and the receipt
 * lookup deduplicates. Cross-origin writes are refused by the app's own
 * origin check (CSRF posture; the session is a bearer token, but the
 * check keeps browser-form misuse out).
 */
const postNote = Effect.fn("routes.postNote")(
	function* (request: Request, body: { readonly id: string; readonly text: string }) {
		const principal = yield* requirePrincipal(request)
		const binding = yield* bindingFor(principal.tenantId)
		const work = requestPolicy(request)
		const noteId = yield* Effect.fromResult(Id128.fromHex(body.id))
		const databases = yield* Databases
		const db = yield* databases.acquire(binding, work)
		const outcome = yield* createNote(db, principal.tenantId, noteId, body.text, work)
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
