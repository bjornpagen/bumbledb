//! Ownership, exact values, cancellation and publication of completed results.
use super::*;
use crate::error::Error;
use crate::work::WorkError;

fn work() -> WorkContext {
    WorkContext::new()
}

fn heap_identity() -> ResultIdentity {
    ResultIdentity {
        source: PinnedSource::Heap(crate::schema::fingerprint::SchemaFingerprint([0; 32])),
        generation: None,
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "fixture rows stay far below 2^52"
)]
fn sample_answers(rows: u64) -> Answers {
    let mut answers = Answers::new();
    answers.begin(3);
    for i in 0..rows {
        answers.push_value(&AnswerValue::U64(i));
        answers.push_value(&AnswerValue::String(&format!("row-{i}")));
        answers.push_value(&AnswerValue::F64(bumbledb_theory::F64::from(
            i as f64 * 0.5,
        )));
    }
    answers
}

#[expect(
    clippy::cast_precision_loss,
    reason = "fixture rows stay far below 2^52"
)]
fn assert_rows(collected: &Answers, expected_rows: u64) {
    assert_eq!(collected.len() as u64, expected_rows);
    for i in 0..expected_rows {
        let row = usize::try_from(i).expect("small fixture");
        assert_eq!(collected.get(row, 0), AnswerValue::U64(i));
        assert_eq!(
            collected.get(row, 1),
            AnswerValue::String(&format!("row-{i}"))
        );
        assert_eq!(
            collected.get(row, 2),
            AnswerValue::F64(bumbledb_theory::F64::from(i as f64 * 0.5))
        );
    }
}

fn sealed(rows: u64) -> CompleteResult {
    CompleteResult::seal(sample_answers(rows), heap_identity(), &work()).unwrap()
}

#[test]
fn sealing_and_consuming_result_move_the_same_storage() {
    let answers = sample_answers(32);
    let pointers = (
        answers.cells.as_ptr(),
        answers.text.as_ptr(),
        answers.blob.as_ptr(),
    );
    let result = CompleteResult::seal(answers, heap_identity(), &work()).unwrap();
    assert_eq!(result.identity(), heap_identity());
    assert_eq!(result.len(), 32);
    assert_eq!(result.arity(), 3);
    let moved = result.into_answers();
    assert_eq!(
        (
            moved.cells.as_ptr(),
            moved.text.as_ptr(),
            moved.blob.as_ptr()
        ),
        pointers
    );
    assert_rows(&moved, 32);
}

#[test]
fn the_cursor_consumes_the_result_and_frames_the_terminal_page() {
    let mut cursor = sealed(7).into_cursor(3);
    assert_eq!(cursor.identity(), heap_identity());
    let mut delivered = 0;
    for (size, terminal) in [(3, false), (3, false), (1, true)] {
        let page = cursor.next_page(&work()).unwrap().unwrap();
        assert_eq!(page.rows.len(), size);
        assert_eq!(page.terminal, terminal);
        for row in page.rows.answers() {
            assert_eq!(row.get(0), AnswerValue::U64(delivered));
            delivered += 1;
        }
    }
    assert_eq!(delivered, 7);
    assert!(cursor.next_page(&work()).unwrap().is_none());
    assert!(cursor.next_page(&work()).unwrap().is_none());
}

#[test]
fn empty_results_frame_completion_and_zero_page_size_makes_progress() {
    let mut cursor = sealed(0).into_cursor(4);
    let page = cursor.next_page(&work()).unwrap().unwrap();
    assert_eq!(page.rows.arity(), 3);
    assert!(page.rows.is_empty());
    assert!(page.terminal);
    assert!(cursor.next_page(&work()).unwrap().is_none());
    let mut cursor = sealed(2).into_cursor(0);
    assert_eq!(cursor.next_page(&work()).unwrap().unwrap().rows.len(), 1);
    assert!(cursor.next_page(&work()).unwrap().unwrap().terminal);
    assert!(cursor.next_page(&work()).unwrap().is_none());
}

#[test]
fn delivery_context_is_independent_of_completed_execution() {
    let execute = work();
    let result = CompleteResult::seal(sample_answers(8), heap_identity(), &execute).unwrap();
    execute.cancel();
    assert!(
        result
            .visit_rows(&execute, |_| panic!("cancelled visitor"))
            .is_err()
    );
    drop(execute);
    let mut seen = 0;
    result
        .visit_rows(&work(), |row| {
            assert_eq!(row.values().next().unwrap(), AnswerValue::U64(seen));
            seen += 1;
            Ok(())
        })
        .unwrap();
    assert_eq!(seen, 8);
    assert_rows(&result.into_answers(), 8);
}

