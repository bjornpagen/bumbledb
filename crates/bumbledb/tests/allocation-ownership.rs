//! Public-path regressions for unrestricted ownership and borrowed delivery.
//! Run allocation windows with nextest (one process per test), not parallel
//! libtest threads: the allocation counters are process-global.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb::{AnswerValue, BindValue, Db, DeliveryTicket, Error, RelationId, Value, WorkContext};

bumbledb::schema! {
    pub AllocationFixture;
    relation Sample { id: u64, label: str, bucket: u64 }
    Sample(id) -> Sample;
}

struct Directory(PathBuf);

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Fixture {
    // Close the database before removing this test's private directory.
    db: Db<AllocationFixture>,
    _directory: Directory,
    labels: Vec<String>,
}

fn fixture(rows: usize, text_bytes: usize) -> Fixture {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let serial = NEXT.fetch_add(1, Ordering::Relaxed);
    let directory = Directory(std::env::temp_dir().join(format!(
        "bumbledb-allocation-ownership-{}-{serial}",
        std::process::id(),
    )));
    let db = Db::create(&directory.0, AllocationFixture, WorkContext::new())
        .expect("create")
        .expect("admitted");
    let labels: Vec<_> = (0..rows)
        .map(|id| format!("{id}:{}", "x".repeat(text_bytes)))
        .collect();
    db.write(WorkContext::new(), |tx| {
        tx.insert_dyn(
            RelationId(0),
            labels.iter().enumerate().map(|(id, label)| {
                vec![
                    Value::U64(id as u64),
                    Value::String(label.as_str().into()),
                    Value::U64(0),
                ]
            }),
        )?;
        Ok(())
    })
    .expect("seed")
    .expect("admitted");
    Fixture {
        db,
        _directory: directory,
        labels,
    }
}

fn query() -> bumbledb::ir::Query {
    (*bumbledb::query!(AllocationFixture { (id, label) | Sample(id, label, bucket); })).clone()
}

fn no_params() -> &'static [BindValue<'static>] {
    &[]
}

#[test]
fn borrowed_complete_rows_and_pages_allocate_nothing_and_consumption_moves_storage() {
    let fixture = fixture(513, 4096);
    let mut prepared = fixture.db.prepare(&query(), WorkContext::new()).unwrap();
    let result = fixture
        .db
        .read(WorkContext::new(), |frame| {
            prepared.execute_complete(frame, no_params())
        })
        .unwrap();
    assert_eq!(result.len(), fixture.labels.len() as u64);
    let mut seen = vec![false; fixture.labels.len()];
    let work = WorkContext::new();
    #[cfg(feature = "alloc-counter")]
    let before = bumbledb::alloc_counter::snapshot().window;
    result
        .visit_rows(&work, |row| {
            let mut values = row.values();
            assert_eq!(values.len(), 2);
            let Some(AnswerValue::U64(id)) = values.next() else {
                panic!("id")
            };
            let Some(AnswerValue::String(label)) = values.next() else {
                panic!("label")
            };
            let index = usize::try_from(id).unwrap();
            assert!(!seen[index]);
            seen[index] = true;
            assert_eq!(label, fixture.labels[index]);
            assert!(values.next().is_none());
            Ok(())
        })
        .unwrap();
    #[cfg(feature = "alloc-counter")]
    assert_eq!(bumbledb::alloc_counter::snapshot().window, before);
    assert!(seen.iter().all(|seen| *seen));

    let first_pointer = match result.rows().next().unwrap().values().nth(1).unwrap() {
        AnswerValue::String(text) => text.as_ptr(),
        _ => panic!("text"),
    };
    #[cfg(feature = "alloc-counter")]
    let before = bumbledb::alloc_counter::snapshot().window;
    let answers = result.into_answers();
    #[cfg(feature = "alloc-counter")]
    assert_eq!(bumbledb::alloc_counter::snapshot().window, before);
    assert!(
        matches!(answers.get(0, 1), AnswerValue::String(text) if text.as_ptr() == first_pointer)
    );

    let complete = fixture
        .db
        .read(WorkContext::new(), |frame| {
            prepared.execute_complete(frame, no_params())
        })
        .unwrap();
    let mut cursor = complete.into_cursor(17);
    let mut total = 0;
    #[cfg(feature = "alloc-counter")]
    let before = bumbledb::alloc_counter::snapshot().window;
    loop {
        let mut ticket = DeliveryTicket::open(&mut cursor);
        let rows = ticket
            .visit_page(&work, |row| {
                assert_eq!(row.values().len(), 2);
                total += 1;
                Ok(())
            })
            .unwrap();
        let Some(rows) = rows else { break };
        assert!((1..=17).contains(&rows));
        ticket.commit();
    }
    #[cfg(feature = "alloc-counter")]
    assert_eq!(bumbledb::alloc_counter::snapshot().window, before);
    assert_eq!(total, fixture.labels.len());
}

