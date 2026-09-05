/**
 * The application's request/job record — the durable coordinate that makes
 * command retries safe (chapters 30/35): the client supplies a stable
 * Idempotency-Key (an Id128), the app derives the ONE command identity
 * from it, persists the sealed command's ref BEFORE dispatch, and records
 * the observed outcome after. A timeout retries the IDENTICAL command or
 * resolves the retained ref; it never mints a fresh identity.
 *
 * Storage is one small JSON file per request under the app data dir with
 * atomic tmp+rename writes. On a serverless host this directory must be
 * the app's own durable store (the interface below is deliberately tiny);
 * LocalHistory development uses the project directory. This is app
 * plumbing, not an SDK hook — the database's own receipt lookup inside
 * submit is what deduplicates admission.
 */
import * as fs from "node:fs"
import * as path from "node:path"
import { randomUUID } from "node:crypto"
import type { Id128 } from "@bjornpagen/bumbledb"
import type { CommandRef, SubmitOutcome } from "@bjornpagen/bumbledb-log"
import { renderCommandRef, renderDecisionStamp } from "@bjornpagen/bumbledb-log"
import { Effect } from "effect"

function requestDir(tenantId: string): string {
	const base = process.env.BUMBLEDB_REQUEST_RECORDS ?? path.join(process.cwd(), ".bumbledb", "requests")
	return path.join(base, tenantId)
}

function recordPath(tenantId: string, requestKey: Id128): string {
	return path.join(requestDir(tenantId), `${requestKey}.json`)
}

function writeAtomic(file: string, body: unknown): void {
	fs.mkdirSync(path.dirname(file), { recursive: true })
	const staged = `${file}.tmp-${randomUUID()}`
	fs.writeFileSync(staged, `${JSON.stringify(body, null, "\t")}\n`)
	fs.renameSync(staged, file)
}

function readRecord(file: string): Record<string, unknown> {
	if (!fs.existsSync(file)) {
		return {}
	}
	const parsed: unknown = JSON.parse(fs.readFileSync(file, "utf8"))
	if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
		return {}
	}
	return parsed
}

/** Persist the ref BEFORE dispatch; the recovery coordinate survives us. */
export const rememberCommandRef = (tenantId: string, requestKey: Id128, ref: CommandRef) =>
	Effect.sync(() => {
		const file = recordPath(tenantId, requestKey)
		const existing = readRecord(file)
		writeAtomic(file, { ...existing, ref: renderCommandRef(ref), refAt: new Date().toISOString() })
	})

/** Record what THIS invocation observed (bounded evidence, no payloads). */
export const rememberSubmitOutcome = (tenantId: string, requestKey: Id128, outcome: SubmitOutcome) =>
	Effect.sync(() => {
		const file = recordPath(tenantId, requestKey)
		const existing = readRecord(file)
		const observed =
			outcome.kind === "decided"
				? {
						kind: outcome.kind,
						outcome: outcome.receipt.outcome.kind,
						decisionAt: renderDecisionStamp(outcome.receipt.decisionAt)
					}
				: { kind: outcome.kind, error: outcome.error.code }
		writeAtomic(file, { ...existing, observed, observedAt: new Date().toISOString() })
	})

/** The stored ref string for a request, if any — resolve() input on recovery. */
export function storedRef(tenantId: string, requestKey: Id128): string | undefined {
	const file = recordPath(tenantId, requestKey)
	if (!fs.existsSync(file)) {
		return undefined
	}
	const record = readRecord(file)
	const ref = record.ref
	return typeof ref === "string" ? ref : undefined
}
