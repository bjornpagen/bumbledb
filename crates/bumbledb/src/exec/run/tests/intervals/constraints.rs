use super::*;

#[test]
fn equal_and_reversed_overlap_constraints_keep_spanning_covers() {
    let schema = tagged_interval_schema(3);
    let a_rows = vec![(1, 1, 2), (2, 1, 2)];
    let c_rows: Vec<_> = (0..40u64)
        .map(|i| {
            let (start, end) = match i % 5 {
                0 => (0, 5),
                1 => (0, 2),
                2 => (3, 5),
                3 => (2, 3),
                _ => (0, u64::MAX),
            };
            (1000 + i, start, end)
        })
        .collect();
    let query = NormalizedQuery {
        dead: None,
        occurrences: (0..3u16)
            .map(|occ| Occurrence {
                occ_id: OccId(occ),
                bind: OccBind::Edb(RelationId(u32::from(occ))),
                role: Role::Positive,
                vars: vec![
                    (FieldId(0), VarId(2 * occ)),
                    (FieldId(1), VarId(2 * occ + 1)),
                ],
                filters: vec![],
                point_vars: vec![],
            })
            .collect(),
        residuals: vec![],
        word_residuals: vec![],
        anti_probes: vec![],
        allen_residuals: [1, 3]
            .into_iter()
            .map(|lhs| FilterPredicate::FieldsAllen {
                left: OperandAddr::from(VarId(lhs)),
                right: OperandAddr::from(VarId(5)),
                mask: AllenMask::INTERSECTS,
            })
            .collect(),
        slot_widths: (0..3u16)
            .flat_map(|occ| {
                [
                    (VarId(2 * occ), SlotWidth::ONE),
                    (VarId(2 * occ + 1), SlotWidth::TWO),
                ]
            })
            .collect(),
    };
    let plan = planned_with_sinks(&query, &schema, &[0, 1, 2], &all_vars(&query));
    let leaf = plan.nodes().len() - 1;
    let expected: BTreeSet<_> = [1, 2]
        .into_iter()
        .flat_map(|a| {
            (0..40u64)
                .filter(|i| matches!(i % 5, 0 | 4))
                .map(move |i| (a, 3, 1000 + i))
        })
        .collect();
    for (start, end) in [(2, 3), (3, 4)] {
        let views = tagged_interval_views(
            &schema,
            &[a_rows.clone(), vec![(3, start, end)], c_rows.clone()],
        );
        for batch in [1, BATCH] {
            let (rows, tally) = run_tallied(&plan, &views, batch);
            let actual: BTreeSet<_> = rows
                .iter()
                .map(|row| {
                    (
                        row[plan.slot_of(VarId(0))],
                        row[plan.slot_of(VarId(2))],
                        row[plan.slot_of(VarId(4))],
                    )
                })
                .collect();
            assert_eq!(
                actual, expected,
                "second constraint [{start},{end}), batch={batch}"
            );
            assert_eq!(
                tally[leaf],
                40 + 16,
                "first cover declines; second uses the combined index filter"
            );
        }
    }
}
