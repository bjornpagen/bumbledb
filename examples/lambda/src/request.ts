/**
 * The Lambda event is a parsed grammar. A POST `id` is a canonical
 * decimal u64 string; a malformed id is a 400 refusal, not a thrown
 * BigInt. `body`, when present, is well-formed UTF-8.
 */

const U64_MAX = 0xffffffffffffffffn

type RefusalReason =
	| "event is not an object"
	| "POST body is not JSON"
	| "POST body is not a JSON object"
	| "id is not a decimal string"
	| "id is not a canonical u64"
	| "body is not a well-formed string"

type Request =
	| { readonly tag: "duty" }
	| { readonly tag: "read" }
	| { readonly tag: "write"; readonly id: bigint; readonly body: string }
	| { readonly tag: "refused"; readonly status: 400; readonly reason: RefusalReason }

function refuse(reason: RefusalReason): Request {
	return { tag: "refused", status: 400, reason }
}

function parseDecimalU64(raw: string): bigint | undefined {
	if (raw.length === 0 || (raw.length > 1 && raw.charCodeAt(0) === 0x30)) {
		return undefined
	}
	let value = 0n
	for (let i = 0; i < raw.length; i++) {
		const digit = raw.charCodeAt(i) - 0x30
		if (digit < 0 || digit > 9) {
			return undefined
		}
		const next = value * 10n + BigInt(digit)
		if (next > U64_MAX) {
			return undefined
		}
		value = next
	}
	return value
}

function parseRequest(event: unknown): Request {
	if (typeof event !== "object" || event === null) {
		return refuse("event is not an object")
	}
	const record = event as Record<string, unknown>
	if (record.duty === true) {
		return { tag: "duty" }
	}
	const http = record.requestContext as { http?: { method?: string } } | undefined
	const method = http?.http?.method ?? "GET"
	if (method !== "POST") {
		return { tag: "read" }
	}
	let payload: unknown
	try {
		payload = JSON.parse(typeof record.body === "string" ? record.body : "{}")
	} catch {
		return refuse("POST body is not JSON")
	}
	if (typeof payload !== "object" || payload === null) {
		return refuse("POST body is not a JSON object")
	}
	const body = payload as Record<string, unknown>
	if (typeof body.id !== "string") {
		return refuse("id is not a decimal string")
	}
	const id = parseDecimalU64(body.id)
	if (id === undefined) {
		return refuse("id is not a canonical u64")
	}
	if (body.body !== undefined && (typeof body.body !== "string" || !body.body.isWellFormed())) {
		return refuse("body is not a well-formed string")
	}
	return { tag: "write", id, body: typeof body.body === "string" ? body.body : "" }
}

export type { RefusalReason, Request }
export { parseRequest }
