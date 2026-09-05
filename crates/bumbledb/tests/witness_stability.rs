//! probe-order-invariant by construction, and the Lean side compares
//! verdicts by that list (`lean/Main.lean:: RVerdict`, list `BEq`).
//! today, the whole rejection value — is invariant under everything a
//! semantics before any probe runs.
use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::{Db, Direction, Theory, Violation, Violations};

fn world_schema() -> bumbledb::Schema {
    WitnessWorld
        .descriptor()
        .validate()
        .expect("the test schema is valid")
}

mod common;

bumbledb::schema! {
    pub WitnessWorld;

    relation WParent {
        id: u64 as WParentId,
        kind: u64,
    }
    relation WChild {
        id: u64 as WChildId,
        parent: u64 as WParentId,
        flag: u64,
    }

    WChild(parent) <= WParent(id);
    WParent(id) <={0..2} WChild(parent);
    // Containment/capacity targets must be declared keys (chapter 10 checked
    // schema premise); declared last so the containment stays StatementId(0)
    // and the capacity StatementId(1).
    WParent(id) -> WParent;
}

/// Expected citation bytes come from the one production encoding — the
/// canonical row codec — never a hand-rolled second writer of the format.
fn canonical_fact(relation: bumbledb::schema::RelationId, values: &[bumbledb::Value]) -> Vec<u8> {
    let schema = world_schema();
    let work = bumbledb::ExecutionPolicy {
        input_bytes: 1 << 20,
        working_bytes: 1 << 20,
        scratch_bytes: 1 << 20,
        result_bytes: 1 << 20,
        rows: 1 << 10,
        work_units: 1 << 20,
        timeout: std::time::Duration::from_secs(60),
    }
    .start()
    .expect("fixture work budget");
    bumbledb::canonical::CanonicalRow::encode(schema.relation(relation).fields(), values, &work)
        .expect("the fixture row encodes")
        .as_bytes()
        .to_vec()
}

fn child_bytes(id: u64, parent: u64, flag: u64) -> Vec<u8> {
    use bumbledb::{Fact as _, Value};
    canonical_fact(
        WChild::RELATION,
        &[Value::U64(id), Value::U64(parent), Value::U64(flag)],
    )
}

fn parent_bytes(id: u64, kind: u64) -> Vec<u8> {
    use bumbledb::{Fact as _, Value};
    canonical_fact(WParent::RELATION, &[Value::U64(id), Value::U64(kind)])
}

fn insert_parent(db: &Db<WitnessWorld>, id: u64) {
    db.write(common::work(), |tx| {
        tx.insert([&WParent {
            id: WParentId(id),
            kind: 0,
        }])
    })
    .expect("seed parent")
    .unwrap();
}

fn insert_child(db: &Db<WitnessWorld>, id: u64, parent: u64) {
    db.write(common::work(), |tx| {
        tx.insert([&WChild {
            id: WChildId(id),
            parent: WParentId(parent),
            flag: 0,
        }])
    })
    .expect("seed child")
    .unwrap();
}

fn rejection<T: std::fmt::Debug>(outcome: bumbledb::Result<bumbledb::Admission<T>>) -> Violations {
    common::expect_rejected(outcome)
}

fn seeded_world(tag: &str) -> (common::TempDir, Db<WitnessWorld>) {
    let dir = common::TempDir::new(tag);
    let db = Db::create(dir.path(), WitnessWorld, common::work())
        .expect("create")
        .expect("accepted");
    for id in [1, 2, 3] {
        insert_parent(&db, id);
    }
    insert_child(&db, 100, 3);
    insert_child(&db, 101, 3);
    (dir, db)
}

fn multi_violation_commit(db: &Db<WitnessWorld>, order: &[usize]) -> Violations {
    let calls: [(u64, u64); 7] = [
        (200, 900),
        (201, 901),
        (202, 902),
        (203, 903),
        (300, 1),
        (301, 1),
        (302, 1),
    ];
    rejection(db.write(common::work(), |tx| {
        if order[0] != 0 {
            tx.delete([&WParent {
                id: WParentId(3),
                kind: 0,
            }])?;
        }
        for &slot in order {
            let (id, parent) = calls[slot];
            tx.insert([&WChild {
                id: WChildId(id),
                parent: WParentId(parent),
                flag: 0,
            }])?;
        }
        if order[0] == 0 {
            tx.delete([&WParent {
                id: WParentId(3),
                kind: 0,
            }])?;
        }
        Ok(())
    }))
}