#[test]
fn failed_cancelled_and_panicking_visitors_cannot_advance_the_cursor() {
    let fixture = fixture(9, 32);
    let source_work = WorkContext::new();
    let mut prepared = fixture.db.prepare(&query(), source_work.clone()).unwrap();
    let result = fixture
        .db
        .read(source_work.clone(), |frame| {
            prepared.execute_complete(frame, no_params())
        })
        .unwrap();
    source_work.cancel();
    prepared.release_memory();
    fixture.db.clear_cache();
    let mut cursor = result.into_cursor(3);
    let first_id = |row: bumbledb::ResultRow<'_>| match row.values().next().unwrap() {
        AnswerValue::U64(id) => id,
        _ => panic!("id"),
    };

    let mut initial = None;
    let mut ticket = DeliveryTicket::open(&mut cursor);
    assert!(
        ticket
            .visit_page(&WorkContext::new(), |row| {
                initial = Some(first_id(row));
                Err(Error::ForeignPreparedQuery)
            })
            .is_err()
    );
    ticket.commit();

    let cancelled = WorkContext::new();
    let mut ticket = DeliveryTicket::open(&mut cursor);
    assert!(
        ticket
            .visit_page(&cancelled, |row| {
                assert_eq!(Some(first_id(row)), initial);
                cancelled.cancel();
                Ok(())
            })
            .is_err()
    );
    ticket.commit();

    let mut ticket = DeliveryTicket::open(&mut cursor);
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = ticket.visit_page(&WorkContext::new(), |row| {
            assert_eq!(Some(first_id(row)), initial);
            panic!("destination failed")
        });
    }));
    assert!(panic.is_err());
    ticket.commit();

    let first = cursor.next_page(&WorkContext::new()).unwrap().unwrap();
    assert_eq!(first.rows.len(), 3);
    assert!(!first.terminal);
    assert!(matches!(first.rows.get(0, 0), AnswerValue::U64(id) if Some(id) == initial));
    let mut count = first.rows.len();
    while let Some(page) = cursor.next_page(&WorkContext::new()).unwrap() {
        count += page.rows.len();
    }
    assert_eq!(count, fixture.labels.len());
}

#[test]
fn query_release_and_cache_clear_preserve_old_readers_and_completed_results() {
    let fixture = fixture(1, 8);
    let work = WorkContext::new();
    let old = fixture.db.snapshot(&work).unwrap();
    let mut prepared = fixture.db.prepare(&query(), work.clone()).unwrap();
    let complete = prepared
        .execute_complete(&old.frame(&work), no_params())
        .unwrap();
    prepared.release_memory();
    fixture.db.clear_cache();
    fixture
        .db
        .write(work.clone(), |tx| {
            tx.delete_dyn(
                RelationId(0),
                [vec![
                    Value::U64(0),
                    Value::String(fixture.labels[0].as_str().into()),
                    Value::U64(0),
                ]],
            )?;
            tx.insert_dyn(
                RelationId(0),
                [vec![
                    Value::U64(0),
                    Value::String("new".into()),
                    Value::U64(0),
                ]],
            )?;
            Ok(())
        })
        .unwrap()
        .unwrap();
    for _ in 0..16 {
        let old_result = prepared
            .execute_complete(&old.frame(&work), no_params())
            .unwrap();
        assert!(
            matches!(old_result.into_answers().get(0, 1), AnswerValue::String(label) if label == fixture.labels[0])
        );
        let current = fixture
            .db
            .read(work.clone(), |frame| {
                prepared.execute_complete(frame, no_params())
            })
            .unwrap();
        assert!(matches!(
            current.into_answers().get(0, 1),
            AnswerValue::String("new")
        ));
    }
    assert!(
        matches!(complete.into_answers().get(0, 1), AnswerValue::String(label) if label == fixture.labels[0])
    );
}
