//! The capacity statement's commit-time judgment: the touched-parent
//! measure walk (`docs/architecture/30-dependencies.md` § enforcement)
//! — the window boundary family (floor / ceiling / exact / star), σ
//! set-membership measuring, the weighted family (sum ceiling/floor,
//! the zero-weight footgun, dependent bounds resolved from the
//! final-state holder, Duration weights), and the phase laws (key
//! preemption; a mixed statement-phase citation set in materialized
//! order).

use crate::encoding::ValueRef;
use crate::error::{Direction, Error, Result, Violation};
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use crate::storage::env::Environment;
use crate::testutil::TempDir;
use bumbledb_theory::Value;
use bumbledb_theory::schema::{
    Bound, FieldDescriptor, FieldId, Generation, LiteralSet, RelationDescriptor, RelationId,
    SchemaDescriptor, Side, StatementDescriptor, StatementId, ValueType, Weight,
};

use super::{apply_delta, committed_data, fact, field, interval, side};

// ---------- the unit-instance fixture (the count spelling survives) ----------

const HOLDER: RelationId = RelationId(0);
const ACCOUNT: RelationId = RelationId(1);

/// Declared statement order.
const HOLDER_KEY: StatementId = StatementId(0);
const ACCOUNT_HOLDER: StatementId = StatementId(1);
/// `Holder(id) <={1..2} Account(holder | kind == 1)` — unit weight, the
/// count instance.
const SAVINGS_CAPACITY: StatementId = StatementId(2);
/// `Holder(id) <={0..3} Account(holder | kind == {1, 2})` — the
/// set-selection capacity law (measures over a union do not decompose).
const ANY_KIND_CAPACITY: StatementId = StatementId(3);

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

