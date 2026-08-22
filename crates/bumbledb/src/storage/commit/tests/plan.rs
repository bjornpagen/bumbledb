use crate::encoding::{ValueRef, encode_interval_u64, encode_u64};
use crate::schema::ValidateDescriptor as _;
use crate::schema::{ContainmentId, Enforcement, KeyId, Schema};
use crate::storage::commit::plan::{CommitPlan, DeleteOp, DeterminantOp, InsertOp, Owed, RKeyOp};
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

#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] 
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

fn commit_base(env: &Environment, schema: &Schema, facts: &[(RelationId, Vec<u8>)]) {
    apply_delta(env, schema, &[], facts)
        .expect("base commit")
        .expect("admitted");
}

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

fn delete_for<'a, 'd>(ops: &'a [DeleteOp<'d>], rel: RelationId, fact: &[u8]) -> &'a DeleteOp<'d> {
    ops.iter()
        .find(|op| op.relation() == rel && op.fact() == fact)
        .expect("an op exists for every net disposition")
}

fn insert_for<'a, 'd>(ops: &'a [InsertOp<'d>], rel: RelationId, fact: &[u8]) -> &'a InsertOp<'d> {
    ops.iter()
        .find(|op| op.relation() == rel && op.fact() == fact)
        .expect("an op exists for every net disposition")
}

fn assert_determinant(
    op: &DeterminantOp,
    statement: StatementId,
    determinant: &[u8],
    pointwise: bool,
) {
    assert_eq!(op.statement(), statement);
    assert_eq!(
        op.determinant().as_bytes(),
        determinant,
        "determinant bytes"
    );
    assert_eq!(
        matches!(op, DeterminantOp::Pointwise { .. }),
        pointwise,
        "pointwise marker"
    );
}

fn assert_edge<W>(schema: &Schema, edge: &RKeyOp<W>, statement: StatementId, key_bytes: &[u8]) {
    let containment = edge.containment().expect("containment R key");
    assert_eq!(schema.containment(containment).id, statement);
    assert_eq!(containment, containment_id(statement));
    assert_eq!(&*edge.key_bytes, key_bytes, "permuted key bytes");
}

fn only_containment<'a, W>(mut keys: impl Iterator<Item = &'a RKeyOp<W>>) -> &'a RKeyOp<W> {
    let edge = keys.next().expect("one containment");
    assert!(keys.next().is_none(), "exactly one containment");
    edge
}

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
    let account_op = insert_for(&plan.inserts, ACCOUNT, &a);
    assert_eq!(account_op.relation(), ACCOUNT);
    let [determinant] = account_op.determinants() else {
        panic!("one key statement");
    };
    assert_determinant(determinant, ACCOUNT_KEY, &encode_u64(7), false);
    assert!(
        account_op.containment_r_keys().next().is_none(),
        "Account has no outgoing"
    );

    let room_op = insert_for(&plan.inserts, ROOM, &r);
    let mut room_determinant = Vec::new();
    room_determinant.extend_from_slice(&encode_u64(3));
    room_determinant.extend_from_slice(&encode_interval_u64(
        bumbledb_theory::Interval::<u64>::new(10, 20).expect("nonempty interval"),
    ));
    let [determinant] = room_op.determinants() else {
        panic!("one key statement");
    };
    assert_determinant(determinant, ROOM_KEY, &room_determinant, true);
}

#[test]
fn fact_ops_carry_the_delta_computed_hash() {

    let dir = TempDir::new("plan-fact-hash");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let a = account(&schema, 7, true, 0);
    let b = account(&schema, 8, true, 0);
    commit_base(&env, &schema, &[(ACCOUNT, a.clone())]);
    let mut delta = WriteDelta::new(&schema);
    let plan = plan_of(
        &env,
        &mut delta,
        &[(ACCOUNT, a.clone())],
        &[(ACCOUNT, b.clone())],
    );
    let deleted = delete_for(&plan.deletes, ACCOUNT, &a);
    assert_eq!(deleted.fact_hash(), &crate::encoding::fact_hash(&a));
    let inserted = insert_for(&plan.inserts, ACCOUNT, &b);
    assert_eq!(inserted.fact_hash(), &crate::encoding::fact_hash(&b));
}

#[test]
fn plan_ops_land_in_relation_then_hash_order() {

    let dir = TempDir::new("plan-op-order");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let base: Vec<(RelationId, Vec<u8>)> = (0..24)
        .map(|i| (ACCOUNT, account(&schema, i, false, 0)))
        .chain((0..24).map(|i| (GRANT, u64_fact(&schema, GRANT, i))))
        .collect();
    commit_base(&env, &schema, &base);
    let mut delta = WriteDelta::new(&schema);
    let plan = plan_of(&env, &mut delta, &base, &[]);
    let order: Vec<(RelationId, [u8; 32])> = plan
        .deletes
        .iter()
        .map(|op| (op.relation(), *op.fact_hash()))
        .collect();
    let mut sorted = order.clone();
    sorted.sort_unstable();
    assert_eq!(order, sorted, "deletes in (relation, fact_hash) order");
    assert_eq!(order.len(), 48);
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

    let edge = only_containment(insert_for(&plan.inserts, REPORT, &urgent).containment_r_keys());
    assert_edge(&schema, edge, REPORT_ACCOUNT, &encode_u64(5));

    assert!(
        insert_for(&plan.inserts, REPORT, &calm)
            .containment_r_keys()
            .next()
            .is_none()
    );
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

    let edge = only_containment(insert_for(&plan.inserts, PARENT, &p).containment_r_keys());
    assert_edge(&schema, edge, TOTALITY, &encode_u64(4));
    let edge = only_containment(insert_for(&plan.inserts, CHILD, &c).containment_r_keys());
    assert_edge(&schema, edge, ARM, &encode_u64(4));
}

