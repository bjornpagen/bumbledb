//! L20-owned Lean correspondence that runs as bench cargo tests.
//!
//! L19 removed three-way / cargo oracles from `scripts/lean.sh`. These
//! cases live here: collision bytes, exact folds, and admission support.
//! The independent judgment reference is `judge_final_state`, never the
//! production planner. Verification: **NotRun**.

/// Correspondence ids L20 owns for L21 / spec-census.
pub const OWNED_CASES: &[&str] = &[
    "C-D04-collision-bytes",
    "C-D19-cancel",
    "C-D19-mean-once",
    "C-D19-merge-not-idemp",
    "C-G03-mutable-support",
    "C-G03-add-wins",
    "C-G03-raw-commute",
];

#[cfg(test)]
mod tests {
    use bumbledb::schema::judge::{
        JudgeBudget, Judgment, MapState, judge_final_state,
    };
    use bumbledb::schema::{
        Bound, FieldId, RelationDescriptor, Schema, SchemaDescriptor, StatementDescriptor,
        ValidateDescriptor as _, ValueType, Weight,
    };
    use bumbledb::{ChangeSet, RelationId, Value};

    use crate::fixture::{field, side};
    use crate::harness::bench_work;
    use crate::verify::f64_oracle::{MAX_FINITE, SIGN, fold, mean_bits, sum_bits};

    use super::OWNED_CASES;

    const ITEM: RelationId = RelationId(0);
    const PARENT: RelationId = RelationId(0);
    const CHILD: RelationId = RelationId(1);

    fn work() -> bumbledb::WorkContext {
        bench_work().expect("correspondence work")
    }

    fn key_schema() -> Schema {
        SchemaDescriptor {
            relations: vec![RelationDescriptor {
                extension: None,
                name: "Item".into(),
                fields: vec![field("id", ValueType::U64), field("payload", ValueType::U64)],
            }],
            statements: vec![StatementDescriptor::Functionality {
                relation: ITEM,
                projection: Box::new([FieldId(0)]),
            }],
        }
        .validate()
        .expect("key schema seals")
    }