fn capacity_schema() -> Schema {
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
            StatementDescriptor::Capacity {
                target: side(HOLDER, &[0]),
                weight: Weight::Unit,
                lo: 1,
                hi: Some(Bound::Lit(2)),
                source: Side {
                    relation: ACCOUNT,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([(FieldId(1), LiteralSet::One(Value::U64(1)))]),
                },
            },
            StatementDescriptor::Capacity {
                target: side(HOLDER, &[0]),
                weight: Weight::Unit,
                lo: 0,
                hi: Some(Bound::Lit(3)),
                source: set_selected(ACCOUNT, &[0], 1, &[1, 2]),
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

fn assert_capacity_violation(
    result: Result<()>,
    statement: StatementId,
    parent_fact: &[u8],
    measure: u128,
) {
    let err = result.unwrap_err();
    let Error::CommitRejected { violations } = &err else {
        panic!("expected a rejected commit, got {err:?}");
    };
    let [
        Violation::Capacity {
            statement: named,
            fact,
            measure: observed,
        },
    ] = violations.as_slice()
    else {
        panic!("expected one capacity citation, got {violations:?}");
    };
    assert_eq!(*named, statement);
    assert_eq!(**fact, *parent_fact, "the violation names the parent fact");
    assert_eq!(*observed, measure);
}

// ---------- the window boundary family (unit instance) ----------

/// Floor: a parent with no φ-children convicts at measure 0, named by
/// the parent's bytes.
#[test]
fn capacity_floor_convicts_a_childless_parent() {
    let schema = capacity_schema();
    let h = holder(&schema, 7);
    let result = base_then_delta("cap-floor", &schema, &[], &[], &[(HOLDER, h.clone())]);
    assert_capacity_violation(result, SAVINGS_CAPACITY, &h, 0);
}

/// Within bounds: one and two φ-children commit — both window ends are
/// inclusive.
#[test]
fn capacity_within_the_window_commits() {
    let schema = capacity_schema();
    let result = base_then_delta(
        "cap-within",
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

/// Ceiling: the third φ-child pushes the unit measure past `hi = 2`; the
/// witnessed measure is the group's total (the clip serves the verdict,
/// the full sum serves the witness — ruled 2026-07-24, C14).
#[test]
fn capacity_ceiling_convicts_the_overflowing_group() {
    let schema = capacity_schema();
    let h = holder(&schema, 7);
    let result = base_then_delta(
        "cap-ceiling",
        &schema,
        &[
            (HOLDER, h.clone()),
            (ACCOUNT, account(&schema, 7, 1, 0)),
            (ACCOUNT, account(&schema, 7, 1, 1)),
        ],
        &[],
        &[(ACCOUNT, account(&schema, 7, 1, 2))],
    );
    assert_capacity_violation(result, SAVINGS_CAPACITY, &h, 3);
}

/// The set binding measures the UNION of its alternatives — a member of
/// either kind counts once, and no conjunction of per-literal windows
/// says this (`lean/Bumbledb/Countermodels.lean:
/// disjunctive_window_not_literal_conjunction`).
#[test]
fn capacity_set_selection_measures_the_union() {
    let schema = capacity_schema();
    let h = holder(&schema, 7);
    let result = base_then_delta(
        "cap-set-ceiling",
        &schema,
        &[
            (HOLDER, h.clone()),
            (ACCOUNT, account(&schema, 7, 1, 0)),
            (ACCOUNT, account(&schema, 7, 2, 0)),
            (ACCOUNT, account(&schema, 7, 2, 1)),
        ],
        &[],
        // kinds 1 and 2 both count toward the set law: this fourth
        // union member overflows its `0..3` ceiling; the savings law
        // (kind 1 alone, measure 1) stays green.
        &[(ACCOUNT, account(&schema, 7, 2, 2))],
    );
    assert_capacity_violation(result, ANY_KIND_CAPACITY, &h, 4);
}

/// A set miss: kinds outside the spelled set never count — toward
/// either capacity law.
#[test]
fn capacity_set_selection_misses_do_not_count() {
    let schema = capacity_schema();
    let result = base_then_delta(
        "cap-set-miss",
        &schema,
        &[
            (HOLDER, holder(&schema, 7)),
            (ACCOUNT, account(&schema, 7, 1, 0)),
            (ACCOUNT, account(&schema, 7, 2, 0)),
            (ACCOUNT, account(&schema, 7, 2, 1)),
        ],
        &[],
        // kind 9 sits outside {1, 2} and outside kind == 1: no measure
        // moves, both laws hold.
        &[(ACCOUNT, account(&schema, 7, 9, 0))],
    );
    result.expect("an out-of-set child is not a member of any group");
}

/// Removal: deleting a φ-child re-measures the touched parent —
/// dropping to the floor commits, dropping below it aborts.
#[test]
fn capacity_removal_remeasures_the_touched_parent() {
    let schema = capacity_schema();
    let h = holder(&schema, 7);
    let dir = TempDir::new("cap-removal");
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
    // One kind-1 child leaves: the savings law still measures 1.
    apply_delta(&env, &schema, &[(ACCOUNT, account(&schema, 7, 1, 1))], &[])
        .expect("the floor still holds at measure 1");
    // The last kind-1 child leaves: measure 0 < lo 1.
    let before = committed_data(&env);
    let result = apply_delta(&env, &schema, &[(ACCOUNT, account(&schema, 7, 1, 0))], &[]);
    assert_capacity_violation(result, SAVINGS_CAPACITY, &h, 0);
    assert_eq!(committed_data(&env), before);
}

/// Deleting the parent releases the group: the whole cluster leaves in
/// one transaction and the final state has no parent to constrain —
/// whole-cluster atomic demolition, the extension form's face of the
/// no-modes law.
#[test]
fn capacity_parent_deletion_releases_the_group() {
    let schema = capacity_schema();
    let result = base_then_delta(
        "cap-release",
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
    result.expect("no parent, no capacity obligation");
}

/// Groups are judged per parent: a new childless parent convicts its own
/// group while an untouched neighbor stays green.
#[test]
fn capacity_judges_each_parent_group_independently() {
    let schema = capacity_schema();
    let h8 = holder(&schema, 8);
    let result = base_then_delta(
        "cap-per-parent",
        &schema,
        &[
            (HOLDER, holder(&schema, 7)),
            (ACCOUNT, account(&schema, 7, 1, 0)),
        ],
        &[],
        &[(HOLDER, h8.clone())],
    );
    assert_capacity_violation(result, SAVINGS_CAPACITY, &h8, 0);
}

// ---------- the exclusion window ----------

/// Declared statement order in [`exclusion_schema`].
const FORBIDDEN_CAPACITY: StatementId = StatementId(1);

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
            StatementDescriptor::Capacity {
                target: side(HOLDER, &[0]),
                weight: Weight::Unit,
                lo: 0,
                hi: Some(Bound::Lit(0)),
                source: Side {
                    relation: ACCOUNT,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([(FieldId(1), LiteralSet::One(Value::U64(9)))]),
                },
            },
        ],
    }
    .validate()
    .expect("the exclusion law seals")
}

/// `{0}` convicts at the first member: a kind-9 child under an existing
/// holder measures 1 > 0, named by the parent's bytes.
#[test]
fn exclusion_window_convicts_the_first_member() {
    let schema = exclusion_schema();
    let h = holder(&schema, 7);
    let result = base_then_delta(
        "cap-exclusion-member",
        &schema,
        &[(HOLDER, h.clone())],
        &[],
        &[(ACCOUNT, account(&schema, 7, 9, 0))],
    );
    assert_capacity_violation(result, FORBIDDEN_CAPACITY, &h, 1);
}

/// `{0}` admits everything outside σ: non-selected kinds commit freely,
/// and so does the childless parent (measure 0 sits inside the window —
/// both ends inclusive, `0..0`).
#[test]
fn exclusion_window_admits_non_members() {
    let schema = exclusion_schema();
    let result = base_then_delta(
        "cap-exclusion-clean",
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
/// constrain (capacity statements never manufacture parents,
/// `lean/Bumbledb/Capacity.lean: capacity_of_empty_parent`).
#[test]
fn exclusion_window_releases_with_the_parent() {
    let schema = exclusion_schema();
    let h = holder(&schema, 7);
    let result = base_then_delta(
        "cap-exclusion-release",
        &schema,
        &[(HOLDER, h.clone())],
        &[(HOLDER, h)],
        &[(ACCOUNT, account(&schema, 7, 9, 0))],
    );
    result.expect("no parent, no exclusion obligation");
}

// ---------- the weighted family ----------

const POOL: RelationId = RelationId(0);
const DEVICE: RelationId = RelationId(1);

/// Declared statement order in [`weighted_schema`].
const WATTS_CAPACITY: StatementId = StatementId(1);

/// `Pool(id) <=[watts]{5..100} Device(pool)` — a column weight under
/// literal bounds: the measure is Σ watts over the pool's devices.
fn weighted_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Pool".into(),
                fields: vec![field("id", ValueType::U64), field("supply", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Device".into(),
                fields: vec![
                    field("pool", ValueType::U64),
                    field("watts", ValueType::U64),
                    // Distinguishes same-weight devices (identity = bytes).
                    field("num", ValueType::U64),
                ],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: POOL,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Capacity {
                target: side(POOL, &[0]),
                weight: Weight::Field(FieldId(1)),
                lo: 5,
                hi: Some(Bound::Lit(100)),
                source: side(DEVICE, &[0]),
            },
        ],
    }
    .validate()
    .expect("the weighted fixture seals")
}

fn pool(schema: &Schema, id: u64, supply: u64) -> Vec<u8> {
    fact(schema, POOL, &[ValueRef::U64(id), ValueRef::U64(supply)])
}

fn device(schema: &Schema, pool: u64, watts: u64, num: u64) -> Vec<u8> {
    fact(
        schema,
        DEVICE,
        &[
            ValueRef::U64(pool),
            ValueRef::U64(watts),
            ValueRef::U64(num),
        ],
    )
}

/// A delete op never derives its capacity value slot — the removal is
/// key-only, and the derive is fallible on a weighted statement (a
/// ray-valued Duration weight refuses): a value the applier never reads
/// must not be able to refuse a delete. The insert twin still derives.
#[test]
fn delete_ops_never_derive_the_capacity_value_slot() {
    let schema = weighted_schema();
    let dir = crate::testutil::TempDir::new("cap-delete-weight-none");
    let env = Environment::create(dir.path(), &schema).expect("create");
    let d = device(&schema, 1, 60, 0);
    let mut insert_delta = crate::storage::delta::WriteDelta::new(&schema);
    {
        let view = env.read_txn().expect("txn");
        insert_delta.insert(&view, DEVICE, &d).expect("record");
    }
    let plan = super::plan_for(&insert_delta, &env);
    let [op] = &*plan.inserts else {
        panic!("one insert op");
    };
    let [edge] = &*op.capacity_edges else {
        panic!("one capacity edge");
    };
    assert_eq!(edge.weight, Some(60), "the insert op derives the slot");
    drop(plan);
    drop(insert_delta);

    apply_delta(&env, &schema, &[], &[(DEVICE, d.clone())]).expect("base commit");
    let mut delete_delta = crate::storage::delta::WriteDelta::new(&schema);
    {
        let view = env.read_txn().expect("txn");
        delete_delta.delete(&view, DEVICE, &d).expect("record");
    }
    let plan = super::plan_for(&delete_delta, &env);
    let [op] = &*plan.deletes else {
        panic!("one delete op");
    };
    let [edge] = &*op.capacity_edges else {
        panic!("one capacity edge");
    };
    assert_eq!(edge.weight, None, "the delete op never derives");
}

/// Sum within bounds: weights sum, not count — three devices measuring
/// 60 + 30 + 10 = 100 sit exactly on the inclusive ceiling.
#[test]
fn capacity_sum_within_bounds_commits() {
    let schema = weighted_schema();
    let result = base_then_delta(
        "cap-sum-within",
        &schema,
        &[
            (POOL, pool(&schema, 1, 0)),
            (DEVICE, device(&schema, 1, 60, 0)),
            (DEVICE, device(&schema, 1, 30, 1)),
        ],
        &[],
        &[(DEVICE, device(&schema, 1, 10, 2))],
    );
    result.expect("Σ watts = 100 sits on the inclusive ceiling");
}

/// Sum ceiling: the third device pushes Σ watts to 180 > 100. The
/// witnessed measure is the group's FULL total — on conviction the judge
/// completes the walk so the report is walk-order-independent (ruled
/// 2026-07-24, C14: the clip serves the verdict, the full sum serves the
/// witness).
#[test]
fn capacity_sum_ceiling_convicts_with_the_full_measure() {
    let schema = weighted_schema();
    let p = pool(&schema, 1, 0);
    let result = base_then_delta(
        "cap-sum-ceiling",
        &schema,
        // The base sits AT the ceiling (50 + 50 = 100 ≤ hi) — the probe
        // delta alone pushes it over.
        &[
            (POOL, p.clone()),
            (DEVICE, device(&schema, 1, 50, 0)),
            (DEVICE, device(&schema, 1, 50, 1)),
        ],
        &[],
        &[(DEVICE, device(&schema, 1, 80, 2))],
    );
    assert_capacity_violation(result, WATTS_CAPACITY, &p, 180);
}

/// Sum floor: a group whose total sits under `lo = 5` convicts with the
/// full measure (a floor conviction always walks the whole group).
#[test]
fn capacity_sum_floor_convicts_the_light_group() {
    let schema = weighted_schema();
    let p = pool(&schema, 1, 0);
    let result = base_then_delta(
        "cap-sum-floor",
        &schema,
        &[],
        &[],
        &[(POOL, p.clone()), (DEVICE, device(&schema, 1, 3, 0))],
    );
    assert_capacity_violation(result, WATTS_CAPACITY, &p, 3);
}

/// The § 6 footgun as commit-path data: zero-weight children EXIST but
/// measure 0 — `Sum in {5..100}` is not an existence claim over rows.
/// Two zero-watt devices convict the floor at measure 0; adding one
/// five-watt device satisfies it (zero-weight rows ride along freely).
#[test]
fn capacity_zero_weight_children_do_not_lift_the_floor() {
    let schema = weighted_schema();
    let p = pool(&schema, 1, 0);
    let result = base_then_delta(
        "cap-zero-weight-floor",
        &schema,
        &[],
        &[],
        &[
            (POOL, p.clone()),
            (DEVICE, device(&schema, 1, 0, 0)),
            (DEVICE, device(&schema, 1, 0, 1)),
        ],
    );
    assert_capacity_violation(result, WATTS_CAPACITY, &p, 0);
    let result = base_then_delta(
        "cap-zero-weight-pass",
        &schema,
        &[],
        &[],
        &[
            (POOL, pool(&schema, 1, 0)),
            (DEVICE, device(&schema, 1, 0, 0)),
            (DEVICE, device(&schema, 1, 0, 1)),
            (DEVICE, device(&schema, 1, 5, 2)),
        ],
    );
    result.expect("one weighted row lifts the floor; zero-weight rows ride along");
}

/// Declared statement order in [`dependent_bound_schema`].
const SUPPLY_CAPACITY: StatementId = StatementId(1);

/// `Pool(id) <=[watts]{0..supply} Device(pool)` — the dependent bound:
/// the ceiling is read from the TARGET row (by name against its full
/// roster, ruled 2026-07-24, C1; hi slot only, C6).
fn dependent_bound_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Pool".into(),
                fields: vec![field("id", ValueType::U64), field("supply", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Device".into(),
                fields: vec![
                    field("pool", ValueType::U64),
                    field("watts", ValueType::U64),
                    field("num", ValueType::U64),
                ],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: POOL,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Capacity {
                target: side(POOL, &[0]),
                weight: Weight::Field(FieldId(1)),
                lo: 0,
                hi: Some(Bound::TargetField(FieldId(1))),
                source: side(DEVICE, &[0]),
            },
        ],
    }
    .validate()
    .expect("the dependent-bound fixture seals")
}

/// The per-group ceiling: two pools, two supplies — each group is judged
/// against ITS parent's resolved bound.
#[test]
fn capacity_dependent_bound_resolves_per_parent() {
    let schema = dependent_bound_schema();
    let small = pool(&schema, 2, 50);
    // Pool 1 (supply 100) absorbs 90 watts; pool 2 (supply 50) convicts
    // at the same load.
    let result = base_then_delta(
        "cap-dep-bound-per-parent",
        &schema,
        &[
            (POOL, pool(&schema, 1, 100)),
            (POOL, small.clone()),
            (DEVICE, device(&schema, 1, 90, 0)),
        ],
        &[],
        &[(DEVICE, device(&schema, 2, 90, 1))],
    );
    assert_capacity_violation(result, SUPPLY_CAPACITY, &small, 90);
}

/// The bound resolves from the FINAL-state holder: replacing the parent
/// with a lower supply re-judges its (untouched-children) group through
/// the remove+add path — the delete side's key marks the group, the new
/// row's bound convicts it.
#[test]
fn capacity_dependent_bound_reads_the_final_state_holder() {
    let schema = dependent_bound_schema();
    let dir = TempDir::new("cap-dep-bound-final-state");
    let env = Environment::create(dir.path(), &schema).expect("create");
    apply_delta(
        &env,
        &schema,
        &[],
        &[
            (POOL, pool(&schema, 1, 100)),
            (DEVICE, device(&schema, 1, 90, 0)),
        ],
    )
    .expect("90 watts under supply 100");
    // The identity rewrite lowers the supply under the standing load.
    let lowered = pool(&schema, 1, 50);
    let before = committed_data(&env);
    let result = apply_delta(
        &env,
        &schema,
        &[(POOL, pool(&schema, 1, 100))],
        &[(POOL, lowered.clone())],
    );
    assert_capacity_violation(result, SUPPLY_CAPACITY, &lowered, 90);
    assert_eq!(committed_data(&env), before, "an abort persists nothing");
    // Raising the supply instead commits: same group, new bound.
    apply_delta(
        &env,
        &schema,
        &[(POOL, pool(&schema, 1, 100))],
        &[(POOL, pool(&schema, 1, 200))],
    )
    .expect("the raised bound admits the standing load");
}

// ---------- the Duration weight (the calendar shape) ----------

const ROOM: RelationId = RelationId(0);
const BOOKING: RelationId = RelationId(1);
const BOOKED_CAPACITY: StatementId = StatementId(1);

fn duration_schema(hi: Bound) -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Room".into(),
                fields: vec![field("id", ValueType::U64), field("span", interval())],
            },
            RelationDescriptor {
                extension: None,
                name: "Booking".into(),
                fields: vec![
                    field("room", ValueType::U64),
                    field("booked", interval()),
                    field("num", ValueType::U64),
                ],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: ROOM,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Capacity {
                target: side(ROOM, &[0]),
                weight: Weight::DurationOf(FieldId(1)),
                lo: 0,
                hi: Some(hi),
                source: side(BOOKING, &[0]),
            },
        ],
    }
    .validate()
    .expect("the Duration-weight fixture seals")
}

