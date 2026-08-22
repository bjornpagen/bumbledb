use super::InstanceBuilder;
use crate::api::db::{Fact, Fresh, OwnedInstance};
use crate::error::{Admission, Conflict, Direction, Error, Violation};
use crate::ir::Value;
use crate::schema::tests::{capacity, closed, containment, fd, field, row, side};
use crate::schema::{SchemaDescriptor, ValidateDescriptor as _};
use crate::storage::catalog::CatalogRead;
use bumbledb_theory::schema::{FieldId, RelationId, StatementId, ValueType};

crate::schema! {
    pub Ledger;

    relation Account {
        id: u64 as AccountId, fresh,
        holder: str,
        balance: i64,
    }
}

crate::schema! {
    pub Named;
    relation Label { name: str }
}

crate::schema! {
    pub WithClosed;
    closed relation Kind as KindId = { Checking };
    relation Item { id: u64 as ItemId, fresh }
}

#[test]
fn builder_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<InstanceBuilder<Ledger>>();
}

#[test]
fn empty_load_is_a_noop() {
    let mut builder = InstanceBuilder::new(Ledger).expect("valid");
    let report = builder.load::<Account>([]).expect("empty");
    assert_eq!(report.submitted(), 0);
    assert_eq!(report.changed(), 0);
}

#[test]
fn load_then_overlay_contains_and_get() {
    let mut builder = InstanceBuilder::new(Ledger).expect("valid");
    let id = builder
        .reserve::<AccountId>(1)
        .expect("reserve")
        .start()
        .expect("nonempty");
    let acct = Account {
        id,
        holder: "ada",
        balance: 10,
    };
    assert_eq!(builder.load([&acct]).expect("load").changed(), 1);
    assert!(builder.contains(&acct).expect("contains"));
    assert_eq!(builder.get(id).expect("get"), Some(acct));
}

#[test]
fn delete_cancels_a_loaded_fact() {
    let mut builder = InstanceBuilder::new(Ledger).expect("valid");
    let id = builder
        .reserve::<AccountId>(1)
        .expect("reserve")
        .start()
        .expect("nonempty");
    let acct = Account {
        id,
        holder: "ada",
        balance: 10,
    };
    builder.load([&acct]).expect("load");
    assert_eq!(builder.delete([&acct]).expect("delete").changed(), 1);
    assert!(!builder.contains(&acct).expect("contains"));
    assert_eq!(builder.get(id).expect("get"), None);
}

#[test]
fn overlay_get_is_last_wins() {
    let mut builder = InstanceBuilder::new(Ledger).expect("valid");
    let id = builder
        .reserve::<AccountId>(1)
        .expect("reserve")
        .start()
        .expect("nonempty");
    let first = Account {
        id,
        holder: "ada",
        balance: 10,
    };
    let second = Account {
        id,
        holder: "ada",
        balance: 42,
    };
    builder.load([&first, &second]).expect("load");
    assert_eq!(builder.get(id).expect("get"), Some(second));
    assert!(builder.contains(&second).expect("contains second"));
    assert!(
        builder
            .contains(&first)
            .expect("first remains a live distinct fact"),
        "last-wins is the keyed overlay, not identity contains"
    );
}

#[test]
fn redundant_load_does_not_change() {
    let mut builder = InstanceBuilder::new(Ledger).expect("valid");
    let id = builder
        .reserve::<AccountId>(1)
        .expect("reserve")
        .start()
        .expect("nonempty");
    let acct = Account {
        id,
        holder: "ada",
        balance: 10,
    };
    assert_eq!(builder.load([&acct]).expect("load").changed(), 1);
    assert_eq!(builder.load([&acct]).expect("again").changed(), 0);
    assert!(builder.contains(&acct).expect("contains"));
}

#[test]
fn reserve_zero_is_empty() {
    let mut builder = InstanceBuilder::new(Ledger).expect("valid");
    let range = builder.reserve::<AccountId>(0).expect("zero");
    assert!(range.is_empty());
    assert!(range.start().is_none());
}