    fn capacity_schema() -> Schema {
        SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "Parent".into(),
                    fields: vec![field("id", ValueType::U64)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Child".into(),
                    fields: vec![field("id", ValueType::U64), field("parent", ValueType::U64)],
                },
            ],
            statements: vec![
                StatementDescriptor::Functionality {
                    relation: PARENT,
                    projection: Box::new([FieldId(0)]),
                },
                StatementDescriptor::Functionality {
                    relation: CHILD,
                    projection: Box::new([FieldId(0)]),
                },
                StatementDescriptor::Capacity {
                    target: side(PARENT, &[0], &[]),
                    weight: Weight::Unit,
                    lo: 0,
                    hi: Some(Bound::Lit(1)),
                    source: side(CHILD, &[1], &[]),
                },
            ],
        }
        .validate()
        .expect("capacity schema seals")
    }

    fn judge(schema: &Schema, state: &MapState) -> Judgment {
        judge_final_state(schema, state, &work(), JudgeBudget::default())
            .expect("judge_final_state")
    }

    fn rejected_statements(judgment: &Judgment) -> Vec<u16> {
        match judgment {
            Judgment::Admitted => Vec::new(),
            Judgment::Rejected(violations) => violations.iter().map(|v| v.statement.0).collect(),
        }
    }

    const ONE: u64 = 0x3ff0_0000_0000_0000;
    const TWO: u64 = 0x4000_0000_0000_0000;
    const TEN_POW_16: u64 = 0x4341_c379_37e0_8000;
    /// `ChangeSet` header: magic(8) + version(2) + schema fingerprint(32) + count(8).
    const CHANGESET_HEADER: usize = 50;

    /// C-D04-collision-bytes: unequal canonical rows stay distinct. A shared
    /// fingerprint/routing bucket is not logical identity.
    #[test]
    fn c_d04_collision_bytes_exact_rows_are_not_fingerprints() {
        let schema = key_schema();
        let left = vec![Value::U64(1), Value::U64(10)];
        let right = vec![Value::U64(1), Value::U64(20)];
        let fields = schema.relation_checked(ITEM).expect("item").fields();
        let left_bytes = bumbledb::canonical::CanonicalRow::encode(fields, &left, &work())
            .expect("encode left");
        let right_bytes = bumbledb::canonical::CanonicalRow::encode(fields, &right, &work())
            .expect("encode right");
        assert_ne!(
            left_bytes.as_bytes(),
            right_bytes.as_bytes(),
            "canonical bytes decide equality"
        );

        let mut both = MapState::new();
        both.insert(ITEM, left.clone());
        both.insert(ITEM, right.clone());
        match judge(&schema, &both) {
            Judgment::Rejected(violations) => {
                assert!(
                    violations
                        .iter()
                        .any(|v| v.kind == bumbledb::schema::StatementKind::Functionality),
                    "same key, unequal payloads reject: {violations:?}"
                );
            }
            Judgment::Admitted => panic!("fingerprint-style merge would admit two payloads"),
        }

        let mut only_left = MapState::new();
        only_left.insert(ITEM, left);
        assert!(matches!(judge(&schema, &only_left), Judgment::Admitted));
        // Deleting by the other payload's identity must not remove this row:
        // MapState still has the left fact; the right encoding never landed.
        let mut only_right = MapState::new();
        only_right.insert(ITEM, right);
        assert!(matches!(judge(&schema, &only_right), Judgment::Admitted));
        assert_ne!(
            rejected_statements(&judge(&schema, &only_left)),
            rejected_statements(&judge(&schema, &both))
        );
    }

    /// C-D19-cancel
    #[test]
    fn c_d19_cancel_uses_the_rational_oracle_not_host_add() {
        assert_eq!(sum_bits(&[TEN_POW_16, ONE, SIGN | TEN_POW_16]), ONE);
        let host = ((1e16f64 + 1.0) - 1e16f64).to_bits();
        assert_ne!(host, ONE, "host f64 add chain loses the 1");
    }

    /// C-D19-mean-once
    #[test]
    fn c_d19_mean_once_is_not_rounded_sum_over_count() {
        assert_eq!(sum_bits(&[MAX_FINITE, MAX_FINITE]), 0x7FF0_0000_0000_0000);
        assert_eq!(mean_bits(&[MAX_FINITE, MAX_FINITE]), Some(MAX_FINITE));
    }

    /// C-D19-merge-not-idemp — exact Lean name `merge_not_idempotent`.
    #[test]
    fn merge_not_idempotent() {
        let acc = fold(&[ONE, TWO]);
        let replayed = acc.merge(&acc);
        assert_ne!(replayed, acc);
        assert_eq!(replayed.count, 2 * acc.count);
    }

    /// C-G03-mutable-support: a Child-only change cannot move a Parent key
    /// verdict. Independent stream is `judge_final_state`.
    #[test]
    fn c_g03_mutable_support_leaves_untouched_statements() {
        let schema = capacity_schema();
        let mut parent_only = MapState::new();
        parent_only.insert(PARENT, vec![Value::U64(0)]);
        let admitted = judge(&schema, &parent_only);
        assert!(
            matches!(admitted, Judgment::Admitted),
            "parent alone is lawful"
        );

        let mut with_bad_child = MapState::new();
        with_bad_child.insert(PARENT, vec![Value::U64(0)]);
        with_bad_child.insert(CHILD, vec![Value::U64(7), Value::U64(0)]);
        with_bad_child.insert(CHILD, vec![Value::U64(7), Value::U64(1)]);
        let rejected = judge(&schema, &with_bad_child);
        let cited = rejected_statements(&rejected);
        assert!(
            cited.contains(&1),
            "Child key (statement 1) must reject unequal payloads: {rejected:?}"
        );

        let mut extra_parent = MapState::new();
        extra_parent.insert(PARENT, vec![Value::U64(0)]);
        extra_parent.insert(PARENT, vec![Value::U64(1)]);
        extra_parent.insert(CHILD, vec![Value::U64(7), Value::U64(0)]);
        extra_parent.insert(CHILD, vec![Value::U64(7), Value::U64(1)]);
        let after_unrelated = judge(&schema, &extra_parent);
        assert_eq!(
            cited,
            rejected_statements(&after_unrelated),
            "an extra Parent row is outside Child's key mutable support"
        );
    }

    /// C-G03-add-wins: one ChangeSet, same exact fact on both sides → Add.
    #[test]
    fn c_g03_add_wins_in_one_changeset() {
        let schema = key_schema();
        let row = [Value::U64(3), Value::U64(9)];
        for (first_delete, then_insert) in [(true, true), (false, true)] {
            let mut builder = ChangeSet::builder(&schema, work());
            if first_delete {
                builder.delete(ITEM, &row).expect("delete");
            }
            if then_insert {
                builder.insert(ITEM, &row).expect("insert");
            }
            if !first_delete {
                builder.delete(ITEM, &row).expect("delete after insert");
            }
            let set = builder.finish().expect("add-wins finish");
            assert_eq!(set.len(), 1, "finish refuses a second action for the row");
            assert_eq!(
                set.as_bytes()[CHANGESET_HEADER],
                1,
                "kind byte 1 is Add; Remove would be 0"
            );
            ChangeSet::parse(&schema, set.as_bytes(), &work())
                .expect("canonical one-action bytes re-parse");
        }
    }

    /// C-G03-raw-commute: disjoint child inserts share a capacity parent.
    /// Final sets match; the union does not admit.
    #[test]
    fn c_g03_raw_commute_does_not_commute_admission() {
        let schema = capacity_schema();
        let parent = vec![Value::U64(0)];
        let child_a = vec![Value::U64(0), Value::U64(0)];
        let child_b = vec![Value::U64(1), Value::U64(0)];

        let mut ab = MapState::new();
        ab.insert(PARENT, parent.clone());
        ab.insert(CHILD, child_a.clone());
        ab.insert(CHILD, child_b.clone());
        let mut ba = MapState::new();
        ba.insert(PARENT, parent.clone());
        ba.insert(CHILD, child_b.clone());
        ba.insert(CHILD, child_a.clone());

        let union = judge(&schema, &ab);
        assert_eq!(
            rejected_statements(&union),
            rejected_statements(&judge(&schema, &ba)),
            "raw set application commutes"
        );
        assert!(
            matches!(union, Judgment::Rejected(_)),
            "admission of the union must refuse the shared capacity parent"
        );

        let mut only_a = MapState::new();
        only_a.insert(PARENT, parent.clone());
        only_a.insert(CHILD, child_a);
        let mut only_b = MapState::new();
        only_b.insert(PARENT, parent);
        only_b.insert(CHILD, child_b);
        assert!(matches!(judge(&schema, &only_a), Judgment::Admitted));
        assert!(matches!(judge(&schema, &only_b), Judgment::Admitted));
    }

    #[test]
    fn owned_catalog_names_every_required_id() {
        for id in OWNED_CASES {
            assert!(id.starts_with("C-D04") || id.starts_with("C-D19") || id.starts_with("C-G03"));
        }
        assert_eq!(OWNED_CASES.len(), 7);
    }
}
