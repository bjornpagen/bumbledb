// Revision 0.9 worked templates; complete Coup integration remains pending.
// Event/Test/Pack and face-aware relation heads now parse on the implementation
// branch. Expectation remains proposed. Nothing here calls TypeSafe.

fn existing_coup_schema_examples() {
    // Keep an Event for impossible holdings, so complement remains composable.
    let held_roles = bumbledb_query::query!(Coup {
        interior fragments(position, seat, role, region: Event(e)) |
            Influence(position, card, seat, revealed == false, when: e),
            Card(id: card, role);
        interior fragments(position, seat, role, region: Event(Empty(given))) |
            PositionSeat(position, seat), Position(id: position, perspective),
            Perspective(id: perspective, given), Role(id: role);
        (position, seat, role, holds: Pack(region)) |
            fragments(position, seat, role, region);
    });

    let threats = bumbledb_query::query!(Coup {
        use held = &held_roles;
        (position, one: Event(Exactly(1, b, c, d)),
                   multiple: Event(AtLeast(2, b, c, d))) |
            held(position, bob, role, b), held(position, cleo, role, c),
            held(position, dana, role, d),
            position == ?position_id, role == ?role_id,
            bob == ?bob_seat, cleo == ?cleo_seat, dana == ?dana_seat,
            bob != cleo, bob != dana, cleo != dana;
    });

    // For a selected decision, subject, and role, classify each PolicyCase.
    // All helpers used by query imports are unparameterized and nonrecursive.
    let classified_cases = bumbledb_query::query!(Coup {
        use held = &held_roles;
        (decision, subject, role, case, cell,
         unreachable: Test(IsEmpty(cell & given)),
         no_yes: Test(IsEmpty(cell & given & holds)),
         no_no: Test(IsEmpty(cell & given & !holds))) |
            Decision(id: decision, position),
            PolicyCase(decision, id: case, when: cell),
            Position(id: position, perspective),
            Perspective(id: perspective, given),
            held(position, subject, role, holds);
    });

    let information_may = bumbledb_query::query!(Coup {
        use cases = &classified_cases;
        interior fragments(decision, subject, role, region: Event(cell)) |
            cases(decision, subject, role, case, cell,
                  unreachable == false, no_yes == false);
        interior fragments(decision, subject, role, region: Event(Empty(cell))) |
            cases(decision, subject, role, case, cell);
        (decision, subject, role, possible: Pack(region)) |
            fragments(decision, subject, role, region);
    });

    let information_must = bumbledb_query::query!(Coup {
        use cases = &classified_cases;
        interior fragments(decision, subject, role, region: Event(cell)) |
            cases(decision, subject, role, case, cell,
                  unreachable == false, no_no == true);
        interior fragments(decision, subject, role, region: Event(Empty(cell))) |
            cases(decision, subject, role, case, cell);
        (decision, subject, role, guaranteed: Pack(region)) |
            fragments(decision, subject, role, region);
    });

    let unresolved = bumbledb_query::query!(Coup {
        use may_cases = &information_may;
        use must_cases = &information_must;
        (decision, subject, role, when: Event(possible & !guaranteed)) |
            may_cases(decision, subject, role, possible),
            must_cases(decision, subject, role, guaranteed),
            decision == ?decision_id, subject == ?subject_seat, role == ?role_id;
    });

    // This reads the existing balance observable. Distinct coin values must
    // partition given; duplicate paths to one value are unioned, never summed.
    let expected_balance = bumbledb_query::query!(Coup {
        (position, seat, expected: Expectation(coins, when, given)) |
            Balance(position, seat, coins, when),
            Position(id: position, perspective),
            Perspective(id: perspective, given),
            position == ?position_id, seat == ?seat_id;
    });

    let _ = (threats, unresolved, expected_balance);
}

// Companion RuleView schema and its constructor obligations are in
// algebra-applications.md §8. Faces are owned descriptors, not scalar fields
// or Model<T> parameters. `step_faces` captures a checked Fibre descriptor.
fn rule_view_examples(step_faces: &bumbledb::EventImport) {
    let action_preconditions = bumbledb_query::query!(RuleView {
        use faces step = step_faces;
        (scenario, option, starts,
         can: Event(May(Relation(r, step), goal)),
         safe: Event(Must(Relation(r, step), goal))) |
            Scenario(id: scenario, starts, goal),
            RuleOutcome(scenario, option, when: r);
    });

    let classified_actions = bumbledb_query::query!(RuleView {
        use pre = &action_preconditions;
        (scenario, option, safe,
         no_start: Test(IsEmpty(starts)),
         guaranteed: Test(Subset(starts, safe))) |
            pre(scenario, option, starts, can, safe);
    });

    // Return all options guaranteeing the goal over the supplied start case.
    // A later model can rank these options. No probability threshold is used.
    let permitted = bumbledb_query::query!(RuleView {
        use checked = &classified_actions;
        (scenario, option, safe) |
            checked(scenario, option, safe, no_start == false, guaranteed == true),
            scenario == ?scenario_id;
    });
    let _ = permitted;
}
