//! The projection sink's construction and its `Sink` consume path.
mod new;
mod sink;

/// One exact physical layout compiled into straight-line projection
/// routing. Batch layouts can change with the adaptive cover or combine
/// ancestor and leaf keys; compare contents, never allocation addresses.
/// Separate scan/batch owners prevent either layout from poisoning the
/// other. Both output lists are bounded by the fixed projection arity.
#[derive(Debug)]
pub(super) struct ProjectionRoute {
    key_slots: Vec<usize>,
    keys: Vec<(usize, usize)>,
    outer: Vec<(usize, usize)>,
}

impl ProjectionRoute {
    fn new(arity: usize, slot_count: usize) -> Self {
        Self {
            key_slots: Vec::with_capacity(slot_count),
            keys: Vec::with_capacity(arity),
            outer: Vec::with_capacity(arity),
        }
    }

    fn clear(&mut self) {
        self.key_slots.clear();
        self.keys.clear();
        self.outer.clear();
    }

    fn matches(&self, slots: &[usize], arity: usize) -> bool {
        self.keys.len() + self.outer.len() == arity && self.key_slots == slots
    }

    fn prepare(&mut self, slots: &[usize], sources: &[usize]) {
        self.clear();
        self.key_slots.extend_from_slice(slots);
        for (output, &slot) in sources.iter().enumerate() {
            if let Some(word) = slots.iter().position(|&key| key == slot) {
                self.keys.push((output, word));
            } else {
                self.outer.push((output, slot));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::exec::run::{Bindings, Flow, LeafBatch, Sink};
    use crate::exec::sink::{FindSpec, ProjectionSink};
    use std::collections::BTreeSet;

    fn generated_scan_case() -> (
        crate::exec::colt::Colt,
        crate::plan::fj::ProjectionDistinctWitness,
    ) {
        use crate::exec::colt::Colt;
        use crate::image::testsupport::TestSource;
        use crate::image::view::{BoundView, View};
        use crate::ir::{Atom, AtomSource, FindTerm, Query, Rule, Term, Value, VarId};
        use crate::schema::{
            FieldDescriptor, FieldId, RelationDescriptor, RelationId, SchemaDescriptor,
            ValidateDescriptor as _, ValueType,
        };

        let schema = SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "rows".into(),
                fields: [
                    ("outer", ValueType::U64),
                    ("flag", ValueType::Bool),
                    ("id", ValueType::U64),
                ]
                .into_iter()
                .map(|(name, value_type)| FieldDescriptor {
                    name: name.into(),
                    value_type,
                })
                .collect(),
                extension: None,
            }],
            statements: vec![],
        }
        .validate()
        .unwrap();
        let query = Query::single(Rule {
            finds: [0, 2, 1].map(|var| FindTerm::Var(VarId(var))).into(),
            atoms: vec![Atom {
                source: AtomSource::Edb(RelationId(0)),
                bindings: (0..3)
                    .map(|field| (FieldId(field), Term::Var(VarId(field))))
                    .collect(),
            }],
            negated: vec![],
            conditions: vec![],
        });
        let validated = crate::ir::validate::validate(&schema, &query).unwrap();
        let normalized = crate::ir::normalize::normalize_rules(&schema, &[], validated.rules());
        let witness = crate::plan::fj::provably_distinct_projection(
            &normalized[0],
            &schema,
            &query.rules()[0].finds,
        )
        .unwrap();
        let facts = (0..600)
            .map(|id| vec![Value::U64(7), Value::Bool(id % 2 == 0), Value::U64(id)])
            .collect();
        let source = TestSource::new(&schema, &[(RelationId(0), facts)]);
        let (_cache, image) = source.image_with_cache(RelationId(0));
        let colt = Colt::new(
            View::Bound(BoundView::All(image)),
            &[],
            vec![vec![0], vec![1, 2]],
        );
        (colt, witness)
    }

    #[test]
    fn generated_resident_rows_preserve_mixed_columns_order_boundaries_and_reuse() {
        assert_generated_scans(&[usize::MAX]);
    }

    #[test]
    fn generated_rows_preserve_mixed_columns_and_order_across_spill_and_reuse() {
        assert_generated_scans(&[65, 0]);
    }

