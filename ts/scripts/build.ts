/**
 * `node scripts/build.ts dev`: debug addon at `bumbledb.<platform>-<arch>.node`, which the
 * loader prefers over platform packages.
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

function stage(out: string): void {
	fs.mkdirSync(out, { recursive: true })
	const manifest = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8"))
	const scratch = fs.mkdtempSync(path.join(out, ".stage-"))
	try {
		for (const entry of [...manifest.files, "README.md", "LICENSE"]) {
			fs.cpSync(path.join(root, entry), path.join(scratch, entry), { recursive: true })
		}
		const optionalDependencies = Object.fromEntries(
			SHIPPED_PLATFORMS.map((platform) => [`@bjornpagen/bumbledb-${platform}`, manifest.version])
		)
		const { devDependencies: _, scripts: __, ...published } = manifest
		fs.writeFileSync(
			path.join(scratch, "package.json"),
			`${JSON.stringify({ ...published, optionalDependencies }, null, "\t")}\n`
		)
		run("pnpm", ["pack", "--pack-destination", path.resolve(out)], scratch)
	} finally {
		fs.rmSync(scratch, { recursive: true, force: true })
	}
	for (const platform of SHIPPED_PLATFORMS) {
		const dir = path.join(root, "npm", platform)
		if (fs.existsSync(path.join(dir, "bumbledb.node")))
			run("pnpm", ["pack", "--pack-destination", path.resolve(out)], dir)
	}
}

const [mode, out] = process.argv.slice(2)
if (mode === "dev") {
	install(addon("debug"), path.join(root, `bumbledb.${target}.node`))
} else if (mode === "release") {
	install(addon("release"), path.join(root, "npm", target, "bumbledb.node"))
	dist()
} else if (mode === "stage" && out !== undefined) {
	stage(out)
} else {
	console.error("usage: node scripts/build.ts dev | release | stage <out>")
	process.exitCode = 2
}
