//! Witness stability under probe re-ordering — the determinism
//! obligation the T8 commit-size sweep carries (the W8 lane in
//! `bumbledb-bench/src/sweep.rs` grades fact hashes to re-order the
//! source-side probes; these fixtures pin what re-ordering may and may
//! not change).
//!
//! The contract, stated exactly (`error.rs :: Violations::seal`): a
//! rejection is the COMPLETE citation set — stable-sorted and deduped
//! by `(statement, direction)` — so the citation LIST is
//! probe-order-invariant by construction, and the Lean side compares
//! verdicts by that list (`lean/Main.lean :: RVerdict`, list `BEq`).
//! What is NOT inside the citation identity is the surviving witness:
//! the stable sort keeps the FIRST-DISCOVERED `fact` per citation, so
//! re-ordering probes can change the cited fact bytes while the
//! citations stay identical.
//!
//! The normative boundary these fixtures draw:
//!
//! - NORMATIVE (the first two tests): the sealed citation list — and,
//!   today, the whole rejection value — is invariant under everything a
//!   host can vary: transaction call order is erased by the delta's set
//!   semantics before any probe runs.
//! - NON-NORMATIVE, pinned so a change is loud (the remaining tests):
//!   WHICH violating fact survives as the witness is an artifact of
//!   each check list's scan order — delta `(relation, fact_hash)` order
//!   on the source side, B-tree key order on the window and target
//!   sides. An engine-side probe re-order (the W8 source sort, if the
//!   sweep says it pays) is LICENSED to flip the source-side witness
//!   pin below — update that one assertion with the sort, citing this
//!   header — but must never touch the citation-list assertions, and a
//!   flip must arrive as a deliberate commit, never a surprise.

use bumbledb::{Db, Direction, Error, Violation, Violations};

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

/// Canonical fact bytes of a child `(id, parent, flag)` — three
/// big-endian words (`encoding/encode.rs :: encode_fact` over an
/// all-u64 relation). The violation payloads cite exactly these bytes.
fn child_bytes(id: u64, parent: u64, flag: u64) -> [u8; 24] {
    let mut out = [0u8; 24];
    out[..8].copy_from_slice(&id.to_be_bytes());
    out[8..16].copy_from_slice(&parent.to_be_bytes());
    out[16..].copy_from_slice(&flag.to_be_bytes());
    out
}

/// Canonical fact bytes of a parent `(id, kind)`.
fn parent_bytes(id: u64, kind: u64) -> [u8; 16] {
    let mut out = [0u8; 16];
    out[..8].copy_from_slice(&id.to_be_bytes());
    out[8..].copy_from_slice(&kind.to_be_bytes());
    out
}

fn insert_parent(db: &Db<WitnessWorld>, id: u64) {
    db.write(|tx| {
        tx.insert(&WParent {
            id: WParentId(id),
            kind: 0,
        })
    })
    .expect("seed parent");
}

fn insert_child(db: &Db<WitnessWorld>, id: u64, parent: u64) {
    db.write(|tx| {
        tx.insert(&WChild {
            id: WChildId(id),
            parent: WParentId(parent),
            flag: 0,
        })
    })
    .expect("seed child");
}

fn rejection<T: std::fmt::Debug>(outcome: Result<T, Error>) -> Violations {
    let Err(Error::CommitRejected { violations }) = outcome else {
        panic!("expected CommitRejected, got {outcome:?}");
    };
    violations
}

/// One identically-seeded world per call: parents 1, 2, 3; children
/// 100 and 101 under parent 3 (committed separately, so their storage
/// row order is commit order — deterministic whatever any within-commit
/// scan does).
fn seeded_world(tag: &str) -> (common::TempDir, Db<WitnessWorld>) {
    let dir = common::TempDir::new(tag);
    let db = Db::ephemeral(dir.path(), WitnessWorld).expect("create");
    for id in [1, 2, 3] {
        insert_parent(&db, id);
    }
    insert_child(&db, 100, 3);
    insert_child(&db, 101, 3);
    (dir, db)
}

/// The multi-citation rejected commit: four children under missing
/// parents (source side, four candidate witnesses), the delete of
/// parent 3 whose two children survive (target side, two candidates),
/// and three children under parent 1 bursting the `{0..2}` window.
/// `order` permutes the transaction's CALL order — the one order a host
/// controls.
fn multi_violation_commit(db: &Db<WitnessWorld>, order: &[usize]) -> Violations {
    // (child id, parent) — parents 900.. are missing; parent 1 exists.
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
        // The delete rides at a different position per order too: odd
        // permutations lead with it.
        if order[0] != 0 {
            tx.delete(&WParent {
                id: WParentId(3),
                kind: 0,
            })?;
        }
        for &slot in order {
            let (id, parent) = calls[slot];
            tx.insert(&WChild {
                id: WChildId(id),
                parent: WParentId(parent),
                flag: 0,
            })?;
        }
        if order[0] == 0 {
            tx.delete(&WParent {
                id: WParentId(3),
                kind: 0,
            })?;
        }
        Ok(())
    }))
}

