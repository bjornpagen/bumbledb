import assert from "node:assert/strict"
import { describe, test } from "node:test"
import { parseRequest } from "./request.ts"

describe("the Lambda request grammar", function suite() {
	test("Scheduler duty is a parsed arm", function duty() {
		assert.deepEqual(parseRequest({ duty: true }), { tag: "duty" })
	})

	test("a Function URL GET is a read", function read() {
		assert.deepEqual(parseRequest({ requestContext: { http: { method: "GET" } } }), { tag: "read" })
	})

	test("a POST with a canonical decimal id is a write", function write() {
		const request = parseRequest({
			requestContext: { http: { method: "POST" } },
			body: JSON.stringify({ id: "18446744073709551615", body: "ok" })
		})
		assert.deepEqual(request, { tag: "write", id: 18446744073709551615n, body: "ok" })
	})

	test("zero is a canonical u64 and a missing body is the empty string", function zero() {
		const request = parseRequest({
			requestContext: { http: { method: "POST" } },
			body: '{"id":"0"}'
		})
		assert.deepEqual(request, { tag: "write", id: 0n, body: "" })
	})

	test("a JSON-number id is a 400, not a BigInt", function numberId() {
		const request = parseRequest({
			requestContext: { http: { method: "POST" } },
			body: JSON.stringify({ id: Date.now(), body: "x" })
		})
		assert.deepEqual(request, { tag: "refused", status: 400, reason: "id is not a decimal string" })
	})

	test("a malformed id string is a 400, not a thrown BigInt", function malformedId() {
		const request = parseRequest({
			requestContext: { http: { method: "POST" } },
			body: JSON.stringify({ id: "not-a-u64" })
		})
		assert.deepEqual(request, { tag: "refused", status: 400, reason: "id is not a canonical u64" })
	})

	test("a leading-zero id is a 400", function leadingZero() {
		const request = parseRequest({
			requestContext: { http: { method: "POST" } },
			body: '{"id":"01"}'
		})
		assert.deepEqual(request, { tag: "refused", status: 400, reason: "id is not a canonical u64" })
	})

	test("an overflowing id is a 400", function overflow() {
		const request = parseRequest({
			requestContext: { http: { method: "POST" } },
			body: '{"id":"18446744073709551616"}'
		})
		assert.deepEqual(request, { tag: "refused", status: 400, reason: "id is not a canonical u64" })
	})

	test("a non-JSON POST body is a 400", function notJson() {
		const request = parseRequest({
			requestContext: { http: { method: "POST" } },
			body: "{"
		})
		assert.deepEqual(request, { tag: "refused", status: 400, reason: "POST body is not JSON" })
	})

	test("a lone-surrogate body is a 400", function surrogate() {
		const request = parseRequest({
			requestContext: { http: { method: "POST" } },
			body: JSON.stringify({ id: "1", body: "\uD800" })
		})
		assert.deepEqual(request, { tag: "refused", status: 400, reason: "body is not a well-formed string" })
	})

	test("a non-object event is a 400", function notObject() {
		assert.deepEqual(parseRequest("duty"), { tag: "refused", status: 400, reason: "event is not an object" })
	})
})
