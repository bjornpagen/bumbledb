//! The window form's commit-time judgment: the cardinality window's
//! touched-parent count walk
//! (`docs/architecture/30-dependencies.md` § enforcement) — the
//! window boundary family (floor / ceiling / exact / star), σ
//! set-membership counting, and the phase laws (key preemption; a mixed
//! statement-phase citation set in materialized order).

use crate::encoding::ValueRef;
use crate::error::{Direction, Error, Result, Violation};
use crate::schema::{
    FieldId, LiteralSet, RelationDescriptor, RelationId, Schema, SchemaDescriptor, Side,
    StatementDescriptor, StatementId, ValueType,
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
/// `Holder(id) <={1..2} Account(holder | kind == 1)`.
const SAVINGS_WINDOW: StatementId = StatementId(2);
/// `Holder(id) <={0..3} Account(holder | kind == {1, 2})` — the
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

// ---------- the exclusion window ----------

/// Declared statement order in [`exclusion_schema`].
const FORBIDDEN_WINDOW: StatementId = StatementId(1);

/// `Holder(id) <={0} Account(holder | kind == 9)` — the `{0}` exclusion:
/// no holder may have a kind-9 account. Its own fixture (the boundary
/// family's schema inserts kind-9 accounts as non-members).
fn exclusion_schema() -> Schema {
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
                    field("num", ValueType::U64),
                ],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: HOLDER,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Cardinality {
                source: Side {
                    relation: ACCOUNT,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([(FieldId(1), LiteralSet::One(Value::U64(9)))]),
                },
                lo: 0,
                hi: Some(0),
                target: side(HOLDER, &[0]),
            },
        ],
    }
    .validate()
    .expect("the exclusion window seals")
}

/// `{0}` convicts at the first member: a kind-9 child under an existing
/// holder counts 1 > 0, named by the parent's bytes.
#[test]
fn exclusion_window_convicts_the_first_member() {
    let schema = exclusion_schema();
    let h = holder(&schema, 7);
    let result = base_then_delta(
        "win-exclusion-member",
        &schema,
        &[(HOLDER, h.clone())],
        &[],
        &[(ACCOUNT, account(&schema, 7, 9, 0))],
    );
    assert_window_violation(result, FORBIDDEN_WINDOW, &h, 1);
}

/// `{0}` admits everything outside σ: non-selected kinds commit freely,
/// and so does the childless parent (count 0 sits inside the window —
/// both ends inclusive, `0..0`).
#[test]
fn exclusion_window_admits_non_members() {
    let schema = exclusion_schema();
    let result = base_then_delta(
        "win-exclusion-clean",
        &schema,
        &[],
        &[],
        &[
            (HOLDER, holder(&schema, 7)),
            (ACCOUNT, account(&schema, 7, 1, 0)),
            (ACCOUNT, account(&schema, 7, 2, 1)),
        ],
    );
    result.expect("out-of-sigma children never count against the exclusion");
}

/// Deleting the parent releases the exclusion: the member lands in the
/// same delta that removes its parent — the final state has no parent to
/// constrain (windows never manufacture parents,
/// `lean/Bumbledb/Cardinality.lean: cardinality_of_empty_parent`).
#[test]
fn exclusion_window_releases_with_the_parent() {
    let schema = exclusion_schema();
    let h = holder(&schema, 7);
    let result = base_then_delta(
        "win-exclusion-release",
        &schema,
        &[(HOLDER, h.clone())],
        &[(HOLDER, h)],
        &[(ACCOUNT, account(&schema, 7, 9, 0))],
    );
    result.expect("no parent, no exclusion obligation");
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
