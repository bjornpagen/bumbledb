// Referee state, using the current bumbledb schema API. See README.md for
// the separate transition, privacy, and proposed probability contracts.
import {
  alternatives, bool, capacity, closed, closedId, contained,
  key, on, relation, schema, uuid, within
} from "@bjornpagen/bumbledb"
import { Game, Role, Seat, SeatNo, Slot, gameLaws, gameMembers } from "./vocabulary.ts"

// Physical identities are REFEREE-PRIVATE. Even an opaque card handle leaks
// its role if a player can join it to this fixed roster.
const Card = closed("Card", [
  "Duke1", "Duke2", "Duke3",
  "Assassin1", "Assassin2", "Assassin3",
  "Captain1", "Captain2", "Captain3",
  "Ambassador1", "Ambassador2", "Ambassador3",
  "Contessa1", "Contessa2", "Contessa3"
], { role: closedId(Role) }, {
  Duke1: { role: "Duke" }, Duke2: { role: "Duke" }, Duke3: { role: "Duke" },
  Assassin1: { role: "Assassin" }, Assassin2: { role: "Assassin" }, Assassin3: { role: "Assassin" },
  Captain1: { role: "Captain" }, Captain2: { role: "Captain" }, Captain3: { role: "Captain" },
  Ambassador1: { role: "Ambassador" }, Ambassador2: { role: "Ambassador" }, Ambassador3: { role: "Ambassador" },
  Contessa1: { role: "Contessa" }, Contessa2: { role: "Contessa" }, Contessa3: { role: "Contessa" }
})
const Place = closed("Place", ["Court", "Influence", "Exchange"])
const CardState = relation("CardState", {
  game: uuid, card: closedId(Card), place: closedId(Place)
})
const CourtCard = relation("CourtCard", { game: uuid, card: closedId(Card) })
const InfluenceCard = relation("InfluenceCard", {
  game: uuid, card: closedId(Card), seat: closedId(SeatNo),
  slot: closedId(Slot), revealed: bool
})
const PendingExchange = relation("PendingExchange", {
  game: uuid, seat: closedId(SeatNo)
})
const ExchangeCard = relation("ExchangeCard", {
  game: uuid, card: closedId(Card), seat: closedId(SeatNo)
})

const cardKey = key(CardState, ["game", "card"])
const places = {
  Court: key(CourtCard, ["game", "card"]),
  Influence: key(InfluenceCard, ["game", "card"]),
  Exchange: key(ExchangeCard, ["game", "card"])
} as const

export const CoupState = schema("CoupState", {
  ...gameMembers, Slot, Card, Place, CardState, CourtCard, InfluenceCard,
  PendingExchange, ExchangeCard
}, [
  ...gameLaws,
  cardKey, places.Court, places.Influence, places.Exchange,
  key(InfluenceCard, ["game", "seat", "slot"]),
  key(PendingExchange, ["game"]),
  key(PendingExchange, ["game", "seat"]),
  contained(on(Card, "role"), on(Role, "id")),
  contained(on(CardState, "game"), on(Game, "id")),
  contained(on(CardState, "card"), on(Card, "id")),

  // 15 distinct IDs from a 15-card roster: every card exists once per game.
  capacity(on(Game, "id"), {
    from: on(CardState, "game"), within: within(15n)
  }),
  ...alternatives(cardKey, "place", Place, places),

  contained(on(InfluenceCard, ["game", "seat"]), on(Seat, ["game", "seat"])),
  contained(on(InfluenceCard, "slot"), on(Slot, "id")),
  // Lost influence remains in its slot, face up. Eliminated seats remain too.
  capacity(on(Seat, ["game", "seat"]), {
    from: on(InfluenceCard, ["game", "seat"]), within: within(2n)
  }),

  contained(on(PendingExchange, ["game", "seat"]), on(Seat, ["game", "seat"])),
  contained(on(ExchangeCard, ["game", "seat"]), on(PendingExchange, ["game", "seat"])),
  capacity(on(PendingExchange, "game"), {
    from: on(ExchangeCard, "game"), within: within(2n)
  })
])
