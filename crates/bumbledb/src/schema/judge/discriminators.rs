//! D04 / D05 / D26 discriminators. Authored now; verification NotRun.

use super::delta_tests::DeltaState;
use super::{
    CandidateFacts, JudgeBudget, JudgeScratch, Judgment, LawfulParent, MapState, judge_complete,
    judge_final_state, judge_final_state_with_scratch, judge_incremental, store_fault,
};
use crate::schema::evidence::encode_judged;
use crate::schema::tests::{
    capacity, capacity_weighted, closed, containment, fd, field, row, side, side_where,
};
use crate::schema::{
    Bound, FieldId, RelationDescriptor, RelationId, Schema, SchemaDescriptor, StatementId,
    ValidateDescriptor as _, ValueType, Weight,
};
use crate::storage::store::StoreError;
use crate::work::ExecutionPolicy;
use crate::{Value, WorkContext};
use std::time::Duration;

fn work() -> WorkContext {
    ExecutionPolicy {
        input_bytes: 1_000_000,
        working_bytes: 1_000_000,
        scratch_bytes: 64 << 20,
        result_bytes: 0,
        rows: 100_000,
        work_units: 1_000_000,
        timeout: Duration::from_secs(60),
    }
    .start()
    .unwrap()
}

fn keyed_users() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "User".into(),
            fields: vec![field("id", ValueType::U64), field("email", ValueType::String)],
        }],
        statements: vec![fd(RelationId(0), &[FieldId(0)]), fd(RelationId(0), &[FieldId(1)])],
    }
    .validate()
    .expect("valid")
}

fn user(id: u64, email: &str) -> Vec<Value> {
    vec![Value::U64(id), Value::String(email.into())]
}

fn parent_users(n: u64) -> Vec<(RelationId, Vec<Value>)> {
    (0..n)
        .map(|id| (RelationId(0), user(id, &format!("{id}@ex"))))
        .collect()
}

/// D04 — complete, incremental and the independent streaming judge agree
/// on admission, violated statements and canonical witnesses. Eligible
/// key laws visit only the touched group while unrelated groups scale.
#[test]
fn d04_compiled_indexes_earn_locality() {
    let schema = keyed_users();
    let parent = parent_users(64);
    let adds = vec![(RelationId(0), user(200, "0@ex"))];
    let state = DeltaState::new(&parent, &adds, &[]);
    let budget = JudgeBudget {
        examples_per_statement: 4,
    };
    let complete = judge_complete(&schema, &state, &work(), budget).expect("complete");
    let incremental = judge_incremental(
        LawfulParent::established(),
        &schema,
        &state,
        &work(),
        budget,
        JudgeScratch::disabled(),
    )
    .expect("incremental");
    let independent = judge_final_state(&schema, &state, &work(), budget).expect("independent");
    assert_eq!(complete, independent, "complete shares independent denotation");
    assert_eq!(
        incremental, complete,
        "incremental matches complete on a lawful parent"
    );
    let Judgment::Rejected(violations) = complete else {
        panic!("duplicate email must reject");
    };
    assert_eq!(violations[0].statement, StatementId(1));
    assert_eq!(violations[0].examples.len(), 2);

    let local = DeltaState::new(&parent, &adds, &[]).refusing_streams();
    let _ = judge_incremental(
        LawfulParent::established(),
        &schema,
        &local,
        &work(),
        budget,
        JudgeScratch::disabled(),
    )
    .expect("indexed key path");
    assert_eq!(
        local.row_visits(),
        0,
        "eligible key judgment must not scan unrelated facts"
    );
    let groups = local.group_visits();
    assert!(groups > 0, "the touched email group is visited");

    let scaled = DeltaState::new(&parent_users(256), &adds, &[]).refusing_streams();
    let _ = judge_incremental(
        LawfulParent::established(),
        &schema,
        &scaled,
        &work(),
        budget,
        JudgeScratch::disabled(),
    )
    .expect("scaled");
    assert_eq!(
        scaled.group_visits(),
        groups,
        "unrelated group growth does not increase group walks"
    );
}

