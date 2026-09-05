/**
 * Bounded load of the committed generated chain. The runner input is
 * `{ manifest, plans, snapshots }` — snapshots are the empty-base schema
 * plus one target per manifest entry. Fabricating plan bytes is not a
 * specimen.
 */
import * as fs from "node:fs"
import * as path from "node:path"
import {
	decodeGeneratedMigrations,
	decodeManifestData,
	type GeneratedMigrations
} from "@bjornpagen/bumbledb-log/migrations"

export const generatedDirectory = (cwd = process.cwd()) => path.join(cwd, "bumbledb", "migrations")

export function loadGeneratedMigrations(directory = generatedDirectory()): GeneratedMigrations {
	const manifestPath = path.join(directory, "manifest.json")
	if (!fs.existsSync(manifestPath)) {
		throw new Error("generated migrations are required: run `pnpm run generate` (F3) — fabricating plan bytes is not a specimen")
	}
	const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"))
	const manifestDecoded = decodeManifestData(manifest)
	if (!manifestDecoded.ok) {
		throw new Error(`generated migrations refuse decoding: ${manifestDecoded.detail}`)
	}
	const plans = manifestDecoded.value.entries.map((entry) =>
		JSON.parse(fs.readFileSync(path.join(directory, `${entry.id}.plan.json`), "utf8"))
	)
	const snapshotsPath = path.join(directory, "snapshots.json")
	if (!fs.existsSync(snapshotsPath)) {
		throw new Error("generated snapshots.json is required: the runner input is { manifest, plans, snapshots }")
	}
	const snapshots = JSON.parse(fs.readFileSync(snapshotsPath, "utf8"))
	const decoded = decodeGeneratedMigrations({ manifest, plans, snapshots })
	if (!decoded.ok) {
		throw new Error(`generated migrations refuse decoding: ${decoded.detail}`)
	}
	return decoded.value
}