/// NORMATIVE: the sealed citation list — one citation per violated `(statement,
/// direction)`, sorted — is invariant under the transaction's call order, and
/// so (today) is the entire rejection value, witnesses included: the delta
/// erases call order before any probe runs.
#[test]
fn the_sealed_citation_list_is_call_order_invariant() {
    let (_keep_a, db_a) = seeded_world("witness-order-a");
    let (_keep_b, db_b) = seeded_world("witness-order-b");
    let (_keep_c, db_c) = seeded_world("witness-order-c");
    let forward = multi_violation_commit(&db_a, &[0, 1, 2, 3, 4, 5, 6]);
    let reversed = multi_violation_commit(&db_b, &[6, 5, 4, 3, 2, 1, 0]);
    let shuffled = multi_violation_commit(&db_c, &[3, 6, 0, 4, 1, 5, 2]);
    assert_eq!(forward, reversed, "call order never reaches the verdict");
    assert_eq!(forward, shuffled, "call order never reaches the verdict");

    // The final-state judge evaluates each one-way statement once: a
    // deleted target and an inserted orphan source are the SAME violated
    // condition (a source without its target), so the containment seals as
    // ONE SourceUnsatisfied citation — never a who-moved pair.
    let [
        (
            Violation::Containment {
                statement: src_stmt,
                direction: Direction::SourceUnsatisfied,
                ..
            },
            _,
        ),
        (
            Violation::Capacity {
                statement: cap_stmt,
                measure: 3,
                ..
            },
            _,
        ),
    ] = forward.as_slice()
    else {
        panic!("expected the two-citation seal, got {forward:?}");
    };
    let schema = world_schema();
    assert!(
        schema.id_of(*cap_stmt).0 > schema.id_of(*src_stmt).0,
        "citation order is materialized statement order"
    );
}

#[test]
fn the_rejection_is_reproducible_across_stores() {
    let (_keep_a, db_a) = seeded_world("witness-repro-a");
    let (_keep_b, db_b) = seeded_world("witness-repro-b");
    let order = [2, 0, 5, 1, 6, 4, 3];
    assert_eq!(
        multi_violation_commit(&db_a, &order),
        multi_violation_commit(&db_b, &order),
    );
}

#[test]
fn the_source_witness_is_the_canonical_least_violator() {
    let kids: [(u64, u64); 6] = [
        (9001, 700),
        (9002, 650),
        (9003, 600),
        (9004, 550),
        (9005, 500),
        (9006, 450),
    ];
    // The successor judge cites in the state's deterministic iteration
    // order: canonical row bytes, i.e. the least child id — never a hash
    // order and (proved by the reversed run below) never call order.
    let expected = kids[0];
    let hash_least = kids
        .iter()
        .copied()
        .min_by_key(|&(id, parent)| *blake3::hash(&child_bytes(id, parent, 0)).as_bytes())
        .expect("nonempty");

    assert_ne!(
        hash_least, expected,
        "re-pick fixture ids: the hash-least violator must differ from the canonical-least one"
    );

    let run = |tag: &str, reverse: bool| -> Violations {
        let dir = common::TempDir::new(tag);
        let db = Db::create(dir.path(), WitnessWorld, common::work())
            .expect("create")
            .expect("accepted");
        rejection(db.write(common::work(), |tx| {
            let mut order: Vec<(u64, u64)> = kids.to_vec();
            if reverse {
                order.reverse();
            }
            for (id, parent) in order {
                tx.insert([&WChild {
                    id: WChildId(id),
                    parent: WParentId(parent),
                    flag: 0,
                }])?;
            }
            Ok(())
        }))
    };
    let violations = run("witness-key-least-fwd", false);
    assert_eq!(violations, run("witness-key-least-rev", true));
    let [
        (
            Violation::Containment {
                direction: Direction::SourceUnsatisfied,
                fact,
                ..
            },
            _,
        ),
    ] = violations.as_slice()
    else {
        panic!("expected one source citation, got {violations:?}");
    };
    let (id, parent) = expected;
    assert_eq!(
        fact.as_ref(),
        child_bytes(id, parent, 0).as_slice(),
        "the surviving source witness is the canonical-least violator (non-normative pin)"
    );
}

