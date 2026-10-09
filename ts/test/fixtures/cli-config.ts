import * as path from "node:path"
import { FsStore } from "../../src/database/fs.ts"
import { Migration } from "../../src/database/migration.ts"
import { App } from "./cli-schema.ts"

const root = process.env.BUMBLEDB_TEST_DIR ?? ""

export default {
	schema: App,
	migrations: [Migration.make({ id: "0001_init", hash: "1".repeat(64), to: App })],
	store: FsStore.make(path.join(root, "log")),
	cache: { directory: path.join(root, "cache") }
}
