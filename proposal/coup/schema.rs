// Proposal, not an engine implementation. `event`, event pointwise laws,
// and typed `true` projections extend today's macro. See README.md.
// Every event in one Perspective uses its captured, aligned joint world scope.
bumbledb::schema! {
    pub Coup;

    closed relation Role as RoleId = { Duke, Assassin, Captain, Ambassador, Contessa };
    closed relation SeatNo as SeatNoId { clockwise: u64 } = {
        A { clockwise: 0 }, B { clockwise: 1 }, C { clockwise: 2 },
        D { clockwise: 3 }, E { clockwise: 4 }, F { clockwise: 5 },
    };
    closed relation GameSize as GameSizeId = { Two, Three, Four, Five, Six };
    closed relation ViewKind as ViewKindId = { Player, Referee };
    closed relation Slot as SlotId = { Left, Right };
    closed relation Place as PlaceId = { Court, Hand, DrawFirst, DrawSecond };
    closed relation Card as CardId { role: u64 as RoleId } = {
        Duke1 { role: Duke }, Duke2 { role: Duke }, Duke3 { role: Duke },
        Assassin1 { role: Assassin }, Assassin2 { role: Assassin }, Assassin3 { role: Assassin },
        Captain1 { role: Captain }, Captain2 { role: Captain }, Captain3 { role: Captain },
        Ambassador1 { role: Ambassador }, Ambassador2 { role: Ambassador }, Ambassador3 { role: Ambassador },
        Contessa1 { role: Contessa }, Contessa2 { role: Contessa }, Contessa3 { role: Contessa },
    };

    relation Game { id: u64 as GameId, size: u64 as GameSizeId }
    relation Seat { game: u64 as GameId, seat: u64 as SeatNoId, name: str }

    // `given` is the accumulated observation event. Its probability survives.
    // Perspective is a view of one adopted law; no process/model field.
    relation Perspective {
        id: u64 as PerspectiveId, game: u64 as GameId, kind: u64 as ViewKindId,
        public_revision: u64, private_revision: u64, given: event,
    }
    relation PlayerView { perspective: u64 as PerspectiveId, game: u64 as GameId, observer: u64 as SeatNoId }
    relation RefereeView { perspective: u64 as PerspectiveId, game: u64 as GameId }
    // Positions share a perspective's world trajectories. A replacement gets
    // a new position; intersecting old and new facts refers to the same trace.
    relation Position {
        id: u64 as PositionId, perspective: u64 as PerspectiveId,
        game: u64 as GameId, size: u64 as GameSizeId, step: u64,
    }
    relation PositionSeat {
        position: u64 as PositionId, game: u64 as GameId, seat: u64 as SeatNoId,
    }
    relation PositionCard { position: u64 as PositionId, card: u64 as CardId }
    relation HandSlot { position: u64 as PositionId, seat: u64 as SeatNoId, slot: u64 as SlotId }

    // One card has one place at every world; sidecars describe the place.
    relation At {
        position: u64 as PositionId, card: u64 as CardId,
        place: u64 as PlaceId, when: event,
    }
    relation CourtCard { position: u64 as PositionId, card: u64 as CardId, when: event }
    relation Influence {
        position: u64 as PositionId, card: u64 as CardId,
        seat: u64 as SeatNoId, slot: u64 as SlotId, revealed: bool, when: event,
    }
    relation PendingExchange { position: u64 as PositionId, seat: u64 as SeatNoId, when: event }
    relation FirstDraw { position: u64 as PositionId, card: u64 as CardId, seat: u64 as SeatNoId, when: event }
    relation SecondDraw { position: u64 as PositionId, card: u64 as CardId, seat: u64 as SeatNoId, when: event }
    relation Balance { position: u64 as PositionId, seat: u64 as SeatNoId, coins: u64, when: event }

    // Recorded declarations are ordinary facts. Holding the claimed role is
    // deliberately not an admission prerequisite for a declaration.
    closed relation ActionKind as ActionKindId = { Income, ForeignAid, Tax, Assassinate, Steal, Exchange, CoupAction };
    closed relation ClaimOrigin as ClaimOriginId = { Action, Block };
    closed relation ActionRoleCatalog as ActionRoleCatalogId {
        kind: u64 as ActionKindId, role: u64 as RoleId,
    } = {
        TaxRule { kind: Tax, role: Duke },
        AssassinateRule { kind: Assassinate, role: Assassin },
        StealRule { kind: Steal, role: Captain },
        ExchangeRule { kind: Exchange, role: Ambassador },
    };
    closed relation BlockRoleCatalog as BlockRoleCatalogId {
        kind: u64 as ActionKindId, role: u64 as RoleId,
    } = {
        AidDuke { kind: ForeignAid, role: Duke },
        AssassinateContessa { kind: Assassinate, role: Contessa },
        StealCaptain { kind: Steal, role: Captain },
        StealAmbassador { kind: Steal, role: Ambassador },
    };
    // These ordinary catalogs are seeded from the closed ground facts.
    // Today's closed targets accept their ID projection, not a composite probe.
    relation AllowedActionRole { id: u64 as ActionRoleCatalogId, kind: u64 as ActionKindId, role: u64 as RoleId }
    relation AllowedBlockRole { id: u64 as BlockRoleCatalogId, kind: u64 as ActionKindId, role: u64 as RoleId }
    relation Move { game: u64 as GameId, id: u64 as MoveId, turn: u64, actor: u64 as SeatNoId, kind: u64 as ActionKindId }
    relation Target { game: u64 as GameId, action: u64 as MoveId, seat: u64 as SeatNoId }
    relation Claim { game: u64 as GameId, id: u64 as ClaimId, speaker: u64 as SeatNoId, role: u64 as RoleId, origin: u64 as ClaimOriginId }
    relation ActionClaim { game: u64 as GameId, claim: u64 as ClaimId, action: u64 as MoveId, actor: u64 as SeatNoId, kind: u64 as ActionKindId, role: u64 as RoleId }
    relation BlockClaim { game: u64 as GameId, claim: u64 as ClaimId, action: u64 as MoveId, speaker: u64 as SeatNoId, kind: u64 as ActionKindId, role: u64 as RoleId }
    relation Challenge { game: u64 as GameId, id: u64 as ChallengeId, claim: u64 as ClaimId, challenger: u64 as SeatNoId }
    relation Proof { game: u64 as GameId, id: u64 as ProofId, claim: u64 as ClaimId, player: u64 as SeatNoId, role: u64 as RoleId, before_revision: u64, after_revision: u64 }
    relation Loss { game: u64 as GameId, id: u64 as LossId, player: u64 as SeatNoId, slot: u64 as SlotId, role: u64 as RoleId, revision: u64 }

    // A recorded observation is bound to its event in this perspective.
    // Evidence conjunction and the bridge to the actual log are maintained
    // derivations, not facts automatically supplied by these foreign keys.
    relation Observation {
        id: u64 as ObservationId, perspective: u64 as PerspectiveId,
        position: u64 as PositionId, description: str, when: event,
    }

    // Each decision has one joint action partition, potentially conditional
    // on several actor information cases. Raw Choice responses live upstream
    // of event construction; no probability is attached independently to a row.
    relation Decision { id: u64 as DecisionId, position: u64 as PositionId, actor: u64 as SeatNoId, occurs: event }
    relation MoveOption { decision: u64 as DecisionId, id: u64 as OptionId, kind: u64 as ActionKindId }
    relation OptionTarget { decision: u64 as DecisionId, option: u64 as OptionId, position: u64 as PositionId, seat: u64 as SeatNoId }
    relation Available { decision: u64 as DecisionId, option: u64 as OptionId, when: event }
    relation NextMove { decision: u64 as DecisionId, option: u64 as OptionId, when: event }
    relation PolicyCase { decision: u64 as DecisionId, id: u64 as PolicyCaseId, input_digest: bytes<32>, when: event }
    relation Forecast {
        decision: u64 as DecisionId, case: u64 as PolicyCaseId,
        request_digest: bytes<32>, question_version: u64, resolved_model: str,
        response: str,
    }

    Game(id) -> Game;
    Game(id, size) -> Game;
    Game(size) <= GameSize(id);
    Seat(game, seat) -> Seat;
    Seat(game) <= Game(id);
    Seat(seat) <= SeatNo(id);
    Game(id | size == Two) <={2} Seat(game);
    Game(id | size == Three) <={3} Seat(game);
    Game(id | size == Four) <={4} Seat(game);
    Game(id | size == Five) <={5} Seat(game);
    Game(id | size == Six) <={6} Seat(game);
    Card(role) <= Role(id);

    Perspective(id) -> Perspective;
    Perspective(id, game) -> Perspective;
    Perspective(game) <= Game(id);
    Perspective(kind) <= ViewKind(id);
    PlayerView(perspective) -> PlayerView;
    RefereeView(perspective) -> RefereeView;
    Perspective(id | kind == Player) == PlayerView(perspective);
    Perspective(id | kind == Referee) == RefereeView(perspective);
    PlayerView(perspective, game) <= Perspective(id, game);
    RefereeView(perspective, game) <= Perspective(id, game);
    PlayerView(game, observer) <= Seat(game, seat);
    Position(id) -> Position;
    Position(id, game) -> Position;
    Position(id, perspective) -> Position;
    Position(perspective, step) -> Position;
    Position(perspective, game) <= Perspective(id, game);
    Position(game, size) <= Game(id, size);
    PositionSeat(position, seat) -> PositionSeat;
    PositionSeat(position, game) <= Position(id, game);
    PositionSeat(game, seat) <= Seat(game, seat);
    Position(id | size == Two) <={2} PositionSeat(position);
    Position(id | size == Three) <={3} PositionSeat(position);
    Position(id | size == Four) <={4} PositionSeat(position);
    Position(id | size == Five) <={5} PositionSeat(position);
    Position(id | size == Six) <={6} PositionSeat(position);

    // Ordinary finite roster counts: cards/slots/seats, never probability mass.
    PositionCard(position, card) -> PositionCard;
    PositionCard(position, card, true) -> PositionCard;
    PositionCard(position) <= Position(id);
    PositionCard(card) <= Card(id);
    Position(id) <={15} PositionCard(position);
    HandSlot(position, seat, slot) -> HandSlot;
    HandSlot(position, seat, slot, true) -> HandSlot;
    HandSlot(position, seat) <= PositionSeat(position, seat);
    HandSlot(slot) <= Slot(id);
    PositionSeat(position, seat) <={2} HandSlot(position, seat);

    At(position, card) <= PositionCard(position, card);
    At(place) <= Place(id);
    At(position, card, when) -> At;
    PositionCard(position, card, true) == At(position, card, when);

    CourtCard(position, card, when) -> CourtCard;
    Influence(position, card, when) -> Influence;
    FirstDraw(position, card, when) -> FirstDraw;
    SecondDraw(position, card, when) -> SecondDraw;
    At(position, card, when | place == Court) == CourtCard(position, card, when);
    At(position, card, when | place == Hand) == Influence(position, card, when);
    At(position, card, when | place == DrawFirst) == FirstDraw(position, card, when);
    At(position, card, when | place == DrawSecond) == SecondDraw(position, card, when);

    Influence(position, seat, slot) <= HandSlot(position, seat, slot);
    Influence(position, seat, slot, when) -> Influence;
    HandSlot(position, seat, slot, true) == Influence(position, seat, slot, when);
    // A revealed card still fills its influence slot and consumes its identity.

    PendingExchange(position, seat) <= PositionSeat(position, seat);
    PendingExchange(position, when) -> PendingExchange;
    PendingExchange(position, seat, when) -> PendingExchange;
    FirstDraw(position, seat, when) -> FirstDraw;
    SecondDraw(position, seat, when) -> SecondDraw;
    PendingExchange(position, seat, when) == FirstDraw(position, seat, when);
    PendingExchange(position, seat, when) == SecondDraw(position, seat, when);

    PositionSeat(position, seat, true) -> PositionSeat;
    Balance(position, seat, when) -> Balance;
    PositionSeat(position, seat, true) == Balance(position, seat, when);

    ActionRoleCatalog(kind) <= ActionKind(id);
    ActionRoleCatalog(role) <= Role(id);
    BlockRoleCatalog(kind) <= ActionKind(id);
    BlockRoleCatalog(role) <= Role(id);
    AllowedActionRole(id) -> AllowedActionRole;
    AllowedActionRole(kind, role) -> AllowedActionRole;
    AllowedActionRole(id, kind, role) -> AllowedActionRole;
    AllowedActionRole(id) <= ActionRoleCatalog(id);
    ActionRoleCatalog(id, kind, role) <= AllowedActionRole(id, kind, role);
    AllowedBlockRole(id) -> AllowedBlockRole;
    AllowedBlockRole(kind, role) -> AllowedBlockRole;
    AllowedBlockRole(id, kind, role) -> AllowedBlockRole;
    AllowedBlockRole(id) <= BlockRoleCatalog(id);
    BlockRoleCatalog(id, kind, role) <= AllowedBlockRole(id, kind, role);

    Move(game, id) -> Move;
    Move(game, turn) -> Move;
    Move(game, id, kind) -> Move;
    Move(game, id, actor, kind) -> Move;
    Move(game, actor) <= Seat(game, seat);
    Move(kind) <= ActionKind(id);
    Target(game, action) -> Target;
    Target(game, action, seat) -> Target;
    Target(game, seat) <= Seat(game, seat);
    Move(game, id | kind == {Steal, Assassinate, CoupAction}) == Target(game, action);
    Claim(game, id) -> Claim;
    Claim(game, id, speaker, role) -> Claim;
    Claim(game, speaker) <= Seat(game, seat);
    Claim(role) <= Role(id);
    Claim(origin) <= ClaimOrigin(id);
    ActionClaim(game, claim) -> ActionClaim;
    BlockClaim(game, claim) -> BlockClaim;
    Claim(game, id | origin == Action) == ActionClaim(game, claim);
    Claim(game, id | origin == Block) == BlockClaim(game, claim);
    ActionClaim(game, claim, actor, role) <= Claim(game, id, speaker, role);
    BlockClaim(game, claim, speaker, role) <= Claim(game, id, speaker, role);
    ActionClaim(game, action) -> ActionClaim;
    ActionClaim(game, action, actor, kind) -> ActionClaim;
    Move(game, id, actor, kind | kind == {Tax, Assassinate, Steal, Exchange}) == ActionClaim(game, action, actor, kind);
    ActionClaim(kind, role) <= AllowedActionRole(kind, role);
    BlockClaim(game, action, kind) <= Move(game, id, kind);
    BlockClaim(kind, role) <= AllowedBlockRole(kind, role);
    BlockClaim(game, action, speaker | kind == {Steal, Assassinate}) <= Target(game, action, seat);
    Challenge(game, id) -> Challenge;
    Challenge(game, claim) -> Challenge;
    Challenge(game, claim) <= Claim(game, id);
    Challenge(game, challenger) <= Seat(game, seat);
    Proof(game, id) -> Proof;
    Proof(game, claim) <= Challenge(game, claim);
    Proof(game, claim, player, role) <= Claim(game, id, speaker, role);
    Loss(game, id) -> Loss;
    Loss(game, player) <= Seat(game, seat);
    Loss(slot) <= Slot(id);
    Loss(role) <= Role(id);

    Observation(id) -> Observation;
    Observation(perspective) <= Perspective(id);
    Observation(position, perspective) <= Position(id, perspective);

    Decision(id) -> Decision;
    Decision(id, position) -> Decision;
    Decision(id, occurs) -> Decision;
    Decision(position, actor) <= PositionSeat(position, seat);
    MoveOption(decision, id) -> MoveOption;
    MoveOption(decision) <= Decision(id);
    MoveOption(kind) <= ActionKind(id);
    OptionTarget(decision, option) -> OptionTarget;
    OptionTarget(decision, position) <= Decision(id, position);
    OptionTarget(position, seat) <= PositionSeat(position, seat);
    MoveOption(decision, id | kind == {Steal, Assassinate, CoupAction}) == OptionTarget(decision, option);
    NextMove(decision, option) <= MoveOption(decision, id);
    Available(decision, option) <= MoveOption(decision, id);
    Available(decision, option, when) -> Available;
    NextMove(decision, option, when) <= Available(decision, option, when);
    NextMove(decision, when) -> NextMove;
    Decision(id, occurs) == NextMove(decision, when);
    PolicyCase(decision, id) -> PolicyCase;
    PolicyCase(decision) <= Decision(id);
    PolicyCase(decision, when) -> PolicyCase;
    Decision(id, occurs) == PolicyCase(decision, when);
    Forecast(decision, case) -> Forecast;
    Forecast(decision, case) == PolicyCase(decision, id);
}
