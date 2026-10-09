import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, test } from "node:test"
import { S3Client } from "@aws-sdk/client-s3"
import { Effect, Schedule } from "effect"
import { FsStore } from "../src/database/fs.ts"
import type { ExecutorOptions, IoRequest, ObjectStore } from "../src/database/io.ts"
import { execute } from "../src/database/io.ts"
import type { Fault } from "../src/database/mem.ts"
import { MemStore } from "../src/database/mem.ts"
import { S3Store } from "../src/database/s3.ts"
import { fakeS3 } from "./fixtures/fake-s3.ts"
import { hostedConformance } from "./fixtures/hosted-conformance.ts"
import { storeConformance } from "./fixtures/store-conformance.ts"

const root = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-fsstore-"))
after(() => fs.rmSync(root, { recursive: true, force: true }))

storeConformance("MemStore", () => MemStore.make(), "conformance")
storeConformance("FsStore", () => FsStore.make(root), "conformance")
hostedConformance("MemStore", () => MemStore.make())
hostedConformance("FsStore", () => FsStore.make(fs.mkdtempSync(path.join(root, "hosted-"))))

const options: ExecutorOptions = { timeout: "1 second", readRetry: Schedule.recurs(3) }
const bytes = (text: string) => new TextEncoder().encode(text)

function faulty(faults: ReadonlyArray<Fault | undefined>) {
	let call = 0
	return MemStore.make({ fault: () => faults[call++] })
}

const get = (key: string): IoRequest => ({
	id: 1n,
	bucket: "Log",
	key,
	op: { _tag: "GetMemory" }
})
const put = (key: string, text: string): IoRequest => ({
	id: 2n,
	bucket: "Log",
	key,
	op: { _tag: "PutBytes", bytes: bytes(text) }
})

test("a failed read is retried until the store answers", async () => {
	const store = faulty([undefined, "Fail", "Fail"])
	await Effect.runPromise(store.putIfAbsent("Log", "log/1", { _tag: "Bytes", bytes: bytes("x") }))
	const response = await Effect.runPromise(execute(store, get("log/1"), options))
	assert.equal(response.id, 1n)
	assert.equal(response.result._tag, "Body")
	assert.ok(response.date !== null)
})

test("a read that keeps failing is reported as Failed", async () => {
	const store = faulty(["Fail", "Fail", "Fail", "Fail"])
	const response = await Effect.runPromise(execute(store, get("log/1"), options))
	assert.deepEqual(response, { id: 1n, date: null, result: { _tag: "Failed" } })
})

test("a create is never retried: its first failure is reported, whether or not it took effect", async () => {
	const failed = faulty(["Fail"])
	assert.equal((await Effect.runPromise(execute(failed, put("log/1", "a"), options))).result._tag, "Failed")
	assert.equal(failed.objects("Log").size, 0)

	const lost = faulty(["Lose"])
	assert.equal((await Effect.runPromise(execute(lost, put("log/1", "a"), options))).result._tag, "Failed")
	assert.deepEqual(lost.objects("Log").get("log/1"), bytes("a"))
})

test("listing or deleting the log bucket is refused without touching the store", async () => {
	const store = MemStore.make()
	for (const op of [{ _tag: "List", maxKeys: 1 }, { _tag: "Delete" }] as const) {
		const response = await Effect.runPromise(execute(store, { id: 3n, bucket: "Log", key: "log/", op }, options))
		assert.equal(response.result._tag, "Failed")
	}
})

test("a store call that never answers times out as Failed", async () => {
	const never: ObjectStore = { ...MemStore.make(), putIfAbsent: () => Effect.never }
	const response = await Effect.runPromise(execute(never, put("log/1", "a"), { ...options, timeout: "1 millis" }))
	assert.equal(response.result._tag, "Failed")
})

const s3 = await fakeS3()
after(() => s3.close())
const client = new S3Client({
	region: "us-east-1",
	endpoint: s3.endpoint,
	forcePathStyle: true,
	credentials: { accessKeyId: "test", secretAccessKey: "test" },
	requestChecksumCalculation: "WHEN_REQUIRED",
	responseChecksumValidation: "WHEN_REQUIRED"
})
storeConformance(
	"S3Store",
	() => S3Store.make({ client, log: { bucket: "log" }, checkpoints: { bucket: "ckpt" }, prefix: "db/" }),
	"conformance"
)
let databases = 0
hostedConformance("S3Store", () => {
	databases += 1
	return S3Store.make({
		client,
		log: { bucket: "log" },
		checkpoints: { bucket: "ckpt" },
		prefix: `hosted-${databases}/`
	})
})

test("S3Store reports the store's Date header with each answer", async () => {
	const store = S3Store.make({
		client,
		log: { bucket: "log" },
		checkpoints: { bucket: "ckpt" },
		prefix: "dated/"
	})
	const before = BigInt(Date.now() - 2000)
	const reply = await Effect.runPromise(store.get("Log", "log/missing", { _tag: "Memory" }))
	assert.equal(reply.result._tag, "Missing")
	assert.ok(reply.date !== null && reply.date >= before)
})
