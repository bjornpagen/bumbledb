//! The plan derivation (PRD: `CommitPlan` — compute, don't accumulate):
//! delta in, plan out, byte-level assertions on determinant bytes, reverse-edge
//! key bytes, probe markers, and the target-side check sets — the class
//! of test the accumulate-during-apply shape could never have. Covers
//! scalar keys, pointwise keys, satisfied and unsatisfied selections, the
//! `==` pair, and the delete+insert re-establishment shapes.
//!
//! Own fixture: a ψ-carrying and two empty-ψ dependents on one scalar
//! key, a pointwise key with a coverage dependent, a σ-gated source, a
//! `==` pair, and a containment whose target projection permutes the
//! target key's order.

use crate::encoding::{ValueRef, encode_interval_u64, encode_u64};
use crate::schema::ValidateDescriptor as _;
use crate::schema::{ContainmentId, Enforcement, KeyId, Schema};
use crate::storage::commit::plan::{CommitPlan, DeterminantOp, EdgeOp, FactOp};
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::testutil::TempDir;
use bumbledb_theory::Value;
use bumbledb_theory::schema::{
    FieldId, RelationDescriptor, RelationId, SchemaDescriptor, StatementDescriptor, StatementId,
    ValueType,
};

use super::{apply_delta, fact, field, interval, plan_for, selected, side};

const ACCOUNT: RelationId = RelationId(0);
const TRANSFER: RelationId = RelationId(1);
const GRANT: RelationId = RelationId(2);
const ROOM: RelationId = RelationId(3);
const STAY: RelationId = RelationId(4);
const REPORT: RelationId = RelationId(5);
const PARENT: RelationId = RelationId(6);
const CHILD: RelationId = RelationId(7);
const COMBO: RelationId = RelationId(8);
const LINK: RelationId = RelationId(9);

/// Declared statement order (no fresh fields, so no auto-keys).
const ACCOUNT_KEY: StatementId = StatementId(0);
const ROOM_KEY: StatementId = StatementId(1);
const TRANSFER_ACCOUNT: StatementId = StatementId(5);
const GRANT_ACCOUNT: StatementId = StatementId(6);
const REPORT_ACCOUNT: StatementId = StatementId(7);
const STAY_ROOM: StatementId = StatementId(8);
const TOTALITY: StatementId = StatementId(9);
const ARM: StatementId = StatementId(10);
const LINK_COMBO: StatementId = StatementId(11);

const fn key_id(statement: StatementId) -> KeyId {
    KeyId(statement.0)
}

const fn containment_id(statement: StatementId) -> ContainmentId {
    ContainmentId(statement.0 - 5)
}

