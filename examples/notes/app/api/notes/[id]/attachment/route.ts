/**
 * Blob first, reference second: the content-addressed blob is uploaded to the app's bucket, then
 * the referencing row is committed. A retry re-uploads identical content to the identical key and
 * submits the identical command.
 */
import { Uuid } from "@bjornpagen/bumbledb"
import { Effect, Result } from "effect"
import { requirePrincipal } from "../../../../../src/auth.ts"
import { putBlob } from "../../../../../src/blob.ts"
import { addAttachment } from "../../../../../src/db/commands.ts"
import { appRuntime, Databases } from "../../../../../src/db/server.ts"
import { exitResponse, respond, submitResponse } from "../../../../../src/http.ts"

export const runtime = "nodejs"
export const dynamic = "force-dynamic"

const MAX_BODY = 4_000_000

const postAttachment = Effect.fn("routes.postAttachment")(
	function* (request: Request, rawId: string, body: Uint8Array) {
		const principal = yield* requirePrincipal(request)
		const noteId = yield* Effect.fromResult(Uuid.parse(rawId))
		const uploaded = yield* putBlob(principal.tenantId, body).pipe(Effect.result)
		if (Result.isFailure(uploaded)) {
			return Response.json({ error: uploaded.failure._tag }, { status: 503 })
		}
		const db = yield* (yield* Databases).get(principal.tenantId)
		const outcome = yield* addAttachment(db, noteId, uploaded.success)
		return submitResponse(outcome)
	},
	Effect.scoped
)

export async function POST(request: Request, context: { params: Promise<{ id: string }> }): Promise<Response> {
	const { id } = await context.params
	const raw = new Uint8Array(await request.arrayBuffer())
	if (raw.byteLength === 0 || raw.byteLength > MAX_BODY) {
		return Response.json({ error: "InvalidBody" }, { status: 400 })
	}
	const exit = await appRuntime.runPromiseExit(respond(postAttachment(request, id, raw)), {
		signal: request.signal
	})
	return exitResponse(exit)
}
