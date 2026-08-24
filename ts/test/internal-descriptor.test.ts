import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import { closed } from "#closed.ts"
import { on } from "#face.ts"
import { str, u64 } from "#fields.ts"
import { lower } from "#lower.ts"
import { dbClose, internalDescriptor, native } from "#native.ts"
import { relation } from "#relation.ts"
import { schema } from "#schema.ts"
import { contained, key, mirrors } from "#statements.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-internal-descriptor-"))
const storeDir = path.join(tmpRoot, "store")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const Status = closed("Status", ["Open", "Frozen"])
const Note = relation("Note", { id: u64.fresh, title: str, status: Status.id })
const Alias = relation("Alias", { note: u64, title: str })
const Theory = schema("Sealed", { Status, Note, Alias }, [
	contained(on(Note, "status"), on(Status, "id")),
	key(Alias, ["note"]),
	mirrors(on(Alias, "note"), on(Note, "id"))
])

describe("internalDescriptor seals without opening a store", function suite() {
	test("relations, closed rosters, materialized statements, and the real fingerprint", async function seal() {
		const spec = lower(Theory)
		const sealed = internalDescriptor(spec)

		assert.deepEqual(
			sealed.relations.map(function nameId(relation) {
				return [relation.name, relation.id]
			}),
			[
				["Status", 0],
				["Note", 1],
				["Alias", 2]
			]
		)

		const status = sealed.relations[0]
		assert.ok(status?.extension, "closed Status carries its resolved roster")
		assert.deepEqual(
			status.fields.map(function field(entry) {
				return [entry.name, entry.id, entry.valueType]
			}),
			[["id", 0, { kind: "u64" }]]
		)
		assert.deepEqual(
			status.extension.map(function row(entry) {
				return [entry.handle, entry.id]
			}),
			[
				["Open", 0n],
				["Frozen", 1n]
			]
		)

		const note = sealed.relations[1]
		assert.equal(note?.extension, undefined)
		assert.deepEqual(
			note?.fields.map(function field(entry) {
				return [entry.name, entry.id]
			}),
			[
				["id", 0],
				["title", 1],
				["status", 2]
			]
		)

		assert.deepEqual(
			sealed.statements.map(function kindId(statement) {
				return [statement.id, statement.kind]
			}),
			[
				[0, "functionality"],
				[1, "functionality"],
				[2, "containment"],
				[3, "functionality"],
				[4, "containment"],
				[5, "containment"]
			]
		)

		const fresh = sealed.statements[0]
		assert.equal(fresh?.kind, "functionality")
		if (fresh.kind === "functionality") {
			assert.deepEqual(fresh, { id: 0, kind: "functionality", relation: 1, projection: [0] })
		}

		const closedKey = sealed.statements[1]
		assert.equal(closedKey?.kind, "functionality")
		if (closedKey.kind === "functionality") {
			assert.deepEqual(closedKey, { id: 1, kind: "functionality", relation: 0, projection: [0] })
		}

		const statusContainment = sealed.statements[2]
		assert.equal(statusContainment?.kind, "containment")
		if (statusContainment.kind === "containment") {
			assert.deepEqual(statusContainment.source, { relation: 1, projection: [2], selection: [] })
			assert.deepEqual(statusContainment.target, { relation: 0, projection: [0], selection: [] })
		}

		const created = await native.dbCreate(storeDir, spec)
		assert.equal(created.tag, "accepted")
		assert.equal(native.dbFingerprint(created.db), sealed.fingerprint)
		assert.deepEqual(native.dbManifest(created.db).relations, sealed.relations)
		dbClose(created.db)
	})

	test("an unresolvable spec throws a typed schema error", function refused() {
		assert.throws(
			function badSpec() {
				internalDescriptor({
					relations: [],
					statements: [{ kind: "fd", relation: "Ghost", projection: ["id"] }]
				})
			},
			function typed(error: unknown) {
				assert.ok(error instanceof Error)
				assert.match(error.message, /Ghost/)
				return true
			}
		)
	})
})