fn room(schema: &Schema, id: u64, span: (u64, u64)) -> Vec<u8> {
    fact(
        schema,
        ROOM,
        &[
            ValueRef::U64(id),
            ValueRef::IntervalU64(crate::Interval::<u64>::new(span.0, span.1).expect("nonempty")),
        ],
    )
}

fn booking(schema: &Schema, room: u64, booked: (u64, u64), num: u64) -> Vec<u8> {
    fact(
        schema,
        BOOKING,
        &[
            ValueRef::U64(room),
            ValueRef::IntervalU64(
                crate::Interval::<u64>::new(booked.0, booked.1).expect("nonempty"),
            ),
            ValueRef::U64(num),
        ],
    )
}

/// `Room(id) <=[Duration(booked)]{0..10} Booking(room)` — the interval
/// measure as weight, under a literal ceiling (C18 refuses only the
/// count-window-vs-Duration-BOUND direction): total booked time sums
/// end − start per booking.
#[test]
fn capacity_duration_weight_sums_the_measures() {
    let schema = duration_schema(Bound::Lit(10));
    let r = room(&schema, 1, (0, 24));
    let result = base_then_delta(
        "cap-duration-weight",
        &schema,
        &[
            (ROOM, r.clone()),
            (BOOKING, booking(&schema, 1, (0, 4), 0)),
            (BOOKING, booking(&schema, 1, (10, 14), 1)),
        ],
        &[],
        // 4 + 4 + 4 = 12 > 10: the calendar overbooks.
        &[(BOOKING, booking(&schema, 1, (20, 24), 2))],
    );
    assert_capacity_violation(result, BOOKED_CAPACITY, &r, 12);
}

