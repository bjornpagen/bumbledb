// Generated from a native-verified schema snapshot.
import * as db from "@bjornpagen/bumbledb"

export const r0 = db.relation("Note", { ["id"]: db.uuid, ["text"]: db.str, ["pinned"]: db.bool })
export const r1 = db.relation("Tag", { ["id"]: db.uuid, ["name"]: db.str })
export const r2 = db.relation("Outbox", { ["id"]: db.uuid, ["note"]: db.uuid, ["kind"]: db.str })
export const r3 = db.relation("Attachment", { ["id"]: db.uuid, ["note"]: db.uuid, ["key"]: db.str, ["bytes"]: db.u64 })

const laws: db.Statement[] = [
  db.key(r0, ["id"]),
  db.key(r1, ["id"]),
  db.key(r2, ["id"]),
  db.key(r3, ["id"]),
  db.contained(db.on(r3, ["note"]), db.on(r0, ["id"])),
]

export const schema = db.schema("Snapshot", {
  ["Note"]: r0,
  ["Tag"]: r1,
  ["Outbox"]: r2,
  ["Attachment"]: r3,
}, laws)
