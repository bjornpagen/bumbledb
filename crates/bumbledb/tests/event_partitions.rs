use bumbledb::{
    Db, WorkContext,
    event::{Error, EventPartition, PartitionLimits, Space, SpaceId},
};

mod common;

bumbledb::schema! {
    pub ObservableSchema;
    relation Parent { group: u64, when: event }
    relation Outcome { group: u64, value: u64, when: event }
    Parent(group) -> Parent;
    Parent(group, when) -> Parent;
    Outcome(group, value) -> Outcome;
    Outcome(group, when) -> Outcome;
    Parent(group, when) == Outcome(group, when);
}

#[test]
fn finite_observables_use_ordinary_scalar_values_and_pointwise_dependencies() {
    let directory = common::TempDir::new("event-observable-roster");
    let db = Db::create(directory.path(), ObservableSchema, common::work())
        .unwrap()
        .unwrap();
    let source = Space::new(SpaceId([91; 32]), 1, &()).unwrap();
    let held = source.coordinate(0, &()).unwrap();
    let limits = PartitionLimits::default();
    let counts = source
        .count_partition(&[held.clone(), held.clone()], limits, &common::work())
        .unwrap();
    // Every input value maps to the same ordinary output value, proving certainty.
    let certain = counts
        .coarsen(&[1, 1, 1], 2, limits, &common::work())
        .unwrap();
    db.write(common::work(), |tx| {
        for (group, partition) in [(1, &counts), (2, &certain)] {
            tx.insert([&Parent {
                group,
                when: partition.parent().clone(),
            }])?;
            for (value, when) in partition.cells().iter().enumerate() {
                tx.insert([&Outcome {
                    group,
                    value: value as u64,
                    when: when.clone(),
                }])?;
            }
        }
        Ok(())
    })
    .unwrap()
    .unwrap();
    // Removing a nonempty value branch would make the observable partial.
    let gap = common::expect_rejected(db.write(common::work(), |tx| {
        tx.delete([&Outcome {
            group: 1,
            value: 2,
            when: held.clone(),
        }])?;
        Ok(())
    }));
    assert_eq!(gap.len(), 1);
    let cancelled = WorkContext::new();
    cancelled.cancel();
    assert_eq!(
        source
            .count_partition(std::slice::from_ref(&held), limits, &cancelled)
            .unwrap_err(),
        Error::Cancelled
    );
    drop((db, source, held, counts, certain));
    let db = Db::open(directory.path(), ObservableSchema, common::work()).unwrap();
    let mut rows: Vec<Outcome> = db
        .read(common::work(), |snapshot| snapshot.scan_facts()?.collect())
        .unwrap();
    drop(db);
    rows.sort_by_key(|row| (row.group, row.value));
    assert_eq!(rows.len(), 5); // Explicit empty buckets survived persistence.
    assert!(rows[1].when.is_empty());
    assert!(rows[3].when.is_empty());
    assert!(rows[4].when.is_full());
    let rebuilt = EventPartition::on(
        &rows[0].when.space().full(),
        &rows[..3]
            .iter()
            .map(|row| row.when.clone())
            .collect::<Vec<_>>(),
        limits,
        &common::work(),
    )
    .unwrap();
    assert_eq!(rebuilt.locate(0, &()).unwrap(), Some(0));
    assert_eq!(rebuilt.locate(1, &()).unwrap(), Some(2));
}
