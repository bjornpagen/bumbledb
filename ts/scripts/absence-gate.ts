import { spawnSync } from "node:child_process"
import * as fs from "node:fs"
import * as path from "node:path"
import { fileURLToPath } from "node:url"
import { Result } from "effect"
import { ScriptError } from "./errors.ts"
import { EFFECT_PIN } from "./pin.ts"

/**
 * The affirmative product-deletion gate (PKG-06 / SDK-013 / ARCH-005 /
 * chapter 32 "Delete the whole C surface"). Deleting a product is proved
 * by checks that FAIL if it comes back, not by its absence from a diff:
 *
 *  1. No C product anywhere in the release tree: no `bumbledb-c` crate,
 *     no public C headers, no cbindgen tooling, no C examples/smoke
 *     programs, no C workflow lane, no crate-type carrying a public C ABI.
 *  2. No public Rust product leaks: the workspace's `bumbledb` and
 *     `bumbledb-log` crates are `publish = false` (the public Rust core
 *     is a source-tree consumer surface until crate publication is
 *     separately authorized; the log crate is INTERNAL forever), and the
 *     log crate's modules are `#[doc(hidden)]` internal implementation —
 *     there is no supported public Rust log SDK.
 *  3. The public Rust core stays log/AWS-free: `crates/bumbledb`'s
 *     dependency table names no log, AWS, S3, object-store or async
 *     transport crate.
 *  4. Both TypeScript packages carry the exact Effect 4.0.0-rc.112
 *     peer+dev pin, no AWS/transport dependency (`ts-log` has NO
 *     `dependencies` at all — S3 lives natively), no committed
 *     `optionalDependencies`, and no source-mutating pack lifecycle hook
 *     (`prepack`/`postpack`/`preinstall`/`postinstall`).
 *  5. `@superbuilders` appears nowhere in maintained manifests, locks,
 *     source, scripts or workflows.
 *  6. No stale binary artifact is tracked: no `.node`, `.dylib`, `.so`,
 *     `.a`, `.tgz` in git.
 *
 * Scans TRACKED files only (`git ls-files`), so gitignored build output
 * and local node_modules never produce false findings, and preserved
 * historical evidence under `audit/` is explicitly exempt where noted.
 *
 * Run from anywhere: `node ts/scripts/absence-gate.ts`. Exit 0 is the
 * gate; every finding is listed before the failure.
 */

const REPO_ROOT = path.join(fileURLToPath(new URL("..", import.meta.url)), "..")

/** Historical evidence stays; the gate never demands audit rewrites. */
const EVIDENCE_PREFIXES = ["audit/", "docs/research/", "final-solution/", "implementation/", "proposals/"] as const

/** Dependency names that must never appear in the core crate's manifest. */
const CORE_FORBIDDEN_DEPS = [
	"bumbledb-log",
	"aws-config",
	"aws-sdk-s3",
	"aws-smithy",
	"object_store",
	"tokio",
	"hyper",
	"reqwest"
] as const

/** Text file extensions the content scans read. */
const TEXT_EXTENSIONS = new Set([
	".json",
	".jsonc",
	".lock",
	".md",
	".mjs",
	".rs",
	".sh",
	".toml",
	".ts",
	".tsx",
	".txt",
	".yaml",
	".yml"
])

function trackedFiles(): string[] {
	const listed = spawnSync("git", ["-c", `safe.directory=${REPO_ROOT}`, "-C", REPO_ROOT, "ls-files", "-z"])
	if (listed.error) {
		throw new ScriptError({ message: "spawn git ls-files", cause: listed.error })
	}
	if (listed.status !== 0) {
		throw new ScriptError({ message: `git ls-files exited with status ${listed.status}: ${listed.stderr.toString()}` })
	}
	return listed.stdout
		.toString("utf8")
		.split("\0")
		.filter((file) => file !== "")
}

function isEvidence(file: string): boolean {
	return EVIDENCE_PREFIXES.some((prefix) => file.startsWith(prefix))
}

function readText(rel: string): string {
	const text = Result.try(() => fs.readFileSync(path.join(REPO_ROOT, rel), "utf8"))
	if (Result.isFailure(text)) {
		throw new ScriptError({ message: `read ${rel}`, cause: text.failure })
	}
	return text.success
}

