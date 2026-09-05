//! Sealed-result behavior: RAM and scratch backings agree, collect caps
//! honestly, the cursor's terminal framing is explicit, and result bytes
//! are charged. Gate anchors: Q-ATOMIC / Q-DISK / Q-LIFETIME (result
//! halves), QRY-001/002.

use super::*;
use crate::api::prepared::source::{PinnedSource, UNBOUNDED_POLICY};

fn work() -> WorkContext {
    UNBOUNDED_POLICY.start().expect("unbounded ledger")
}

fn heap_identity() -> ResultIdentity {
    ResultIdentity {
        source: PinnedSource::Heap,
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

#[test]
fn ram_and_scratch_backings_agree_on_every_cell() {
    let work = work();
    let ram = CompleteResult::seal(sample_answers(32), heap_identity(), &work, usize::MAX)
        .expect("seal RAM");
    // Forcing the allowance to zero moves the same rows into scratch.
    let scratch =
        CompleteResult::seal(sample_answers(32), heap_identity(), &work, 0).expect("seal scratch");
    let mut ram = ram;
    let mut scratch = scratch;
    assert_eq!(ram.len(), 32);
    assert_eq!(scratch.len(), 32);
    let a = ram.collect(u64::MAX).expect("collect RAM");
    let b = scratch.collect(u64::MAX).expect("collect scratch");
    assert_rows(&a, 32);
    assert_rows(&b, 32);
}

#[test]
fn collect_cap_refuses_and_leaves_the_backing_available() {
    let work = work();
    let mut sealed =
        CompleteResult::seal(sample_answers(10), heap_identity(), &work, 0).expect("seal");
    let refused = sealed.collect(5);
    assert!(matches!(refused, Err(Error::ResultBytesOverflow)));
    // The sealed backing is still whole after the cap refusal.
    let collected = sealed.collect(10).expect("collect after refusal");
    assert_rows(&collected, 10);
}

#[test]
fn the_cursor_consumes_the_result_and_frames_the_terminal_page() {
    let work = work();
    let sealed = CompleteResult::seal(sample_answers(7), heap_identity(), &work, 0).expect("seal");
    let mut cursor = sealed.into_cursor(3);
    let mut delivered = 0u64;
    let mut pages = 0;
    let mut saw_terminal = false;
    while let Some(page) = cursor.next_page().expect("page") {
        pages += 1;
        delivered += page.rows.len() as u64;
        if page.terminal {
            saw_terminal = true;
        }
    }
    assert_eq!(delivered, 7, "every sealed row is delivered exactly once");
    assert_eq!(pages, 3, "7 rows over pages of 3");
    assert!(saw_terminal, "the terminal frame is explicit");
    assert!(
        cursor.next_page().expect("spent cursor").is_none(),
        "a drained cursor stays drained"
    );
}

#[test]
fn an_empty_result_still_frames_completion() {
    let work = work();
    let mut answers = Answers::new();
    answers.begin(2);
    let sealed = CompleteResult::seal(answers, heap_identity(), &work, usize::MAX).expect("seal");
    let mut cursor = sealed.into_cursor(4);
    let page = cursor.next_page().expect("page").expect("one frame");
    assert!(page.rows.is_empty());
    assert!(page.terminal, "empty complete sets are still complete");
}

/// The chapter-35 retained-result seam: a sealed result kept past its
/// execute operation stops charging that operation's ledger once rebound.
/// Reads under the exhausted execute ledger refuse typed (the defect this
/// closes made them refuse SPURIOUSLY, forever); after `rebind_work` the
/// same sealed rows read whole under the fresh ledger, and every release
/// stays exactly-once (the sealed-byte charge keeps its origin).
#[test]
fn rebinding_re_homes_retained_scratch_reads_onto_a_fresh_ledger() {
    // The execute operation's ledger: enough to seal, bounded work units so
    // its post-execute exhaustion is deterministic (the test stand-in for
    // an expired deadline).
    let execute = crate::work::ExecutionPolicy {
        work_units: 4096,
        ..UNBOUNDED_POLICY
    }
    .start()
    .expect("start");
    let mut sealed = CompleteResult::seal(sample_answers(16), heap_identity(), &execute, 0)
        .expect("seal scratch");
    let charge = sealed.byte_len();
    assert!(charge > 0, "sealed rows hold a result-byte charge");

    // The execute operation ends; its ledger runs out.
    while execute.step(1).is_ok() {}

    // Un-rebound, retained scratch reads still consult the exhausted
    // execute ledger — the typed refusal, with the backing left whole.
    let refused = sealed.collect(u64::MAX);
    assert!(
        matches!(refused, Err(Error::Store(_))),
        "reads under the exhausted execute ledger refuse typed"
    );

    // Re-homed onto the retaining caller's fresh ledger, the same sealed
    // rows read whole — repeatedly (collect leaves the backing).
    let retained = work();
    sealed.rebind_work(&retained);
    assert_rows(&sealed.collect(u64::MAX).expect("collect rebound"), 16);
    assert_rows(&sealed.collect(u64::MAX).expect("collect again"), 16);
    assert_eq!(
        sealed.byte_len(),
        charge,
        "rebinding never re-prices the sealed charge"
    );

    // The consuming cursor carries the charge and the rebound backing, and
    // rebinds again on its own: a zero-work ledger refuses the page read,
    // a fresh one delivers the complete set with its terminal frame.
    let mut cursor = sealed.into_cursor(5);
    assert_eq!(cursor.byte_len(), charge);
    let zero_work = crate::work::ExecutionPolicy {
        work_units: 0,
        ..UNBOUNDED_POLICY
    }
    .start()
    .expect("start");
    cursor.rebind_work(&zero_work);
    assert!(
        matches!(cursor.next_page(), Err(Error::Store(_))),
        "cursor pages charge the ledger the cursor is bound to"
    );
    cursor.rebind_work(&work());
    let mut delivered = 0u64;
    let mut saw_terminal = false;
    while let Some(page) = cursor.next_page().expect("page") {
        delivered += page.rows.len() as u64;
        if page.terminal {
            saw_terminal = true;
        }
    }
    assert_eq!(delivered, 16, "the failed page delivered nothing twice");
    assert!(saw_terminal, "the rebound cursor completes with its frame");
}

/// Rebinding a RAM-backed result is a harmless no-op: RAM reads never
/// consult a ledger, so a retained RAM result reads whole even under an
/// exhausted execute ledger, before and after rebinding.
#[test]
fn ram_backed_results_read_without_a_ledger_and_rebind_is_a_no_op() {
    let execute = crate::work::ExecutionPolicy {
        work_units: 4096,
        ..UNBOUNDED_POLICY
    }
    .start()
    .expect("start");
    let mut sealed = CompleteResult::seal(sample_answers(8), heap_identity(), &execute, usize::MAX)
        .expect("seal RAM");
    while execute.step(1).is_ok() {}
    assert_rows(&sealed.collect(u64::MAX).expect("RAM collect"), 8);
    sealed.rebind_work(&work());
    assert_rows(&sealed.collect(u64::MAX).expect("still whole"), 8);
}

#[test]
fn sealed_results_charge_the_result_ledger() {
    let context = crate::work::ExecutionPolicy {
        result_bytes: 64,
        ..UNBOUNDED_POLICY
    }
    .start()
    .expect("start");
    let refused = CompleteResult::seal(sample_answers(64), heap_identity(), &context, usize::MAX);
    assert!(
        refused.is_err(),
        "64 result bytes cannot own a kilobyte of text"
    );
}

/// D12/D25: two rows that fit individually but not together become two
/// successful pages. Predelivery refusal after copy returns no data and
/// retries at the same row. Verification: NotRun.
#[test]
fn d25_two_row_page_cap_and_predelivery_abort() {
    let work = work();
    let mut answers = Answers::new();
    answers.begin(1);
    answers.push_value(&AnswerValue::String("aaaaaaaa"));
    answers.push_value(&AnswerValue::String("bbbbbbbb"));
    answers.push_value(&AnswerValue::String("c"));
    let sealed = CompleteResult::seal(answers, heap_identity(), &work, usize::MAX).expect("seal");
    let mut cursor = sealed.into_cursor(8);
    let mut ticket = DeliveryTicket::open(&mut cursor);
    let row1_bytes = {
        let preview = ticket
            .preview_page(&work, 24)
            .expect("row1 fits")
            .expect("nonempty");
        assert_eq!(preview.len(), 1, "row2 must not join row1");
        super::logical_bytes_for_test(preview)
    };
    let adopted = ticket.adopt().expect("preview");
    assert_eq!(adopted.len(), 1);
    let charge = work
        .reserve(crate::work::ByteKind::Result, row1_bytes)
        .expect("register output");
    drop(charge);
    ticket.commit();

    let mut ticket = DeliveryTicket::open(&mut cursor);
    ticket.preview_page(&work, 24).expect("row2 preview");
    ticket.abort();
    assert_eq!(
        cursor.len().saturating_sub(cursor_next_for_test(&cursor)),
        2,
        "abort leaves row2 undelivered"
    );

    let tiny = crate::work::ExecutionPolicy {
        result_bytes: 1,
        ..UNBOUNDED_POLICY
    }
    .start()
    .expect("tiny");
    let refused = cursor.next_page_with_work(&tiny, 1);
    assert!(refused.is_err(), "predelivery refusal returns no page");
    assert_eq!(
        cursor.debug_next_row(),
        1,
        "predelivery refusal leaves the cursor on the refused row"
    );
    let retry = cursor
        .next_page_with_work(&work, 24)
        .expect("retry after abort")
        .expect("row2 still there");
    assert_eq!(retry.rows.len(), 1);

    cursor.inject_backing_failure(Error::Corruption(
        crate::error::CorruptionError::MalformedValue("result row sequence"),
    ));
    assert!(cursor.next_page().is_err(), "backing failure is failed");
    assert!(
        cursor.next_page().is_err(),
        "a failed cursor never becomes EOF"
    );
    assert!(
        cursor.next_page_with_work(&work, 64).is_err(),
        "later pulls stay failed"
    );
}

/// Oversized first row: fit is refused from the sealed lengths, the
/// cursor stays on that row, and a later admitted pull still delivers it.
/// Verification: NotRun.
#[test]
fn oversized_first_row_refuses_with_cursor_unchanged() {
    let work = work();
    let mut answers = Answers::new();
    answers.begin(1);
    answers.push_value(&AnswerValue::String("this-row-is-too-large-for-eight-bytes"));
    answers.push_value(&AnswerValue::String("ok"));
    for ram_allowance in [usize::MAX, 0] {
        let sealed =
            CompleteResult::seal(clone_answers(&answers), heap_identity(), &work, ram_allowance)
                .expect("seal");
        let mut cursor = sealed.into_cursor(8);
        let mut ticket = DeliveryTicket::open(&mut cursor);
        let before = work.used(crate::work::Resource::ResultBytes);
        let refused = ticket.preview_page(&work, 8);
        assert!(
            refused.is_err(),
            "first row exceeds eight encoded bytes, got {refused:?}"
        );
        assert_eq!(
            ticket.preview_charged_bytes(),
            0,
            "oversized refuse does not take a preview reservation"
        );
        assert_eq!(
            work.used(crate::work::Resource::ResultBytes),
            before,
            "oversized refuse leaves no preview charge"
        );
        drop(ticket);
        assert_eq!(
            work.used(crate::work::Resource::ResultBytes),
            before,
            "refused oversized row left no uncharged allocation to refund"
        );
        assert_eq!(
            cursor.debug_next_row(),
            0,
            "predelivery refusal leaves next_row"
        );
        let retry = cursor
            .next_page_with_work(&work, 1024)
            .expect("retry")
            .expect("row still there");
        assert_eq!(retry.rows.len(), 1);
        assert_eq!(retry.rows.get(0, 0), AnswerValue::String("this-row-is-too-large-for-eight-bytes"));
    }
}

fn clone_answers(answers: &Answers) -> Answers {
    let mut copy = Answers::new();
    copy.begin(answers.arity());
    for row in 0..answers.len() {
        for column in 0..answers.arity() {
            copy.push_value(&answers.get(row, column));
        }
    }
    copy
}

/// Several rows that jointly fit become one page. Size comes from the
/// sealed cells / one scratch load — the page is not encoded into an
/// uncharged Vec and then rejected. `into_cursor(page_rows)` is the cap.
/// Verification: NotRun.
#[test]
fn multirow_page_fits_under_byte_allowance() {
    let work = work();
    let mut answers = Answers::new();
    answers.begin(1);
    answers.push_value(&AnswerValue::U64(1));
    answers.push_value(&AnswerValue::U64(2));
    answers.push_value(&AnswerValue::U64(3));
    // One U64 encodes as 9 bytes; three fit in 32. Row cap 2 must win.
    for ram_allowance in [usize::MAX, 0] {
        let sealed =
            CompleteResult::seal(clone_answers(&answers), heap_identity(), &work, ram_allowance)
                .expect("seal");
        let mut cursor = sealed.into_cursor(2);
        let mut ticket = DeliveryTicket::open(&mut cursor);
        let preview = ticket
            .preview_page(&work, 32)
            .expect("preview")
            .expect("nonempty");
        assert_eq!(
            preview.len(),
            2,
            "byte allowance admits three; page_rows is 2"
        );
        assert_eq!(preview.get(0, 0), AnswerValue::U64(1));
        assert_eq!(preview.get(1, 0), AnswerValue::U64(2));
        assert_eq!(cursor.debug_next_row(), 0);
        assert!(
            ticket.preview_charged_bytes() > 0,
            "multirow preview reserved before copy"
        );
        let adopted = ticket.adopt().expect("adopt");
        let charge = work
            .reserve(crate::work::ByteKind::Result, super::logical_bytes_for_test(&adopted))
            .expect("register output");
        drop(charge);
        ticket.commit();
        assert_eq!(cursor.debug_next_row(), 2);

        let mut ticket = DeliveryTicket::open(&mut cursor);
        let last = ticket
            .preview_page(&work, 32)
            .expect("last page")
            .expect("row 3");
        assert_eq!(last.len(), 1);
        assert_eq!(last.get(0, 0), AnswerValue::U64(3));
        ticket.abort();
        assert_eq!(cursor.debug_next_row(), 2);
    }
}

#[test]
fn cancelled_preview_aborts_without_advancing() {
    let work = work();
    let sealed = CompleteResult::seal(sample_answers(3), heap_identity(), &work, usize::MAX)
        .expect("seal");
    let mut cursor = sealed.into_cursor(8);
    work.cancel();
    let mut ticket = DeliveryTicket::open(&mut cursor);
    assert!(
        ticket.preview_page(&work, 1024).is_err(),
        "cancelled work is a resource abort"
    );
    drop(ticket);
    assert_eq!(cursor.debug_next_row(), 0);
}

fn cursor_next_for_test(cursor: &ResultCursor) -> u64 {
    cursor.debug_next_row()
}

/// Preview growth is reserved onto the ticket before copy; abort refunds
/// that owner. An oversized first row never takes a charge.
/// Verification: NotRun.
#[test]
fn preview_growth_is_charged_and_oversized_leaves_no_uncharged_alloc() {
    let work = work();
    let mut answers = Answers::new();
    answers.begin(1);
    answers.push_value(&AnswerValue::String("aaaaaaaa"));
    answers.push_value(&AnswerValue::String("bbbbbbbb"));
    let sealed = CompleteResult::seal(answers, heap_identity(), &work, usize::MAX).expect("seal");
    let baseline = work.used(crate::work::Resource::ResultBytes);
    let mut cursor = sealed.into_cursor(8);

    let mut ticket = DeliveryTicket::open(&mut cursor);
    let refused = ticket.preview_page(&work, 8);
    assert!(refused.is_err(), "first row does not fit eight bytes");
    assert_eq!(ticket.preview_charged_bytes(), 0);
    assert_eq!(work.used(crate::work::Resource::ResultBytes), baseline);
    drop(ticket);
    assert_eq!(
        work.used(crate::work::Resource::ResultBytes),
        baseline,
        "oversized refuse did not leave a live or refundable uncharged buffer"
    );
    assert_eq!(cursor.debug_next_row(), 0);

    let mut ticket = DeliveryTicket::open(&mut cursor);
    let preview = ticket
        .preview_page(&work, 24)
        .expect("row1 fits")
        .expect("nonempty");
    assert_eq!(preview.len(), 1);
    let charged = ticket.preview_charged_bytes();
    assert!(charged > 0, "admitted preview reserved before copy");
    assert_eq!(
        work.used(crate::work::Resource::ResultBytes),
        baseline + charged,
        "the reservation stays on the ticket, not a dropped temp"
    );
    ticket.abort();
    assert_eq!(
        work.used(crate::work::Resource::ResultBytes),
        baseline,
        "abort refunds the ticket-owned preview charge"
    );
    assert_eq!(cursor.debug_next_row(), 0);

    let page = cursor
        .next_page_with_work(&work, 24)
        .expect("retry")
        .expect("row1");
    assert_eq!(page.rows.len(), 1);
    assert!(
        page.charged_bytes() > 0,
        "published page owns the preview reservation"
    );
    let after_page = work.used(crate::work::Resource::ResultBytes);
    assert!(after_page > baseline);
    drop(page);
    assert_eq!(
        work.used(crate::work::Resource::ResultBytes),
        baseline,
        "dropping the page refunds its owned charge"
    );
}

/// Consume available result capacity, attempt a pull that fits its
/// caller cap but cannot reserve overlap, then retry the **same** cursor
/// with sufficient resources and receive the same first row. A budget
/// refusal is not a sticky cursor failure.
/// Verification: NotRun.
#[test]
fn resource_refusal_retries_same_first_row() {
    let seal = work();
    let mut answers = Answers::new();
    answers.begin(1);
    answers.push_value(&AnswerValue::String("first-row-payload"));
    answers.push_value(&AnswerValue::String("second"));
    for ram_allowance in [usize::MAX, 0] {
        let sealed =
            CompleteResult::seal(clone_answers(&answers), heap_identity(), &seal, ram_allowance)
                .expect("seal");
        let mut cursor = sealed.into_cursor(8);
        let tight = crate::work::ExecutionPolicy {
            result_bytes: 8,
            ..UNBOUNDED_POLICY
        }
        .start()
        .expect("tight delivery ledger");
        cursor.rebind_work(&tight);
        let mut ticket = DeliveryTicket::open(&mut cursor);
        let refused = ticket.preview_page(&tight, 1024);
        assert!(
            refused.is_err(),
            "row fits the caller cap but cannot reserve result overlap"
        );
        drop(ticket);
        assert_eq!(
            cursor.debug_next_row(),
            0,
            "resource abort leaves next_row"
        );
        let retry = cursor
            .next_page_with_work(&seal, 1024)
            .expect("retry after resource abort must not be a failed cursor")
            .expect("same first row");
        assert_eq!(retry.rows.len(), 1);
        assert_eq!(
            retry.rows.get(0, 0),
            AnswerValue::String("first-row-payload")
        );
    }
}

/// True backing / corruption failure stays terminal: later pulls return
/// that error, never EOF or a successful page.
/// Verification: NotRun.
#[test]
fn backing_failure_stays_terminal() {
    let work = work();
    let sealed = CompleteResult::seal(sample_answers(2), heap_identity(), &work, usize::MAX)
        .expect("seal");
    let mut cursor = sealed.into_cursor(8);
    cursor.inject_backing_failure(Error::Corruption(
        crate::error::CorruptionError::MalformedValue("result row sequence"),
    ));
    assert!(cursor.next_page().is_err(), "injected backing failure");
    assert!(
        cursor.next_page().is_err(),
        "a failed cursor never becomes EOF"
    );
    assert!(
        cursor.next_page_with_work(&work, 64).is_err(),
        "later pulls stay failed"
    );
    assert_eq!(cursor.debug_next_row(), 0, "failure does not advance");
}

/// Adopt-and-abort discards the ticket-local pending advance. A fresh
/// unpreviewed ticket's `commit` must not steal that abandoned page.
/// Verification: NotRun.
#[test]
fn adopt_and_abort_leaves_nothing_a_fresh_ticket_can_commit() {
    let work = work();
    let mut answers = Answers::new();
    answers.begin(1);
    answers.push_value(&AnswerValue::String("only"));
    answers.push_value(&AnswerValue::String("next"));
    for ram_allowance in [usize::MAX, 0] {
        let sealed =
            CompleteResult::seal(clone_answers(&answers), heap_identity(), &work, ram_allowance)
                .expect("seal");
        let mut cursor = sealed.into_cursor(8);
        let mut ticket = DeliveryTicket::open(&mut cursor);
        ticket
            .preview_page(&work, 1024)
            .expect("preview")
            .expect("row");
        let adopted = ticket.adopt().expect("adopt");
        assert_eq!(adopted.get(0, 0), AnswerValue::String("only"));
        ticket.abort();
        assert_eq!(cursor.debug_next_row(), 0);
        let fresh = DeliveryTicket::open(&mut cursor);
        fresh.commit();
        assert_eq!(
            cursor.debug_next_row(),
            0,
            "fresh unpreviewed ticket cannot commit an abandoned preview"
        );
        let retry = cursor
            .next_page_with_work(&work, 1024)
            .expect("retry after adopt-abort")
            .expect("first row still there");
        assert_eq!(retry.rows.get(0, 0), AnswerValue::String("only"));
    }
}

/// Large spilled results have no resident per-row length directory.
/// Paging must stay inside the working-memory envelope (not 8 bytes ×
/// row count of extra resident index).
/// Verification: NotRun.
#[test]
fn spilled_results_stay_within_working_memory_envelope() {
    let work = work();
    let mut answers = Answers::new();
    answers.begin(1);
    let n = 2048u64;
    for i in 0..n {
        answers.push_value(&AnswerValue::U64(i));
    }
    let sealed = CompleteResult::seal(answers, heap_identity(), &work, 0).expect("seal");
    let after_seal = work.used(crate::work::Resource::WorkingBytes);
    let mut cursor = sealed.into_cursor(8);
    let mut ticket = DeliveryTicket::open(&mut cursor);
    let preview = ticket
        .preview_page(&work, 1024)
        .expect("preview")
        .expect("first page");
    assert_eq!(preview.get(0, 0), AnswerValue::U64(0));
    let after_preview = work.used(crate::work::Resource::WorkingBytes);
    assert!(
        after_preview.saturating_sub(after_seal) < n,
        "one page must not grow an 8-byte-per-row resident length directory"
    );
    ticket.abort();
    let mut delivered = 0u64;
    loop {
        match cursor.next_page_with_work(&work, 256).expect("page") {
            None => break,
            Some(page) => {
                delivered += page.rows.len() as u64;
                if page.terminal {
                    break;
                }
            }
        }
    }
    assert_eq!(delivered, n);
    let after_pages = work.used(crate::work::Resource::WorkingBytes);
    assert!(
        after_pages.saturating_sub(after_seal) < n * 8,
        "paging a spill must not retain an 8-byte-per-row working-memory directory"
    );
}

#[test]
fn encoded_value_len_matches_the_codec() {
    let mut buf = Vec::new();
    let values = [
        AnswerValue::Bool(true),
        AnswerValue::U64(7),
        AnswerValue::I64(-3),
        AnswerValue::String("fit-check"),
        AnswerValue::FixedBytes(&[1, 2, 3, 4]),
        AnswerValue::Id128(bumbledb_theory::Id128::from_bytes([0; 16])),
    ];
    for value in values {
        buf.clear();
        super::encode_value(&value, &mut buf);
        assert_eq!(
            buf.len() as u64,
            super::encoded_value_len(&value),
            "fit length must match the codec for {value:?}"
        );
    }
}
