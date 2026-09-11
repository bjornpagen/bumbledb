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
export const r2 = db.relation("Document", { ["id"]: db.u64, ["digest"]: db.str })
export const r3 = db.relation("Imported", { ["submission"]: db.u64, ["reference"]: db.str })
export const r4 = db.relation("Electronic", { ["submission"]: db.u64, ["reference"]: db.str })
export const r5 = db.relation("Postal", { ["submission"]: db.u64, ["reference"]: db.str })

const laws: db.Statement[] = [
  db.key(r1, ["id"]),
  db.key(r2, ["id"]),
  db.key(r3, ["submission"]),
  db.key(r4, ["submission"]),
  db.key(r5, ["submission"]),
  db.contained(db.on(r1, ["method"]), db.on(r0, ["id"])),
  db.contained(db.on(db.select(r1, { ["method"]: "Imported" }), ["id"]), db.on(r3, ["submission"])),
  db.contained(db.on(r3, ["submission"]), db.on(db.select(r1, { ["method"]: "Imported" }), ["id"])),
  db.contained(db.on(db.select(r1, { ["method"]: "Electronic" }), ["id"]), db.on(r4, ["submission"])),
  db.contained(db.on(r4, ["submission"]), db.on(db.select(r1, { ["method"]: "Electronic" }), ["id"])),
  db.contained(db.on(db.select(r1, { ["method"]: "Postal" }), ["id"]), db.on(r5, ["submission"])),
  db.contained(db.on(r5, ["submission"]), db.on(db.select(r1, { ["method"]: "Postal" }), ["id"])),
]

export const schema = db.schema("Snapshot", {
  ["Kind"]: r0,
  ["Submission"]: r1,
  ["Document"]: r2,
  ["Imported"]: r3,
  ["Electronic"]: r4,
  ["Postal"]: r5,
}, laws)
