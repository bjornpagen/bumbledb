// Proposal-only declarations using the current authoring API. No database I/O.
import {
  capacity, closed, closedId, contained, key, on, relation, str,
  u64, uuid, within
} from "@bjornpagen/bumbledb"

export const Role = closed("Role", [
  "Duke", "Assassin", "Captain", "Ambassador", "Contessa"
])
export const Slot = closed("Slot", ["Left", "Right"])
export const SeatNo = closed("SeatNo", ["A", "B", "C", "D", "E", "F"], {
  clockwise: u64
}, {
  A: { clockwise: 0n }, B: { clockwise: 1n }, C: { clockwise: 2n },
  D: { clockwise: 3n }, E: { clockwise: 4n }, F: { clockwise: 5n }
})

// Admitted games have already been dealt. Lobby/setup states are out of scope.
export const Game = relation("Game", { id: uuid, publicRevision: u64 })
export const Seat = relation("Seat", {
  game: uuid, seat: closedId(SeatNo), name: str, coins: u64
})

export const gameMembers = { Role, SeatNo, Game, Seat } as const
export const gameLaws = [
  key(Game, ["id"]),
  key(Seat, ["game", "seat"]),
  contained(on(Seat, "game"), on(Game, "id")),
  contained(on(Seat, "seat"), on(SeatNo, "id")),
  capacity(on(Game, "id"), {
    from: on(Seat, "game"), within: within(2n, 6n)
  })
] as const
