// Proposal only: today's query! does not parse Event(...) or Probability(...).
// New: total Boolean event expressions, query values including scoped Empty,
// event-valued Pack (union), and exact conditional probability observations.
// Existing: ordinary atoms, named parameters, multi-rule interiors, imports.
// See ../query-algebra.md and query-walkthrough.md. No engine changes implied.

fn examples() {
    // Unparameterized helper. Seed every admitted position/seat/role group
    // with empty, so an impossible role has an event value to complement.
    let held_roles = bumbledb_query::query!(Coup {
        interior fragments(position, seat, role, region: Event(e)) |
            Influence(position, card, seat, revealed == false, when: e),
            Card(id: card, role);
        interior fragments(position, seat, role, region: Event(Empty(given))) |
            PositionSeat(position, seat),
            Position(id: position, perspective),
            Perspective(id: perspective, given),
            Role(id: role);
        (position, seat, role, holds: Pack(region)) |
            fragments(position, seat, role, region);
    });

    // Select the claim's matching pre-resolution position. For posterior odds,
    // `given` must contain its observation: joining Claim is not evidence.
    let inspect_claim = bumbledb_query::query!(Coup {
        use held = &held_roles;
        interior unsupported(claim, when: Event(!can_prove), given) |
            Claim(game, id: claim, speaker, role),
            Position(id: position, game, perspective),
            Perspective(id: perspective, given),
            held(position, speaker, role, can_prove),
            claim == ?claim_id, position == ?position_id;
        (claim, when, chance: Probability(when, given)) |
            unsupported(claim, when, given);
    });

    // Card support, not guaranteed success of a steal: Cleo can still bluff.
    let steal_window = bumbledb_query::query!(Coup {
        use held = &held_roles;
        interior window(position, when: Event(b & !(c | a)), given) |
            held(position, bob, captain, b),
            held(position, cleo, captain, c),
            held(position, cleo, ambassador, a),
            bob == ?bob_seat, cleo == ?cleo_seat,
            captain == ?captain_role, ambassador == ?ambassador_role,
            Position(id: position, perspective),
            Perspective(id: perspective, given),
            position == ?position_id;
        (position, when, chance: Probability(when, given)) |
            window(position, when, given);
    });

    // Bind the Captain roster ID. The outputs remain composable events.
    let captain_claims = bumbledb_query::query!(Coup {
        use held = &held_roles;
        (position, exactly_one: Event(b ^ c),
                   both: Event(b & c), neither: Event(!(b | c))) |
            held(position, bob, captain, b),
            held(position, cleo, captain, c),
            bob == ?bob_seat, cleo == ?cleo_seat,
            captain == ?captain_role, position == ?position_id;
    });

    // Same perspective and joint trajectories. Bind the Duke roster ID and
    // the actual transition endpoints; the query does not invent a transition.
    let lost_the_proved_duke = bumbledb_query::query!(Coup {
        use held = &held_roles;
        interior changed(perspective, when: Event(old & !new), given) |
            Position(id: before, perspective),
            Position(id: after, perspective),
            held(before, player, duke, old),
            held(after, player, duke, new),
            Perspective(id: perspective, given),
            before == ?before_position, after == ?after_position,
            player == ?player_seat, duke == ?duke_role;
        (perspective, when, chance: Probability(when, given)) |
            changed(perspective, when, given);
    });

    // Complete action-kind groups too; union target-specific options by kind.
    let move_kinds = bumbledb_query::query!(Coup {
        interior fragments(decision, kind, region: Event(e)) |
            NextMove(decision, option, when: e),
            MoveOption(decision, id: option, kind);
        interior fragments(decision, kind, region: Event(Empty(occurs))) |
            Decision(id: decision, occurs), ActionKind(id: kind);
        (decision, kind, happens: Pack(region)) |
            fragments(decision, kind, region);
    });

    // Hypothetically observe this named decision's Tax outcome. Reusing tax
    // in evidence never allocates another decision. `given` is normally the
    // pre-action evidence here; including tax in it again is harmless.
    let after_hearing_tax = bumbledb_query::query!(Coup {
        use held = &held_roles;
        use moves = &move_kinds;
        (decision, bluff: Event(tax & !duke),
                   chance: Probability(!duke, given & tax)) |
            Decision(id: decision, position, actor),
            Position(id: position, perspective),
            Perspective(id: perspective, given),
            held(position, actor, role, duke), role == ?duke_role,
            moves(decision, kind, tax), kind == ?tax_kind,
            decision == ?decision_id;
    });

    // Bob speaks; Cleo's holding probability changes through the shared deck.
    // No explicit Bob-holding join is needed: tax already retains its joint
    // dependence on the hand and this named decision's policy outcome.
    let another_players_hand_after_tax = bumbledb_query::query!(Coup {
        use held = &held_roles;
        use moves = &move_kinds;
        (decision, other, chance: Probability(duke, given & tax)) |
            Decision(id: decision, position, actor),
            Position(id: position, perspective),
            Perspective(id: perspective, given),
            held(position, other, role, duke), role == ?duke_role,
            other == ?other_seat, other != actor,
            moves(decision, kind, tax), kind == ?tax_kind,
            decision == ?decision_id;
    });

    // NOT Tax includes worlds where this future decision never occurs.
    let future_non_tax = bumbledb_query::query!(Coup {
        use moves = &move_kinds;
        (decision, another_move: Event(occurs & !tax),
                   absent_or_other: Event(!tax),
                   chance_if_reached: Probability(!tax, given & occurs)) |
            Decision(id: decision, position, occurs),
            Position(id: position, perspective),
            Perspective(id: perspective, given),
            moves(decision, kind, tax), kind == ?tax_kind,
            decision == ?decision_id;
    });

    // Illustrative templates; this file does not execute queries.
    let _ = (inspect_claim, steal_window, captain_claims,
             lost_the_proved_duke, after_hearing_tax,
             another_players_hand_after_tax, future_non_tax);
}
