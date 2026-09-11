import assert from "node:assert/strict"
import { execFileSync } from "node:child_process"
import fs from "node:fs"
import os from "node:os"
import path from "node:path"

// Focused public-package measurements, never part of the correctness battery.
const family = path.resolve(process.argv[2] ?? "")
assert(process.argv[2], "usage: node scripts/measure-cutover.mjs FAMILY")
const record = JSON.parse(fs.readFileSync(path.join(family, "family.json"), "utf8"))
const root = path.resolve(import.meta.dirname, "..")
const temporary = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-cutover-measure-"))
const core = JSON.parse(fs.readFileSync(path.join(family, "source/ts/package.json"), "utf8"))
const log = JSON.parse(fs.readFileSync(path.join(family, "source/ts-log/package.json"), "utf8"))
const tarball = (name) => path.join(family, "packages", `bjornpagen-${name}-${core.version}.tgz`)
try {
	fs.writeFileSync(
		path.join(temporary, "package.json"),
		JSON.stringify({
			private: true,
			type: "module",
			packageManager: core.packageManager,
			dependencies: {
				"@bjornpagen/bumbledb": `file:${tarball("bumbledb")}`,
				"@bjornpagen/bumbledb-log": `file:${tarball("bumbledb-log")}`,
				effect: core.peerDependencies.effect
			},
			devDependencies: { typescript: log.devDependencies.typescript, "@types/node": log.devDependencies["@types/node"] }
		})
	)
	const host = `${process.platform}-${process.arch}`
	fs.writeFileSync(
		path.join(temporary, "pnpm-workspace.yaml"),
		`packages: ["."]\noverrides:\n  "@bjornpagen/bumbledb-${host}": "file:${tarball(`bumbledb-${host}`)}"\n`
	)
	fs.copyFileSync(path.join(root, "scripts/measure-cutover.ts"), path.join(temporary, "measure.ts"))
	fs.cpSync(path.join(root, "ts-log/test/fixtures/transition"), path.join(temporary, "fixtures/transition"), {
		recursive: true
	})
	execFileSync("pnpm", ["install", "--ignore-scripts", "--prefer-offline"], {
		cwd: temporary,
		stdio: ["ignore", "ignore", "inherit"]
	})
	execFileSync(
		path.join(temporary, "node_modules/.bin/tsc"),
		[
			"--noEmit",
			"--strict",
			"--exactOptionalPropertyTypes",
			"--target",
			"es2024",
			"--module",
			"nodenext",
			"--types",
			"node",
			"--allowImportingTsExtensions",
			"measure.ts"
		],
		{ cwd: temporary, stdio: "inherit" }
	)
	console.log(
		JSON.stringify({
			family: record,
			host,
			node: process.version,
			cpus: os.cpus()[0]?.model,
			memoryBytes: os.totalmem()
		})
	)
	for (const mode of ["heads", "transition"]) {
		for (const count of [1000, 10000, 100000]) {
			const output = execFileSync(process.execPath, ["--expose-gc", "measure.ts", mode, String(count)], {
				cwd: temporary,
				encoding: "utf8",
				stdio: ["ignore", "pipe", "inherit"]
			})
			process.stdout.write(output)
		}
	}
} finally {
	fs.rmSync(temporary, { recursive: true, force: true })
}
