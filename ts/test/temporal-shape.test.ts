import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import { Db, ErrSpentHandle, InstanceBuilder, relation, schema, str, u64 } from "#index.ts"
import { accepted } from "#test/accepted.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-temporal-"))
const packageRoot = path.resolve(import.meta.dirname, "..")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const Holder = relation("Holder", { id: u64.fresh, name: str })
const Publish = schema("Publish", { Holder }, [])

function camelCase(name: string): string {
	return name.replace(/_([a-z])/g, function upper(_match: string, letter: string): string {
		return letter.toUpperCase()
	})
}

function napiExports(source: string): { readonly sync: readonly string[]; readonly asyncTasks: readonly string[] } {
	const sync: string[] = []
	const asyncTasks: string[] = []
	for (const chunk of source.split("#[napi]").slice(1)) {
		const named = chunk.match(/pub fn (\w+)/)
		if (named === null || named[1] === undefined) {
			continue
		}
		const returns = chunk.match(/->\s*napi::Result<([\s\S]*?)>\s*\{/)
		if (returns === null || returns[1] === undefined) {
			continue
		}
		const camel = camelCase(named[1])
		if (returns[1].includes("AsyncTask")) {
			asyncTasks.push(camel)
		} else {
			sync.push(camel)
		}
	}
	return { sync, asyncTasks }
}

describe("one temporal shape: async means AsyncTask", function suite() {
	test("no sync #[napi] fn is awaited by the SDK", function grepGate() {
		const lib = fs.readFileSync(path.join(packageRoot, "crate/src/lib.rs"), "utf8")
		const { sync, asyncTasks } = napiExports(lib)
		assert.ok(asyncTasks.includes("dbCreate"), "dbCreate is an AsyncTask")
		assert.ok(asyncTasks.includes("dbOpen"), "dbOpen is an AsyncTask")
		assert.ok(asyncTasks.includes("dbFromInstance"), "dbFromInstance is an AsyncTask")
		assert.ok(asyncTasks.includes("instanceBuilderAdmit"), "instanceBuilderAdmit is an AsyncTask")
		const sdk = ["src/db.ts", "src/native.ts"]
			.map(function read(file) {
				return fs.readFileSync(path.join(packageRoot, file), "utf8")
			})
			.join("\n")
		for (const method of sync) {
			assert.doesNotMatch(sdk, new RegExp(`await native\\.${method}\\b`), `SDK must not await sync native.${method}`)
		}
	})

	test("a large fromInstance does not block a concurrently ticking JS timer", async function intervalKeepsFiring() {
		const builder = InstanceBuilder.create(Publish)
		const facts = Array.from({ length: 8_000 }, function row(_unused, index) {
			return { id: BigInt(index), name: `holder-${index}-${"x".repeat(48)}` }
		})
		builder.load(Holder, facts)
		const instance = accepted(await builder.admit())
		const dest = path.join(tmpRoot, "large")
		let ticks = 0
		const timer = setInterval(function tick() {
			ticks += 1
		}, 4)
		try {
			await Db.fromInstance(dest, instance)
		} finally {
			clearInterval(timer)
			instance[Symbol.dispose]()
		}
		assert.ok(ticks > 0, "the event loop kept ticking while fromInstance copied the catalog")
	})

	test("dispose-during-publish is the typed spent-handle refusal", async function disposeDuringPublish() {
		const builder = InstanceBuilder.create(Publish)
		builder.load(Holder, [{ id: 1n, name: "ada" }])
		const instance = accepted(await builder.admit())
		const dest = path.join(tmpRoot, "leased")
		const publish = Db.fromInstance(dest, instance)
		assert.throws(
			function dispose() {
				instance[Symbol.dispose]()
			},
			function isSpent(error: unknown) {
				return error instanceof Error && error instanceof ErrSpentHandle && /leased for publish/.test(String(error))
			}
		)
		await publish
		instance[Symbol.dispose]()
	})
})
