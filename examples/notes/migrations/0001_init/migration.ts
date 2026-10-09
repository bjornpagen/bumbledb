import { Migration } from "@bjornpagen/bumbledb"
import { r1 as Tag, schema as to } from "./schema.ts"

export default Migration.make({
	id: "0001_init",
	hash: "52189bd68f4ffbee5ef373d62f0fbc0faa1ab025bb6b88ae1ca4bd48d77b6dcf",
	to,
	populate: ({ into }) =>
		into.insert(Tag, [
			{ id: "00000000-0000-0000-0000-000000000001", name: "inbox" },
			{ id: "00000000-0000-0000-0000-000000000002", name: "archive" }
		])
})