#[test]
fn cancellation_cannot_seal_a_complete_owner() {
    let execute = work();
    let answers = sample_answers(3);
    execute.cancel();
    assert!(
        matches!(CompleteResult::seal(answers, heap_identity(), &execute),
        Err(Error::Store(error)) if matches!(*error, crate::store::StoreError::Work(WorkError::Cancelled)))
    );
}

#[test]
fn adopt_and_abort_leaves_nothing_a_fresh_ticket_can_commit() {
    let mut cursor = sealed(3).into_cursor(2);
    let mut ticket = DeliveryTicket::open(&mut cursor);
    ticket.preview_page(&work()).unwrap().unwrap();
    let adopted = ticket.adopt().unwrap();
    assert_rows(&adopted, 2);
    ticket.abort();
    assert_eq!(cursor.debug_next_row(), 0);
    DeliveryTicket::open(&mut cursor).commit();
    assert_eq!(cursor.debug_next_row(), 0);
    assert_rows(&cursor.next_page(&work()).unwrap().unwrap().rows, 2);
    drop(cursor);
    assert_rows(&adopted, 2);
}

#[test]
fn dropping_a_successful_ticket_does_not_advance() {
    let mut cursor = sealed(3).into_cursor(2);
    {
        let mut ticket = DeliveryTicket::open(&mut cursor);
        ticket.preview_page(&work()).unwrap();
        assert_eq!(ticket.previewed_rows(), 2);
    }
    assert_eq!(cursor.debug_next_row(), 0);
    assert_rows(&cursor.next_page(&work()).unwrap().unwrap().rows, 2);
}

#[test]
fn repeated_preview_cancellation_discards_the_old_pending_advance() {
    let mut cursor = sealed(3).into_cursor(2);
    let delivery = work();
    let mut ticket = DeliveryTicket::open(&mut cursor);
    ticket.preview_page(&delivery).unwrap();
    assert_eq!(ticket.previewed_rows(), 2);
    let adopted = ticket.adopt().unwrap();
    delivery.cancel();
    assert!(ticket.preview_page(&delivery).is_err());
    assert_eq!(ticket.previewed_rows(), 0);
    assert!(!ticket.will_be_terminal());
    assert!(ticket.adopt().is_none());
    ticket.commit();
    assert_eq!(cursor.debug_next_row(), 0);
    assert_rows(&adopted, 2);
    assert_rows(&cursor.next_page(&work()).unwrap().unwrap().rows, 2);
}

#[test]
fn borrowed_page_cancellation_at_first_or_last_row_retries_the_same_page() {
    let mut cursor = sealed(3).into_cursor(3);
    for cancel_at in [1, 3] {
        let delivery = work();
        let mut ticket = DeliveryTicket::open(&mut cursor);
        let mut seen = 0;
        let failed = ticket.visit_page(&delivery, |row| {
            assert_eq!(row.values().next().unwrap(), AnswerValue::U64(seen));
            seen += 1;
            if seen == cancel_at {
                delivery.cancel();
            }
            Ok(())
        });
        assert!(matches!(failed, Err(Error::Store(error))
            if matches!(*error, crate::store::StoreError::Work(WorkError::Cancelled))));
        assert_eq!(seen, cancel_at);
        assert_eq!(ticket.previewed_rows(), 0);
        assert!(ticket.adopt().is_none());
        ticket.commit();
        assert_eq!(cursor.debug_next_row(), 0);
    }
    let mut ticket = DeliveryTicket::open(&mut cursor);
    let mut seen = 0;
    assert_eq!(
        ticket
            .visit_page(&work(), |row| {
                assert_eq!(row.values().next().unwrap(), AnswerValue::U64(seen));
                seen += 1;
                Ok(())
            })
            .unwrap(),
        Some(3)
    );
    assert!(
        ticket.adopt().is_none(),
        "borrowed delivery creates no Answers"
    );
    assert!(ticket.will_be_terminal());
    ticket.commit();
    assert_eq!(cursor.debug_next_row(), 3);
}

#[test]
fn failed_or_panicking_destination_cannot_commit_a_prefix() {
    let mut cursor = sealed(4).into_cursor(3);
    let mut ticket = DeliveryTicket::open(&mut cursor);
    let mut seen = 0;
    let failure = ticket.visit_page(&work(), |_| {
        seen += 1;
        if seen == 2 {
            return Err(Error::ResultBytesOverflow);
        }
        Ok(())
    });
    assert_eq!(failure, Err(Error::ResultBytesOverflow));
    assert_eq!(ticket.previewed_rows(), 0);
    ticket.commit();
    let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut ticket = DeliveryTicket::open(&mut cursor);
        let _ = ticket.visit_page(&work(), |_| panic!("destination failed"));
    }));
    assert!(unwind.is_err());
    assert_eq!(cursor.debug_next_row(), 0);
    assert_rows(&cursor.next_page(&work()).unwrap().unwrap().rows, 3);
}

