//! The extension forms' commit-time judgment: the cardinality window's
//! touched-parent count walk and the order mark's touched-group ordinal
//! walk (`docs/architecture/30-dependencies.md` § enforcement) — the
//! window boundary family (floor / ceiling / exact / star), the order
//! family (gap / duplicate / adjacent / removal), the ranked family
//! (monotone / inverted / rankless / hop escalation), σ set-membership
//! counting, and the phase laws (key preemption; a mixed statement-phase
//! citation set in materialized order).

use crate::encoding::ValueRef;
use crate::error::{Direction, Error, OrderDefect, Result, Violation};
use crate::schema::{
    FieldId, LiteralSet, RankChain, RankHop, RelationDescriptor, RelationId, Schema,
    SchemaDescriptor, Side, StatementDescriptor, StatementId, ValueType,
};
use crate::storage::env::Environment;
use crate::testutil::TempDir;
use crate::value::Value;

use super::{apply_delta, committed_data, fact, field, side};

// ---------- the window fixture ----------

const HOLDER: RelationId = RelationId(0);
const ACCOUNT: RelationId = RelationId(1);

/// Declared statement order.
const HOLDER_KEY: StatementId = StatementId(0);
const ACCOUNT_HOLDER: StatementId = StatementId(1);
/// `Account(holder | kind == 1) in 1..2 per Holder(id)`.
const SAVINGS_WINDOW: StatementId = StatementId(2);
/// `Account(holder | kind == {1, 2}) in 0..3 per Holder(id)` — the
/// set-selection window (counts over a union do not decompose).
const ANY_KIND_WINDOW: StatementId = StatementId(3);

/// A side selected by one literal-SET binding.
fn set_selected(relation: RelationId, projection: &[u16], field: u16, set: &[u64]) -> Side {
    Side {
        relation,
        projection: projection.iter().map(|&f| FieldId(f)).collect(),
        selection: Box::new([(
            FieldId(field),
            LiteralSet::Many(set.iter().map(|&v| Value::U64(v)).collect()),
        )]),
    }
}

fn window_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Holder".into(),
                fields: vec![field("id", ValueType::U64), field("tag", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Account".into(),
                fields: vec![
                    field("holder", ValueType::U64),
                    field("kind", ValueType::U64),
                    // Distinguishes same-kind children (identity = bytes).
                    field("num", ValueType::U64),
                ],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: HOLDER,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Containment {
                source: side(ACCOUNT, &[0]),
                target: side(HOLDER, &[0]),
            },
            StatementDescriptor::Cardinality {
                source: Side {
                    relation: ACCOUNT,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([(FieldId(1), LiteralSet::One(Value::U64(1)))]),
                },
                lo: 1,
                hi: Some(2),
                target: side(HOLDER, &[0]),
            },
            StatementDescriptor::Cardinality {
                source: set_selected(ACCOUNT, &[0], 1, &[1, 2]),
                lo: 0,
                hi: Some(3),
                target: side(HOLDER, &[0]),
            },
        ],
    }
    .validate()
    .expect("valid fixture")
}

fn holder(schema: &Schema, id: u64) -> Vec<u8> {
    fact(schema, HOLDER, &[ValueRef::U64(id), ValueRef::U64(0)])
}

fn account(schema: &Schema, holder: u64, kind: u64, num: u64) -> Vec<u8> {
    fact(
        schema,
        ACCOUNT,
        &[
            ValueRef::U64(holder),
            ValueRef::U64(kind),
            ValueRef::U64(num),
        ],
    )
}

/// Commits `base` (when nonempty), then applies one further delta; on an
/// abort, asserts the base state survived untouched.
fn base_then_delta(
    name: &str,
    schema: &Schema,
    env_base: &[(RelationId, Vec<u8>)],
    deletes: &[(RelationId, Vec<u8>)],
    inserts: &[(RelationId, Vec<u8>)],
) -> Result<()> {
    let dir = TempDir::new(name);
    let env = Environment::create(dir.path(), schema).expect("create");
    if !env_base.is_empty() {
        apply_delta(&env, schema, &[], env_base).expect("base commit");
    }
    let before = committed_data(&env);
    let result = apply_delta(&env, schema, deletes, inserts);
    if result.is_err() {
        assert_eq!(committed_data(&env), before, "an abort persists nothing");
    }
    drop(env);
    drop(dir);
    result
}