#[test]
fn reserve_at_uses_the_fresh_field_witness() {
    let mut builder = InstanceBuilder::new(Ledger).expect("valid");
    let field = builder
        .fresh_field(AccountId::RELATION, AccountId::FIELD)
        .expect("fresh");
    let range = builder.reserve_at(field, 2).expect("reserve_at");
    assert_eq!(range.len(), 2);
    assert_eq!(range.start(), Some(0));
}

#[test]
fn never_interned_delete_does_not_grow_the_dictionary() {
    let mut builder = InstanceBuilder::new(Named).expect("valid");
    let ghost = [Value::String("ghost".into())];
    assert_eq!(
        builder
            .delete_dyn(Label::RELATION, [&ghost])
            .expect("delete")
            .changed(),
        0
    );
    assert_eq!(builder.intern_count(), 0);
    let real = [Value::String("real".into())];
    assert_eq!(
        builder
            .load_dyn(Label::RELATION, [&real])
            .expect("load")
            .changed(),
        1
    );
    assert_eq!(builder.intern_count(), 1);
    assert!(
        builder
            .contains_dyn(Label::RELATION, &real)
            .expect("contains")
    );
}

#[test]
fn dyn_parse_all_first_does_not_stage_a_prefix() {
    let mut builder = InstanceBuilder::new(Named).expect("valid");
    let ok = vec![Value::String("ok".into())];
    let bad = Vec::<Value>::new();
    let err = builder
        .load_dyn(Label::RELATION, [ok.clone(), bad])
        .expect_err("shape");
    assert!(matches!(err, Error::FactShape(_)), "{err:?}");
    assert!(
        !builder
            .contains_dyn(Label::RELATION, &ok)
            .expect("prefix absent")
    );
}

#[test]
fn typed_get_dyn_through_the_fresh_key() {
    let mut builder = InstanceBuilder::new(Ledger).expect("valid");
    let id = builder
        .reserve::<AccountId>(1)
        .expect("reserve")
        .start()
        .expect("nonempty");
    let acct = Account {
        id,
        holder: "ada",
        balance: 10,
    };
    builder.load([&acct]).expect("load");
    let got = builder
        .get_dyn(Account::RELATION, StatementId(0), &[Value::U64(id.fresh())])
        .expect("get_dyn")
        .expect("hit");
    assert_eq!(got[0], Value::U64(id.fresh()));
    assert_eq!(got[1], Value::String("ada".into()));
    assert_eq!(got[2], Value::I64(10));
}

#[test]
fn closed_relation_load_is_refused() {
    let mut builder = InstanceBuilder::new(WithClosed).expect("valid");
    let err = builder
        .load_dyn(WithClosed::KIND, [&[Value::U64(0)]])
        .expect_err("closed");
    assert!(matches!(err, Error::ClosedRelationWrite { .. }), "{err:?}");
}

#[test]
fn overlay_get_last_wins_after_deleting_an_earlier_same_key_fact() {
    let mut builder = InstanceBuilder::new(Ledger).expect("valid");
    let id = builder
        .reserve::<AccountId>(1)
        .expect("reserve")
        .start()
        .expect("nonempty");
    let first = Account {
        id,
        holder: "ada",
        balance: 1,
    };
    let second = Account {
        id,
        holder: "ada",
        balance: 2,
    };
    let third = Account {
        id,
        holder: "ada",
        balance: 3,
    };
    builder.load([&first, &second, &third]).expect("load");
    assert_eq!(builder.delete([&first]).expect("delete").changed(), 1);
    assert_eq!(builder.get(id).expect("get"), Some(third));
    assert!(!builder.contains(&first).expect("deleted"));
    assert!(builder.contains(&second).expect("middle still live"));
    assert!(builder.contains(&third).expect("latest still live"));
}

#[test]
fn overlay_get_misses_an_absent_key() {
    let mut builder = InstanceBuilder::new(Ledger).expect("valid");
    let id = builder
        .reserve::<AccountId>(1)
        .expect("reserve")
        .start()
        .expect("nonempty");
    assert_eq!(builder.get(id).expect("get"), None);
    assert_eq!(
        builder
            .get_dyn(Account::RELATION, StatementId(0), &[Value::U64(id.fresh())])
            .expect("get_dyn"),
        None
    );
}