function readJson(rel: string): Record<string, unknown> {
	const parsed = Result.try(() => JSON.parse(readText(rel)) as Record<string, unknown>)
	if (Result.isFailure(parsed)) {
		throw new ScriptError({ message: `parse ${rel}`, cause: parsed.failure })
	}
	return parsed.success
}

function record(value: unknown): Record<string, unknown> {
	return typeof value === "object" && value !== null ? (value as Record<string, unknown>) : {}
}

function checkCSurface(files: readonly string[], findings: string[]): void {
	for (const file of files) {
		if (isEvidence(file)) {
			continue
		}
		if (/(^|\/)bumbledb-c(\/|$)/.test(file)) {
			findings.push(`C crate path survives: ${file}`)
		}
		if (/^crates\/.*\/include\//.test(file) || /^include\//.test(file)) {
			findings.push(`public C header directory survives: ${file}`)
		}
		if (file.endsWith(".h") || file.endsWith(".hpp")) {
			findings.push(`C header survives: ${file}`)
		}
		if (path.basename(file) === "cbindgen.toml") {
			findings.push(`C header generator config survives: ${file}`)
		}
		if (file.startsWith("examples/") && (file.endsWith(".c") || path.basename(file) === "Makefile")) {
			findings.push(`C example survives: ${file}`)
		}
	}
	for (const file of files) {
		if (!file.startsWith(".github/") || !(file.endsWith(".yml") || file.endsWith(".yaml"))) {
			continue
		}
		const text = readText(file)
		if (/bumbledb-c\b|bumbledb_c\b/.test(text)) {
			findings.push(`workflow still names the C crate: ${file}`)
		}
	}
	// No crate manifest may emit a public C ABI artifact.
	for (const file of files) {
		if (!file.startsWith("crates/") || path.basename(file) !== "Cargo.toml") {
			continue
		}
		const text = readText(file)
		if (/crate-type\s*=\s*\[[^\]]*"(cdylib|staticlib)"/.test(text)) {
			findings.push(`${file} builds a C-ABI artifact (cdylib/staticlib) inside the workspace`)
		}
	}
}

function cargoManifest(rel: string): string {
	return readText(rel)
}

function checkRustProducts(findings: string[]): void {
	for (const crate of ["crates/bumbledb/Cargo.toml", "crates/bumbledb-log/Cargo.toml"]) {
		const text = cargoManifest(crate)
		if (!/^publish = false$/m.test(text)) {
			findings.push(`${crate} is publishable — no Rust crate publication is authorized (publish = false required)`)
		}
	}
	const core = cargoManifest("crates/bumbledb/Cargo.toml")
	for (const dep of CORE_FORBIDDEN_DEPS) {
		const pattern = new RegExp(`^\\s*"?${dep.replace(/[-_]/g, "[-_]")}"?\\s*=`, "m")
		if (pattern.test(core)) {
			findings.push(`crates/bumbledb depends on ${dep} — the public core stays log/AWS/transport-free`)
		}
	}
	// The internal log crate exposes no supported public Rust log SDK: every
	// top-level module is #[doc(hidden)] internal implementation.
	const logLib = readText("crates/bumbledb-log/src/lib.rs")
	const lines = logLib.split("\n")
	for (let i = 0; i < lines.length; i += 1) {
		const line = lines[i] as string
		const declared = /^pub mod ([a-z_]+);/.exec(line)
		if (declared === null) {
			continue
		}
		let hidden = false
		for (let back = i - 1; back >= 0; back -= 1) {
			const previous = (lines[back] as string).trim()
			if (previous.startsWith("#[doc(hidden)]")) {
				hidden = true
				break
			}
			if (previous.startsWith("///") || previous.startsWith("//")) {
				continue
			}
			break
		}
		if (!hidden) {
			findings.push(
				`crates/bumbledb-log/src/lib.rs: pub mod ${declared[1]} is not #[doc(hidden)] — the log crate is internal, never a public Rust log SDK`
			)
		}
	}
}

