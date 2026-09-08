import * as fs from "node:fs"
import * as path from "node:path"

/** Replace, never overwrite, a potentially loaded/code-signed native inode. */
export function installNativeArtifact(source: string, destination: string): void {
	const staging = fs.mkdtempSync(path.join(path.dirname(destination), ".native-install-"))
	try {
		const candidate = path.join(staging, "bumbledb.node")
		fs.copyFileSync(source, candidate)
		fs.renameSync(candidate, destination)
	} finally {
		fs.rmSync(staging, { recursive: true, force: true })
	}
}