    fn assert_generated_scans(allowances: &[usize]) {
        use crate::exec::colt::SuffixRun;
        use crate::exec::run::{LeafScan, ScanOffer};
        use crate::exec::sink::SinkBudget;

        let (colt, witness) = generated_scan_case();
        let crate::image::ColumnView::Bytes(flags) = colt.suffix_column(1, 0) else {
            panic!("byte column")
        };
        let crate::image::ColumnView::Words(ids) = colt.suffix_column(1, 1) else {
            panic!("word column")
        };
        let positions: Vec<u32> = (300..580).rev().collect();
        for &allowance in allowances {
            let mut dense = ProjectionSink::new(vec![0, 2, 1]);
            dense.elide_output_hashing(witness);
            let mut hashed = ProjectionSink::new(vec![0, 2, 1]);
            for outer in [7, 9] {
                let mut bindings = Bindings::new(3);
                bindings.set(0, outer);
                let scan = LeafScan {
                    colt: &colt,
                    level: 1,
                    key_slots: &[1, 2],
                    bindings: &bindings,
                };
                let pinned_keys: Vec<_> =
                    (1000..1017u64).flat_map(|id| [id, outer, id % 2]).collect();
                let survivors: Vec<u32> = (0..17).rev().collect();
                let batch = LeafBatch {
                    keys: &pinned_keys,
                    arity: 3,
                    survivors: &survivors,
                    key_slots: &[2, 0, 1],
                    bindings: &bindings,
                };
                for sink in [&mut dense, &mut hashed] {
                    sink.reset();
                    sink.begin(Some(SinkBudget {
                        work: crate::api::db::test_operation().unwrap(),
                        ram_bytes: allowance,
                    }));
                    sink.prepare_scan(scan.key_slots);
                    assert_eq!(sink.begin_scan(&scan), ScanOffer::Open);
                    sink.scan_run(&scan, SuffixRun::Identity { start: 3, len: 259 });
                    // Combined/permuted pinned layout must use its own
                    // route and retain its survivor order between scans.
                    assert_eq!(sink.emit_batch(&batch), Flow::Continue);
                    sink.scan_run(&scan, SuffixRun::Positions(&positions));
                    assert_eq!(sink.end_scan(&scan), 539);
                }
                let source_row = |p: usize| vec![outer, ids[p], u64::from(flags[p])];
                let expected: Vec<_> = (3..262)
                    .map(source_row)
                    .chain(survivors.iter().map(|&entry| {
                        let id = 1000 + u64::from(entry);
                        vec![outer, id, id % 2]
                    }))
                    .chain(positions.iter().map(|&p| source_row(p as usize)))
                    .collect();
                for sink in [&mut dense, &mut hashed] {
                    let mut actual = Vec::new();
                    sink.for_each_answer(&mut |row| {
                        actual.push(row.to_vec());
                        Ok(())
                    })
                    .unwrap();
                    assert_eq!(actual, expected, "allowance={allowance}, outer={outer}");
                }
                assert_eq!(
                    dense.scan_rows.capacity(),
                    0,
                    "proved scans allocate no intermediate batch"
                );
            }
        }
    }

    #[test]
    fn batch_routes_follow_exact_layouts_and_rule_aim_without_growing() {
        let mut projected = [0, 2, 0, 7, 4];
        let mut sink = ProjectionSink::new(projected.to_vec());
        let mut bindings = Bindings::new(8);
        let mut slots = Vec::with_capacity(8);
        let mut expected = BTreeSet::new();
        let capacities = (
            sink.batch_route.key_slots.capacity(),
            sink.batch_route.keys.capacity(),
            sink.batch_route.outer.capacity(),
        );
        // Repeated layouts, equal-size permutations at the SAME address,
        // combined ancestor/leaf keys, and empty keys with only outer data.
        let layouts: [&[usize]; 6] = [
            &[2, 4, 0],
            &[2, 4, 0],
            &[7, 0, 4],
            &[0, 2, 4, 7],
            &[],
            &[2, 4, 0],
        ];
        for (turn, layout) in layouts.into_iter().enumerate() {
            if turn == 5 {
                projected = [7, 0, 4, 2, 7];
                sink.aim(&projected.map(|slot| FindSpec::Var { slot, width: 1 }), 8);
            }
            slots.clear();
            slots.extend_from_slice(layout);
            for slot in 0..8 {
                bindings.set(slot, (turn * 100 + slot) as u64);
            }
            let keys: Vec<_> = (0..2)
                .flat_map(|entry| {
                    (0..slots.len()).map(move |word| (1000 + turn * 100 + entry * 10 + word) as u64)
                })
                .collect();
            let batch = LeafBatch {
                keys: &keys,
                arity: slots.len(),
                survivors: &[1, 0],
                key_slots: &slots,
                bindings: &bindings,
            };
            let skip = turn == 2;
            let flow = if skip {
                sink.emit_batch_until_skip(&batch)
            } else {
                sink.emit_batch(&batch)
            };
            assert_eq!(
                flow,
                if skip {
                    Flow::SkipSuffix
                } else {
                    Flow::Continue
                }
            );
            for &entry in batch.survivors.iter().take(if skip { 1 } else { 2 }) {
                expected.insert(
                    projected
                        .iter()
                        .map(|slot| {
                            layout
                                .iter()
                                .position(|key| key == slot)
                                .map_or_else(|| bindings.get(*slot), |word| batch.key(entry, word))
                        })
                        .collect::<Vec<_>>(),
                );
            }
            assert_eq!(
                sink.answers().map(<[u64]>::to_vec).collect::<BTreeSet<_>>(),
                expected
            );
            assert_eq!(
                capacities,
                (
                    sink.batch_route.key_slots.capacity(),
                    sink.batch_route.keys.capacity(),
                    sink.batch_route.outer.capacity(),
                )
            );
        }
    }
}