/// `Room(id) <=[Duration(booked)]{0..Duration(span)} Booking(room)` —
/// calendar capacity whole: the ceiling is the TARGET row's own
/// interval measure (the dependent Duration bound).
#[test]
fn capacity_duration_bound_reads_the_target_span() {
    let schema = duration_schema(Bound::TargetDuration(FieldId(1)));
    // Room 1 spans [0, 8): 8 hours of capacity.
    let r = room(&schema, 1, (0, 8));
    let result = base_then_delta(
        "cap-duration-bound",
        &schema,
        &[
            (ROOM, r.clone()),
            (BOOKING, booking(&schema, 1, (0, 4), 0)),
            (BOOKING, booking(&schema, 1, (4, 8), 1)),
        ],
        &[],
        // 4 + 4 + 4 = 12 > Duration([0, 8)) = 8.
        &[(BOOKING, booking(&schema, 1, (8, 12), 2))],
    );
    assert_capacity_violation(result, BOOKED_CAPACITY, &r, 12);
}

/// A ray booking — `[s, ∞)` in the weighed field. Bypasses the
/// nonempty-interval helper deliberately: rays are legal interval VALUES
/// (the engine's own representation, end = MAX), it is their MEASURE
/// that is undefined.
fn ray_booking(schema: &Schema, room: u64, start: u64, num: u64) -> Vec<u8> {
    fact(
        schema,
        BOOKING,
        &[
            ValueRef::U64(room),
            ValueRef::IntervalU64(crate::Interval::<u64>::ray(start).expect("ray")),
            ValueRef::U64(num),
        ],
    )
}