/// Account(id, active, note; key id) with three dependents — Transfer <=
/// Account(id | active == true) (ψ-carrying), Grant <= Account(id) and
/// Report(subject | urgent == true) <= Account(id) (empty ψ; Report's σ
/// also gates the source side). Room(room, during, tag; key (room,
/// during)) with the coverage dependent Stay <= Room. Parent == Child
/// lowered to [`TOTALITY`] and [`ARM`]. Link(p, q) <= Combo(y, x) against
/// key Combo(x, y): a non-identity key permutation.
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // one fixture schema, a table
fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Account".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field("active", ValueType::Bool),
                    field("note", ValueType::U64),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Transfer".into(),
                fields: vec![field("account", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Grant".into(),
                fields: vec![field("account", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Room".into(),
                fields: vec![
                    field("room", ValueType::U64),
                    field("during", interval()),
                    field("tag", ValueType::U64),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Stay".into(),
                fields: vec![field("room", ValueType::U64), field("during", interval())],
            },
            RelationDescriptor {
                extension: None,
                name: "Report".into(),
                fields: vec![
                    field("subject", ValueType::U64),
                    field("urgent", ValueType::Bool),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Parent".into(),
                fields: vec![field("id", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Child".into(),
                fields: vec![field("parent", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Combo".into(),
                fields: vec![field("x", ValueType::U64), field("y", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Link".into(),
                fields: vec![field("p", ValueType::U64), field("q", ValueType::U64)],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: ACCOUNT,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Functionality {
                relation: ROOM,
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
            StatementDescriptor::Functionality {
                relation: COMBO,
                projection: Box::new([FieldId(0), FieldId(1)]),
            },
            StatementDescriptor::Containment {
                source: side(TRANSFER, &[0]),
                target: selected(ACCOUNT, &[0], &[(1, Value::Bool(true))]),
            },
            StatementDescriptor::Containment {
                source: side(GRANT, &[0]),
                target: side(ACCOUNT, &[0]),
            },
            StatementDescriptor::Containment {
                source: selected(REPORT, &[0], &[(1, Value::Bool(true))]),
                target: side(ACCOUNT, &[0]),
            },
            StatementDescriptor::Containment {
                source: side(STAY, &[0, 1]),
                target: side(ROOM, &[0, 1]),
            },
            StatementDescriptor::Containment {
                source: side(PARENT, &[0]),
                target: side(CHILD, &[0]),
            },
            StatementDescriptor::Containment {
                source: side(CHILD, &[0]),
                target: side(PARENT, &[0]),
            },
            StatementDescriptor::Containment {
                source: side(LINK, &[0, 1]),
                target: side(COMBO, &[1, 0]),
            },
        ],
    }
    .validate()
    .expect("valid fixture")
}

fn account(schema: &Schema, id: u64, active: bool, note: u64) -> Vec<u8> {
    fact(
        schema,
        ACCOUNT,
        &[
            ValueRef::U64(id),
            ValueRef::Bool(active),
            ValueRef::U64(note),
        ],
    )
}

fn room(schema: &Schema, room: u64, start: u64, end: u64, tag: u64) -> Vec<u8> {
    fact(
        schema,
        ROOM,
        &[
            ValueRef::U64(room),
            ValueRef::IntervalU64(
                bumbledb_theory::Interval::<u64>::new(start, end).expect("nonempty interval"),
            ),
            ValueRef::U64(tag),
        ],
    )
}

fn stay(schema: &Schema, room: u64, start: u64, end: u64) -> Vec<u8> {
    fact(
        schema,
        STAY,
        &[
            ValueRef::U64(room),
            ValueRef::IntervalU64(
                bumbledb_theory::Interval::<u64>::new(start, end).expect("nonempty interval"),
            ),
        ],
    )
}

fn report(schema: &Schema, subject: u64, urgent: bool) -> Vec<u8> {
    fact(
        schema,
        REPORT,
        &[ValueRef::U64(subject), ValueRef::Bool(urgent)],
    )
}

fn u64_fact(schema: &Schema, rel: RelationId, v: u64) -> Vec<u8> {
    fact(schema, rel, &[ValueRef::U64(v)])
}

fn link(schema: &Schema, p: u64, q: u64) -> Vec<u8> {
    fact(schema, LINK, &[ValueRef::U64(p), ValueRef::U64(q)])
}

/// Commits `facts` as one base delta.
fn commit_base(env: &Environment, schema: &Schema, facts: &[(RelationId, Vec<u8>)]) {
    apply_delta(env, schema, &[], facts).expect("base commit");
}

/// Records `deletes` then `inserts` into one delta and derives its plan,
/// handing both back (the plan borrows the delta's arena).
fn plan_of<'d>(
    env: &Environment,
    delta: &'d mut WriteDelta<'_>,
    deletes: &[(RelationId, Vec<u8>)],
    inserts: &[(RelationId, Vec<u8>)],
) -> CommitPlan<'d> {
    let view = env.read_txn().expect("txn");
    for (rel, fact) in deletes {
        delta.delete(&view, *rel, fact).expect("record delete");
    }
    for (rel, fact) in inserts {
        delta.insert(&view, *rel, fact).expect("record insert");
    }
    drop(view);
    plan_for(delta, env)
}

/// The op of one fact, found by relation and canonical bytes (op order is
/// the delta's `(relation, fact_hash)` order — hash order is not
/// meaningful to assert against, and facts of different relations may
/// share bytes).
fn op_for<'a, 'd>(ops: &'a [FactOp<'d>], rel: RelationId, fact: &[u8]) -> &'a FactOp<'d> {
    ops.iter()
        .find(|op| op.relation == rel && op.fact == fact)
        .expect("an op exists for every net disposition")
}

fn assert_determinant(
    op: &DeterminantOp,
    statement: StatementId,
    determinant: &[u8],
    pointwise: bool,
) {
    assert_eq!(op.statement, statement);
    assert_eq!(&*op.determinant, determinant, "determinant bytes");
    assert_eq!(op.pointwise.is_some(), pointwise, "pointwise marker");
}

fn assert_edge(schema: &Schema, edge: &EdgeOp, statement: StatementId, key_bytes: &[u8]) {
    assert_eq!(edge.statement, statement);
    assert_eq!(edge.containment, containment_id(statement));
    assert_eq!(&*edge.key_bytes, key_bytes, "permuted key bytes");
    assert_eq!(schema.containment(edge.containment).id, statement);
}

// ---------- per-fact ops: determinants and edges ----------

#[test]
fn scalar_and_pointwise_determinants_carry_exact_bytes() {
    let dir = TempDir::new("plan-determinants");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let a = account(&schema, 7, true, 0);
    let r = room(&schema, 3, 10, 20, 1);
    let mut delta = WriteDelta::new(&schema);
    let plan = plan_of(
        &env,
        &mut delta,
        &[],
        &[(ACCOUNT, a.clone()), (ROOM, r.clone())],
    );

    assert!(plan.deletes.is_empty());
    assert_eq!(plan.inserts.len(), 2);
    let account_op = op_for(&plan.inserts, ACCOUNT, &a);
    assert_eq!(account_op.relation, ACCOUNT);
    let [determinant] = &*account_op.determinants else {
        panic!("one key statement");
    };
    assert_determinant(determinant, ACCOUNT_KEY, &encode_u64(7), false);
    assert!(account_op.edges.is_empty(), "Account has no outgoing");

    // The pointwise determinant: scalar prefix ‖ the interval's whole 16 bytes,
    // marked for the ordered-neighbor probe.
    let room_op = op_for(&plan.inserts, ROOM, &r);
    let mut room_determinant = Vec::new();
    room_determinant.extend_from_slice(&encode_u64(3));
    room_determinant.extend_from_slice(&encode_interval_u64(
        bumbledb_theory::Interval::<u64>::new(10, 20).expect("nonempty interval"),
    ));
    let [determinant] = &*room_op.determinants else {
        panic!("one key statement");
    };
    assert_determinant(determinant, ROOM_KEY, &room_determinant, true);
}

#[test]
fn source_selection_gates_the_edges() {
    let dir = TempDir::new("plan-sigma");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let urgent = report(&schema, 5, true);
    let calm = report(&schema, 6, false);
    let mut delta = WriteDelta::new(&schema);
    let plan = plan_of(
        &env,
        &mut delta,
        &[],
        &[(REPORT, urgent.clone()), (REPORT, calm.clone())],
    );

    // Inside σ: one edge, projection bytes in target key order.
    let [edge] = &*op_for(&plan.inserts, REPORT, &urgent).edges else {
        panic!("one satisfied containment");
    };
    assert_edge(&schema, edge, REPORT_ACCOUNT, &encode_u64(5));
    // Outside σ: no edge, so no R put and no source probe — by absence.
    assert!(op_for(&plan.inserts, REPORT, &calm).edges.is_empty());
}

#[test]
fn pair_statements_edge_their_own_directions() {
    let dir = TempDir::new("plan-pair");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let p = u64_fact(&schema, PARENT, 4);
    let c = u64_fact(&schema, CHILD, 4);
    let mut delta = WriteDelta::new(&schema);
    let plan = plan_of(
        &env,
        &mut delta,
        &[],
        &[(PARENT, p.clone()), (CHILD, c.clone())],
    );

    // The == pair is two statements; each side owes exactly its own probe.
    let [edge] = &*op_for(&plan.inserts, PARENT, &p).edges else {
        panic!("one outgoing statement");
    };
    assert_edge(&schema, edge, TOTALITY, &encode_u64(4));
    let [edge] = &*op_for(&plan.inserts, CHILD, &c).edges else {
        panic!("one outgoing statement");
    };
    assert_edge(&schema, edge, ARM, &encode_u64(4));
}

#[test]
fn edge_key_bytes_land_in_target_key_order() {
    // Link(p, q) <= Combo(y, x) against key Combo(x, y): projection
    // element p maps to determinant position 1, q to 0 — the plan's key bytes
    // are pre-permuted, byte-for-byte.
    let dir = TempDir::new("plan-permutation");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let l = link(&schema, 1, 2);
    let mut delta = WriteDelta::new(&schema);
    let plan = plan_of(&env, &mut delta, &[], &[(LINK, l.clone())]);

    let mut expected = Vec::new();
    expected.extend_from_slice(&encode_u64(2)); // q -> Combo.x
    expected.extend_from_slice(&encode_u64(1)); // p -> Combo.y
    let [edge] = &*op_for(&plan.inserts, LINK, &l).edges else {
        panic!("one outgoing statement");
    };
    assert_edge(&schema, edge, LINK_COMBO, &expected);
}

#[test]
fn interval_edges_are_marked_for_the_coverage_walk() {
    let dir = TempDir::new("plan-coverage-edge");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let s = stay(&schema, 3, 12, 15);
    let mut delta = WriteDelta::new(&schema);
    let plan = plan_of(&env, &mut delta, &[], &[(STAY, s.clone())]);

    let mut expected = Vec::new();
    expected.extend_from_slice(&encode_u64(3));
    expected.extend_from_slice(&encode_interval_u64(
        bumbledb_theory::Interval::<u64>::new(12, 15).expect("nonempty interval"),
    ));
    let [edge] = &*op_for(&plan.inserts, STAY, &s).edges else {
        panic!("one outgoing statement");
    };
    assert_edge(&schema, edge, STAY_ROOM, &expected);
    assert!(matches!(
        schema.containment(edge.containment).enforcement,
        Enforcement::IntervalCoverage { .. }
    ));
}

#[test]
fn delete_ops_carry_the_byte_symmetric_edges() {
    let dir = TempDir::new("plan-delete-edges");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let a = account(&schema, 5, true, 0);
    let r = report(&schema, 5, true);
    commit_base(&env, &schema, &[(ACCOUNT, a), (REPORT, r.clone())]);

    let mut delta = WriteDelta::new(&schema);
    let plan = plan_of(&env, &mut delta, &[(REPORT, r.clone())], &[]);
    assert!(plan.inserts.is_empty());
    let op = op_for(&plan.deletes, REPORT, &r);
    assert!(op.determinants.is_empty(), "Report has no key statements");
    let [edge] = &*op.edges else {
        panic!("one satisfied containment");
    };
    assert_edge(&schema, edge, REPORT_ACCOUNT, &encode_u64(5));
    // Report has no keys, so nothing was disestablished.
    assert!(plan.target_checks.is_empty());
}

// ---------- the target-side check sets ----------

#[test]
fn disestablished_tuple_expands_per_dependent_statement() {
    let dir = TempDir::new("plan-check-set");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let a = account(&schema, 9, true, 0);
    commit_base(&env, &schema, &[(ACCOUNT, a.clone())]);

    let mut delta = WriteDelta::new(&schema);
    let plan = plan_of(&env, &mut delta, &[(ACCOUNT, a)], &[]);
    let [check] = &*plan.target_checks else {
        panic!("one disestablished tuple");
    };
    assert_eq!(check.key, key_id(ACCOUNT_KEY));
    assert_eq!(schema.key(check.key).relation, ACCOUNT);
    assert_eq!(&*check.determinant, encode_u64(9).as_slice());
    // Not re-established: every dependent checks unconditionally, in
    // materialized order.
    let statements: Vec<_> = check
        .dependents
        .iter()
        .map(|d| (schema.containment(d.containment).id, d.psi_qualified))
        .collect();
    assert_eq!(
        statements,
        [
            (TRANSFER_ACCOUNT, false),
            (GRANT_ACCOUNT, false),
            (REPORT_ACCOUNT, false),
        ]
    );
}

#[test]
fn reestablishment_drops_empty_psi_and_marks_psi_carrying_dependents() {
    let dir = TempDir::new("plan-reestablish");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let old = account(&schema, 9, true, 0);
    commit_base(&env, &schema, &[(ACCOUNT, old.clone())]);

    // Delete + insert re-lands the exact determinant bytes (only the non-key
    // `note` differs): the plain set difference discharges the empty-ψ
    // dependents at plan time; the ψ-carrying dependent stays, marked —
    // only the judgment phase can read the establishing fact.
    let new = account(&schema, 9, true, 1);
    let mut delta = WriteDelta::new(&schema);
    let plan = plan_of(&env, &mut delta, &[(ACCOUNT, old)], &[(ACCOUNT, new)]);
    let [check] = &*plan.target_checks else {
        panic!("one disestablished tuple");
    };
    assert_eq!(check.key, key_id(ACCOUNT_KEY));
    assert_eq!(&*check.determinant, encode_u64(9).as_slice());
    let [dependent] = &*check.dependents else {
        panic!("only the ψ-carrying dependent survives");
    };
    assert_eq!(
        schema.containment(dependent.containment).id,
        TRANSFER_ACCOUNT
    );
    assert!(dependent.psi_qualified);
}

#[test]
fn pointwise_tuple_keeps_its_interval_tail_and_coverage_evidence() {
    let dir = TempDir::new("plan-check-interval");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let r = room(&schema, 3, 10, 20, 1);
    commit_base(&env, &schema, &[(ROOM, r.clone())]);

    let mut delta = WriteDelta::new(&schema);
    let plan = plan_of(&env, &mut delta, &[(ROOM, r)], &[]);
    let [check] = &*plan.target_checks else {
        panic!("one disestablished tuple");
    };
    assert_eq!(check.key, key_id(ROOM_KEY));
    assert_eq!(schema.key(check.key).relation, ROOM);
    let mut determinant = Vec::new();
    determinant.extend_from_slice(&encode_u64(3));
    determinant.extend_from_slice(&encode_interval_u64(
        bumbledb_theory::Interval::<u64>::new(10, 20).expect("nonempty interval"),
    ));
    assert_eq!(&*check.determinant, determinant.as_slice());
    let [dependent] = &*check.dependents else {
        panic!("one dependent");
    };
    assert_eq!(schema.containment(dependent.containment).id, STAY_ROOM);
    assert!(matches!(
        schema.containment(dependent.containment).enforcement,
        Enforcement::IntervalCoverage { .. }
    ));
    assert!(!dependent.psi_qualified);
}
