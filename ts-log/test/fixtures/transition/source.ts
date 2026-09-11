// Generated from a native-verified schema snapshot.
import * as db from "@bjornpagen/bumbledb"

const h0 = ["Imported", "Electronic", "Postal"] as const
const c0 = { kind: "u64", closed: { name: "Kind", handles: h0 } } as const
export const r0 = db.closed("Kind", h0, {  }, {
  ["Imported"]: {  },
  ["Electronic"]: {  },
  ["Postal"]: {  },
})
export const r1 = db.relation("Submission", { ["id"]: db.u64, ["method"]: c0, ["recordedAt"]: db.i64 })
export const r2 = db.relation("Proof", { ["submission"]: db.u64, ["reference"]: db.str })
export const r3 = db.relation("Document", { ["id"]: db.u64, ["digest"]: db.str })

const laws: db.Statement[] = [
  db.key(r1, ["id"]),
  db.key(r2, ["submission"]),
  db.key(r3, ["id"]),
  db.contained(db.on(r1, ["method"]), db.on(r0, ["id"])),
  db.contained(db.on(r1, ["id"]), db.on(r2, ["submission"])),
  db.contained(db.on(r2, ["submission"]), db.on(r1, ["id"])),
]

export const schema = db.schema("Snapshot", {
  ["Kind"]: r0,
  ["Submission"]: r1,
  ["Proof"]: r2,
  ["Document"]: r3,
}, laws)