/// D04 — floor from source removal and selected target replacement match
/// the independent final-state model.
#[test]
fn d04_capacity_floor_and_target_replacement_match_complete() {
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Room".into(),
                fields: vec![field("id", ValueType::U64), field("cap", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Booking".into(),
                fields: vec![field("id", ValueType::U64), field("room", ValueType::U64)],
            },
        ],
        statements: vec![
            fd(RelationId(0), &[FieldId(0)]),
            fd(RelationId(1), &[FieldId(0)]),
            capacity(
                side(RelationId(1), &[FieldId(1)]),
                1,
                Some(2),
                side(RelationId(0), &[FieldId(0)]),
            ),
        ],
    }
    .validate()
    .expect("valid");
    let parent = vec![
        (RelationId(0), vec![Value::U64(1), Value::U64(2)]),
        (RelationId(1), vec![Value::U64(10), Value::U64(1)]),
    ];
    let removed = vec![(RelationId(1), vec![Value::U64(10), Value::U64(1)])];
    let floor = DeltaState::new(&parent, &[], &removed);
    let budget = JudgeBudget::default();
    let complete = judge_complete(&schema, &floor, &work(), budget).expect("complete");
    let incremental = judge_incremental(
        LawfulParent::established(),
        &schema,
        &floor,
        &work(),
        budget,
        JudgeScratch::disabled(),
    )
    .expect("incremental");
    assert_eq!(complete, incremental);
    let Judgment::Rejected(violations) = complete else {
        panic!("floor after removal must reject");
    };
    assert_eq!(violations[0].measure, Some(0));

    let replaced_schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Room".into(),
                fields: vec![field("id", ValueType::U64), field("cap", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Booking".into(),
                fields: vec![field("id", ValueType::U64), field("room", ValueType::U64)],
            },
        ],
        statements: vec![
            fd(RelationId(0), &[FieldId(0)]),
            fd(RelationId(1), &[FieldId(0)]),
            capacity_weighted(
                side(RelationId(0), &[FieldId(0)]),
                Weight::Unit,
                0,
                Some(Bound::TargetField(FieldId(1))),
                side(RelationId(1), &[FieldId(1)]),
            ),
        ],
    }
    .validate()
    .expect("valid");
    let replaced = DeltaState::new(
        &parent,
        &[(RelationId(0), vec![Value::U64(1), Value::U64(0)])],
        &[(RelationId(0), vec![Value::U64(1), Value::U64(2)])],
    );
    let complete = judge_complete(&replaced_schema, &replaced, &work(), budget).expect("complete");
    let incremental = judge_incremental(
        LawfulParent::established(),
        &replaced_schema,
        &replaced,
        &work(),
        budget,
        JudgeScratch::disabled(),
    )
    .expect("incremental");
    assert_eq!(
        complete, incremental,
        "selected target replacement matches the independent model"
    );
    assert!(
        matches!(complete, Judgment::Rejected(_)),
        "cap 0 cannot cover the standing booking"
    );

    let local = DeltaState::new(&parent, &[], &removed).refusing_streams();
    let _ = judge_incremental(
        LawfulParent::established(),
        &schema,
        &local,
        &work(),
        budget,
        JudgeScratch::disabled(),
    )
    .expect("compiled capacity path");
    assert_eq!(
        local.row_visits(),
        0,
        "compiled capacity must not stream unrelated facts"
    );
    assert!(
        local.group_visits() > 0,
        "the touched capacity group is visited"
    );
}

/// D04 — incremental containment consumes interned group visits. A selected
/// target removal rejects with the same verdict as complete judgment; the
/// compiled path must not stream unrelated rooms or bookings, and extra
/// lawful groups must not add walks.
#[test]
fn d04_incremental_containment_consumes_compiled_groups() {
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Room".into(),
                fields: vec![field("id", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Booking".into(),
                fields: vec![field("id", ValueType::U64), field("room", ValueType::U64)],
            },
        ],
        statements: vec![
            fd(RelationId(0), &[FieldId(0)]),
            fd(RelationId(1), &[FieldId(0)]),
            containment(
                side(RelationId(1), &[FieldId(1)]),
                side(RelationId(0), &[FieldId(0)]),
            ),
        ],
    }
    .validate()
    .expect("valid");
    let mut parent = vec![
        (RelationId(0), vec![Value::U64(1)]),
        (RelationId(1), vec![Value::U64(10), Value::U64(1)]),
    ];
    for id in 2..64u64 {
        parent.push((RelationId(0), vec![Value::U64(id)]));
        parent.push((RelationId(1), vec![Value::U64(100 + id), Value::U64(id)]));
    }
    let removed = vec![(RelationId(0), vec![Value::U64(1)])];
    let state = DeltaState::new(&parent, &[], &removed);
    let budget = JudgeBudget::default();
    let complete = judge_complete(&schema, &state, &work(), budget).expect("complete");
    let incremental = judge_incremental(
        LawfulParent::established(),
        &schema,
        &state,
        &work(),
        budget,
        JudgeScratch::disabled(),
    )
    .expect("incremental");
    assert_eq!(complete, incremental);
    assert!(
        matches!(complete, Judgment::Rejected(_)),
        "removing the covering room must strand its booking"
    );

    let local = DeltaState::new(&parent, &[], &removed).refusing_streams();
    let _ = judge_incremental(
        LawfulParent::established(),
        &schema,
        &local,
        &work(),
        budget,
        JudgeScratch::disabled(),
    )
    .expect("compiled containment path");
    assert_eq!(
        local.row_visits(),
        0,
        "compiled containment must not stream unrelated facts"
    );
    let groups = local.group_visits();
    assert!(groups > 0, "the stranded booking group is visited");

    let mut scaled = parent;
    for id in 64..256u64 {
        scaled.push((RelationId(0), vec![Value::U64(id)]));
        scaled.push((RelationId(1), vec![Value::U64(100 + id), Value::U64(id)]));
    }
    let scaled = DeltaState::new(&scaled, &[], &removed).refusing_streams();
    let _ = judge_incremental(
        LawfulParent::established(),
        &schema,
        &scaled,
        &work(),
        budget,
        JudgeScratch::disabled(),
    )
    .expect("scaled containment");
    assert_eq!(
        scaled.group_visits(),
        groups,
        "unrelated group growth does not increase compiled containment walks"
    );
}