/// NON-NORMATIVE PIN, capacity side: the capacity check list is a B-tree of
/// touched parents, so a multi-parent capacity rejection's witness is the
/// KEY-LEAST violating parent — already what a sorted source side would
/// produce; the W8 sort must not change this one.
#[test]
fn citation_examples_are_canonical_least_not_insertion_order() {
    let run = |tag: &str, reverse: bool| -> Violations {
        let dir = common::TempDir::new(tag);
        let db = Db::create(dir.path(), WitnessWorld, common::work())
            .expect("create")
            .expect("accepted");
        let kids: [(u64, u64); 4] = [(9100, 800), (9101, 800), (9102, 800), (9103, 800)];
        rejection(db.write(common::work(), |tx| {
            let mut order: Vec<(u64, u64)> = kids.to_vec();
            if reverse {
                order.reverse();
            }
            for (id, parent) in order {
                tx.insert([&WChild {
                    id: WChildId(id),
                    parent: WParentId(parent),
                    flag: 0,
                }])?;
            }
            Ok(())
        }))
    };
    let forward = run("witness-canonical-order-fwd", false);
    let reverse = run("witness-canonical-order-rev", true);
    assert_eq!(forward, reverse, "insertion order must not reach citations");
    let [
        (
            Violation::Containment {
                direction: Direction::SourceUnsatisfied,
                fact,
                ..
            },
            _,
        ),
    ] = forward.as_slice()
    else {
        panic!("expected one containment citation, got {forward:?}");
    };
    let (least_id, least_parent) = (9100, 800);
    assert_eq!(
        fact.as_ref(),
        child_bytes(least_id, least_parent, 0).as_slice(),
        "canonical-least violator, not insertion order"
    );
}

#[test]
fn the_capacity_witness_is_the_key_least_violating_parent() {
    let dir = common::TempDir::new("witness-capacity");
    let db = Db::create(dir.path(), WitnessWorld, common::work())
        .expect("create")
        .expect("accepted");
    insert_parent(&db, 10);
    insert_parent(&db, 20);
    let violations = rejection(db.write(common::work(), |tx| {
        for (id, parent) in [
            (400, 20),
            (401, 20),
            (402, 20),
            (500, 10),
            (501, 10),
            (502, 10),
        ] {
            tx.insert([&WChild {
                id: WChildId(id),
                parent: WParentId(parent),
                flag: 0,
            }])?;
        }
        Ok(())
    }));
    let [
        (
            Violation::Capacity {
                fact, measure: 3, ..
            },
            _,
        ),
    ] = violations.as_slice()
    else {
        panic!("expected one capacity citation, got {violations:?}");
    };
    assert_eq!(
        fact.as_ref(),
        parent_bytes(10, 0).as_slice(),
        "both parents violate; the sorted check list surfaces the key-least one"
    );
}

#[test]
fn the_target_witness_is_the_first_committed_survivor() {
    let dir = common::TempDir::new("witness-target");
    let db = Db::create(dir.path(), WitnessWorld, common::work())
        .expect("create")
        .expect("accepted");
    insert_parent(&db, 30);
    insert_child(&db, 600, 30);
    insert_child(&db, 601, 30);
    let violations = rejection(db.write(common::work(), |tx| {
        tx.delete([&WParent {
            id: WParentId(30),
            kind: 0,
        }])
    }));
    // Final-state semantics: deleting the target leaves its sources
    // unsatisfied — the one direction the judge speaks. The witness is the
    // canonical-least surviving source (child 600).
    let [
        (
            Violation::Containment {
                direction: Direction::SourceUnsatisfied,
                fact,
                ..
            },
            _,
        ),
    ] = violations.as_slice()
    else {
        panic!("expected one containment citation, got {violations:?}");
    };
    assert_eq!(
        fact.as_ref(),
        child_bytes(600, 30, 0).as_slice(),
        "the canonical-least surviving source is the witness"
    );
}
