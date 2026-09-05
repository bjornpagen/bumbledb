/**
 * Development/test helpers for app-owned identity:
 *
 *   # a signed session token (1 hour) for local/deployed request tests:
 *   SESSION_SECRET=... node --experimental-strip-types scripts/mint-session.ts token <tenantId>
 *
 *   # one fresh Id128 (note ids, request keys, operation/database ids):
 *   node --experimental-strip-types scripts/mint-session.ts id
 *
 * Ids are minted ONCE for an original intent and persisted by the caller;
 * a retry reuses the recorded value, never a fresh one.
 */
import { randomBytes } from "node:crypto"
import { signSession } from "../src/auth.ts"

const [command, tenantId] = process.argv.slice(2)

if (command === "id") {
	console.log(Buffer.from(randomBytes(16)).toString("hex"))
} else if (command === "token" && tenantId !== undefined) {
	const expires = Math.floor(Date.now() / 1000) + 3600
	console.log(signSession(tenantId, expires))
} else {
	console.error("usage: mint-session.ts <token <tenantId> | id>")
	process.exit(2)
}
