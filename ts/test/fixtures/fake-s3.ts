import { createServer, type IncomingMessage, type ServerResponse } from "node:http"
import type { AddressInfo } from "node:net"

interface Stored {
	readonly bytes: Buffer
	readonly lastModified: Date
}

const xml = (body: string) => `<?xml version="1.0" encoding="UTF-8"?>${body}`
const escapeXml = (text: string) => text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")

function error(response: ServerResponse, status: number, code: string): void {
	response.writeHead(status, { "content-type": "application/xml" })
	response.end(xml(`<Error><Code>${code}</Code><Message>${code}</Message></Error>`))
}

async function body(request: IncomingMessage): Promise<Buffer> {
	const chunks: Buffer[] = []
	for await (const chunk of request) chunks.push(chunk as Buffer)
	return Buffer.concat(chunks)
}

/**
 * A path-style S3 endpoint over memory: GetObject, PutObject (honouring `If-None-Match: *`),
 * DeleteObject and ListObjectsV2. Enough to check S3Store's request shapes and status mapping;
 * the real-store lanes run the same conformance suite against S3 itself.
 */
export async function fakeS3(): Promise<{ readonly endpoint: string; readonly close: () => Promise<void> }> {
	const buckets = new Map<string, Map<string, Stored>>()
	const server = createServer(async (request, response) => {
		const url = new URL(request.url ?? "/", "http://fake")
		const [, bucketName = "", ...rest] = url.pathname.split("/")
		const key = decodeURIComponent(rest.join("/"))
		const bucket = buckets.get(bucketName) ?? new Map<string, Stored>()
		buckets.set(bucketName, bucket)
		response.setHeader("date", new Date().toUTCString())
		if (request.method === "GET" && url.searchParams.get("list-type") === "2") {
			const prefix = url.searchParams.get("prefix") ?? ""
			const startAfter = url.searchParams.get("start-after") ?? ""
			const maxKeys = Number(url.searchParams.get("max-keys") ?? "1000")
			const keys = [...bucket.keys()]
				.filter((candidate) => candidate.startsWith(prefix) && candidate > startAfter)
				.sort()
				.slice(0, maxKeys)
			const contents = keys
				.map((candidate) => `<Contents><Key>${escapeXml(candidate)}</Key><Size>0</Size></Contents>`)
				.join("")
			response.writeHead(200, { "content-type": "application/xml" })
			response.end(
				xml(
					`<ListBucketResult><Name>${bucketName}</Name><Prefix>${escapeXml(prefix)}</Prefix><KeyCount>${keys.length}</KeyCount><MaxKeys>${maxKeys}</MaxKeys><IsTruncated>false</IsTruncated>${contents}</ListBucketResult>`
				)
			)
			return
		}
		if (request.method === "GET") {
			const stored = bucket.get(key)
			if (stored === undefined) return error(response, 404, "NoSuchKey")
			response.writeHead(200, {
				"content-length": stored.bytes.length,
				"last-modified": stored.lastModified.toUTCString()
			})
			response.end(stored.bytes)
			return
		}
		if (request.method === "PUT") {
			const bytes = await body(request)
			if (request.headers["if-none-match"] === "*" && bucket.has(key)) return error(response, 412, "PreconditionFailed")
			bucket.set(key, { bytes, lastModified: new Date() })
			response.writeHead(200, { etag: '"fake"' })
			response.end()
			return
		}
		if (request.method === "DELETE") {
			bucket.delete(key)
			response.writeHead(204)
			response.end()
			return
		}
		error(response, 405, "MethodNotAllowed")
	})
	await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve))
	const { port } = server.address() as AddressInfo
	return {
		endpoint: `http://127.0.0.1:${port}`,
		close: () => new Promise<void>((resolve, reject) => server.close((cause) => (cause ? reject(cause) : resolve())))
	}
}
