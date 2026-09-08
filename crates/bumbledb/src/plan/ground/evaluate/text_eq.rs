//! Sealed-row String filters use a pinned resolver, never an absent namespace.
use super::{SealedRow, sealed_row_survives};
use crate::encoding::FactLayout;
use crate::image::TextEq;
use crate::image::intern::{InternerHandle, SENTINEL_WORD};
use crate::image::view::{Const, FilterPredicate, OperandAddr, Operands};
use crate::ir::WordCmp;
use crate::work::WorkContext;
use bumbledb_theory::schema::{FieldId, ValueType};

fn eq_text(field: u16, value: Const) -> FilterPredicate {
    FilterPredicate::Compare {
        field: OperandAddr::from(FieldId(field)),
        op: WordCmp::Eq,
        value,
    }
}

#[test]
fn sealed_row_string_filter_uses_the_exact_pinned_text_identity() {
    let work = WorkContext::new();
    let generation = crate::image::test_generation();
    let interner = InternerHandle::new(&generation, &work);
    let shared = interner.intern("shared").unwrap();
    let same = interner.intern("shared").unwrap();
    let other = interner.intern("other").unwrap();
    let layout = FactLayout::new(&[ValueType::U64, ValueType::String, ValueType::I64]);
    let mut fact = Vec::from(7u64.to_be_bytes());
    fact.extend_from_slice(&shared.word.to_be_bytes());
    fact.extend_from_slice(&(1u64 << 63).to_be_bytes());
    let ops = SealedRow {
        fact: layout.encoded(&fact),
    };
    assert!(!ops.string_field(OperandAddr::from(FieldId(0))));
    assert!(ops.string_field(OperandAddr::from(FieldId(1))));
    assert!(!ops.string_field(OperandAddr::from(FieldId(2))));
    let eq = TextEq::bind(&generation);
    assert!(sealed_row_survives(&ops, &[eq_text(1, Const::Text(same))], eq).unwrap());
    assert!(!sealed_row_survives(&ops, &[eq_text(1, Const::Text(other))], eq).unwrap());
    assert!(!sealed_row_survives(&ops, &[eq_text(1, Const::Word(SENTINEL_WORD))], eq).unwrap());
}

#[test]
fn sealed_row_missing_resolver_is_error_not_a_dropped_id() {
    let work = WorkContext::new();
    let generation = crate::image::test_generation();
    let shared = InternerHandle::new(&generation, &work)
        .intern("shared")
        .unwrap();
    let layout = FactLayout::new(&[ValueType::String]);
    let fact = shared.word.to_be_bytes();
    let ops = SealedRow {
        fact: layout.encoded(&fact),
    };
    let filters = [eq_text(0, Const::Text(shared))];
    let verdict = sealed_row_survives(&ops, &filters, TextEq::from_optional_generation(None));
    assert!(
        matches!(verdict, Err(crate::Error::Corruption(_))),
        "an absent namespace cannot become equality or inequality"
    );
}
