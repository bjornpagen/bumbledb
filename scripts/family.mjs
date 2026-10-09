// The npm package family: the core package plus one package per platform addon.
//
//   node scripts/family.mjs pack <natives-dir> <out-dir>
//   node scripts/family.mjs smoke <out-dir>
//
// pack puts each bdb.<platform>-<arch>.node from <natives-dir> into its
// platform package (and no other addon), builds dist, packs the core and those
// platform packages into the empty <out-dir>, and writes SHA256SUMS.
// smoke installs the core and host tarballs into a fresh project, typechecks the
// core-ts consumer example strictly, and runs it.
import { execFileSync } from "node:child_process"
import { createHash } from "node:crypto"
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { fileURLToPath } from "node:url"

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..")
const ts = path.join(root, "ts")
const platforms = fs.readdirSync(path.join(ts, "npm"))
const host = `${process.platform}-${process.arch}`

const run = (cwd, command, ...args) => execFileSync(command, args, { cwd, stdio: "inherit" })
const manifest = (dir) => JSON.parse(fs.readFileSync(path.join(dir, "package.json"), "utf8"))
const sha256 = (file) => createHash("sha256").update(fs.readFileSync(file)).digest("hex")
const tarball = (name, version) => `bjornpagen-${name}-${version}.tgz`

function pack(natives, out) {
	const shipped = fs
		.readdirSync(natives)
		.map((file) => /^bdb\.(.+)\.node$/.exec(file)?.[1])
		.filter((platform) => platform !== undefined)
	const unknown = shipped.filter((platform) => !platforms.includes(platform))
	if (shipped.length === 0 || unknown.length > 0) {
		throw Error(`${natives} must hold bdb.<platform>.node for platforms among ${platforms.join(", ")}`)
	}
	fs.mkdirSync(out, { recursive: true })
	if (fs.readdirSync(out).length > 0) throw Error(`${out} is not empty`)
	for (const platform of platforms) {
		const installed = path.join(ts, "npm", platform, "bdb.node")
		fs.rmSync(installed, { force: true })
		if (shipped.includes(platform)) fs.copyFileSync(path.join(natives, `bdb.${platform}.node`), installed)
	}
	run(ts, "node", "scripts/build.ts", "dist")
	run(ts, "node", "scripts/build.ts", "stage", path.resolve(out))
	const { version } = manifest(ts)
	const expected = [tarball("bumbledb", version), ...shipped.map((p) => tarball(`bumbledb-${p}`, version))].sort()
	const packed = fs.readdirSync(out).filter((file) => file.endsWith(".tgz")).sort()
	if (packed.join() !== expected.join()) throw Error(`packed ${packed.join(", ")}; expected ${expected.join(", ")}`)
	fs.writeFileSync(path.join(out, "SHA256SUMS"), packed.map((file) => `${sha256(path.join(out, file))}  ${file}\n`).join(""))
}

function smoke(out) {
	const core = manifest(ts)
	const local = (name) => {
		const file = path.resolve(out, tarball(name, core.version))
		return fs.existsSync(file) ? `file:${file}` : undefined
	}
	if (local("bumbledb") === undefined || local(`bumbledb-${host}`) === undefined) {
		throw Error(`${out} lacks the core or the ${host} tarball`)
	}
	const project = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-smoke-"))
	try {
		const consumer = {
			name: "bumbledb-packed-smoke",
			private: true,
			type: "module",
			packageManager: core.packageManager,
			dependencies: { "@bjornpagen/bumbledb": local("bumbledb"), effect: core.peerDependencies.effect },
			devDependencies: { "@types/node": core.devDependencies["@types/node"], typescript: core.devDependencies.typescript },
		}
		fs.writeFileSync(path.join(project, "package.json"), `${JSON.stringify(consumer, null, 2)}\n`)
		// Platform packages this family does not carry are removed rather than fetched.
		const overrides = platforms.map((p) => `  "@bjornpagen/bumbledb-${p}": "${local(`bumbledb-${p}`) ?? "-"}"\n`)
		fs.writeFileSync(path.join(project, "pnpm-workspace.yaml"), `packages:\n  - "."\noverrides:\n${overrides.join("")}`)
		fs.copyFileSync(path.join(root, "examples/consumers/core-ts/consumer.ts"), path.join(project, "consumer.ts"))
		fs.copyFileSync(path.join(root, "scripts/packed-consumer.ts"), path.join(project, "smoke.ts"))
		run(project, "pnpm", "install", "--ignore-scripts")
		run(project, path.join(project, "node_modules/.bin/tsc"), "--strict", "--exactOptionalPropertyTypes",
			"--target", "es2024", "--module", "nodenext", "--types", "node", "--allowImportingTsExtensions",
			"--noEmit", "smoke.ts")
		run(project, "node", "smoke.ts")
	} finally {
		fs.rmSync(project, { recursive: true, force: true })
	}
}

const [command, ...args] = process.argv.slice(2)
if (command === "pack" && args.length === 2) pack(args[0], args[1])
else if (command === "smoke" && args.length === 1) smoke(args[0])
else {
	console.error("usage: node scripts/family.mjs pack <natives-dir> <out-dir> | smoke <out-dir>")
	process.exitCode = 2
}