function checkTsPackages(findings: string[]): void {
	for (const rel of [
		"ts/package.json",
		"ts-log/package.json",
		"ts/npm/darwin-arm64/package.json",
		"ts/npm/linux-arm64/package.json",
		"ts/npm/linux-x64/package.json"
	]) {
		const manifest = readJson(rel)
		if ("optionalDependencies" in manifest) {
			findings.push(`${rel} carries committed optionalDependencies — pins live only in the staged manifest`)
		}
		const scripts = record(manifest.scripts)
		for (const hook of ["prepack", "postpack", "preinstall", "postinstall", "prepare"]) {
			if (hook in scripts) {
				findings.push(`${rel} carries the ${hook} lifecycle hook — packing must not mutate or depend on the checkout`)
			}
		}
	}
	for (const rel of ["ts/package.json", "ts-log/package.json"]) {
		const manifest = readJson(rel)
		const peers = record(manifest.peerDependencies)
		if (peers.effect !== EFFECT_PIN) {
			findings.push(`${rel} peerDependencies.effect is ${String(peers.effect)}, expected exactly ${EFFECT_PIN}`)
		}
		const dev = record(manifest.devDependencies)
		if (dev.effect !== EFFECT_PIN) {
			findings.push(`${rel} devDependencies.effect is ${String(dev.effect)}, expected exactly ${EFFECT_PIN}`)
		}
		const deps = record(manifest.dependencies)
		for (const name of Object.keys(deps)) {
			if (name.startsWith("@aws-sdk/") || name === "aws-sdk" || name.startsWith("@smithy/")) {
				findings.push(`${rel} depends on ${name} — transport lives in the native runtime, never a JS client`)
			}
		}
	}
	const log = readJson("ts-log/package.json")
	if (Object.keys(record(log.dependencies)).length > 0) {
		findings.push(
			`ts-log/package.json carries dependencies ${JSON.stringify(log.dependencies)} — the log package is thin: core + effect peers only`
		)
	}
	const core = readJson("ts/package.json")
	const peer = record(log.peerDependencies)["@bjornpagen/bumbledb"]
	if (peer !== core.version) {
		findings.push(
			`ts-log peer on @bjornpagen/bumbledb is ${String(peer)}, expected the exact release version ${String(core.version)}`
		)
	}
}

/** Maintained code/manifest scopes; prose (PROMPT/proposal/docs) may NAME the ban. */
const CODE_PREFIXES = ["crates/", "ts/", "ts-log/", "examples/", "scripts/", ".github/", "lean/"] as const

function checkBannedText(files: readonly string[], findings: string[]): void {
	for (const file of files) {
		if (isEvidence(file) || !CODE_PREFIXES.some((prefix) => file.startsWith(prefix))) {
			continue
		}
		const extension = path.extname(file)
		if (!TEXT_EXTENSIONS.has(extension) && path.basename(file) !== "pnpm-lock.yaml") {
			continue
		}
		// The gate names its own token; skip self-matches in this file.
		if (file === "ts/scripts/absence-gate.ts") {
			continue
		}
		const text = readText(file)
		if (text.includes("@superbuilders")) {
			findings.push(`${file} mentions @superbuilders — direct Effect tagged errors only`)
		}
	}
}

function checkTrackedArtifacts(files: readonly string[], findings: string[]): void {
	for (const file of files) {
		if (isEvidence(file)) {
			continue
		}
		if (/\.(node|dylib|so|a|tgz)$/.test(file)) {
			findings.push(`tracked binary artifact: ${file} — release inputs are built fresh, never committed`)
		}
	}
}

function main(): void {
	const files = trackedFiles()
	const findings: string[] = []
	checkCSurface(files, findings)
	checkRustProducts(findings)
	checkTsPackages(findings)
	checkBannedText(files, findings)
	checkTrackedArtifacts(files, findings)
	if (findings.length > 0) {
		for (const finding of findings) {
			console.error(`absence-gate: ${finding}`)
		}
		throw new ScriptError({ message: `absence gate failed with ${findings.length} finding(s)` })
	}
	console.log(
		"absence-gate: OK — no C surface, no public Rust log product, core log/AWS-free, exact Effect pins, no pack hooks, no tracked binaries, no @superbuilders"
	)
}

main()
