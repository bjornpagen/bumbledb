/**
 * The Lambda POST body is a parsed grammar: a JSON object whose `id`
 * is a canonical decimal u64 string and whose `body` is a well-formed
 * string. A malformed id is a domain refusal, never a runtime crash.
 */

const U64_MAX = 0xffffffffffffffffn

type Request =
	| { readonly tag: "duty" }
	| { readonly tag: "read" }
	| { readonly tag: "write"; readonly id: bigint; readonly body: string }
	| { readonly tag: "refused"; readonly status: 400; readonly reason: string }

function parseDecimalU64(raw: string): bigint | undefined {
	if (raw.length === 0 || (raw.length > 1 && raw.startsWith("0")) || /[^0-9]/.test(raw)) {
		return undefined
	}
	const value = BigInt(raw)
	return value > U64_MAX ? undefined : value
}

function parseRequest(event: unknown): Request {
	if (typeof event !== "object" || event === null) {
		return { tag: "refused", status: 400, reason: "event is not an object" }
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
		return { tag: "refused", status: 400, reason: "POST body is not JSON" }
	}
	if (typeof payload !== "object" || payload === null) {
		return { tag: "refused", status: 400, reason: "POST body is not a JSON object" }
	}
	const body = payload as Record<string, unknown>
	if (typeof body.id !== "string") {
		return { tag: "refused", status: 400, reason: "id is not a decimal string" }
	}
	const id = parseDecimalU64(body.id)
	if (id === undefined) {
		return { tag: "refused", status: 400, reason: "id is not a canonical u64" }
	}
	if (body.body !== undefined && (typeof body.body !== "string" || !body.body.isWellFormed())) {
		return { tag: "refused", status: 400, reason: "body is not a well-formed string" }
	}
	return { tag: "write", id, body: typeof body.body === "string" ? body.body : "" }
}

export type { Request }
export { parseRequest }
