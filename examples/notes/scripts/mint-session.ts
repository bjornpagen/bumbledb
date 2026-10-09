/**
 * Development helpers:
 *   SESSION_SECRET=... node scripts/mint-session.ts token <tenant>   a one-hour session token
 *   node scripts/mint-session.ts id                                  a fresh UUID (note id, request key)
 * Mint an id once per intent and reuse it on retries.
 */
import { randomUUID } from "node:crypto"
import { signSession } from "../src/auth.ts"

const [command, tenantId] = process.argv.slice(2)

if (command === "id") {
	console.log(randomUUID())
} else if (command === "token" && tenantId !== undefined) {
	const expires = Math.floor(Date.now() / 1000) + 3600
	console.log(signSession(tenantId, expires))
} else {
	console.error("usage: mint-session.ts <token <tenantId> | id>")
	process.exit(2)
}