#[test]
fn delete_does_not_rewind_the_fresh_floor() {
    let mut builder = InstanceBuilder::new(Ledger).expect("valid");
    let id = builder
        .reserve::<AccountId>(1)
        .expect("reserve")
        .start()
        .expect("nonempty");
    let acct = Account {
        id,
        holder: "ada",
        balance: 10,
    };
    builder.load([&acct]).expect("load");
    builder.delete([&acct]).expect("delete");
    let next = builder
        .reserve::<AccountId>(1)
        .expect("reserve")
        .start()
        .expect("nonempty");
    assert_eq!(next.fresh(), 1);
}

#[test]
fn closed_contains_and_get_read_the_extension() {
    let mut builder = InstanceBuilder::new(WithClosed).expect("valid");
    assert!(
        builder
            .contains_dyn(WithClosed::KIND, &[Value::U64(0)])
            .expect("Checking is sealed")
    );
    assert!(
        !builder
            .contains_dyn(WithClosed::KIND, &[Value::U64(1)])
            .expect("no second axiom")
    );
}

#[test]
fn reserve_exhaustion_after_a_load_poisons() {
    let mut builder = InstanceBuilder::new(Ledger).expect("valid");
    let id = builder
        .reserve::<AccountId>(1)
        .expect("reserve")
        .start()
        .expect("nonempty");
    let acct = Account {
        id,
        holder: "ada",
        balance: 10,
    };
    builder.load([&acct]).expect("load");
    let first = builder
        .reserve::<AccountId>(u64::MAX)
        .expect_err("exhausted");
    assert!(
        matches!(first, Error::FreshExhausted { .. }),
        "first failure is the original: {first:?}"
    );
    let poisoned = builder.load([&acct]).expect_err("poisoned");
    assert!(
        matches!(poisoned, Error::TransactionPoisoned { .. }),
        "{poisoned:?}"
    );
}

fn ordinary_floor_theory() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            crate::schema::RelationDescriptor {
                extension: None,
                name: "Holder".into(),
                fields: vec![crate::schema::tests::fresh_field("id")],
            },
            crate::schema::RelationDescriptor {
                extension: None,
                name: "Account".into(),
                fields: vec![
                    crate::schema::tests::fresh_field("id"),
                    field("holder", ValueType::U64),
                ],
            },
        ],
        statements: vec![
            containment(
                side(RelationId(1), &[FieldId(1)]),
                side(RelationId(0), &[FieldId(0)]),
            ),
            capacity(
                side(RelationId(1), &[FieldId(1)]),
                2,
                None,
                side(RelationId(0), &[FieldId(0)]),
            ),
        ],
    }
}

fn closed_source_ordinary_target() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            closed(
                "Kind",
                vec![],
                vec![row("Soft", vec![]), row("Hard", vec![])],
            ),
            crate::schema::RelationDescriptor {
                extension: None,
                name: "Bucket".into(),
                fields: vec![field("id", ValueType::U64)],
            },
        ],
        statements: vec![
            fd(RelationId(1), &[FieldId(0)]),
            containment(
                side(RelationId(0), &[FieldId(0)]),
                side(RelationId(1), &[FieldId(0)]),
            ),
        ],
    }
}

fn closed_parent_capacity() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            closed("Kind", vec![], vec![row("Only", vec![])]),
            crate::schema::RelationDescriptor {
                extension: None,
                name: "Item".into(),
                fields: vec![field("kind", ValueType::U64)],
            },
        ],
        statements: vec![
            fd(RelationId(1), &[FieldId(0)]),
            capacity(
                side(RelationId(1), &[FieldId(0)]),
                2,
                None,
                side(RelationId(0), &[FieldId(0)]),
            ),
        ],
    }
}

