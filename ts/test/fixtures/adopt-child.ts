/**
 * Subprocess half of the exhume suite's adoption test: opens the legacy
 * store copy under its CREATING schema (`fixtures/legacy-schema.ts`) from a
 * REAL second process. The parent must never open it itself — `Db` values
 * are cached for the life of the process and hold the store's exclusive
 * lock, which would refuse the parent's own exhume afterward. The
 * fingerprint-matching open BACK-FILLS the persisted descriptor (engine
 * 50-storage.md § the `_meta` block); this process then exits cleanly,
 * releasing the lock, and the parent exhumes the now-self-describing store.
 */

import { Db } from "#index.ts"
import { Doc, legacySchema, Tagged } from "#test/fixtures/legacy-schema.ts"

const dir = process.argv[2]
if (dir === undefined) {
	process.stderr.write("usage: adopt-child.ts <store-dir>\n")
	process.exit(2)
}

const db = await Db.open(dir, legacySchema)
const report = JSON.stringify({
	docRows: db.scan(Doc).length,
	taggedRows: db.scan(Tagged).length
})
process.stdout.write(`${report}\n`)