/// C10: a ray-valued Duration WEIGHT met at measure time is the typed
/// commit refusal naming the row — never a violation (the law is not
/// judged false; its measure is undefined), never a silent `MAX`
/// (ruled 2026-07-24; the R6 `MeasureOfRay` precedent at the law site).
#[test]
fn capacity_duration_weight_of_a_ray_refuses_typed() {
    let schema = duration_schema(Bound::Lit(10));
    let b = ray_booking(&schema, 1, 5, 0);
    let result = base_then_delta(
        "cap-ray-weight",
        &schema,
        &[],
        &[],
        &[(ROOM, room(&schema, 1, (0, 24))), (BOOKING, b.clone())],
    );
    let err = result.unwrap_err();
    let Error::CapacityRayMeasure { statement, fact } = &err else {
        panic!("expected the typed ray refusal, got {err:?}");
    };
    assert_eq!(*statement, BOOKED_CAPACITY);
    assert_eq!(**fact, *b, "the refusal names the weighed row");
}

/// An INVERTED interval in the weighed field (`end < start`) —
/// unrepresentable through the value codec, so the raw bytes model
/// hostile stored data. The measure must convict typed corruption,
/// never underflow into a garbage weight.
#[test]
fn capacity_duration_weight_of_an_inverted_tail_is_corruption() {
    let schema = duration_schema(Bound::Lit(10));
    let mut inverted = booking(&schema, 1, (7, 9), 0);
    // Swap the interval's start/end words in the raw bytes: [7, 9) reads
    // back as start 9, end 7.
    let offset = schema.relation(BOOKING).layout().field_offset(1);
    let (a, b) = {
        let span = &inverted[offset..offset + 16];
        let mut a = [0u8; 8];
        let mut b = [0u8; 8];
        a.copy_from_slice(&span[..8]);
        b.copy_from_slice(&span[8..]);
        (a, b)
    };
    inverted[offset..offset + 8].copy_from_slice(&b);
    inverted[offset + 8..offset + 16].copy_from_slice(&a);
    let result = base_then_delta(
        "cap-inverted-weight",
        &schema,
        &[],
        &[],
        &[(ROOM, room(&schema, 1, (0, 24))), (BOOKING, inverted)],
    );
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            Error::Corruption(crate::error::CorruptionError::MalformedValue(
                "capacity interval inverted"
            ))
        ),
        "expected the inverted-tail corruption conviction, got {err:?}"
    );
}

