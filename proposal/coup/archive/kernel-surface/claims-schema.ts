// Public declaration structure. These laws validate the shape of a bluff,
// its action, and its target. They do not require the claimed card to exist
// in the speaker's hand. Temporal legality is a separate transition contract.
import {
  alternatives, closed, closedId, contained, key, mirrors, on,
  relation, schema, select, u64, uuid
} from "@bjornpagen/bumbledb"
import { Role, Seat, SeatNo, gameLaws, gameMembers } from "./vocabulary.ts"

const ActionKind = closed("ActionKind", [
  "Income", "ForeignAid", "Tax", "Assassinate", "Steal", "Exchange", "Coup"
])
const ActionRole = closed("ActionRole", ["Tax", "Assassinate", "Steal", "Exchange"], {
  kind: closedId(ActionKind), role: closedId(Role)
}, {
  Tax: { kind: "Tax", role: "Duke" },
  Assassinate: { kind: "Assassinate", role: "Assassin" },
  Steal: { kind: "Steal", role: "Captain" },
  Exchange: { kind: "Exchange", role: "Ambassador" }
})
const BlockRole = closed("BlockRole", [
  "AidByDuke", "AssassinationByContessa", "StealByCaptain", "StealByAmbassador"
], { kind: closedId(ActionKind), role: closedId(Role) }, {
  AidByDuke: { kind: "ForeignAid", role: "Duke" },
  AssassinationByContessa: { kind: "Assassinate", role: "Contessa" },
  StealByCaptain: { kind: "Steal", role: "Captain" },
  StealByAmbassador: { kind: "Steal", role: "Ambassador" }
})
// Closed targets are addressed by id only. Pin their complete ground facts
// into ordinary keyed relations to support a composite (kind, role) probe.
const AllowedActionRole = relation("AllowedActionRole", {
  id: closedId(ActionRole), kind: closedId(ActionKind), role: closedId(Role)
})
const AllowedBlockRole = relation("AllowedBlockRole", {
  id: closedId(BlockRole), kind: closedId(ActionKind), role: closedId(Role)
})
const ClaimOrigin = closed("ClaimOrigin", ["Action", "Block"])

const Action = relation("Action", {
  game: uuid, id: uuid, turn: u64, actor: closedId(SeatNo), kind: closedId(ActionKind)
})
const Target = relation("Target", {
  game: uuid, action: uuid, seat: closedId(SeatNo)
})
const Claim = relation("Claim", {
  game: uuid, id: uuid, speaker: closedId(SeatNo),
  role: closedId(Role), origin: closedId(ClaimOrigin)
})
const ActionClaim = relation("ActionClaim", {
  game: uuid, claim: uuid, action: uuid, actor: closedId(SeatNo),
  kind: closedId(ActionKind), role: closedId(Role)
})
const BlockClaim = relation("BlockClaim", {
  game: uuid, claim: uuid, action: uuid, speaker: closedId(SeatNo),
  kind: closedId(ActionKind), role: closedId(Role)
})
// One admitted challenge per claim; competing requests belong to a reaction
// window, whose arbitration is not specified by this historical relation.
const Challenge = relation("Challenge", {
  game: uuid, id: uuid, claim: uuid, challenger: closedId(SeatNo)
})

const claimKey = key(Claim, ["game", "id"])
const claimOrigins = {
  Action: key(ActionClaim, ["game", "claim"]),
  Block: key(BlockClaim, ["game", "claim"])
} as const

export const CoupDeclarations = schema("CoupDeclarations", {
  ...gameMembers, ActionKind, ActionRole, BlockRole,
  AllowedActionRole, AllowedBlockRole, ClaimOrigin,
  Action, Target, Claim, ActionClaim, BlockClaim, Challenge
}, [
  ...gameLaws,
  key(AllowedActionRole, ["id"]), key(AllowedActionRole, ["kind", "role"]),
  key(AllowedActionRole, ["id", "kind", "role"]),
  key(AllowedBlockRole, ["id"]), key(AllowedBlockRole, ["kind", "role"]),
  key(AllowedBlockRole, ["id", "kind", "role"]),
  key(Action, ["game", "id"]), key(Action, ["game", "turn"]),
  key(Action, ["game", "id", "kind"]),
  key(Action, ["game", "id", "actor", "kind"]),
  key(Target, ["game", "action"]), key(Target, ["game", "action", "seat"]),
  claimKey, key(Claim, ["game", "id", "speaker", "role"]),
  claimOrigins.Action, claimOrigins.Block,
  key(ActionClaim, ["game", "action"]),
  key(ActionClaim, ["game", "action", "actor", "kind"]),
  key(Challenge, ["game", "id"]), key(Challenge, ["game", "claim"]),

  contained(on(ActionRole, "kind"), on(ActionKind, "id")),
  contained(on(ActionRole, "role"), on(Role, "id")),
  contained(on(BlockRole, "kind"), on(ActionKind, "id")),
  contained(on(BlockRole, "role"), on(Role, "id")),
  contained(on(AllowedActionRole, "id"), on(ActionRole, "id")),
  contained(on(ActionRole, ["id", "kind", "role"]), on(AllowedActionRole, ["id", "kind", "role"])),
  contained(on(AllowedBlockRole, "id"), on(BlockRole, "id")),
  contained(on(BlockRole, ["id", "kind", "role"]), on(AllowedBlockRole, ["id", "kind", "role"])),
  contained(on(Action, ["game", "actor"]), on(Seat, ["game", "seat"])),
  contained(on(Action, "kind"), on(ActionKind, "id")),

  // Exactly the three targeted action kinds have exactly one target row.
  mirrors(
    on(select(Action, { kind: ["Steal", "Assassinate", "Coup"] }), ["game", "id"]),
    on(Target, ["game", "action"])
  ),
  contained(on(Target, ["game", "seat"]), on(Seat, ["game", "seat"])),

  contained(on(Claim, ["game", "speaker"]), on(Seat, ["game", "seat"])),
  contained(on(Claim, "role"), on(Role, "id")),
  ...alternatives(claimKey, "origin", ClaimOrigin, claimOrigins),
  contained(
    on(ActionClaim, ["game", "claim", "actor", "role"]),
    on(Claim, ["game", "id", "speaker", "role"])
  ),
  mirrors(
    on(select(Action, { kind: ["Tax", "Assassinate", "Steal", "Exchange"] }), ["game", "id", "actor", "kind"]),
    on(ActionClaim, ["game", "action", "actor", "kind"])
  ),
  contained(on(ActionClaim, ["kind", "role"]), on(AllowedActionRole, ["kind", "role"])),

  contained(
    on(BlockClaim, ["game", "claim", "speaker", "role"]),
    on(Claim, ["game", "id", "speaker", "role"])
  ),
  contained(on(BlockClaim, ["game", "action", "kind"]), on(Action, ["game", "id", "kind"])),
  contained(on(BlockClaim, ["kind", "role"]), on(AllowedBlockRole, ["kind", "role"])),
  contained(on(BlockClaim, "kind"), on(ActionKind, "id")),
  // Only the target can block a steal or assassination. Foreign Aid allows
  // other players to block; the actor != blocker rule remains a transition law.
  contained(
    on(select(BlockClaim, { kind: ["Steal", "Assassinate"] }), ["game", "action", "speaker"]),
    on(Target, ["game", "action", "seat"])
  ),
  contained(on(Challenge, ["game", "claim"]), on(Claim, ["game", "id"])),
  contained(on(Challenge, ["game", "challenger"]), on(Seat, ["game", "seat"]))
])