#[test]
fn empty_ledger_admits() {
    let builder = InstanceBuilder::new(Ledger).expect("valid");
    builder.admit().expect("admit").expect("accepted");
}

#[test]
fn load_and_reserve_admits() {
    let mut builder = InstanceBuilder::new(Ledger).expect("valid");
    let id = builder
        .reserve::<AccountId>(1)
        .expect("reserve")
        .start()
        .expect("nonempty");
    let acct = Account {
        id,
        holder: "ada",
        balance: 10,
    };
    builder.load([&acct]).expect("load");
    let instance = builder.admit().expect("admit").expect("accepted");
    assert!(instance.identity().same(instance.identity()));
}

#[test]
fn owned_instance_is_send_sync() {
    fn require<T: Send + Sync>() {}
    require::<OwnedInstance<Ledger>>();
}

#[test]
fn closed_source_missing_ordinary_target_rejects() {
    closed_source_ordinary_target()
        .validate()
        .expect("validates");
    let builder = InstanceBuilder::new(closed_source_ordinary_target()).expect("valid");
    let Admission::Rejected(violations) = builder.admit().expect("admit") else {
        panic!("empty incremental plan would accept this; complete admission must reject");
    };
    assert!(
        violations.iter().any(|v| matches!(
            v,
            Violation::Containment {
                direction: Direction::SourceUnsatisfied,
                ..
            }
        )),
        "{violations}"
    );
}

#[test]
fn closed_source_with_ordinary_targets_admits() {
    let mut builder = InstanceBuilder::new(closed_source_ordinary_target()).expect("valid");
    builder
        .load_dyn(RelationId(1), [&[Value::U64(0)], &[Value::U64(1)]])
        .expect("buckets");
    builder.admit().expect("admit").expect("accepted");
}

#[test]
fn fresh_row_collision_is_functionality() {
    let mut builder = InstanceBuilder::new(Ledger).expect("valid");
    let id = builder
        .reserve::<AccountId>(1)
        .expect("reserve")
        .start()
        .expect("nonempty");
    let first = Account {
        id,
        holder: "ada",
        balance: 1,
    };
    let second = Account {
        id,
        holder: "bev",
        balance: 2,
    };
    builder.load([&first, &second]).expect("load");
    let Admission::Rejected(violations) = builder.admit().expect("admit") else {
        panic!("duplicate fresh row must reject");
    };
    assert!(
        violations.iter().any(|v| matches!(
            v,
            Violation::Functionality {
                conflict: Conflict::Scalar,
                ..
            }
        )),
        "{violations}"
    );
}

#[test]
fn ordinary_positive_floor_childless_rejects() {
    let mut builder = InstanceBuilder::new(ordinary_floor_theory()).expect("valid");
    builder
        .load_dyn(RelationId(0), [&[Value::U64(0)]])
        .expect("holder without children");
    let Admission::Rejected(violations) = builder.admit().expect("admit") else {
        panic!("floor 2 with zero children must reject");
    };
    assert!(
        violations
            .iter()
            .any(|v| matches!(v, Violation::Capacity { .. })),
        "{violations}"
    );
}

#[test]
fn closed_positive_floor_childless_rejects() {
    let builder = InstanceBuilder::new(closed_parent_capacity()).expect("valid");
    let Admission::Rejected(violations) = builder.admit().expect("admit") else {
        panic!("closed parent with floor 2 and no children must reject");
    };
    assert!(
        violations
            .iter()
            .any(|v| matches!(v, Violation::Capacity { .. })),
        "{violations}"
    );
}

#[test]
fn dead_intern_is_not_in_the_frozen_dict() {
    let mut builder = InstanceBuilder::new(Named).expect("valid");
    let row = [Value::String("ada".into())];
    builder.load_dyn(Label::RELATION, [&row]).expect("load");
    builder.delete_dyn(Label::RELATION, [&row]).expect("delete");
    let instance = builder.admit().expect("admit").expect("accepted");
    assert_eq!(
        instance.catalog().dict_lookup(b"ada").expect("lookup"),
        None
    );
    assert_eq!(instance.catalog().dict_next_id().expect("next").raw(), 0);
}

