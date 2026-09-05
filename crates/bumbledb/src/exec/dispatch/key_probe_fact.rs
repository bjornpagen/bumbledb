//! The key-probe row fetch over the successor store: exact value
//! semantics, no image, no COLT.
//!
//! - **Membership** (every field bound to a constant): reconstruct the
//!   probed fact's canonical bytes and ask the source for exact
//!   membership — a fingerprint bucket walk plus full-byte comparison on
//!   a store (HASH-02: the fingerprint only selects candidates), a binary
//!   search on a heap instance.
//! - **Uniqueness** (a declared key's determinant bound): on a store
//!   source, the probe projects the key's scalar determinant through the
//!   store's one projection convention, enumerates that determinant
//!   bucket at the pinned snapshot, and confirms each candidate by exact
//!   span comparison over every bound field — work is bucket-shaped,
//!   never relation-shaped, and a forced fingerprint collision widens the
//!   bucket without ever changing the answer. Committed state satisfies
//!   its keys, so at most one row matches; that row is re-decoded in
//!   intern mode (its text becomes answer-resolvable tokens) before
//!   residual filters run. Heap sources (admitted in-memory instances)
//!   keep the bounded reference walk over their sorted rows — a scan,
//!   named one — which is also the store path's exact oracle in the
//!   regression suites.
//!
//! The old `FreshRow` row-id fast branch is deleted with the fresh
//! reservation machinery (E-NO-RESERVE); the old fixed-layout fact fetch
//! and the persisted-dictionary lookups are deleted with the transitional
//! storage (ENG-006).

use super::KeyProbePlan;
use super::fact_word::FactOperand;
use crate::api::prepared::source::{QuerySource, work_error};
use crate::error::{Error, Result};
use crate::image::canon::RowWords;
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
    store: &mut Option<crate::image::NonresidentTextStore>,
    work: &crate::work::WorkContext,
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
            return const_words(
                interner,
                store,
                work,
                &params[usize::from(param.0)],
                params,
                out,
            );
        }
        Const::ParamSet(_) | Const::WordSet(_) => {
            unreachable!("classification: a set binding never reaches the key-probe path")
        }
        Const::PendingIntern { bytes } => {
            let text = std::str::from_utf8(bytes)
                .expect("IR string literals are UTF-8 by construction (Value::String)");
            out.push(crate::api::prepared::intern_admitted(
                interner, store, text, work,
            )?);
        }
    }
    Ok(())
}

