//! The key-probe row fetch over the successor store: exact value
//! semantics, no image, no COLT.
//!
//! - **Membership** (every field bound to a constant): reconstruct the
//!   probed fact's canonical bytes and ask the source for exact
//!   membership — a fingerprint bucket walk plus full-byte comparison on
//!   a store (HASH-02: the fingerprint only selects candidates), a binary
//!   search on a heap instance.
//! - **Uniqueness** (a declared key's determinant bound): the reference
//!   path walks the relation's committed rows, decodes each through the
//!   one canonical walker in lookup-only text mode, and compares the
//!   determinant spans exactly. Committed state satisfies its keys, so at
//!   most one row matches; that row is re-decoded in intern mode (its
//!   text becomes answer-resolvable tokens) before residual filters run.
//!   Determinant acceleration (order-preserving or bucketed probes) is
//!   the recorded C04 follow-up with P02R — admission and this read are
//!   correct without it.
//!
//! The old `FreshRow` row-id fast branch is deleted with the fresh
//! reservation machinery (E-NO-RESERVE); the old fixed-layout fact fetch
//! and the persisted-dictionary lookups are deleted with the transitional
//! storage (ENG-006).

use super::KeyProbePlan;
use super::fact_word::FactOperand;
use crate::api::prepared::source::{QuerySource, work_error};
use crate::error::{Error, Result};
use crate::image::canon::{RowWords, TextWords};
use crate::image::intern::InternerHandle;
use crate::image::view::{Const, Loaded, OperandAddr, Operands, holds};
use crate::ir::Value;
use crate::obs;
use crate::schema::Schema;
use bumbledb_theory::schema::{FieldId, IntervalElement, ValueType};

/// Resolve one probe constant to its column words (interval constants are
/// two words; canonical rows carry both fixed-interval bounds).
fn const_words(
    interner: &InternerHandle<'_>,
    value: &Const,
    params: &[Const],
    out: &mut Vec<u64>,
) -> Result<()> {
    match value {
        Const::Word(word) => out.push(*word),
        Const::Byte(byte) => out.push(u64::from(*byte)),
        Const::Words(words) => out.extend_from_slice(words),
        Const::Interval { start, end } => out.extend([*start, *end]),
        Const::Param(param) => {
            return const_words(interner, &params[usize::from(param.0)], params, out);
        }
        Const::ParamSet(_) | Const::WordSet(_) => {
            unreachable!("classification: a set binding never reaches the key-probe path")
        }
        Const::PendingIntern { bytes } => out.push(interner.latch(bytes)?),
    }
    Ok(())
}

/// Rebuild the probed value from its column words — the membership path's
/// canonical-bytes reconstruction. Exact inverses of the walker's word
/// conventions; a probe word that cannot embed refuses as a mismatch.
fn value_of_words(interner: &InternerHandle<'_>, ty: &ValueType, words: &[u64]) -> Option<Value> {
    Some(match ty {
        ValueType::Bool => Value::Bool(words[0] != 0),
        ValueType::U64 => Value::U64(words[0]),
        ValueType::I64 => Value::I64((words[0] ^ (1 << 63)).cast_signed()),
        ValueType::F64 => Value::F64(bumbledb_theory::F64::from_order_key(words[0]).ok()?),
        ValueType::String => {
            Value::String(interner.with_text(words[0], |text| Box::<str>::from(text))?)
        }
        ValueType::Id128 => {
            let mut bytes = [0u8; 16];
            bytes[..8].copy_from_slice(&words[0].to_be_bytes());
            bytes[8..].copy_from_slice(&words[1].to_be_bytes());
            Value::Id128(bumbledb_theory::Id128::from_bytes(bytes))
        }
        ValueType::FixedBytes { len } => {
            let mut bytes = Vec::with_capacity(usize::from(*len));
            for word in words {
                bytes.extend_from_slice(&word.to_be_bytes());
            }
            bytes.truncate(usize::from(*len));
            Value::FixedBytes(bytes.into_boxed_slice())
        }
        ValueType::Interval {
            element: IntervalElement::U64,
        }
        | ValueType::FixedInterval {
            element: bumbledb_theory::schema::FixedIntervalElement::U64,
            ..
        } => Value::IntervalU64(bumbledb_theory::Interval::new(words[0], words[1])?),
        ValueType::Interval {
            element: IntervalElement::I64,
        }
        | ValueType::FixedInterval {
            element: bumbledb_theory::schema::FixedIntervalElement::I64,
            ..
        } => Value::IntervalI64(bumbledb_theory::Interval::new(
            (words[0] ^ (1 << 63)).cast_signed(),
            (words[1] ^ (1 << 63)).cast_signed(),
        )?),
        ValueType::Interval {
            element: IntervalElement::F64,
        } => Value::IntervalF64(bumbledb_theory::Interval::new(
            bumbledb_theory::F64::from_order_key(words[0]).ok()?,
            bumbledb_theory::F64::from_order_key(words[1]).ok()?,
        )?),
    })
}

/// Residual-filter operands over one decoded row's words.
struct ProbeRow<'a> {
    row: &'a RowWords,
    interner: &'a InternerHandle<'a>,
}