fn agree_reject(schema: &Schema, parent: &[(RelationId, Vec<Value>)], removed: &[(RelationId, Vec<Value>)]) {
    let state = DeltaState::new(parent, &[], removed);
    let budget = JudgeBudget::default();
    let complete = judge_complete(schema, &state, &work(), budget).expect("complete");
    let incremental = judge_incremental(
        LawfulParent::established(),
        schema,
        &state,
        &work(),
        budget,
        JudgeScratch::disabled(),
    )
    .expect("incremental");
    assert_eq!(
        complete, incremental,
        "complete and incremental must share logical group coordinates"
    );
    assert!(
        matches!(complete, Judgment::Rejected(_)),
        "the deletion must reject"
    );
}

/// D04 — closed Source[a,b] ⊆ Target[b,a]: closed projected (a=1,b=2)
/// and lawful target (a=2,b=1) share one logical group (`group_key`).
/// Intern order is `index_key` at `visit_compiled_group` only. Removing
/// that target must reject under complete and incremental judgment.
#[test]
fn d04_permuted_closed_source_target_deletion_agrees() {
    let schema = SchemaDescriptor {
        relations: vec![
            closed(
                "Need",
                vec![field("a", ValueType::U64), field("b", ValueType::U64)],
                vec![row("need", vec![Value::U64(1), Value::U64(2)])],
            ),
            RelationDescriptor {
                extension: None,
                name: "Have".into(),
                fields: vec![field("a", ValueType::U64), field("b", ValueType::U64)],
            },
        ],
        statements: vec![
            fd(RelationId(1), &[FieldId(0), FieldId(1)]),
            containment(
                side(RelationId(0), &[FieldId(0), FieldId(1)]),
                side(RelationId(1), &[FieldId(1), FieldId(0)]),
            ),
        ],
    }
    .validate()
    .expect("valid");
    let parent = vec![(RelationId(1), vec![Value::U64(2), Value::U64(1)])];
    let removed = vec![(RelationId(1), vec![Value::U64(2), Value::U64(1)])];
    agree_reject(&schema, &parent, &removed);

    let hetero = SchemaDescriptor {
        relations: vec![
            closed(
                "Need",
                vec![field("x", ValueType::I64), field("y", ValueType::U64)],
                vec![row("need", vec![Value::I64(2), Value::U64(1)])],
            ),
            RelationDescriptor {
                extension: None,
                name: "Have".into(),
                fields: vec![field("a", ValueType::U64), field("b", ValueType::I64)],
            },
        ],
        statements: vec![
            fd(RelationId(1), &[FieldId(0), FieldId(1)]),
            containment(
                side(RelationId(0), &[FieldId(0), FieldId(1)]),
                side(RelationId(1), &[FieldId(1), FieldId(0)]),
            ),
        ],
    }
    .validate()
    .expect("valid hetero");
    let parent = vec![(RelationId(1), vec![Value::U64(1), Value::I64(2)])];
    let removed = vec![(RelationId(1), vec![Value::U64(1), Value::I64(2)])];
    agree_reject(&hetero, &parent, &removed);

    let selected = SchemaDescriptor {
        relations: vec![
            closed(
                "Need",
                vec![field("a", ValueType::U64), field("b", ValueType::U64)],
                vec![row("need", vec![Value::U64(1), Value::U64(2)])],
            ),
            RelationDescriptor {
                extension: None,
                name: "Have".into(),
                fields: vec![field("a", ValueType::U64), field("b", ValueType::U64)],
            },
        ],
        statements: vec![
            fd(RelationId(1), &[FieldId(0), FieldId(1)]),
            containment(
                side_where(
                    RelationId(0),
                    &[FieldId(0), FieldId(1)],
                    vec![(FieldId(0), Value::U64(1))],
                ),
                side(RelationId(1), &[FieldId(1), FieldId(0)]),
            ),
        ],
    }
    .validate()
    .expect("valid selected");
    agree_reject(
        &selected,
        &[(RelationId(1), vec![Value::U64(2), Value::U64(1)])],
        &[(RelationId(1), vec![Value::U64(2), Value::U64(1)])],
    );

    let with_capacity = SchemaDescriptor {
        relations: vec![
            closed(
                "Need",
                vec![field("a", ValueType::U64), field("b", ValueType::U64)],
                vec![row("need", vec![Value::U64(1), Value::U64(2)])],
            ),
            RelationDescriptor {
                extension: None,
                name: "Have".into(),
                fields: vec![field("a", ValueType::U64), field("b", ValueType::U64)],
            },
        ],
        statements: vec![
            fd(RelationId(1), &[FieldId(0), FieldId(1)]),
            containment(
                side(RelationId(0), &[FieldId(0), FieldId(1)]),
                side(RelationId(1), &[FieldId(1), FieldId(0)]),
            ),
            capacity(
                side(RelationId(0), &[FieldId(0), FieldId(1)]),
                1,
                Some(1),
                side(RelationId(1), &[FieldId(1), FieldId(0)]),
            ),
        ],
    }
    .validate()
    .expect("valid capacity");
    agree_reject(
        &with_capacity,
        &[(RelationId(1), vec![Value::U64(2), Value::U64(1)])],
        &[(RelationId(1), vec![Value::U64(2), Value::U64(1)])],
    );
}