/// C10, the BOUND direction: a parent whose dependent Duration bound is
/// a ray refuses any commit touching its group — the refusal names the
/// bound-carrying TARGET row.
#[test]
fn capacity_duration_bound_of_a_ray_refuses_typed() {
    let schema = duration_schema(Bound::TargetDuration(FieldId(1)));
    let r = fact(
        &schema,
        ROOM,
        &[
            ValueRef::U64(1),
            ValueRef::IntervalU64(crate::Interval::<u64>::ray(0).expect("ray")),
        ],
    );
    let result = base_then_delta(
        "cap-ray-bound",
        &schema,
        &[],
        &[],
        &[(ROOM, r.clone()), (BOOKING, booking(&schema, 1, (0, 4), 0))],
    );
    let err = result.unwrap_err();
    let Error::CapacityRayMeasure { statement, fact } = &err else {
        panic!("expected the typed ray refusal, got {err:?}");
    };
    assert_eq!(*statement, BOOKED_CAPACITY);
    assert_eq!(**fact, *r, "the refusal names the bound-carrying row");
}

// ---------- R16 interplay: fresh-keyed relations under a weighted law ----------

/// A fresh u64 field (the one id allocator's mint, R16).
fn fresh(name: &str) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
        generation: Generation::Fresh,
    }
}

