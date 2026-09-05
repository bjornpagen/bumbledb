/**
 * Application-owned authentication — a HOST responsibility the database
 * never provides (chapter 33: auth/binding functions are application-owned
 * Effects, not SDK authentication). This example uses a compact
 * HMAC-signed bearer token so the boundary is real, small and auditable:
 *
 *   token := <tenantId>.<expiresAtUnixSeconds>.<hex hmac-sha256>
 *   hmac  := HMAC(SESSION_SECRET, `${tenantId}.${expiresAt}`)
 *
 * `scripts/mint-session.ts` mints tokens for local development and the
 * deployed request tests. Anonymous, malformed, expired or forged tokens
 * refuse BEFORE any database open (APP-02). The Alchemy Function URL is
 * public — every route authenticates even when a CDN sits in front.
 */
import { createHmac, timingSafeEqual } from "node:crypto"
import { Effect, Schema } from "effect"

export class Unauthenticated extends Schema.TaggedError<Unauthenticated>()("Unauthenticated", {
	detail: Schema.String
}) {}

export interface Principal {
	readonly tenantId: string
}

const TOKEN_LIMIT = 256

function isLowerAlphanumeric(code: number): boolean {
	return (code >= 97 && code <= 122) || (code >= 48 && code <= 57)
}

/** `[a-z0-9][a-z0-9-]{0,62}` — spelled as explicit checks. */
function isTenantId(value: string): boolean {
	if (value.length === 0 || value.length > 63) {
		return false
	}
	if (!isLowerAlphanumeric(value.charCodeAt(0))) {
		return false
	}
	for (let i = 1; i < value.length; i += 1) {
		const code = value.charCodeAt(i)
		if (!isLowerAlphanumeric(code) && code !== 45) {
			return false
		}
	}
	return true
}

function isDecimal(value: string, max: number): boolean {
	if (value.length === 0 || value.length > max) {
		return false
	}
	for (let i = 0; i < value.length; i += 1) {
		const code = value.charCodeAt(i)
		if (code < 48 || code > 57) {
			return false
		}
	}
	return true
}

function isLowerHex(value: string, exact: number): boolean {
	if (value.length !== exact) {
		return false
	}
	for (let i = 0; i < value.length; i += 1) {
		const code = value.charCodeAt(i)
		if (!((code >= 48 && code <= 57) || (code >= 97 && code <= 102))) {
			return false
		}
	}
	return true
}

function secret(): Uint8Array {
	const value = process.env.SESSION_SECRET
	if (value === undefined || value.length < 32) {
		// Refusing to run without a real secret beats a default key.
		throw new Error("SESSION_SECRET (>= 32 chars) is required")
	}
	return new TextEncoder().encode(value)
}

export function signSession(tenantId: string, expiresAtUnixSeconds: number): string {
	if (!isTenantId(tenantId)) {
		throw new Error(`invalid tenant id: ${tenantId}`)
	}
	const payload = `${tenantId}.${expiresAtUnixSeconds}`
	const mac = createHmac("sha256", secret()).update(payload).digest("hex")
	return `${payload}.${mac}`
}

/** Parse + verify the bearer token; typed refusal before any I/O. */
export const requirePrincipal = Effect.fn("auth.requirePrincipal")(function* (request: Request) {
	const header = request.headers.get("authorization")
	if (header === null || !header.startsWith("Bearer ")) {
		return yield* new Unauthenticated({ detail: "missing bearer token" })
	}
	const token = header.slice("Bearer ".length)
	if (token.length > TOKEN_LIMIT) {
		return yield* new Unauthenticated({ detail: "token too long" })
	}
	const parts = token.split(".")
	if (parts.length !== 3) {
		return yield* new Unauthenticated({ detail: "malformed token" })
	}
	const [tenantId, expiresRaw, mac] = parts as [string, string, string]
	if (!isTenantId(tenantId) || !isDecimal(expiresRaw, 12) || !isLowerHex(mac, 64)) {
		return yield* new Unauthenticated({ detail: "malformed token" })
	}
	const expected = createHmac("sha256", secret()).update(`${tenantId}.${expiresRaw}`).digest()
	const supplied = Buffer.from(mac, "hex")
	if (expected.length !== supplied.length || !timingSafeEqual(expected, supplied)) {
		return yield* new Unauthenticated({ detail: "bad signature" })
	}
	if (Number(expiresRaw) * 1000 < Date.now()) {
		return yield* new Unauthenticated({ detail: "expired" })
	}
	return { tenantId } satisfies Principal
})
