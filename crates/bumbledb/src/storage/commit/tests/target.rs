//! The containment target-side judgment (PRD 09 criteria): scalar
//! reverse-edge scans (stranded source, cluster demolition, statement
//! scoping under identical key bytes), the interval re-walk matrix
//! (shrink under / outside a source, chain segment deletion, delete plus
//! covering replacement in one delta), the `==` pair's delete-time
//! totality, and ψ-qualified re-establishment (both forms).
//!
//! Own fixture: two scalar containments sharing one target key (their
//! `R` key bytes collide byte-for-byte — only the statement id separates
//! them), a coverage statement over a pointwise key, a `==` pair, and
//! two σ-carrying containments whose targets can re-land a key guard
//! with a changed ψ-relevant non-key field.

use crate::encoding::{encode_fact, ValueRef};
use crate::error::{Direction, Error, Result};
use crate::schema::{
    FieldDescriptor, FieldId, Generation, IntervalElement, RelationDescriptor, RelationId, Schema,
    SchemaDescriptor, Side, StatementDescriptor, StatementId, ValueType,
};
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::testutil::TempDir;
use crate::value::Value;

use super::committed_data;

const TARGET: RelationId = RelationId(0);
const CLAIM_A: RelationId = RelationId(1);
const CLAIM_B: RelationId = RelationId(2);
const SHIFT: RelationId = RelationId(3);
const SESSION: RelationId = RelationId(4);
const PARENT: RelationId = RelationId(5);
const CHILD: RelationId = RelationId(6);
const ACCOUNT: RelationId = RelationId(7);
const TRANSFER: RelationId = RelationId(8);
const ROSTER: RelationId = RelationId(9);
const REST: RelationId = RelationId(10);

/// Declared statement order (no serial fields, so no auto-keys).
const CLAIM_A_TARGET: StatementId = StatementId(4);
const CLAIM_B_TARGET: StatementId = StatementId(5);
const SESSION_COVER: StatementId = StatementId(6);
const TOTALITY: StatementId = StatementId(7);
const ARM: StatementId = StatementId(8);
const TRANSFER_ACCOUNT: StatementId = StatementId(11);
const REST_COVER: StatementId = StatementId(12);