fn assert_window_violation(
    result: Result<()>,
    statement: StatementId,
    parent_fact: &[u8],
    count: u64,
) {
    let err = result.unwrap_err();
    let Error::CommitRejected { violations } = &err else {
        panic!("expected a rejected commit, got {err:?}");
    };
    let [
        Violation::Cardinality {
            statement: named,
            fact,
            count: observed,
        },
    ] = violations.as_slice()
    else {
        panic!("expected one cardinality citation, got {violations:?}");
    };
    assert_eq!(*named, statement);
    assert_eq!(**fact, *parent_fact, "the violation names the parent fact");
    assert_eq!(*observed, count);
}

// ---------- the window boundary family ----------

/// Floor: a parent with no φ-children convicts at count 0, named by the
/// parent's bytes.
#[test]
fn window_floor_convicts_a_childless_parent() {
    let schema = window_schema();
    let h = holder(&schema, 7);
    let result = base_then_delta("win-floor", &schema, &[], &[], &[(HOLDER, h.clone())]);
    assert_window_violation(result, SAVINGS_WINDOW, &h, 0);
}

/// Within bounds: one and two φ-children commit — both window ends are
/// inclusive.
#[test]
fn window_within_bounds_commits() {
    let schema = window_schema();
    let result = base_then_delta(
        "win-within",
        &schema,
        &[
            (HOLDER, holder(&schema, 7)),
            (ACCOUNT, account(&schema, 7, 1, 0)),
        ],
        &[],
        &[(ACCOUNT, account(&schema, 7, 1, 1))],
    );
    result.expect("two selected children sit inside 1..2 and 0..3");
}

/// Ceiling: the third φ-child pushes the count past `hi = 2`; the walk
/// reports the first count past the ceiling.
#[test]
fn window_ceiling_convicts_the_overflowing_group() {
    let schema = window_schema();
    let h = holder(&schema, 7);
    let result = base_then_delta(
        "win-ceiling",
        &schema,
        &[
            (HOLDER, h.clone()),
            (ACCOUNT, account(&schema, 7, 1, 0)),
            (ACCOUNT, account(&schema, 7, 1, 1)),
        ],
        &[],
        &[(ACCOUNT, account(&schema, 7, 1, 2))],
    );
    assert_window_violation(result, SAVINGS_WINDOW, &h, 3);
}

/// The set binding counts the UNION of its alternatives — a member of
/// either kind counts once, and no conjunction of per-literal windows
/// says this (`lean/Bumbledb/Countermodels.lean:
/// disjunctive_window_not_literal_conjunction`).
#[test]
fn window_set_selection_counts_the_union() {
    let schema = window_schema();
    let h = holder(&schema, 7);
    let result = base_then_delta(
        "win-set-ceiling",
        &schema,
        &[
            (HOLDER, h.clone()),
            (ACCOUNT, account(&schema, 7, 1, 0)),
            (ACCOUNT, account(&schema, 7, 2, 0)),
            (ACCOUNT, account(&schema, 7, 2, 1)),
        ],
        &[],
        // kinds 1 and 2 both count toward the set window: this fourth
        // union member overflows its `0..3` ceiling; the savings window
        // (kind 1 alone, count 1) stays green.
        &[(ACCOUNT, account(&schema, 7, 2, 2))],
    );
    assert_window_violation(result, ANY_KIND_WINDOW, &h, 4);
}

