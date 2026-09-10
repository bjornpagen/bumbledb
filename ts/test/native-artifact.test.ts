import assert from "node:assert/strict"
import { createHash } from "node:crypto"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { test } from "node:test"
import { assertNativeProvenance, installNativeArtifact } from "../scripts/native-artifact.ts"

test("native staging rejects missing or stale source, platform, and binary provenance", () => {
	const dir = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-native-provenance-"))
	const binary = path.join(dir, "bumbledb.node")
	const stampPath = path.join(dir, ".native-provenance.json")
	const expected = { candidateSourceDigest: "a".repeat(64), specificationRevision: "b".repeat(64) }
	const platform = "linux-arm64"
	const stamp = {
		...expected,
		platform,
		artifact: { path: `ts/npm/${platform}/bumbledb.node`, sha256: createHash("sha256").update("addon").digest("hex") }
	}
	try {
		fs.writeFileSync(binary, "addon")
		assert.throws(() => assertNativeProvenance(binary, platform, expected), /provenance missing/)
		for (const invalid of [
			{ ...stamp, candidateSourceDigest: "stale" },
			{ ...stamp, specificationRevision: "stale" },
			{ ...stamp, platform: "darwin-arm64" },
			{ ...stamp, artifact: { ...stamp.artifact, path: "another.node" } },
			{ ...stamp, artifact: { ...stamp.artifact, sha256: "wrong" } }
		]) {
			fs.writeFileSync(stampPath, JSON.stringify(invalid))
			assert.throws(() => assertNativeProvenance(binary, platform, expected), /stale or mismatched/)
		}
		fs.writeFileSync(stampPath, JSON.stringify(stamp))
		assert.doesNotThrow(() => assertNativeProvenance(binary, platform, expected))
		fs.writeFileSync(binary, "replacement addon")
		assert.throws(() => assertNativeProvenance(binary, platform, expected), /stale or mismatched/)
	} finally {
		fs.rmSync(dir, { recursive: true, force: true })
	}
})

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