/// Rebuild the probed value from its column words — the membership path's
/// canonical-bytes reconstruction. Exact inverses of the walker's word
/// conventions; a probe word that cannot embed refuses as a mismatch.
fn value_of_words(
    interner: &InternerHandle<'_>,
    store: Option<&mut crate::image::NonresidentTextStore>,
    ty: &ValueType,
    words: &[u64],
) -> Option<Value> {
    Some(match ty {
        ValueType::Bool => Value::Bool(words[0] != 0),
        ValueType::U64 => Value::U64(words[0]),
        ValueType::I64 => Value::I64((words[0] ^ (1 << 63)).cast_signed()),
        ValueType::F64 => Value::F64(bumbledb_theory::F64::from_order_key(words[0]).ok()?),
        ValueType::String => Value::String(
            crate::api::prepared::owned_text(interner, store, words[0])
                .ok()
                .flatten()?,
        ),
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
    field_types: &'a [ValueType],
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

    fn string_field(&self, at: OperandAddr) -> bool {
        self.field_types
            .get(usize::from(at.field().0))
            .is_some_and(|ty| matches!(ty, ValueType::String))
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
    store: &mut Option<crate::image::NonresidentTextStore>,
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
        const_words(interner, store, source.work(), value, params, scratch)?;
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
                if let Some(value) =
                    value_of_words(interner, store.as_mut(), ty, &scratch[range.clone()])
                {
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
                    crate::api::prepared::decode_row(
                        row,
                        fields,
                        encoded.as_bytes(),
                        interner,
                        store,
                        source.work(),
                        true,
                    )?;
                    true
                } else {
                    false
                }
            } else {
                false
            }
        }
        super::KeyProbeKind::Uniqueness { .. } => {
            let indexed = match source {
                QuerySource::Store { snapshot, work, .. } => probe_uniqueness_indexed(
                    snapshot,
                    work,
                    plan.relation,
                    fields,
                    &field_types,
                    &key_words,
                    scratch,
                    interner,
                    store,
                    row,
                )?,
                QuerySource::Heap { .. } => None,
            };
            match indexed {
                Some(hit) => hit,
                // Heap sources (and, defensively, a statement the compiled
                // determinant table does not carry): the bounded reference
                // walk — the exact oracle for the indexed path.
                None => probe_uniqueness_scan(
                    plan,
                    source,
                    schema,
                    fields,
                    &field_types,
                    &key_words,
                    scratch,
                    interner,
                    store,
                    row,
                )?,
            }
        }
    };
    probe_span.set_flag(hit);
    if !hit {
        return Ok(false);
    }

    let ops = ProbeRow {
        row,
        interner,
        field_types: &field_types,
    };
    let eq = interner.generation().text_eq(store.as_ref());
    for filter in &plan.remaining_filters {
        if !holds(filter, &ops, params, eq)?.unwrap_or(false) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// The indexed uniqueness probe over one committed store snapshot:
/// determinant bucket plus exact span confirmation. `Ok(Some(true))` leaves
/// the matched row's words (text interned) in `row`; `Ok(None)` means the
/// compiled determinant table does not carry the statement (defensive —
/// classification only emits sealed keys of ordinary relations) and the
/// caller must fall back to the reference walk.
#[expect(
    clippy::too_many_arguments,
    reason = "one probe's borrowed working set, threaded rather than \
              re-bundled into a transient struct"
)]
fn probe_uniqueness_indexed(
    snapshot: &crate::storage::store::OwnedSnapshot,
    work: &crate::work::WorkContext,
    relation: bumbledb_theory::schema::RelationId,
    fields: &[bumbledb_theory::schema::FieldDescriptor],
    field_types: &[ValueType],
    key_words: &[(FieldId, std::ops::Range<usize>)],
    scratch: &[u64],
    interner: &InternerHandle<'_>,
    store: &mut Option<crate::image::NonresidentTextStore>,
    row: &mut RowWords,
) -> Result<Option<bool>> {
    use crate::api::prepared::source::store_error;
    let key_fields: Vec<FieldId> = key_words.iter().map(|(field, _)| *field).collect();
    let Some(key) = snapshot.determinants().key_for(relation, &key_fields) else {
        return Ok(None);
    };
    debug_assert_eq!(
        key.projection.len(),
        key_words.len(),
        "classification binds the complete sealed projection, in order"
    );
    let mut determinant = Vec::with_capacity(key.scalar_positions.len());
    for &position in &key.scalar_positions {
        let (field, range) = &key_words[position];
        let ty = &field_types[usize::from(field.0)];
        match value_of_words(interner, store.as_mut(), ty, &scratch[range.clone()]) {
            Some(value) => determinant.push(value),
            None => return Ok(Some(false)),
        }
    }
    let projected =
        crate::storage::store::det_index::determinant_bytes(key, &determinant, work)
            .map_err(store_error)?;
    let mut probe = RowWords::new(field_types);
    let mut hit = false;
    let mut visit_err: Option<Error> = None;
    snapshot
        .visit_projection(key.id, &projected, work, &mut |_id, bytes| {
            if visit_err.is_some() || hit {
                return Ok(false);
            }
            work.step(1).map_err(crate::storage::store::StoreError::Work)?;
            if let Err(error) = crate::api::prepared::decode_row(
                &mut probe,
                fields,
                bytes,
                interner,
                store,
                work,
                false,
            ) {
                visit_err = Some(error);
                return Ok(false);
            }
            let matches = match key_spans_match(
                interner,
                store,
                field_types,
                key_words,
                &probe,
                scratch,
            ) {
                Ok(matched) => matched,
                Err(error) => {
                    visit_err = Some(error);
                    return Ok(false);
                }
            };
            if matches {
                if let Err(error) = crate::api::prepared::decode_row(
                    row,
                    fields,
                    bytes,
                    interner,
                    store,
                    work,
                    true,
                ) {
                    visit_err = Some(error);
                    return Ok(false);
                }
                hit = true;
                Ok(false)
            } else {
                Ok(true)
            }
        })
        .map_err(store_error)?;
    if let Some(error) = visit_err {
        return Err(error);
    }
    Ok(Some(hit))
}