/// A set miss: kinds outside the spelled set never count — toward
/// either window.
#[test]
fn window_set_selection_misses_do_not_count() {
    let schema = window_schema();
    let result = base_then_delta(
        "win-set-miss",
        &schema,
        &[
            (HOLDER, holder(&schema, 7)),
            (ACCOUNT, account(&schema, 7, 1, 0)),
            (ACCOUNT, account(&schema, 7, 2, 0)),
            (ACCOUNT, account(&schema, 7, 2, 1)),
        ],
        &[],
        // kind 9 sits outside {1, 2} and outside kind == 1: no count
        // moves, both windows hold.
        &[(ACCOUNT, account(&schema, 7, 9, 0))],
    );
    result.expect("an out-of-set child is not a member of any group");
}

/// Removal: deleting a φ-child re-counts the touched parent — dropping
/// to the floor commits, dropping below it aborts.
#[test]
fn window_removal_recounts_the_touched_parent() {
    let schema = window_schema();
    let h = holder(&schema, 7);
    let dir = TempDir::new("win-removal");
    let env = Environment::create(dir.path(), &schema).expect("create");
    apply_delta(
        &env,
        &schema,
        &[],
        &[
            (HOLDER, h.clone()),
            (ACCOUNT, account(&schema, 7, 1, 0)),
            (ACCOUNT, account(&schema, 7, 1, 1)),
        ],
    )
    .expect("base");
    // One kind-1 child leaves: the savings window still counts 1.
    apply_delta(&env, &schema, &[(ACCOUNT, account(&schema, 7, 1, 1))], &[])
        .expect("the floor still holds at count 1");
    // The last kind-1 child leaves: count 0 < lo 1.
    let before = committed_data(&env);
    let result = apply_delta(&env, &schema, &[(ACCOUNT, account(&schema, 7, 1, 0))], &[]);
    assert_window_violation(result, SAVINGS_WINDOW, &h, 0);
    assert_eq!(committed_data(&env), before);
}

/// Deleting the parent releases the group: the whole cluster leaves in
/// one transaction and the final state has no parent to constrain —
/// whole-cluster atomic demolition, the extension form's face of the
/// no-modes law.
#[test]
fn window_parent_deletion_releases_the_group() {
    let schema = window_schema();
    let result = base_then_delta(
        "win-release",
        &schema,
        &[
            (HOLDER, holder(&schema, 7)),
            (ACCOUNT, account(&schema, 7, 1, 0)),
        ],
        &[
            (HOLDER, holder(&schema, 7)),
            (ACCOUNT, account(&schema, 7, 1, 0)),
        ],
        &[],
    );
    result.expect("no parent, no window obligation");
}

/// Groups are judged per parent: a new childless parent convicts its own
/// group while an untouched neighbor stays green.
#[test]
fn window_judges_each_parent_group_independently() {
    let schema = window_schema();
    let h8 = holder(&schema, 8);
    let result = base_then_delta(
        "win-per-parent",
        &schema,
        &[
            (HOLDER, holder(&schema, 7)),
            (ACCOUNT, account(&schema, 7, 1, 0)),
        ],
        &[],
        &[(HOLDER, h8.clone())],
    );
    assert_window_violation(result, SAVINGS_WINDOW, &h8, 0);
}

// ---------- the phase laws ----------

/// Key violations preempt the statement phase: a delta violating both
/// the holder key and the savings window cites ONLY the key statement
/// (`lean/Bumbledb/Txn.lean: judge_key_preempts`).
#[test]
fn key_violation_preempts_the_window_judgment() {
    let schema = window_schema();
    let result = base_then_delta(
        "win-preempt",
        &schema,
        &[],
        &[],
        &[
            // Two distinct facts, one key tuple — and no kind-1 children
            // anywhere, so the window is violated too.
            (
                HOLDER,
                fact(&schema, HOLDER, &[ValueRef::U64(7), ValueRef::U64(0)]),
            ),
            (
                HOLDER,
                fact(&schema, HOLDER, &[ValueRef::U64(7), ValueRef::U64(1)]),
            ),
        ],
    );
    let err = result.unwrap_err();
    let Error::CommitRejected { violations } = &err else {
        panic!("expected a rejected commit, got {err:?}");
    };
    let [Violation::Functionality { statement, .. }] = violations.as_slice() else {
        panic!("expected the lone key citation, got {violations:?}");
    };
    assert_eq!(*statement, HOLDER_KEY);
}

