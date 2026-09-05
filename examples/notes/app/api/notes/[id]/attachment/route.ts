/**
 * Blob-first attachment (OPS-003): upload the immutable content-addressed
 * blob to the app's own bucket FIRST, then commit the referencing fact.
 * A crash after upload and before commit leaves an orphan object (the
 * app's sweep policy handles it); a reference to missing bytes is
 * unrepresentable. Retrying the whole request re-uploads identical
 * content to the identical key and rebuilds the identical command.
 */
import { Id128 } from "@bjornpagen/bumbledb"
import { Effect, Result } from "effect"
import { requirePrincipal } from "../../../../../src/auth.ts"
import { putBlob } from "../../../../../src/blob.ts"
import { bindingFor } from "../../../../../src/db/bindings.ts"
import { addAttachment } from "../../../../../src/db/commands.ts"
import { requestPolicy } from "../../../../../src/db/runtime-policy.ts"
import { appRuntime, Databases } from "../../../../../src/db/server.ts"
import { exitResponse, respond, submitResponse } from "../../../../../src/http.ts"

export const runtime = "nodejs"
export const dynamic = "force-dynamic"

const MAX_BODY = 4_000_000

const postAttachment = Effect.fn("routes.postAttachment")(
	function* (request: Request, rawId: string, body: Uint8Array) {
		const principal = yield* requirePrincipal(request)
		const binding = yield* bindingFor(principal.tenantId)
		const noteId = yield* Effect.fromResult(Id128.fromHex(rawId))
		const work = requestPolicy(request)
		// 1. Immutable blob first — app-owned S3, content-addressed key.
		const uploaded = yield* putBlob(principal.tenantId, body).pipe(Effect.result)
		if (Result.isFailure(uploaded)) {
			return Response.json({ error: uploaded.failure._tag }, { status: 503 })
		}
		// 2. Reference/receipt second — one atomic tenant command.
		const databases = yield* Databases
		const db = yield* databases.acquire(binding, work)
		const outcome = yield* addAttachment(db, principal.tenantId, noteId, uploaded.success, work)
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