/// The bounded reference walk (heap sources, and the indexed path's exact
/// oracle): decode every source row in lookup-only text mode and compare
/// the determinant spans exactly. A scan, and named one.
#[expect(
    clippy::too_many_arguments,
    reason = "one probe's borrowed working set, threaded rather than \
              re-bundled into a transient struct"
)]
fn probe_uniqueness_scan(
    plan: &KeyProbePlan,
    source: &QuerySource<'_>,
    schema: &Schema,
    fields: &[bumbledb_theory::schema::FieldDescriptor],
    field_types: &[ValueType],
    key_words: &[(FieldId, std::ops::Range<usize>)],
    scratch: &[u64],
    interner: &InternerHandle<'_>,
    store: &mut Option<crate::image::NonresidentTextStore>,
    row: &mut RowWords,
) -> Result<bool> {
    let mut found = false;
    let mut probe = RowWords::new(field_types);
    let theory = schema
        .compiled_theory()
        .map_err(crate::api::prepared::source::compile_error)?;
    let witness = match &plan.kind {
        super::KeyProbeKind::Uniqueness { statement, .. } => theory
            .key_witness(*statement)
            .unwrap_or(crate::schema::CompiledTheory::full_row_witness()),
        super::KeyProbeKind::Membership { .. } => crate::schema::CompiledTheory::full_row_witness(),
    };
    let fields_owned = key_words.iter().map(|(f, _)| *f).collect::<Vec<_>>();
    let words: Vec<u64> = key_words.iter().map(|(_, range)| scratch[range.start]).collect();
    if let Some(_outcome) = source.consume_compiled_visits(
        schema,
        plan.relation,
        witness,
        &fields_owned,
        &words,
        &mut |bytes| {
            crate::api::prepared::decode_row(
                &mut probe,
                fields,
                bytes,
                interner,
                store,
                source.work(),
                false,
            )?;
            let matches = key_spans_match(
                interner,
                store,
                field_types,
                key_words,
                &probe,
                scratch,
            )?;
            if matches {
                crate::api::prepared::decode_row(
                    row,
                    fields,
                    bytes,
                    interner,
                    store,
                    source.work(),
                    true,
                )?;
                found = true;
                Ok(crate::schema::VisitControl::Stop)
            } else {
                Ok(crate::schema::VisitControl::Continue)
            }
        },
    )? {
        return Ok(found);
    }
    Err(crate::error::Error::Corruption(
        crate::error::CorruptionError::MalformedValue("compiled key witness"),
    ))
}

fn key_spans_match(
    interner: &InternerHandle<'_>,
    store: &mut Option<crate::image::NonresidentTextStore>,
    field_types: &[ValueType],
    key_words: &[(FieldId, std::ops::Range<usize>)],
    probe: &RowWords,
    scratch: &[u64],
) -> Result<bool> {
    let eq = interner.generation().text_eq(store.as_ref());
    for (field, range) in key_words {
        let ty = &field_types[usize::from(field.0)];
        let left = probe.span_words(*field);
        let right = &scratch[range.clone()];
        let same = if matches!(ty, ValueType::String) {
            eq.tokens_equal(left[0], right[0])
        } else {
            left == right
        };
        if !same {
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