/// A mixed statement-phase rejection carries containment AND window
/// citations, complete, in materialized statement order — never a mix
/// with the key phase (`lean/Bumbledb/Txn.lean: rejection_is_complete`,
/// `rejection_never_mixes`).
#[test]
fn statement_phase_cites_containments_and_windows_together() {
    let schema = window_schema();
    let h8 = holder(&schema, 8);
    let orphan = account(&schema, 9, 2, 0);
    let result = base_then_delta(
        "win-mixed-phase",
        &schema,
        &[],
        &[],
        &[
            // Holder 8 lands childless (window floor) and account 9→
            // lands parentless (containment) in one delta.
            (HOLDER, h8.clone()),
            (ACCOUNT, orphan.clone()),
        ],
    );
    let err = result.unwrap_err();
    let Error::CommitRejected { violations } = &err else {
        panic!("expected a rejected commit, got {err:?}");
    };
    let [
        Violation::Containment {
            statement: c_stmt,
            direction,
            fact: c_fact,
        },
        Violation::Cardinality {
            statement: w_stmt,
            fact: w_fact,
            count,
        },
    ] = violations.as_slice()
    else {
        panic!("expected containment then window citations, got {violations:?}");
    };
    assert_eq!(
        (*c_stmt, *direction),
        (ACCOUNT_HOLDER, Direction::SourceUnsatisfied)
    );
    assert_eq!(**c_fact, *orphan);
    assert_eq!((*w_stmt, *count), (SAVINGS_WINDOW, 0));
    assert_eq!(**w_fact, *h8);
}

// ---------- the order fixture ----------

const ITEM: RelationId = RelationId(0);
const STEP: RelationId = RelationId(1);
const KIND_RANK: RelationId = RelationId(2);

/// Declared statement order (statement 0 is the `KindRank` key).
const ITEM_ORDER: StatementId = StatementId(1);
const STEP_ORDER: StatementId = StatementId(2);

/// Item(doc, pos, note) with `order Item(pos) per Item(doc)`;
/// Step(flow, pos, kind) with
/// `order Step(pos) per Step(flow) by kind -> KindRank(rank)` over
/// KindRank(kind, rank; key kind).
fn order_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Item".into(),
                fields: vec![
                    field("doc", ValueType::U64),
                    field("pos", ValueType::U64),
                    field("note", ValueType::U64),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Step".into(),
                fields: vec![
                    field("flow", ValueType::U64),
                    field("pos", ValueType::U64),
                    field("kind", ValueType::U64),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "KindRank".into(),
                fields: vec![field("kind", ValueType::U64), field("rank", ValueType::U64)],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: KIND_RANK,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Order {
                relation: ITEM,
                position: FieldId(1),
                grouping: Box::new([FieldId(0)]),
                ranking: None,
            },
            StatementDescriptor::Order {
                relation: STEP,
                position: FieldId(1),
                grouping: Box::new([FieldId(0)]),
                ranking: Some(RankChain {
                    link: FieldId(2),
                    hops: Box::new([RankHop {
                        relation: KIND_RANK,
                        key: FieldId(0),
                        read: FieldId(1),
                    }]),
                }),
            },
        ],
    }
    .validate()
    .expect("valid fixture")
}

fn item(schema: &Schema, doc: u64, pos: u64, note: u64) -> Vec<u8> {
    fact(
        schema,
        ITEM,
        &[ValueRef::U64(doc), ValueRef::U64(pos), ValueRef::U64(note)],
    )
}

fn step(schema: &Schema, flow: u64, pos: u64, kind: u64) -> Vec<u8> {
    fact(
        schema,
        STEP,
        &[ValueRef::U64(flow), ValueRef::U64(pos), ValueRef::U64(kind)],
    )
}

fn kind_rank(schema: &Schema, kind: u64, rank: u64) -> Vec<u8> {
    fact(
        schema,
        KIND_RANK,
        &[ValueRef::U64(kind), ValueRef::U64(rank)],
    )
}