#[test]
fn edge_key_bytes_land_in_target_key_order() {

    let dir = TempDir::new("plan-permutation");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let l = link(&schema, 1, 2);
    let mut delta = WriteDelta::new(&schema);
    let plan = plan_of(&env, &mut delta, &[], &[(LINK, l.clone())]);

    let mut expected = Vec::new();
    expected.extend_from_slice(&encode_u64(2)); 
    expected.extend_from_slice(&encode_u64(1)); 
    let edge = only_containment(insert_for(&plan.inserts, LINK, &l).containment_r_keys());
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
    let edge = only_containment(insert_for(&plan.inserts, STAY, &s).containment_r_keys());
    assert_edge(&schema, edge, STAY_ROOM, &expected);
    assert!(matches!(
        schema
            .containment(edge.containment().expect("containment"))
            .enforcement,
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
    let op = delete_for(&plan.deletes, REPORT, &r);
    assert!(op.determinants().is_empty(), "Report has no key statements");
    let edge = only_containment(op.containment_r_keys());
    assert_edge(&schema, edge, REPORT_ACCOUNT, &encode_u64(5));

    assert!(plan.target_checks.is_empty());
}

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

    let statements: Vec<_> = check
        .dependents
        .iter()
        .map(|d| {
            (
                schema.containment(d.containment).id,
                matches!(d.owed, Owed::IfEstablisherFails),
            )
        })
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
    assert!(matches!(dependent.owed, Owed::IfEstablisherFails));
}

#[test]
fn inserts_fact_is_exact_over_the_byte_sorted_index() {
    let dir = TempDir::new("plan-inserts-fact");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");

    let parents: Vec<Vec<u8>> = [9u64, 5, 1]
        .iter()
        .map(|&v| u64_fact(&schema, PARENT, v))
        .collect();

    let children: Vec<Vec<u8>> = [9u64, 5]
        .iter()
        .map(|&v| u64_fact(&schema, CHILD, v))
        .collect();
    let combos: Vec<Vec<u8>> = [(9u64, 5u64), (5, 9)]
        .iter()
        .map(|&(x, y)| fact(&schema, COMBO, &[ValueRef::U64(x), ValueRef::U64(y)]))
        .collect();
    let mut inserts: Vec<(RelationId, Vec<u8>)> = Vec::new();
    inserts.extend(parents.iter().map(|f| (PARENT, f.clone())));
    inserts.extend(children.iter().map(|f| (CHILD, f.clone())));
    inserts.extend(combos.iter().map(|f| (COMBO, f.clone())));
    let mut delta = WriteDelta::new(&schema);
    let plan = plan_of(&env, &mut delta, &[], &inserts);

    for (rel, fact_bytes) in &inserts {
        assert!(
            plan.inserts_fact(*rel, fact_bytes),
            "inserted fact must be found: rel {rel:?}"
        );
    }

    assert!(plan.inserts_fact(PARENT, &u64_fact(&schema, PARENT, 1)));
    assert!(!plan.inserts_fact(CHILD, &u64_fact(&schema, PARENT, 1)));

    let full = u64_fact(&schema, PARENT, 9);
    assert!(!plan.inserts_fact(PARENT, &full[..7]));
    let mut extended = full.clone();
    extended.push(0);
    assert!(!plan.inserts_fact(PARENT, &extended));

    assert!(!plan.inserts_fact(PARENT, &u64_fact(&schema, PARENT, 7)));
    assert!(!plan.inserts_fact(COMBO, &u64_fact(&schema, PARENT, 7)));

    let base_parent = u64_fact(&schema, PARENT, 42);
    let base_child = u64_fact(&schema, CHILD, 42);
    commit_base(
        &env,
        &schema,
        &[(PARENT, base_parent.clone()), (CHILD, base_child.clone())],
    );
    let mut delta2 = WriteDelta::new(&schema);
    let plan2 = plan_of(
        &env,
        &mut delta2,
        &[(PARENT, base_parent.clone()), (CHILD, base_child)],
        &[],
    );
    assert!(!plan2.inserts_fact(PARENT, &base_parent));
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
    assert!(matches!(dependent.owed, Owed::Unconditional));
}

#[test]
fn empty_delta_incremental_roster_is_empty_complete_roster_is_not() {
    let schema = schema();
    assert!(
        schema.complete_obligations().iter().next().is_some(),
        "complete roster is the spine, not the empty incremental plan"
    );

    let dir = TempDir::new("empty-incremental");
    let env = Environment::create(dir.path(), &schema).expect("create");
    let delta = WriteDelta::new(&schema);
    let plan = plan_for(&delta, &env);
    assert!(
        plan.incremental_obligations().is_empty(),
        "empty delta enumerates no incremental obligations — not complete admission"
    );
}
