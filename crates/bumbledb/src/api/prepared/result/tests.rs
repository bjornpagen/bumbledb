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