fn assert_order_violation(result: Result<()>, statement: StatementId, defect: OrderDefect) {
    let err = result.unwrap_err();
    let Error::CommitRejected { violations } = &err else {
        panic!("expected a rejected commit, got {err:?}");
    };
    let [
        Violation::Order {
            statement: named,
            defect: found,
            ..
        },
    ] = violations.as_slice()
    else {
        panic!("expected one order citation, got {violations:?}");
    };
    assert_eq!(*named, statement);
    assert_eq!(*found, defect);
}

// ---------- the order family ----------

/// Adjacent positions `1, 2, 3` are exactly `1..k` — the mark holds.
#[test]
fn order_adjacent_positions_commit() {
    let schema = order_schema();
    let result = base_then_delta(
        "ord-adjacent",
        &schema,
        &[],
        &[],
        &[
            (ITEM, item(&schema, 1, 1, 10)),
            (ITEM, item(&schema, 1, 2, 11)),
            (ITEM, item(&schema, 1, 3, 12)),
        ],
    );
    result.expect("1..3 is contiguous, 1-based, duplicate-free");
}

/// The gap: positions `{1, 3}` fail downward closure at 2
/// (`lean/Bumbledb/Countermodels.lean: order_gap` — the countermodel,
/// executed).
#[test]
fn order_gap_convicts() {
    let schema = order_schema();
    let result = base_then_delta(
        "ord-gap",
        &schema,
        &[],
        &[],
        &[
            (ITEM, item(&schema, 1, 1, 10)),
            (ITEM, item(&schema, 1, 3, 12)),
        ],
    );
    assert_order_violation(result, ITEM_ORDER, OrderDefect::PositionGap);
}

/// The duplicate: two distinct facts at position 1
/// (`lean/Bumbledb/Countermodels.lean: order_duplicate`, executed).
#[test]
fn order_duplicate_convicts() {
    let schema = order_schema();
    let result = base_then_delta(
        "ord-duplicate",
        &schema,
        &[],
        &[],
        &[
            (ITEM, item(&schema, 1, 1, 10)),
            (ITEM, item(&schema, 1, 1, 11)),
        ],
    );
    assert_order_violation(result, ITEM_ORDER, OrderDefect::DuplicatePosition);
}

/// 1-basedness: a lone position 2 is a gap below the first member.
#[test]
fn order_not_one_based_convicts() {
    let schema = order_schema();
    let result = base_then_delta(
        "ord-based",
        &schema,
        &[],
        &[],
        &[(ITEM, item(&schema, 1, 2, 10))],
    );
    assert_order_violation(result, ITEM_ORDER, OrderDefect::PositionGap);
}

/// A removal can break downward closure — the removes clause of the
/// touched notion is load-bearing
/// (`lean/Bumbledb/Txn/DeltaRestriction.lean` § the touched notions).
#[test]
fn order_removal_breaking_closure_convicts() {
    let schema = order_schema();
    let result = base_then_delta(
        "ord-removal",
        &schema,
        &[
            (ITEM, item(&schema, 1, 1, 10)),
            (ITEM, item(&schema, 1, 2, 11)),
        ],
        &[(ITEM, item(&schema, 1, 1, 10))],
        &[],
    );
    assert_order_violation(result, ITEM_ORDER, OrderDefect::PositionGap);
}

/// A whole-group renumber in one transaction is judged on the final
/// state alone — legal however the ops interleave.
#[test]
fn order_renumber_in_one_transaction_commits() {
    let schema = order_schema();
    let result = base_then_delta(
        "ord-renumber",
        &schema,
        &[
            (ITEM, item(&schema, 1, 1, 10)),
            (ITEM, item(&schema, 1, 2, 11)),
        ],
        &[
            (ITEM, item(&schema, 1, 1, 10)),
            (ITEM, item(&schema, 1, 2, 11)),
        ],
        &[
            (ITEM, item(&schema, 1, 1, 11)),
            (ITEM, item(&schema, 1, 2, 10)),
        ],
    );
    result.expect("the final state is exactly 1..2");
}

