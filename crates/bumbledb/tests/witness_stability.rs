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
        id: u64 as WParentId, fresh,
        kind: u64,
    }
    relation WChild {
        id: u64 as WChildId, fresh,
        parent: u64 as WParentId,
        flag: u64,
    }

    WChild(parent) <= WParent(id);
    WParent(id) <={0..2} WChild(parent);
}

fn child_bytes(id: u64, parent: u64, flag: u64) -> [u8; 24] {
    let mut out = [0u8; 24];
    out[..8].copy_from_slice(&id.to_be_bytes());
    out[8..16].copy_from_slice(&parent.to_be_bytes());
    out[16..].copy_from_slice(&flag.to_be_bytes());
    out
}

fn parent_bytes(id: u64, kind: u64) -> [u8; 16] {
    let mut out = [0u8; 16];
    out[..8].copy_from_slice(&id.to_be_bytes());
    out[8..].copy_from_slice(&kind.to_be_bytes());
    out
}

fn insert_parent(db: &Db<WitnessWorld>, id: u64) {
    db.write(|tx| {
        tx.insert([&WParent {
            id: WParentId(id),
            kind: 0,
        }])
    })
    .expect("seed parent")
    .unwrap();
}

fn insert_child(db: &Db<WitnessWorld>, id: u64, parent: u64) {
    db.write(|tx| {
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
    let db = Db::create(dir.path(), WitnessWorld)
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
    rejection(db.write(|tx| {
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
            Violation::Containment {
                statement: tgt_stmt,
                direction: Direction::TargetRequired,
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
        panic!("expected the three-citation seal, got {forward:?}");
    };
    assert_eq!(src_stmt, tgt_stmt, "one containment, both directions");
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
fn the_source_witness_is_the_key_least_violator() {
    let kids: [(u64, u64); 6] = [
        (9001, 700),
        (9002, 650),
        (9003, 600),
        (9004, 550),
        (9005, 500),
        (9006, 450),
    ];
    let expected = *kids.last().expect("nonempty");
    let hash_least = kids
        .iter()
        .copied()
        .min_by_key(|&(id, parent)| *blake3::hash(&child_bytes(id, parent, 0)).as_bytes())
        .expect("nonempty");

    assert_ne!(
        hash_least.1, 450,
        "re-pick fixture ids: the hash-least violator must differ from the key-least one"
    );
    assert_ne!(
        hash_least, kids[0],
        "re-pick fixture ids: the hash-least violator must differ from the first-called one"
    );

    let run = |tag: &str, reverse: bool| -> Violations {
        let dir = common::TempDir::new(tag);
        let db = Db::create(dir.path(), WitnessWorld)
            .expect("create")
            .expect("accepted");
        rejection(db.write(|tx| {
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
        "the surviving source witness is the key-least violator (non-normative; header)"
    );
}

/// NON-NORMATIVE PIN, capacity side: the capacity check list is a B-tree of
/// touched parents, so a multi-parent capacity rejection's witness is the
/// KEY-LEAST violating parent — already what a sorted source side would
/// produce; the W8 sort must not change this one.
#[test]
fn the_capacity_witness_is_the_key_least_violating_parent() {
    let dir = common::TempDir::new("witness-capacity");
    let db = Db::create(dir.path(), WitnessWorld)
        .expect("create")
        .expect("accepted");
    insert_parent(&db, 10);
    insert_parent(&db, 20);
    let violations = rejection(db.write(|tx| {
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
    let db = Db::create(dir.path(), WitnessWorld)
        .expect("create")
        .expect("accepted");
    insert_parent(&db, 30);
    insert_child(&db, 600, 30);
    insert_child(&db, 601, 30);
    let violations = rejection(db.write(|tx| {
        tx.delete([&WParent {
            id: WParentId(30),
            kind: 0,
        }])
    }));
    let [
        (
            Violation::Containment {
                direction: Direction::TargetRequired,
                fact,
                ..
            },
            _,
        ),
    ] = violations.as_slice()
    else {
        panic!("expected one target citation, got {violations:?}");
    };
    assert_eq!(
        fact.as_ref(),
        child_bytes(600, 30, 0).as_slice(),
        "the R-prefix walk surfaces the first-committed survivor"
    );
}