#[test]
fn poisoned_builder_admits_as_err() {
    let mut builder = InstanceBuilder::new(Ledger).expect("valid");
    let id = builder
        .reserve::<AccountId>(1)
        .expect("reserve")
        .start()
        .expect("nonempty");
    builder
        .load([&Account {
            id,
            holder: "ada",
            balance: 10,
        }])
        .expect("load");
    let _ = builder
        .reserve::<AccountId>(u64::MAX)
        .expect_err("exhausted");
    match builder.admit() {
        Err(err) => assert!(matches!(err, Error::TransactionPoisoned { .. }), "{err:?}"),
        Ok(_) => panic!("poisoned builder must not admit"),
    }
}

/// The accepted transport against the empty base
/// (`proposals/one-representation/20`): `load_accepted` /
/// `delete_accepted` are the builder-fed twins of `load_dyn` /
/// `delete_dyn` — same reports, same never-mint delete, same one-parse
/// law (a shape-illegal row refuses the WHOLE collection at
/// construction, so nothing an accepted verb can receive ever stages a
/// prefix).
#[test]
fn accepted_verbs_mirror_the_dyn_lane_on_the_heap_base() {
    let mut builder = InstanceBuilder::new(Named).expect("valid");
    let schema = crate::api::db::CodecRead::schema(&builder);
    let fields = schema.relation(Label::RELATION).fields().to_vec();

    // Delete before any insert: resolve-only, the dictionary stays empty.
    let ghost = crate::AcceptedCollection::from_value_rows(
        Label::RELATION,
        &fields,
        [&[Value::String("ghost".into())]],
    )
    .expect("shape-lawful");
    assert_eq!(
        builder.delete_accepted(&ghost).expect("delete").changed(),
        0
    );
    assert_eq!(builder.intern_count(), 0);

    // The parse-all-first law lives at the constructor now: the illegal
    // row refuses the whole collection, so no partial proof reaches the
    // stage.
    let ok = vec![Value::String("ok".into())];
    let bad = Vec::<Value>::new();
    let err =
        crate::AcceptedCollection::from_value_rows(Label::RELATION, &fields, [ok.clone(), bad])
            .expect_err("shape");
    assert!(matches!(err, Error::FactShape(_)), "{err:?}");
    assert!(
        !builder
            .contains_dyn(Label::RELATION, &ok)
            .expect("nothing staged")
    );

    // The load disposition mints exactly as `load_dyn`.
    let real = crate::AcceptedCollection::from_value_rows(Label::RELATION, &fields, [&ok])
        .expect("shape-lawful");
    let report = builder.load_accepted(&real).expect("load");
    assert_eq!(report.submitted(), 1);
    assert_eq!(report.changed(), 1);
    assert_eq!(builder.intern_count(), 1);
    assert!(builder.contains_dyn(Label::RELATION, &ok).expect("staged"));

    // Empty is lawful, both dispositions.
    let empty = crate::AcceptedCollection::from_value_rows(
        Label::RELATION,
        &fields,
        std::iter::empty::<&[Value]>(),
    )
    .expect("empty seals lawfully");
    assert_eq!(builder.load_accepted(&empty).expect("empty").submitted(), 0);
    assert_eq!(
        builder.delete_accepted(&empty).expect("empty").submitted(),
        0
    );
}

/// The closed wall holds on the accepted lane exactly as on `load_dyn`.
#[test]
fn closed_relation_accepted_load_is_refused() {
    let mut builder = InstanceBuilder::new(WithClosed).expect("valid");
    let schema = crate::api::db::CodecRead::schema(&builder);
    let fields = schema.relation(WithClosed::KIND).fields().to_vec();
    let row =
        crate::AcceptedCollection::from_value_rows(WithClosed::KIND, &fields, [&[Value::U64(0)]])
            .expect("shape-lawful against the sealed roster");
    let err = builder.load_accepted(&row).expect_err("closed");
    assert!(matches!(err, Error::ClosedRelationWrite { .. }), "{err:?}");
}