/// Groups are independent: doc 2's discipline does not read doc 1's
/// positions.
#[test]
fn order_groups_are_independent() {
    let schema = order_schema();
    let result = base_then_delta(
        "ord-groups",
        &schema,
        &[
            (ITEM, item(&schema, 1, 1, 10)),
            (ITEM, item(&schema, 1, 2, 11)),
        ],
        &[],
        &[(ITEM, item(&schema, 2, 1, 20))],
    );
    result.expect("each group is its own 1..k");
}

// ---------- the ranked family ----------

/// Monotone ranks in position order commit.
#[test]
fn ranked_monotone_ranks_commit() {
    let schema = order_schema();
    let result = base_then_delta(
        "rank-monotone",
        &schema,
        &[
            (KIND_RANK, kind_rank(&schema, 10, 1)),
            (KIND_RANK, kind_rank(&schema, 20, 2)),
        ],
        &[],
        &[
            (STEP, step(&schema, 1, 1, 10)),
            (STEP, step(&schema, 1, 2, 20)),
        ],
    );
    result.expect("rank 1 sits before rank 2");
}

/// A strictly smaller rank sitting strictly later convicts
/// (`lean/Bumbledb/Order.lean: RankedOrderMark.mono`).
#[test]
fn ranked_inversion_convicts() {
    let schema = order_schema();
    let result = base_then_delta(
        "rank-inverted",
        &schema,
        &[
            (KIND_RANK, kind_rank(&schema, 10, 1)),
            (KIND_RANK, kind_rank(&schema, 20, 2)),
        ],
        &[],
        &[
            (STEP, step(&schema, 1, 1, 20)),
            (STEP, step(&schema, 1, 2, 10)),
        ],
    );
    assert_order_violation(result, STEP_ORDER, OrderDefect::RankOrder);
}

/// A fact whose chain misses a hop has NO rank and imposes nothing —
/// the relational reading (`lean/Bumbledb/Order.lean: RankChain.rankOf`),
/// asserted as unreachable-by-refusal nowhere: the miss is legal data.
#[test]
fn ranked_rankless_members_impose_nothing() {
    let schema = order_schema();
    let result = base_then_delta(
        "rank-rankless",
        &schema,
        &[
            (KIND_RANK, kind_rank(&schema, 10, 1)),
            (KIND_RANK, kind_rank(&schema, 20, 2)),
        ],
        &[],
        &[
            (STEP, step(&schema, 1, 1, 10)),
            // kind 30 resolves no KindRank fact: rankless, unconstrained.
            (STEP, step(&schema, 1, 2, 30)),
            (STEP, step(&schema, 1, 3, 20)),
        ],
    );
    result.expect("the rankless middle member breaks no monotonicity");
}

/// The escalation: a delta touching only the HOP relation re-judges
/// every group — a rank rewrite can invert a group the ordered relation's
/// own delta never touched
/// (`lean/Bumbledb/Txn/DeltaRestriction.lean: rankedTouched`).
#[test]
fn ranked_dirty_hop_relation_touches_every_group() {
    let schema = order_schema();
    let dir = TempDir::new("rank-escalation");
    let env = Environment::create(dir.path(), &schema).expect("create");
    apply_delta(
        &env,
        &schema,
        &[],
        &[
            (KIND_RANK, kind_rank(&schema, 10, 1)),
            (KIND_RANK, kind_rank(&schema, 20, 2)),
            (STEP, step(&schema, 1, 1, 10)),
            (STEP, step(&schema, 1, 2, 20)),
        ],
    )
    .expect("base: monotone");
    // Rewrite kind 10's rank to 3: flow 1 becomes (3, 2) — inverted —
    // though no Step fact is touched.
    let before = committed_data(&env);
    let result = apply_delta(
        &env,
        &schema,
        &[(KIND_RANK, kind_rank(&schema, 10, 1))],
        &[(KIND_RANK, kind_rank(&schema, 10, 3))],
    );
    assert_order_violation(result, STEP_ORDER, OrderDefect::RankOrder);
    assert_eq!(committed_data(&env), before);
}