/// NORMATIVE: the sealed citation list — one citation per violated
/// `(statement, direction)`, sorted — is invariant under the
/// transaction's call order, and so (today) is the entire rejection
/// value, witnesses included: the delta erases call order before any
/// probe runs.
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

    // The complete set: the containment cited once per direction, the
    // window cited once — whatever the count of convicting facts.
    let [
        Violation::Containment {
            statement: src_stmt,
            direction: Direction::SourceUnsatisfied,
            ..
        },
        Violation::Containment {
            statement: tgt_stmt,
            direction: Direction::TargetRequired,
            ..
        },
        Violation::Cardinality {
            statement: win_stmt,
            count: 3,
            ..
        },
    ] = forward.as_slice()
    else {
        panic!("expected the three-citation seal, got {forward:?}");
    };
    assert_eq!(src_stmt, tgt_stmt, "one containment, both directions");
    assert!(
        win_stmt.0 > src_stmt.0,
        "citation order is materialized statement order"
    );
}

/// NORMATIVE: the same fact set produces the same rejection on an
/// independently built store — the verdict is a function of
/// (state, delta), never of process history.
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

/// NON-NORMATIVE PIN, source side — the one assertion the W8 sort is
/// licensed to flip (see the header). Today `check_source` discovers in
/// the delta's `(relation, fact_hash)` order, so the surviving witness
/// of a multi-violation source citation is the HASH-LEAST violator —
/// computed here independently (blake3 over the canonical bytes, the
/// engine's `encoding/fact_hash.rs` identity). A key-sorted probe order
/// would instead surface the parent-key-least violator, which this
/// fixture deliberately makes a DIFFERENT fact: silence is impossible.
#[test]
fn the_source_witness_is_the_delta_hash_least_violator() {
    // Parents descend with call order, so first-called, hash-least, and
    // key-least are all distinct candidates unless the pin says so.
    let kids: [(u64, u64); 6] = [
        (9001, 700),
        (9002, 650),
        (9003, 600),
        (9004, 550),
        (9005, 500),
        (9006, 450),
    ];
    let expected = kids
        .iter()
        .copied()
        .min_by_key(|&(id, parent)| *blake3::hash(&child_bytes(id, parent, 0)).as_bytes())
        .expect("nonempty");
    // The fixture's discrimination preconditions: if an encoding change
    // re-rolls the hashes into a coincidence, re-pick the ids above.
    assert_ne!(
        expected.1, 450,
        "re-pick fixture ids: the hash-least violator must differ from the key-least one"
    );
    assert_ne!(
        expected, kids[0],
        "re-pick fixture ids: the hash-least violator must differ from the first-called one"
    );

    let run = |tag: &str, reverse: bool| -> Violations {
        let dir = common::TempDir::new(tag);
        let db = Db::ephemeral(dir.path(), WitnessWorld).expect("create");
        rejection(db.write(|tx| {
            let mut order: Vec<(u64, u64)> = kids.to_vec();
            if reverse {
                order.reverse();
            }
            for (id, parent) in order {
                tx.insert(&WChild {
                    id: WChildId(id),
                    parent: WParentId(parent),
                    flag: 0,
                })?;
            }
            Ok(())
        }))
    };
    let violations = run("witness-hash-least-fwd", false);
    assert_eq!(violations, run("witness-hash-least-rev", true));
    let [
        Violation::Containment {
            direction: Direction::SourceUnsatisfied,
            fact,
            ..
        },
    ] = violations.as_slice()
    else {
        panic!("expected one source citation, got {violations:?}");
    };
    let (id, parent) = expected;
    assert_eq!(
        fact.as_ref(),
        child_bytes(id, parent, 0).as_slice(),
        "the surviving source witness is the hash-least violator (non-normative; header)"
    );
}

/// NON-NORMATIVE PIN, window side: the window check list is a B-tree of
/// touched parents, so a multi-parent window rejection's witness is the
/// KEY-LEAST violating parent — already what a sorted source side would
/// produce; the W8 sort must not change this one.
#[test]
fn the_window_witness_is_the_key_least_violating_parent() {
    let dir = common::TempDir::new("witness-window");
    let db = Db::ephemeral(dir.path(), WitnessWorld).expect("create");
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
            tx.insert(&WChild {
                id: WChildId(id),
                parent: WParentId(parent),
                flag: 0,
            })?;
        }
        Ok(())
    }));
    let [Violation::Cardinality { fact, count: 3, .. }] = violations.as_slice() else {
        panic!("expected one window citation, got {violations:?}");
    };
    assert_eq!(
        fact.as_ref(),
        parent_bytes(10, 0).as_slice(),
        "both parents violate; the sorted check list surfaces the key-least one"
    );
}

/// NON-NORMATIVE PIN, target side: the surviving-source scan walks the
/// statement's `R` prefix in key order — source row ids ascend within
/// one determinant — so the witness of a delete rejection is the
/// FIRST-COMMITTED surviving requirer. Also untouched by the W8 sort
/// (the target check list is already sorted).
#[test]
fn the_target_witness_is_the_first_committed_survivor() {
    let dir = common::TempDir::new("witness-target");
    let db = Db::ephemeral(dir.path(), WitnessWorld).expect("create");
    insert_parent(&db, 30);
    insert_child(&db, 600, 30);
    insert_child(&db, 601, 30);
    let violations = rejection(db.write(|tx| {
        tx.delete(&WParent {
            id: WParentId(30),
            kind: 0,
        })
    }));
    let [
        Violation::Containment {
            direction: Direction::TargetRequired,
            fact,
            ..
        },
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