/// `FreshPool(id fresh, supply)` / `FreshDevice(id fresh, pool, watts)`
/// with `FreshPool(id) <=[watts]{0..supply} FreshDevice(pool)` — both
/// sides fresh-keyed, so the fresh auto-keys are the target key AND the
/// child rows' `F` identities.
fn fresh_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "FreshPool".into(),
                fields: vec![fresh("id"), field("supply", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "FreshDevice".into(),
                fields: vec![
                    fresh("id"),
                    field("pool", ValueType::U64),
                    field("watts", ValueType::U64),
                ],
            },
        ],
        statements: vec![StatementDescriptor::Capacity {
            target: side(FRESH_POOL, &[0]),
            weight: Weight::Field(FieldId(2)),
            lo: 0,
            hi: Some(Bound::TargetField(FieldId(1))),
            source: side(FRESH_DEVICE, &[1]),
        }],
    }
    .validate()
    .expect("the fresh-keyed weighted fixture seals")
}

const FRESH_POOL: RelationId = RelationId(0);
const FRESH_DEVICE: RelationId = RelationId(1);

fn fresh_pool(schema: &Schema, id: u64, supply: u64) -> Vec<u8> {
    fact(
        schema,
        FRESH_POOL,
        &[ValueRef::U64(id), ValueRef::U64(supply)],
    )
}

