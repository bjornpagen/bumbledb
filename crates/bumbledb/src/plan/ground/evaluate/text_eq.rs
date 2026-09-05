//! Closed-row String filters use L04 [`TextEq`], never raw word identity.
//! Authored now; verification NotRun.

use super::{SealedRow, sealed_row_survives};
use crate::api::prepared::source::UNBOUNDED_POLICY;
use crate::encoding::FactLayout;
use crate::exec::scratch::capability::ScratchPolicy;
use crate::exec::scratch::ScratchCapability;
use crate::image::intern::InternerHandle;
use crate::image::view::{Const, FilterPredicate, OperandAddr, Operands};
use crate::image::{CacheGeneration, ResidentAdmit, TextEq};
use crate::ir::WordCmp;
use crate::work::{CacheLedger, CachePolicy, GenerationHandle, GenerationState};
use bumbledb_theory::schema::{FieldId, ValueType};

fn word_bytes(word: u64) -> [u8; 8] {
    word.to_be_bytes()
}

fn eq_word(field: u16, word: u64) -> FilterPredicate {
    FilterPredicate::Compare {
        field: OperandAddr::from(FieldId(field)),
        op: WordCmp::Eq,
        value: Const::Word(word),
    }
}

fn generation(cache_bytes: u64) -> GenerationHandle {
    GenerationHandle::new(GenerationState::new(
        CacheGeneration::initial(),
        CacheLedger::new(CachePolicy { cache_bytes }),
    ))
}

/// A String-column filter on [`SealedRow`] uses [`TextEq`].
/// Verification: NotRun.
#[test]
fn sealed_row_string_filter_uses_text_eq() {
    let work = crate::api::prepared::source::unbounded_work().expect("work");
    let tiny = generation(8);
    let admitted = InternerHandle::new(&tiny, &work)
        .intern_or_spill("a-text-that-cannot-fit-eight-cache-bytes")
        .expect("spill");
    let ResidentAdmit::BeyondMemory(exhausted) = admitted else {
        panic!("tiny cache must spill");
    };
    let cap = ScratchCapability::start(UNBOUNDED_POLICY, ScratchPolicy::unbounded()).expect("scratch");
    let mut store = exhausted.open_nonresident(&cap);
    let scratch = store.intern("shared", cap.work()).expect("scratch");

    let fat = generation(u64::MAX);
    let intern = fat
        .lock_resolver()
        .intern("shared", &work, fat.ledger())
        .expect("intern");
    assert_ne!(scratch, intern, "raw words stay disjoint");

    let layout = FactLayout::new(&[ValueType::U64, ValueType::String, ValueType::I64]);
    let mut fact = Vec::from(word_bytes(7));
    fact.extend_from_slice(&word_bytes(intern));
    fact.extend_from_slice(&word_bytes(1u64 << 63));
    let ops = SealedRow {
        fact: layout.encoded(&fact),
    };
    assert!(!ops.string_field(OperandAddr::from(FieldId(0))));
    assert!(ops.string_field(OperandAddr::from(FieldId(1))));
    assert!(!ops.string_field(OperandAddr::from(FieldId(2))));

    let eq = TextEq::bind(&fat, Some(&store));
    let hit = crate::image::view::holds(&eq_word(1, scratch), &ops, &[], eq)
        .expect("holds")
        .expect("verdict");
    assert!(hit, "String-column Eq on SealedRow unifies intern and scratch");
    let miss = crate::image::view::holds(&eq_word(1, intern.wrapping_add(1)), &ops, &[], eq)
        .expect("holds")
        .expect("verdict");
    assert!(!miss, "TextEq inequality stays a boolean miss");
}

/// A resolver refusal on a String-column SealedRow filter is `Err`, not a
/// dropped id. Verification: NotRun.
#[test]
fn sealed_row_resolver_refusal_is_err_not_dropped_id() {
    let work = crate::api::prepared::source::unbounded_work().expect("work");
    let tiny = generation(8);
    let admitted = InternerHandle::new(&tiny, &work)
        .intern_or_spill("a-text-that-cannot-fit-eight-cache-bytes")
        .expect("spill");
    let ResidentAdmit::BeyondMemory(exhausted) = admitted else {
        panic!("tiny cache must spill");
    };
    let cap = ScratchCapability::start(UNBOUNDED_POLICY, ScratchPolicy::unbounded()).expect("scratch");
    let mut store = exhausted.open_nonresident(&cap);
    let scratch = store.intern("shared", cap.work()).expect("scratch");

    let fat = generation(u64::MAX);
    let intern = fat
        .lock_resolver()
        .intern("shared", &work, fat.ledger())
        .expect("intern");
    assert_ne!(scratch, intern, "raw words stay disjoint");

    let layout = FactLayout::new(&[ValueType::String]);
    let fact = word_bytes(intern);
    let ops = SealedRow {
        fact: layout.encoded(&fact),
    };
    assert!(ops.string_field(OperandAddr::from(FieldId(0))));

    work.cancel();
    let eq = TextEq::bind(&fat, Some(&store));
    let filters = [eq_word(0, scratch)];
    let verdict = sealed_row_survives(&ops, &filters, eq);
    assert!(
        verdict.is_err(),
        "resolver refusal is Err, not a dropped id"
    );
    assert_ne!(
        verdict,
        Ok(false),
        "storage failure must not become text inequality"
    );
}
