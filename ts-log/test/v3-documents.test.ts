import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as path from "node:path"
import { describe, test } from "node:test"
import * as errors from "@superbuilders/errors"
import { toHex } from "#bytes.ts"
import { parseSidecar, renderSidecar } from "#chain.ts"
import { ErrRefused, refusalOf } from "#errors.ts"
import { parseCheckpoint, parseManifest, renderCheckpoint, renderManifest } from "#manifest.ts"
import { corpusRoot, documentIdentity, loadDescriptors } from "#test/v3-support.ts"

const present = fs.existsSync(path.join(corpusRoot, "inventory.json"))

if (!present) {
	describe("v3 documents", function suite() {
		test("skipped: conformance/v3 is not in the tree", { skip: true }, function absent() {})
	})
} else {
	const descriptors = loadDescriptors()

	describe("v3 document goldens", function suite() {
		for (const kind of ["manifest", "checkpoint", "sidecar"] as const) {
			const dir = path.join(corpusRoot, "documents", kind)
			for (const file of fs.readdirSync(dir)) {
				if (!file.endsWith(".json")) {
					continue
				}
				const stem = file.slice(0, -5)
				test(`documents/${kind}/${stem}`, function golden() {
					const fixture = JSON.parse(fs.readFileSync(path.join(dir, file), "utf8")) as {
						expect: "ok" | "refusal"
						schema?: string
						refusal?: string
					}
					const bytes = new Uint8Array(fs.readFileSync(path.join(dir, `${stem}.bin`)))
					const descriptor = fixture.schema === undefined ? undefined : descriptors.get(fixture.schema)
					const known = descriptor === undefined ? undefined : new Set(descriptor.braidMembers.keys())
					const ran = errors.trySync(function parseIt() {
						if (kind === "manifest") {
							return parseManifest(bytes)
						}
						if (kind === "checkpoint") {
							return parseCheckpoint(bytes, known)
						}
						return parseSidecar(bytes, known)
					})
					if (fixture.expect === "refusal") {
						assert.ok(ran.error, `${stem}: expected a refusal`)
						assert.ok(errors.is(ran.error, ErrRefused), `${stem}: expected ErrRefused`)
						const cause = refusalOf(ran.error)
						assert.ok(cause !== undefined, `${stem}: refusal carries a kind`)
						assert.equal(documentIdentity(cause.kind), fixture.refusal, `${stem}: refusal identity`)
						return
					}
					assert.equal(ran.error, undefined, `${stem}: ${ran.error?.message}`)
					if (kind === "manifest") {
						assert.equal(toHex(renderManifest(ran.data as ReturnType<typeof parseManifest>)), toHex(bytes))
						return
					}
					if (kind === "checkpoint") {
						assert.equal(toHex(renderCheckpoint(ran.data as ReturnType<typeof parseCheckpoint>)), toHex(bytes))
						return
					}
					assert.equal(
						toHex(new TextEncoder().encode(renderSidecar(ran.data as ReturnType<typeof parseSidecar>))),
						toHex(bytes)
					)
				})
			}
		}
	})
}