/// D05 — opposite insertion order, reminted identities and resident versus
/// forced-scratch citation keep the same evidence bytes. Selection is by
/// logical fact bytes before the budget, not by row id.
#[test]
fn d05_rejection_evidence_is_portable() {
    let schema = keyed_users();
    let mut forward = MapState::new();
    let mut reverse = MapState::new();
    for id in 0..8u64 {
        forward.insert(RelationId(0), user(id, "shared@ex"));
        reverse.insert(RelationId(0), user(7 - id, "shared@ex"));
    }
    let budget = JudgeBudget {
        examples_per_statement: 3,
    };
    let left = match judge_complete(&schema, &forward, &work(), budget).expect("forward") {
        Judgment::Rejected(violations) => violations,
        Judgment::Admitted => panic!("must reject"),
    };
    let right = match judge_complete(&schema, &reverse, &work(), budget).expect("reverse") {
        Judgment::Rejected(violations) => violations,
        Judgment::Admitted => panic!("must reject"),
    };
    assert_eq!(left, right, "insertion order cannot change citations");
    assert_eq!(left[0].examples.len(), 3);
    assert!(left[0].examples_truncated);
    let cited: Vec<u64> = left[0]
        .examples
        .iter()
        .map(|fact| match fact.values[0] {
            Value::U64(id) => id,
            _ => panic!("id"),
        })
        .collect();
    let mut expected: Vec<u64> = (0..8).collect();
    expected.sort_by(|a, b| {
        let left = crate::canonical::fact_sort_key(
            schema.relation(RelationId(0)).fields(),
            &user(*a, "shared@ex"),
            &work(),
        )
        .expect("key");
        let right = crate::canonical::fact_sort_key(
            schema.relation(RelationId(0)).fields(),
            &user(*b, "shared@ex"),
            &work(),
        )
        .expect("key");
        left.cmp(&right)
    });
    assert_eq!(
        cited,
        expected[..3],
        "citations are the canonical top-k, not first-seen row ids"
    );

    let reminted = {
        let mut state = MapState::new();
        for id in (0..8u64).rev() {
            state.insert(RelationId(0), user(id, "shared@ex"));
        }
        state
    };
    let reminted = match judge_complete(&schema, &reminted, &work(), budget).expect("remint") {
        Judgment::Rejected(violations) => violations,
        Judgment::Admitted => panic!("must reject"),
    };
    let ctx = work();
    let live = encode_judged(&schema, &left, 1 << 16, &ctx).expect("encode live");
    let imported = encode_judged(&schema, &reminted, 1 << 16, &ctx).expect("encode remint");
    assert_eq!(live, imported, "receipt bytes survive remint");

    struct StoreChannel(MapState);
    impl CandidateFacts for StoreChannel {
        type Error = StoreError;
        fn visit_rows(
            &self,
            relation: RelationId,
            visit: &mut dyn FnMut(&[Value]) -> Result<bool, Self::Error>,
        ) -> Result<(), Self::Error> {
            let mut error = None;
            self.0
                .visit_rows(relation, &mut |row| match visit(row) {
                    Ok(keep) => Ok(keep),
                    Err(failure) => {
                        error = Some(failure);
                        Ok(false)
                    }
                })
                .unwrap_or_else(|impossible| match impossible {});
            match error {
                Some(failure) => Err(failure),
                None => Ok(()),
            }
        }
    }
    let resident = judge_final_state(&schema, &forward, &work(), budget).expect("resident");
    let spilled = judge_final_state_with_scratch(
        &schema,
        &StoreChannel(forward),
        &work(),
        budget,
        JudgeScratch::channel(store_fault),
    )
    .expect("scratch");
    assert_eq!(resident, spilled, "forced scratch cannot change the receipt");
}

