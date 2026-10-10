/**
 * `node scripts/build.ts dev`: debug addon at `bdb.<platform>-<arch>.node`, which the
 * loader prefers over platform packages.
 * `node scripts/build.ts dist`: the compiled package (`dist/`), for consumers that link this package.
 * `node scripts/build.ts release`: optimized addon into `npm/<platform>-<arch>/` plus `dist/`.
 * `node scripts/build.ts stage <out>`: packs the main package (platform packages pinned as
 * optional dependencies) and every platform package that holds an addon into `<out>`.
 */
import { execFileSync } from "node:child_process"
import * as fs from "node:fs"
import * as path from "node:path"
import { fileURLToPath } from "node:url"
import { SHIPPED_PLATFORMS } from "../src/native/load.ts"

const root = fileURLToPath(new URL("..", import.meta.url))
const repo = path.join(root, "..")
const target = `${process.platform}-${process.arch}`

function run(command: string, args: readonly string[], cwd = root): void {
	execFileSync(command, args, { cwd, stdio: "inherit" })
}

function addon(profile: "debug" | "release"): string {
	run("cargo", ["build", "-p", "bumbledb-node", ...(profile === "release" ? ["--release"] : [])], repo)
	const library = process.platform === "darwin" ? "libbumbledb_node.dylib" : "libbumbledb_node.so"
	return path.join(process.env.CARGO_TARGET_DIR ?? path.join(repo, "target"), profile, library)
}

function install(from: string, to: string): void {
	fs.rmSync(to, { force: true })
	fs.copyFileSync(from, to)
}

function dist(): void {
	fs.rmSync(path.join(root, "dist"), { recursive: true, force: true })
	run(path.join(root, "node_modules", ".bin", "tsc"), ["-p", "tsconfig.build.json"])
	const binding = path.join(root, "src", "native", "binding.d.ts")
	if (fs.existsSync(binding)) fs.copyFileSync(binding, path.join(root, "dist", "native", "binding.d.ts"))
}

/** Packs `files` from `dir` into `out`, with `manifest` as the packed package.json. */
function packAs(dir: string, files: readonly string[], manifest: object, out: string): void {
	const scratch = fs.mkdtempSync(path.join(out, ".stage-"))
	try {
		for (const entry of files) fs.cpSync(path.join(dir, entry), path.join(scratch, entry), { recursive: true })
		fs.writeFileSync(path.join(scratch, "package.json"), `${JSON.stringify(manifest, null, "\t")}\n`)
		run("pnpm", ["pack", "--pack-destination", path.resolve(out)], scratch)
	} finally {
		fs.rmSync(scratch, { recursive: true, force: true })
	}
}

const readManifest = (dir: string) => JSON.parse(fs.readFileSync(path.join(dir, "package.json"), "utf8"))

/** The main package's version is the only version; every platform package is stamped with it. */
function stage(out: string): void {
	fs.mkdirSync(out, { recursive: true })
	const manifest = readManifest(root)
	const optionalDependencies = Object.fromEntries(
		SHIPPED_PLATFORMS.map((platform) => [`@bjornpagen/bumbledb-${platform}`, manifest.version])
	)
	const { devDependencies: _, scripts: __, ...published } = manifest
	packAs(root, [...manifest.files, "README.md", "LICENSE"], { ...published, optionalDependencies }, out)
	for (const platform of SHIPPED_PLATFORMS) {
		const dir = path.join(root, "npm", platform)
		if (!fs.existsSync(path.join(dir, "bdb.node"))) continue
		const { name, ...rest } = readManifest(dir)
		packAs(dir, [...rest.files, "LICENSE"], { name, version: manifest.version, ...rest }, out)
	}
}

const [mode, out] = process.argv.slice(2)
if (mode === "dev") {
	install(addon("debug"), path.join(root, `bdb.${target}.node`))
} else if (mode === "dist") {
	dist()
} else if (mode === "release") {
	install(addon("release"), path.join(root, "npm", target, "bdb.node"))
	dist()
} else if (mode === "stage" && out !== undefined) {
	stage(out)
} else {
	console.error("usage: node scripts/build.ts dev | dist | release | stage <out>")
	process.exitCode = 2
}
