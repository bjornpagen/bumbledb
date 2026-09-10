import { createHash } from "node:crypto"
import * as fs from "node:fs"
import * as path from "node:path"
import { ScriptError } from "./errors.ts"

/** A package label cannot establish which source produced a native binary. */
export function assertNativeProvenance(
	binary: string,
	platform: string,
	expected: { readonly candidateSourceDigest: string; readonly specificationRevision: string }
): void {
	const stampPath = path.join(path.dirname(binary), ".native-provenance.json")
	if (!fs.existsSync(stampPath))
		throw new ScriptError({
			message: `native provenance missing for ${platform}; rebuild or obtain the matching CI artifact and provenance`
		})
	const stamp = JSON.parse(fs.readFileSync(stampPath, "utf8"))
	const hash = createHash("sha256").update(fs.readFileSync(binary)).digest("hex")
	if (
		stamp?.candidateSourceDigest !== expected.candidateSourceDigest ||
		stamp?.specificationRevision !== expected.specificationRevision ||
		stamp?.platform !== platform ||
		stamp?.artifact?.path !== `ts/npm/${platform}/bumbledb.node` ||
		stamp?.artifact?.sha256 !== hash
	) {
		throw new ScriptError({
			message: `native provenance is stale or mismatched for ${platform}; refuse to label this binary as the current release`
		})
	}
}

/** Replace, never overwrite, a potentially loaded/code-signed native inode. */
export function installNativeArtifact(source: string, destination: string): void {
	const staging = fs.mkdtempSync(path.join(path.dirname(destination), "native-install-"))
	try {
		const candidate = path.join(staging, "bumbledb.node")
		fs.copyFileSync(source, candidate)
		fs.renameSync(candidate, destination)
	} finally {
		fs.rmSync(staging, { recursive: true, force: true })
	}
}