/// D26 — complete judgment cannot borrow a lawful-parent premise. A
/// populated invalid stage with an empty delta must reject under complete
/// judgment; incremental-with-parent would incorrectly admit.
#[test]
fn d26_complete_judgment_cannot_borrow_a_lawful_parent() {
    let schema = keyed_users();
    let mut populated = MapState::new();
    populated.insert(RelationId(0), user(1, "dup@ex"));
    populated.insert(RelationId(0), user(2, "dup@ex"));
    let complete = judge_complete(&schema, &populated, &work(), JudgeBudget::default())
        .expect("complete");
    assert!(
        matches!(complete, Judgment::Rejected(_)),
        "invalid populated final state must reject"
    );

    let empty_delta = DeltaState::new(
        &[
            (RelationId(0), user(1, "dup@ex")),
            (RelationId(0), user(2, "dup@ex")),
        ],
        &[],
        &[],
    );
    let incremental = judge_incremental(
        LawfulParent::established(),
        &schema,
        &empty_delta,
        &work(),
        JudgeBudget::default(),
        JudgeScratch::disabled(),
    )
    .expect("incremental empty");
    assert_eq!(
        incremental,
        Judgment::Admitted,
        "empty-delta incremental admits under a parent capability — staging must not call it"
    );
    let still_complete = judge_complete(&schema, &empty_delta, &work(), JudgeBudget::default())
        .expect("complete empty delta");
    assert!(
        matches!(still_complete, Judgment::Rejected(_)),
        "complete entry judges the populated state even when the delta is empty"
    );
}

/// D26 positive dual: a nonempty-required law rejects the empty final
/// state and admits once the required ordinary witness is present.
#[test]
fn d26_valid_nonempty_required_state_admits() {
    let schema = SchemaDescriptor {
        relations: vec![
            closed(
                "Required",
                vec![field("parent", ValueType::U64)],
                vec![row("need", vec![Value::U64(1)])],
            ),
            RelationDescriptor {
                extension: None,
                name: "Parent".into(),
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
    .validate()
    .expect("valid nonempty-required");
    let empty = MapState::new();
    assert!(
        matches!(
            judge_complete(&schema, &empty, &work(), JudgeBudget::default()).expect("empty"),
            Judgment::Rejected(_)
        ),
        "empty nonempty-required state rejects"
    );
    let mut filled = MapState::new();
    filled.insert(RelationId(1), vec![Value::U64(1)]);
    assert_eq!(
        judge_complete(&schema, &filled, &work(), JudgeBudget::default()).expect("filled"),
        Judgment::Admitted
    );
}

/// Resource refusal is not an invariant rejection.
#[test]
fn resource_refusal_is_not_rejection() {
    let schema = keyed_users();
    let mut state = MapState::new();
    for id in 0..20u64 {
        state.insert(RelationId(0), user(id, "shared@ex"));
    }
    let tiny = ExecutionPolicy {
        input_bytes: 0,
        working_bytes: 0,
        scratch_bytes: 0,
        result_bytes: 0,
        rows: 0,
        work_units: 4,
        timeout: Duration::from_secs(1),
    }
    .start()
    .unwrap();
    assert!(
        judge_complete(&schema, &state, &tiny, JudgeBudget::default()).is_err(),
        "exhausted work is an error, not a shorter rejection"
    );
}