#[test]
fn result_value_iterator_borrows_payloads_and_preserves_every_value_kind() {
    use bumbledb_theory::{F64, Interval, Uuid};
    let values = [
        AnswerValue::Bool(false),
        AnswerValue::Bool(true),
        AnswerValue::U64(u64::MAX),
        AnswerValue::I64(i64::MIN),
        AnswerValue::F64(F64::from(0.5)),
        AnswerValue::F64(F64::NAN),
        AnswerValue::String("borrowed 🐝\0text"),
        AnswerValue::FixedBytes(&[0, 255, 128, 42]),
        AnswerValue::Uuid(Uuid::from_u128(u128::MAX)),
        AnswerValue::IntervalU64(Interval::new(0, u64::MAX).unwrap()),
        AnswerValue::IntervalI64(Interval::new(i64::MIN, i64::MAX).unwrap()),
        AnswerValue::IntervalF64(Interval::new(F64::NEG_INFINITY, F64::INFINITY).unwrap()),
    ];
    let mut answers = Answers::new();
    answers.begin(values.len());
    for value in &values {
        answers.push_value(value);
    }
    let text_range = answers.text.as_bytes().as_ptr_range();
    let blob_range = answers.blob.as_ptr_range();
    let result = CompleteResult::seal(answers, heap_identity(), &work()).unwrap();
    for _ in 0..2 {
        let mut count = 0;
        result
            .visit_rows(&work(), |row| {
                assert_eq!(row.arity(), values.len());
                let mut actual = row.values();
                assert_eq!(actual.len(), values.len());
                for expected in values {
                    let value = actual.next().unwrap();
                    assert_eq!(value, expected);
                    match value {
                        AnswerValue::String(text) => assert!(text_range.contains(&text.as_ptr())),
                        AnswerValue::FixedBytes(bytes) => {
                            assert!(blob_range.contains(&bytes.as_ptr()));
                        }
                        _ => {}
                    }
                }
                assert!(actual.next().is_none());
                assert!(actual.next().is_none());
                count += 1;
                Ok(())
            })
            .unwrap();
        assert_eq!(count, 1);
    }
}

#[cfg(feature = "alloc-counter")]
#[test]
fn borrowing_completed_rows_and_pages_does_not_allocate() {
    let result = sealed(512);
    let delivery = work();
    for _ in 0..2 {
        let mut count = 0;
        let before = crate::alloc_counter::snapshot().window;
        result
            .visit_rows(&delivery, |row| {
                for value in row.values() {
                    std::hint::black_box(value);
                }
                count += 1;
                Ok(())
            })
            .unwrap();
        let after = crate::alloc_counter::snapshot().window;
        assert_eq!(count, 512);
        assert_eq!(after.allocs, before.allocs);
        assert_eq!(after.alloc_bytes, before.alloc_bytes);
    }
    let mut cursor = result.into_cursor(7);
    let before = crate::alloc_counter::snapshot();
    let mut total = 0;
    loop {
        let mut ticket = DeliveryTicket::open(&mut cursor);
        let Some(count) = ticket
            .visit_page(&delivery, |row| {
                for value in row.values() {
                    std::hint::black_box(value);
                }
                Ok(())
            })
            .unwrap()
        else {
            break;
        };
        total += count;
        ticket.commit();
    }
    let after = crate::alloc_counter::snapshot();
    assert_eq!(total, 512);
    assert_eq!(after.window.allocs, before.window.allocs);
    assert_eq!(after.window.alloc_bytes, before.window.alloc_bytes);
    assert_eq!(after.absolute.live_bytes, before.absolute.live_bytes);
}

#[cfg(feature = "alloc-counter")]
#[test]
fn large_result_seals_without_another_copy_or_size_directory() {
    let execute = work();
    let mut answers = Answers::new();
    answers.begin(1);
    answers.push_value(&AnswerValue::String(&"x".repeat(9 << 20)));
    let address = answers.text.as_ptr();
    let before = crate::alloc_counter::snapshot();
    let result = CompleteResult::seal(answers, heap_identity(), &execute).unwrap();
    let after = crate::alloc_counter::snapshot();
    assert_eq!(after.window.allocs, before.window.allocs);
    assert_eq!(after.absolute.live_bytes, before.absolute.live_bytes);
    let moved = result.into_answers();
    assert_eq!(moved.text.as_ptr(), address);
    assert_eq!(moved.text.len(), 9 << 20);
}
