import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as path from "node:path"
import { describe, test } from "node:test"
import * as errors from "@superbuilders/errors"
import { parseSidecar } from "#chain.ts"
import { decodeBatch } from "#codec.ts"
import { ErrRefused, refusalOf } from "#errors.ts"
import { parseCheckpoint, parseManifest } from "#manifest.ts"
import { corpusRoot, documentIdentity, loadDescriptors, pinned } from "#test/v3-support.ts"

const present = fs.existsSync(path.join(corpusRoot, "fuzz", "storm.json"))

if (!present) {
	describe("v3 fuzz", function suite() {
		test("skipped: conformance/v3/fuzz is not in the tree", { skip: true }, function absent() {})
	})
} else {
	const descriptors = loadDescriptors()

	describe("v3 materialised fuzz", function suite() {
		for (const folder of ["batch", "documents"] as const) {
			const dir = path.join(corpusRoot, "fuzz", folder)
			for (const file of fs.readdirSync(dir, { recursive: true })) {
				const name = String(file)
				if (!name.endsWith(".json")) {
					continue
				}
				test(`fuzz/${folder}/${name}`, function golden() {
					const fixture = JSON.parse(fs.readFileSync(path.join(dir, name), "utf8")) as {
						expect: "refusal"
						refusal: string
						schema?: string
						fingerprint?: string
						kind?: "manifest" | "checkpoint" | "sidecar"
					}
					const bytes = new Uint8Array(fs.readFileSync(path.join(dir, name.replace(/\.json$/, ".bin"))))
					const descriptor =
						fixture.schema === undefined
							? undefined
							: fixture.fingerprint === undefined
								? descriptors.get(fixture.schema)
								: pinned(descriptors, fixture.schema, fixture.fingerprint)
					const known = descriptor === undefined ? undefined : new Set(descriptor.braidMembers.keys())
					const ran = errors.trySync(function parseIt() {
						if (folder === "batch") {
							assert.ok(descriptor !== undefined, "batch fuzz cites a schema")
							return decodeBatch(descriptor, bytes)
						}
						if (fixture.kind === "manifest") {
							return parseManifest(bytes)
						}
						if (fixture.kind === "checkpoint") {
							return parseCheckpoint(bytes, known)
						}
						return parseSidecar(bytes, known)
					})
					assert.ok(ran.error, `${name}: expected a refusal`)
					assert.ok(errors.is(ran.error, ErrRefused), `${name}: expected ErrRefused`)
					const cause = refusalOf(ran.error)
					assert.ok(cause !== undefined, `${name}: typed identity`)
					assert.equal(documentIdentity(cause.kind), fixture.refusal, `${name}: refusal identity`)
				})
			}
		}
	})

	describe("v3 storm recipe is present", function suite() {
		test("storm.json names the XorShift64 mutation lane", function recipe() {
			const storm = JSON.parse(fs.readFileSync(path.join(corpusRoot, "fuzz", "storm.json"), "utf8")) as {
				prng: { name: string; batch_storm_iters: number }
				goldens: { batch: readonly string[] }
			}
			assert.equal(storm.prng.name, "XorShift64")
			assert.ok(storm.goldens.batch.length > 0)
			assert.ok(storm.prng.batch_storm_iters >= 2000)
		})
	})
}