impl Operands for ProbeRow<'_> {
    type Error = Error;

    fn word(&self, at: OperandAddr) -> std::result::Result<u64, Self::Error> {
        match self.row.operand(at.field()) {
            FactOperand::Word(w) => Ok(w),
            FactOperand::Pair(..) | FactOperand::Block { .. } => {
                unreachable!("validated: word operands are scalar fields")
            }
        }
    }

    fn pair(&self, at: OperandAddr) -> std::result::Result<(u64, u64), Self::Error> {
        match self.row.operand(at.field()) {
            FactOperand::Pair(s, e) => Ok((s, e)),
            FactOperand::Word(_) | FactOperand::Block { .. } => {
                unreachable!("validated: interval predicates read interval fields")
            }
        }
    }

    fn block(&self, at: OperandAddr) -> std::result::Result<([u64; 8], u8), Self::Error> {
        match self.row.operand(at.field()) {
            FactOperand::Block { words, count } => Ok((words, count)),
            FactOperand::Word(_) | FactOperand::Pair(..) => {
                unreachable!("validated: block operands are bytes<N>")
            }
        }
    }

    fn loaded(&self, at: OperandAddr) -> std::result::Result<Loaded, Self::Error> {
        Ok(match self.row.operand(at.field()) {
            FactOperand::Word(w) => Loaded::Word(w),
            FactOperand::Pair(s, e) => Loaded::Pair(s, e),
            FactOperand::Block { words, count } => Loaded::Block { words, count },
        })
    }

    fn intern(&self, bytes: &[u8]) -> std::result::Result<u64, Self::Error> {
        // Lookup-only: a text never interned equals no interned word. The
        // matched row's own text was interned at capture, so equal texts
        // meet at one token and unequal texts never alias.
        let text = std::str::from_utf8(bytes)
            .expect("IR string literals are UTF-8 by construction (Value::String)");
        Ok(self.interner.lookup_word(text))
    }
}

/// The determinant/full-fact probe. `Ok(true)` leaves the matched row's
/// words (text interned) in `row`.
/// # Errors
/// Storage failure, stopped work, or corrupt stored bytes.
pub(crate) fn key_probe_row(
    plan: &KeyProbePlan,
    source: &QuerySource<'_>,
    schema: &Schema,
    interner: &InternerHandle<'_>,
    params: &[Const],
    row: &mut RowWords,
    scratch: &mut Vec<u64>,
) -> Result<bool> {
    let relation = schema.relation(plan.relation);
    let fields = relation.fields();
    let field_types: Vec<ValueType> = fields.iter().map(|f| f.value_type).collect();

    // Resolve the probe's constant spans once, in key order.
    let mut key_words: Vec<(FieldId, std::ops::Range<usize>)> = Vec::new();
    scratch.clear();
    for (field, value) in plan.kind.key() {
        let start = scratch.len();
        const_words(interner, value, params, scratch)?;
        key_words.push((*field, start..scratch.len()));
    }

    let mut probe_span = obs::span(obs::names::KEY_PROBE);
    let hit = match &plan.kind {
        super::KeyProbeKind::Membership { .. } => {
            // All fields bound: reconstruct the canonical fact and ask for
            // exact membership. A word that cannot embed (for example an
            // inverted interval from a hostile template) is a nonmatch.
            let mut values = Vec::with_capacity(fields.len());
            let mut ok = true;
            for (field, range) in &key_words {
                let ty = &fields[usize::from(field.0)].value_type;
                if let Some(value) = value_of_words(interner, ty, &scratch[range.clone()]) {
                    values.push(value);
                } else {
                    ok = false;
                    break;
                }
            }
            if ok {
                let encoded =
                    crate::canonical::CanonicalRow::encode(fields, &values, source.work())
                        .map_err(row_error)?;
                if source.contains(plan.relation, encoded.as_bytes())? {
                    // The row IS the probe: decode the canonical bytes we
                    // just built (intern mode) so finds and filters read
                    // the same words a scan would produce.
                    let mut text = TextWords::HandleIntern(interner);
                    row.decode(fields, encoded.as_bytes(), &mut text)?;
                    true
                } else {
                    false
                }
            } else {
                false
            }
        }
        super::KeyProbeKind::Uniqueness { .. } => {
            let mut found = false;
            let mut probe = RowWords::new(&field_types);
            let work = source.work();
            source.scan(plan.relation, &mut |bytes| {
                if found {
                    return Ok(());
                }
                work.step(1).map_err(work_error)?;
                {
                    let mut text = TextWords::HandleLookup(interner);
                    probe.decode(fields, bytes, &mut text)?;
                }
                let matches = key_words
                    .iter()
                    .all(|(field, range)| probe.span_words(*field) == &scratch[range.clone()]);
                if matches {
                    // Re-decode in intern mode: the matched row's text
                    // becomes real tokens for filters and answers.
                    let mut text = TextWords::HandleIntern(interner);
                    row.decode(fields, bytes, &mut text)?;
                    found = true;
                }
                Ok(())
            })?;
            found
        }
    };
    probe_span.set_flag(hit);
    if !hit {
        return Ok(false);
    }

    let ops = ProbeRow { row, interner };
    for filter in &plan.remaining_filters {
        if !holds(filter, &ops, params)?.unwrap_or(false) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn row_error(error: crate::canonical::RowError) -> Error {
    match error {
        crate::canonical::RowError::Work(work) => work_error(work),
        crate::canonical::RowError::Allocation => {
            crate::api::prepared::source::store_error(crate::storage::store::StoreError::Allocation)
        }
        _ => Error::Corruption(crate::error::CorruptionError::MalformedValue(
            "key-probe canonical reconstruction",
        )),
    }
}