fn fresh_device(schema: &Schema, id: u64, pool: u64, watts: u64) -> Vec<u8> {
    fact(
        schema,
        FRESH_DEVICE,
        &[ValueRef::U64(id), ValueRef::U64(pool), ValueRef::U64(watts)],
    )
}

/// The R16 interplay (`docs/architecture/50-storage.md` § key layout):
/// on a fresh-keyed relation the fresh field's value IS the `F` row id,
/// so the capacity edges minted in THIS commit name rows the same
/// commit's measure walk must resolve — the weight written at mint time
/// is seen by the same commit's walk (the value slot itself, the C17
/// slot law), and the parent
/// probe resolves the fresh-row holder through `F` directly (no `U`
/// tree), its dependent bound read from that same row.
#[test]
fn capacity_weight_on_fresh_keyed_relations_is_seen_by_the_same_commits_walk() {
    let schema = fresh_schema();
    let capacity = schema.capacities()[0].id;
    // Not `base_then_delta`: an aborted commit on a fresh-keyed relation
    // legitimately persists its escaped `Q` marks (the never-reissue
    // ratchet, `lean/Bumbledb/Txn/Fresh.lean: never_reissue_observable`),
    // so the byte-identity assert does not apply here — the abort
    // semantics themselves are the functionality estate's pins.
    let dir = TempDir::new("cap-fresh-r16");
    let env = Environment::create(dir.path(), &schema).expect("create");
    // Everything mints in ONE commit, over budget: supply 100 < 60 + 60.
    let p = fresh_pool(&schema, 7, 100);
    let result = apply_delta(
        &env,
        &schema,
        &[],
        &[
            (FRESH_POOL, p.clone()),
            (FRESH_DEVICE, fresh_device(&schema, 1, 7, 60)),
            (FRESH_DEVICE, fresh_device(&schema, 2, 7, 60)),
        ],
    );
    assert_capacity_violation(result, capacity, &p, 120);
    // Exactly at the dependent ceiling: commits whole (fresh ids past
    // the burned ones — escaped mints never reissue).
    apply_delta(
        &env,
        &schema,
        &[],
        &[
            (FRESH_POOL, fresh_pool(&schema, 7, 100)),
            (FRESH_DEVICE, fresh_device(&schema, 11, 7, 60)),
            (FRESH_DEVICE, fresh_device(&schema, 12, 7, 40)),
        ],
    )
    .expect("Σ watts = 100 sits on the fresh pool's supply ceiling");
}

// ---------- the phase laws ----------

/// Key violations preempt the statement phase: a delta violating both
/// the holder key and the savings law cites ONLY the key statement
/// (`lean/Bumbledb/Txn.lean: judge_key_preempts`).
#[test]
fn key_violation_preempts_the_capacity_judgment() {
    let schema = capacity_schema();
    let result = base_then_delta(
        "cap-preempt",
        &schema,
        &[],
        &[],
        &[
            // Two distinct facts, one key tuple — and no kind-1 children
            // anywhere, so the capacity law is violated too.
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

/// A mixed statement-phase rejection carries containment AND capacity
/// citations, complete, in materialized statement order — never a mix
/// with the key phase (`lean/Bumbledb/Txn.lean: rejection_is_complete`,
/// `rejection_never_mixes`).
#[test]
fn statement_phase_cites_containments_and_capacities_together() {
    let schema = capacity_schema();
    let h8 = holder(&schema, 8);
    let orphan = account(&schema, 9, 2, 0);
    let result = base_then_delta(
        "cap-mixed-phase",
        &schema,
        &[],
        &[],
        &[
            // Holder 8 lands childless (capacity floor) and account 9→
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
        Violation::Capacity {
            statement: w_stmt,
            fact: w_fact,
            measure,
        },
    ] = violations.as_slice()
    else {
        panic!("expected containment then capacity citations, got {violations:?}");
    };
    assert_eq!(
        (*c_stmt, *direction),
        (ACCOUNT_HOLDER, Direction::SourceUnsatisfied)
    );
    assert_eq!(**c_fact, *orphan);
    assert_eq!((*w_stmt, *measure), (SAVINGS_CAPACITY, 0));
    assert_eq!(**w_fact, *h8);
}