/// Target(id, note; key id) referenced by two claims: ClaimA(t) <=
/// Target(id) and ClaimB(t) <= Target(id) — same target key, identical
/// key bytes, distinct statements (`note` lets a different fact
/// re-establish the same key tuple). Session(worker, span) <=
/// Shift(worker, span) against Shift's pointwise key. Parent(id; key id)
/// == Child(parent; key parent), lowered to [`TOTALITY`] and [`ARM`].
/// The ψ-qualification pair: Transfer(account) <= Account(id | active ==
/// true) with key Account(id) and a non-key `note` (a distinct in-ψ fact
/// can re-land the guard), and Rest(worker, span) <= Roster(worker, span
/// | rested == true) with pointwise key Roster(worker, span) and non-key
/// `rested` (a byte-identical segment can re-land outside ψ).
#[allow(clippy::too_many_lines)] // one fixture schema, a table
fn schema() -> Schema {
    let field = |name: &str, value_type: ValueType| FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    };
    let interval = ValueType::Interval {
        element: IntervalElement::U64,
    };
    let roster_interval = interval.clone();
    let rest_interval = interval.clone();
    let side = |relation: RelationId, projection: &[u16]| Side {
        relation,
        projection: projection.iter().map(|&f| FieldId(f)).collect(),
        selection: Box::new([]),
    };
    let selected = |relation: RelationId, projection: &[u16], field: u16, literal: bool| Side {
        relation,
        projection: projection.iter().map(|&f| FieldId(f)).collect(),
        selection: Box::new([(FieldId(field), Value::Bool(literal))]),
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Target".into(),
                fields: vec![field("id", ValueType::U64), field("note", ValueType::U64)],
            },
            RelationDescriptor {
                name: "ClaimA".into(),
                fields: vec![field("t", ValueType::U64)],
            },
            RelationDescriptor {
                name: "ClaimB".into(),
                fields: vec![field("t", ValueType::U64)],
            },
            RelationDescriptor {
                name: "Shift".into(),
                fields: vec![
                    field("worker", ValueType::U64),
                    field("span", interval.clone()),
                ],
            },
            RelationDescriptor {
                name: "Session".into(),
                fields: vec![field("worker", ValueType::U64), field("span", interval)],
            },
            RelationDescriptor {
                name: "Parent".into(),
                fields: vec![field("id", ValueType::U64)],
            },
            RelationDescriptor {
                name: "Child".into(),
                fields: vec![field("parent", ValueType::U64)],
            },
            RelationDescriptor {
                name: "Account".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field("active", ValueType::Bool),
                    field("note", ValueType::U64),
                ],
            },
            RelationDescriptor {
                name: "Transfer".into(),
                fields: vec![field("account", ValueType::U64)],
            },
            RelationDescriptor {
                name: "Roster".into(),
                fields: vec![
                    field("worker", ValueType::U64),
                    field("span", roster_interval),
                    field("rested", ValueType::Bool),
                ],
            },
            RelationDescriptor {
                name: "Rest".into(),
                fields: vec![
                    field("worker", ValueType::U64),
                    field("span", rest_interval),
                ],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: TARGET,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Functionality {
                relation: SHIFT,
                projection: Box::new([FieldId(0), FieldId(1)]),
            },
            StatementDescriptor::Functionality {
                relation: PARENT,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Functionality {
                relation: CHILD,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Containment {
                source: side(CLAIM_A, &[0]),
                target: side(TARGET, &[0]),
            },
            StatementDescriptor::Containment {
                source: side(CLAIM_B, &[0]),
                target: side(TARGET, &[0]),
            },
            StatementDescriptor::Containment {
                source: side(SESSION, &[0, 1]),
                target: side(SHIFT, &[0, 1]),
            },
            StatementDescriptor::Containment {
                source: side(PARENT, &[0]),
                target: side(CHILD, &[0]),
            },
            StatementDescriptor::Containment {
                source: side(CHILD, &[0]),
                target: side(PARENT, &[0]),
            },
            StatementDescriptor::Functionality {
                relation: ACCOUNT,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Functionality {
                relation: ROSTER,
                projection: Box::new([FieldId(0), FieldId(1)]),
            },
            StatementDescriptor::Containment {
                source: side(TRANSFER, &[0]),
                target: selected(ACCOUNT, &[0], 1, true),
            },
            StatementDescriptor::Containment {
                source: side(REST, &[0, 1]),
                target: selected(ROSTER, &[0, 1], 2, true),
            },
        ],
    }
    .validate()
    .expect("valid fixture")
}

fn u64_fact(schema: &Schema, rel: RelationId, v: u64) -> Vec<u8> {
    let mut b = Vec::new();
    encode_fact(&[ValueRef::U64(v)], schema.relation(rel).layout(), &mut b);
    b
}

fn target_fact(schema: &Schema, id: u64, note: u64) -> Vec<u8> {
    let mut b = Vec::new();
    encode_fact(
        &[ValueRef::U64(id), ValueRef::U64(note)],
        schema.relation(TARGET).layout(),
        &mut b,
    );
    b
}

fn span_fact(schema: &Schema, rel: RelationId, worker: u64, start: u64, end: u64) -> Vec<u8> {
    let mut b = Vec::new();
    encode_fact(
        &[ValueRef::U64(worker), ValueRef::IntervalU64(start, end)],
        schema.relation(rel).layout(),
        &mut b,
    );
    b
}

fn account_fact(schema: &Schema, id: u64, active: bool, note: u64) -> Vec<u8> {
    let mut b = Vec::new();
    encode_fact(
        &[
            ValueRef::U64(id),
            ValueRef::Bool(active),
            ValueRef::U64(note),
        ],
        schema.relation(ACCOUNT).layout(),
        &mut b,
    );
    b
}

fn roster_fact(schema: &Schema, worker: u64, start: u64, end: u64, rested: bool) -> Vec<u8> {
    let mut b = Vec::new();
    encode_fact(
        &[
            ValueRef::U64(worker),
            ValueRef::IntervalU64(start, end),
            ValueRef::Bool(rested),
        ],
        schema.relation(ROSTER).layout(),
        &mut b,
    );
    b
}

/// Records `deletes` then `inserts` in one delta and commits (order is
/// semantically irrelevant — the delta is set arithmetic).
fn apply_delta(
    env: &Environment,
    schema: &Schema,
    deletes: &[(RelationId, Vec<u8>)],
    inserts: &[(RelationId, Vec<u8>)],
) -> Result<()> {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (rel, fact) in deletes {
        delta.delete(&view, *rel, fact).expect("record delete");
    }
    for (rel, fact) in inserts {
        delta.insert(&view, *rel, fact).expect("record insert");
    }
    drop(view);
    commit(delta, env).map(|_| ())
}

/// Commits `base`, then applies a second delta of `deletes` + `inserts`;
/// on an abort, asserts the base state survived untouched.
fn base_then_delta(
    name: &str,
    base: &[(RelationId, Vec<u8>)],
    deletes: &[(RelationId, Vec<u8>)],
    inserts: &[(RelationId, Vec<u8>)],
) -> Result<()> {
    let dir = TempDir::new(name);
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    apply_delta(&env, &schema, &[], base).expect("base commit");
    let before = committed_data(&env);
    let result = apply_delta(&env, &schema, deletes, inserts);
    if result.is_err() {
        assert_eq!(committed_data(&env), before);
    }
    result
}

fn assert_target_violation(result: Result<()>, statement: StatementId, source_fact: &[u8]) {
    let err = result.unwrap_err();
    let Error::ContainmentViolation {
        statement: named,
        direction,
        fact,
    } = &err
    else {
        panic!("expected a containment violation, got {err:?}");
    };
    assert_eq!(*named, statement);
    assert_eq!(*direction, Direction::TargetRequired);
    assert_eq!(**fact, *source_fact, "the violation names the source fact");
}

// ---------- scalar form ----------

#[test]
fn deleting_a_referenced_target_alone_aborts_naming_the_source() {
    let schema = schema();
    let claim = u64_fact(&schema, CLAIM_A, 5);
    assert_target_violation(
        base_then_delta(
            "tgt-scalar-strand",
            &[
                (TARGET, target_fact(&schema, 5, 0)),
                (CLAIM_A, claim.clone()),
            ],
            &[(TARGET, target_fact(&schema, 5, 0))],
            &[],
        ),
        CLAIM_A_TARGET,
        &claim,
    );
}

#[test]
fn cluster_demolition_commits() {
    // Target + every source deleted in one delta: the final state is
    // clean — the deleted source's R entries went with it in phase 1.
    let schema = schema();
    base_then_delta(
        "tgt-scalar-demolition",
        &[
            (TARGET, target_fact(&schema, 5, 0)),
            (CLAIM_A, u64_fact(&schema, CLAIM_A, 5)),
        ],
        &[
            (TARGET, target_fact(&schema, 5, 0)),
            (CLAIM_A, u64_fact(&schema, CLAIM_A, 5)),
        ],
        &[],
    )
    .expect("the whole cluster leaves together");
}

#[test]
fn surviving_source_of_the_other_statement_aborts_on_its_own_id() {
    // Both claims require Target(5) through distinct statements whose R
    // key bytes are byte-identical. ClaimA leaves with the target; ClaimB
    // survives — the scan must convict CLAIM_B_TARGET, not CLAIM_A_TARGET.
    let schema = schema();
    let survivor = u64_fact(&schema, CLAIM_B, 5);
    assert_target_violation(
        base_then_delta(
            "tgt-scalar-statement-scope",
            &[
                (TARGET, target_fact(&schema, 5, 0)),
                (CLAIM_A, u64_fact(&schema, CLAIM_A, 5)),
                (CLAIM_B, survivor.clone()),
            ],
            &[
                (TARGET, target_fact(&schema, 5, 0)),
                (CLAIM_A, u64_fact(&schema, CLAIM_A, 5)),
            ],
            &[],
        ),
        CLAIM_B_TARGET,
        &survivor,
    );
}

#[test]
fn delete_and_reestablish_by_a_different_fact_commits() {
    // The key tuple (id = 5) is deleted with one fact and re-established
    // by another (a changed non-key field): the claims carry no target ψ,
    // so the plain set difference empties their check set and the
    // surviving claim stays satisfied.
    let schema = schema();
    base_then_delta(
        "tgt-scalar-reestablish",
        &[
            (TARGET, target_fact(&schema, 5, 0)),
            (CLAIM_A, u64_fact(&schema, CLAIM_A, 5)),
        ],
        &[(TARGET, target_fact(&schema, 5, 0))],
        &[(TARGET, target_fact(&schema, 5, 1))],
    )
    .expect("the re-established key tuple satisfies the survivor");
}

// ---------- interval form ----------

#[test]
fn shrink_under_a_covered_source_aborts() {
    // Delete [0,10), insert [0,7) under a source [5,9): the hole [7,9)
    // fails the re-walk.
    let schema = schema();
    let s = span_fact(&schema, SESSION, 1, 5, 9);
    assert_target_violation(
        base_then_delta(
            "tgt-shrink-under",
            &[
                (SHIFT, span_fact(&schema, SHIFT, 1, 0, 10)),
                (SESSION, s.clone()),
            ],
            &[(SHIFT, span_fact(&schema, SHIFT, 1, 0, 10))],
            &[(SHIFT, span_fact(&schema, SHIFT, 1, 0, 7))],
        ),
        SESSION_COVER,
        &s,
    );
}

#[test]
fn shrink_outside_the_source_commits() {
    // The same shrink under a source [2,6): the surviving [0,7) still
    // covers it — the re-walk runs against the final U state.
    let schema = schema();
    base_then_delta(
        "tgt-shrink-outside",
        &[
            (SHIFT, span_fact(&schema, SHIFT, 1, 0, 10)),
            (SESSION, span_fact(&schema, SESSION, 1, 2, 6)),
        ],
        &[(SHIFT, span_fact(&schema, SHIFT, 1, 0, 10))],
        &[(SHIFT, span_fact(&schema, SHIFT, 1, 0, 7))],
    )
    .expect("the shrunk segment still covers the source");
}

#[test]
fn deleting_one_segment_of_a_covering_chain_aborts() {
    let schema = schema();
    let s = span_fact(&schema, SESSION, 1, 2, 9);
    assert_target_violation(
        base_then_delta(
            "tgt-chain-break",
            &[
                (SHIFT, span_fact(&schema, SHIFT, 1, 0, 5)),
                (SHIFT, span_fact(&schema, SHIFT, 1, 5, 10)),
                (SESSION, s.clone()),
            ],
            &[(SHIFT, span_fact(&schema, SHIFT, 1, 5, 10))],
            &[],
        ),
        SESSION_COVER,
        &s,
    );
}

#[test]
fn delete_plus_replacement_covering_the_hole_commits() {
    let schema = schema();
    base_then_delta(
        "tgt-chain-replace",
        &[
            (SHIFT, span_fact(&schema, SHIFT, 1, 0, 5)),
            (SHIFT, span_fact(&schema, SHIFT, 1, 5, 10)),
            (SESSION, span_fact(&schema, SESSION, 1, 2, 9)),
        ],
        &[(SHIFT, span_fact(&schema, SHIFT, 1, 5, 10))],
        &[(SHIFT, span_fact(&schema, SHIFT, 1, 5, 9))],
    )
    .expect("the replacement covers the hole in the same delta");
}

#[test]
fn two_disestablished_segments_of_one_group_walk_the_source_once() {
    // Both chain segments leave and two replacements land: the source
    // intersects both disestablished windows, the affected set dedupes it
    // to one walk, and the walk passes against the final chain.
    let schema = schema();
    base_then_delta(
        "tgt-chain-dedupe",
        &[
            (SHIFT, span_fact(&schema, SHIFT, 1, 0, 5)),
            (SHIFT, span_fact(&schema, SHIFT, 1, 5, 10)),
            (SESSION, span_fact(&schema, SESSION, 1, 2, 9)),
        ],
        &[
            (SHIFT, span_fact(&schema, SHIFT, 1, 0, 5)),
            (SHIFT, span_fact(&schema, SHIFT, 1, 5, 10)),
        ],
        &[
            (SHIFT, span_fact(&schema, SHIFT, 1, 0, 6)),
            (SHIFT, span_fact(&schema, SHIFT, 1, 6, 9)),
        ],
    )
    .expect("the rebuilt chain covers the source");
}

#[test]
fn segment_outside_every_source_deletes_freely() {
    // The disestablished window [20,30) intersects no source interval:
    // the group scan filters it out and nothing is walked.
    let schema = schema();
    base_then_delta(
        "tgt-outside-window",
        &[
            (SHIFT, span_fact(&schema, SHIFT, 1, 0, 10)),
            (SHIFT, span_fact(&schema, SHIFT, 1, 20, 30)),
            (SESSION, span_fact(&schema, SESSION, 1, 2, 9)),
        ],
        &[(SHIFT, span_fact(&schema, SHIFT, 1, 20, 30))],
        &[],
    )
    .expect("a non-intersecting segment is unreferenced");
}

// ---------- ψ-qualified re-establishment ----------

#[test]
fn reestablishment_outside_psi_aborts_naming_the_source() {
    // Delete Account(9, active=true) + insert Account(9, active=false)
    // in one delta: the key guard (id = 9) re-lands, but the establishing
    // fact fails TRANSFER_ACCOUNT's ψ — the tuple stays in that
    // statement's check set and the surviving transfer convicts.
    let schema = schema();
    let t = u64_fact(&schema, TRANSFER, 9);
    assert_target_violation(
        base_then_delta(
            "tgt-psi-scalar-strand",
            &[
                (ACCOUNT, account_fact(&schema, 9, true, 0)),
                (TRANSFER, t.clone()),
            ],
            &[(ACCOUNT, account_fact(&schema, 9, true, 0))],
            &[(ACCOUNT, account_fact(&schema, 9, false, 0))],
        ),
        TRANSFER_ACCOUNT,
        &t,
    );
}

#[test]
fn reestablishment_inside_psi_commits() {
    // The re-establishing fact satisfies ψ (only the non-selected `note`
    // changed): the tuple counts as re-established for the σ-carrying
    // dependent too.
    let schema = schema();
    base_then_delta(
        "tgt-psi-scalar-satisfied",
        &[
            (ACCOUNT, account_fact(&schema, 9, true, 0)),
            (TRANSFER, u64_fact(&schema, TRANSFER, 9)),
        ],
        &[(ACCOUNT, account_fact(&schema, 9, true, 0))],
        &[(ACCOUNT, account_fact(&schema, 9, true, 1))],
    )
    .expect("an in-ψ establisher re-establishes the tuple");
}

#[test]
fn interval_reestablishment_outside_psi_aborts() {
    // Byte-identical segment delete + reinsert whose ψ-relevant non-key
    // field changed: the guard bytes re-land, the establisher fails
    // REST_COVER's ψ, and the re-walk's per-segment σ check convicts,
    // naming the surviving rest.
    let schema = schema();
    let r = span_fact(&schema, REST, 1, 2, 6);
    assert_target_violation(
        base_then_delta(
            "tgt-psi-interval-strand",
            &[
                (ROSTER, roster_fact(&schema, 1, 0, 10, true)),
                (REST, r.clone()),
            ],
            &[(ROSTER, roster_fact(&schema, 1, 0, 10, true))],
            &[(ROSTER, roster_fact(&schema, 1, 0, 10, false))],
        ),
        REST_COVER,
        &r,
    );
}

// ---------- the == pair (both directions on delete) ----------

#[test]
fn parent_and_child_deleted_together_commit() {
    let schema = schema();
    base_then_delta(
        "tgt-pair-demolition",
        &[
            (PARENT, u64_fact(&schema, PARENT, 1)),
            (CHILD, u64_fact(&schema, CHILD, 1)),
        ],
        &[
            (PARENT, u64_fact(&schema, PARENT, 1)),
            (CHILD, u64_fact(&schema, CHILD, 1)),
        ],
        &[],
    )
    .expect("the == cluster leaves whole");
}

#[test]
fn child_alone_deleted_aborts_on_the_totality_direction() {
    // The surviving parent still requires its child: the totality
    // statement's target side convicts, naming the parent.
    let schema = schema();
    let p = u64_fact(&schema, PARENT, 1);
    assert_target_violation(
        base_then_delta(
            "tgt-pair-child-alone",
            &[(PARENT, p.clone()), (CHILD, u64_fact(&schema, CHILD, 1))],
            &[(CHILD, u64_fact(&schema, CHILD, 1))],
            &[],
        ),
        TOTALITY,
        &p,
    );
}

#[test]
fn noop_parent_reinsert_with_child_delete_aborts_target_side() {
    // The ==/totality corner (normative rule, `60-validation.md`: "source
    // side" means facts the transaction *actually added*): a redundant
    // re-insert of the committed parent records nothing — the delta is
    // {delete child} and the violation surfaces target-side via the
    // parent's standing R edge, naming the parent, never source-side.
    let schema = schema();
    let p = u64_fact(&schema, PARENT, 1);
    assert_target_violation(
        base_then_delta(
            "tgt-pair-noop-reinsert",
            &[(PARENT, p.clone()), (CHILD, u64_fact(&schema, CHILD, 1))],
            &[(CHILD, u64_fact(&schema, CHILD, 1))],
            &[(PARENT, p.clone())],
        ),
        TOTALITY,
        &p,
    );
}

#[test]
fn parent_delete_reinsert_with_child_delete_aborts_target_side() {
    // The cancellation form of the same corner: delete(parent) +
    // insert(parent) nets to nothing, so the parent is never judged
    // source-side (that judgment previously flipped the Direction label
    // against the naive model). Same target-side verdict as above.
    let schema = schema();
    let p = u64_fact(&schema, PARENT, 1);
    assert_target_violation(
        base_then_delta(
            "tgt-pair-cancel-reinsert",
            &[(PARENT, p.clone()), (CHILD, u64_fact(&schema, CHILD, 1))],
            &[(PARENT, p.clone()), (CHILD, u64_fact(&schema, CHILD, 1))],
            &[(PARENT, p.clone())],
        ),
        TOTALITY,
        &p,
    );
}

#[test]
fn parent_alone_deleted_aborts_on_the_arm_direction() {
    // Symmetric machinery: the surviving child still requires its parent.
    let schema = schema();
    let c = u64_fact(&schema, CHILD, 1);
    assert_target_violation(
        base_then_delta(
            "tgt-pair-parent-alone",
            &[(PARENT, u64_fact(&schema, PARENT, 1)), (CHILD, c.clone())],
            &[(PARENT, u64_fact(&schema, PARENT, 1))],
            &[],
        ),
        ARM,
        &c,
    );
}
