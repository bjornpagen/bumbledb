import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { test } from "node:test"
import { installNativeArtifact } from "../scripts/native-artifact.ts"

test("native installation replaces the inode without altering an existing reader", () => {
	const dir = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-native-install-"))
	const source = path.join(dir, "compiler-output")
	const destination = path.join(dir, "bumbledb.node")
	try {
		fs.writeFileSync(source, "new addon")
		fs.writeFileSync(destination, "old addon")
		const original = fs.statSync(destination).ino
		const open = fs.openSync(destination, "r")
		try {
			installNativeArtifact(source, destination)
			assert.notEqual(fs.statSync(destination).ino, original, "do not overwrite a signed/mapped inode")
			assert.equal(fs.readFileSync(open, "utf8"), "old addon")
			assert.equal(fs.readFileSync(destination, "utf8"), "new addon")
		} finally {
			fs.closeSync(open)
		}
		assert.deepEqual(fs.readdirSync(dir).sort(), ["bumbledb.node", "compiler-output"])
	} finally {
		fs.rmSync(dir, { recursive: true, force: true })
	}
})

test("failed native installation preserves the previous addon and cleans staging", () => {
	const dir = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-native-install-"))
	const destination = path.join(dir, "bumbledb.node")
	try {
		fs.writeFileSync(destination, "old addon")
		assert.throws(() => installNativeArtifact(path.join(dir, "missing"), destination))
		assert.equal(fs.readFileSync(destination, "utf8"), "old addon")
		assert.deepEqual(fs.readdirSync(dir), ["bumbledb.node"])
	} finally {
		fs.rmSync(dir, { recursive: true, force: true })
	}
})
