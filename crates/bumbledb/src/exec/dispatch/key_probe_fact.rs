//! The key-probe row fetch: exact value semantics, no image, no COLT. A membership
//! probe reconstructs the row's canonical bytes and asks the source for exact
//! membership; a uniqueness probe walks the key's determinant bucket and confirms
//! each candidate by exact span comparison, so a fingerprint collision widens the
//! bucket but never changes the answer. Heap sources take the bounded reference walk.

use super::fact_word::FactOperand;
use super::{KeyProbePart, KeyProbePlan, ProbeBuffers, ProbeCtx};
use crate::api::prepared::source::QuerySource;
use crate::error::{Error, Result};
use crate::image::canon::RowWords;
use crate::image::intern::InternerHandle;
use crate::image::view::{Const, Loaded, OperandAddr, Operands, holds};
use crate::ir::Value;
use bumbledb_theory::schema::{IntervalElement, ValueType};

/// Resolve one probe constant to its column words (interval constants are
/// two words; canonical rows carry both fixed-interval bounds).
fn const_words(
    interner: &InternerHandle<'_>,
    work: &crate::work::WorkContext,
    value: &Const,
    params: &[Const],
    out: &mut crate::image::view::ResolvedWords,
) -> Result<()> {
    work.checkpoint()?;
    match value {
        Const::Word(scalar) => out.words.push(*scalar),
        Const::Text(text) => out.push_text(text.clone()),
        Const::Byte(byte) => out.words.push(u64::from(*byte)),
        Const::Words(words) => out.words.extend_from_slice(words),
        Const::Interval { start, end } => out.words.extend([*start, *end]),
        Const::Param(param) => {
            return const_words(interner, work, &params[usize::from(param.0)], params, out);
        }
        Const::ParamSet(_) | Const::WordSet(_) => {
            unreachable!("classification: a set binding never reaches the key-probe path")
        }
        Const::DenseOrderParam(_) => {
            unreachable!("classification: an order bound never reaches the key-probe path")
        }
        Const::PendingIntern { bytes } => {
            let text = std::str::from_utf8(bytes)
                .expect("IR string literals are UTF-8 by construction (Value::String)");
            out.push_text(interner.intern(text)?);
        }
    }
    Ok(())
}

/// Rebuild the probed value from its column words — the membership path's
/// canonical-bytes reconstruction. Exact inverses of the walker's word
/// conventions; a probe word that cannot embed refuses as a mismatch.
/// Resolver failures are operational errors, never successful nonmatches.
fn value_of_words(interner: &InternerHandle<'_>, ty: &ValueType, words: &[u64]) -> Option<Value> {
    match ty {
        ValueType::Bool => Some(Value::Bool(words[0] != 0)),
        ValueType::U64 => Some(Value::U64(words[0])),
        ValueType::I64 => Some(Value::I64((words[0] ^ (1 << 63)).cast_signed())),
        ValueType::F64 => bumbledb_theory::F64::from_order_key(words[0])
            .ok()
            .map(Value::F64),
        ValueType::String => {
            crate::api::prepared::owned_text(interner, words[0]).map(Value::String)
        }
        ValueType::Uuid => {
            let mut bytes = [0u8; 16];
            bytes[..8].copy_from_slice(&words[0].to_be_bytes());
            bytes[8..].copy_from_slice(&words[1].to_be_bytes());
            Some(Value::Uuid(bumbledb_theory::Uuid::from_bytes(bytes)))
        }
        ValueType::FixedBytes { len } => {
            let mut bytes = Vec::with_capacity(usize::from(*len));
            for word in words {
                bytes.extend_from_slice(&word.to_be_bytes());
            }
            bytes.truncate(usize::from(*len));
            Some(Value::FixedBytes(bytes.into_boxed_slice()))
        }
        ValueType::Interval {
            element: IntervalElement::U64,
        }
        | ValueType::FixedInterval {
            element: bumbledb_theory::schema::FixedIntervalElement::U64,
            ..
        } => bumbledb_theory::Interval::new(words[0], words[1]).map(Value::IntervalU64),
        ValueType::Interval {
            element: IntervalElement::I64,
        }
        | ValueType::FixedInterval {
            element: bumbledb_theory::schema::FixedIntervalElement::I64,
            ..
        } => bumbledb_theory::Interval::new(
            (words[0] ^ (1 << 63)).cast_signed(),
            (words[1] ^ (1 << 63)).cast_signed(),
        )
        .map(Value::IntervalI64),
        ValueType::Interval {
            element: IntervalElement::F64,
        } => bumbledb_theory::F64::from_order_key(words[0])
            .ok()
            .zip(bumbledb_theory::F64::from_order_key(words[1]).ok())
            .and_then(|(start, end)| bumbledb_theory::Interval::new(start, end))
            .map(Value::IntervalF64),
    }
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
        self.row.field_is_string(at.field())
    }
}

/// The determinant or full-row probe. `Ok(true)` leaves the matched row's
/// words (text interned) in `buf.row`.
/// # Errors
/// Storage failure, stopped work, or corrupt stored bytes.
pub(crate) fn key_probe_row(
    plan: &KeyProbePlan,
    cx: ProbeCtx<'_>,
    buf: &mut ProbeBuffers,
) -> Result<bool> {
    let ProbeCtx {
        source,
        schema,
        interner,
        params,
    } = cx;
    let fields = schema.relation(plan.relation).fields();

    // Word ranges are sealed once at prepare, not allocated on every probe.
    let key_words = plan.kind.key();
    buf.key.clear();
    for part in key_words {
        const_words(interner, source.work(), &part.value, params, &mut buf.key)?;
        debug_assert_eq!(
            buf.key.words.len(),
            usize::from(part.end),
            "validated key word width"
        );
    }

    let hit = match &plan.kind {
        super::KeyProbeKind::Membership { .. } => {
            // All fields bound: reconstruct the canonical row and ask for
            // exact membership. A word that cannot embed (an inverted
            // interval from a hostile template) is a nonmatch.
            if key_values(interner, fields, key_words.iter(), buf) {
                let encoded =
                    crate::canonical::CanonicalRow::encode(fields, &buf.values, source.work())
                        .map_err(row_error)?;
                if source.contains(plan.relation, encoded.as_bytes())? {
                    // The row is the probe: decode the canonical bytes just
                    // built (intern mode) so finds and filters read the same
                    // words a scan would produce.
                    crate::api::prepared::decode_row(
                        &mut buf.row,
                        fields,
                        encoded.as_bytes(),
                        interner,
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
        super::KeyProbeKind::Uniqueness { projection, .. } => {
            let indexed = match source {
                QuerySource::Store { .. } => {
                    probe_uniqueness_indexed(cx, *projection, fields, key_words, buf)?
                }
                QuerySource::Heap { .. } => None,
            };
            match indexed {
                Some(hit) => hit,
                // Heap sources (and a statement the compiled determinant
                // table does not carry): the bounded reference walk, which
                // is also the exact oracle for the indexed path.
                None => probe_uniqueness_scan(plan, cx, fields, key_words, buf)?,
            }
        }
    };
    if !hit {
        return Ok(false);
    }

    let ops = ProbeRow {
        row: &buf.row,
        interner,
    };
    let eq = interner.text_eq();
    for filter in &plan.remaining_filters {
        if !holds(filter, &ops, params, eq)?.unwrap_or(false) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Fills `buf.values` with the key's values, one per part. `false` when a
/// word does not embed as a value of its field.
fn key_values<'p>(
    interner: &InternerHandle<'_>,
    fields: &[bumbledb_theory::schema::FieldDescriptor],
    parts: impl Iterator<Item = &'p KeyProbePart>,
    buf: &mut ProbeBuffers,
) -> bool {
    buf.values.clear();
    for part in parts {
        let ty = &fields[usize::from(part.field.0)].value_type;
        match value_of_words(interner, ty, &buf.key.words[part.words()]) {
            Some(value) => buf.values.push(value),
            None => return false,
        }
    }
    true
}

/// The indexed uniqueness probe over one committed store snapshot:
/// determinant bucket plus exact span confirmation. `Ok(Some(true))` leaves
/// the matched row's words (text interned) in `buf.row`; `Ok(None)` means
/// the compiled determinant table does not carry the statement and the
/// caller falls back to the reference walk.
fn probe_uniqueness_indexed(
    cx: ProbeCtx<'_>,
    projection: crate::schema::ProjectionId,
    fields: &[bumbledb_theory::schema::FieldDescriptor],
    key_words: &[KeyProbePart],
    buf: &mut ProbeBuffers,
) -> Result<Option<bool>> {
    let ProbeCtx {
        source, interner, ..
    } = cx;
    let QuerySource::Store { snapshot, work, .. } = source else {
        return Ok(None);
    };
    let Some(projection) = snapshot.projection(projection) else {
        return Ok(None);
    };
    let key = projection.compiled();
    debug_assert_eq!(
        key.projection.len(),
        key_words.len(),
        "classification binds the complete sealed projection, in order"
    );
    let mut exact = [0u8; 16];
    let encoded;
    let projected: &[u8] = match key.encoding {
        crate::schema::KeyEncoding::ExactBounded { scalar_width } => {
            if encode_exact_words(key, key_words, &buf.key.words, &mut exact).is_none() {
                return Ok(Some(false));
            }
            &exact[..usize::from(scalar_width)]
        }
        crate::schema::KeyEncoding::FingerprintBucket => {
            let scalar_parts = key.scalar_positions.iter().map(|&p| &key_words[p]);
            if !key_values(interner, fields, scalar_parts, buf) {
                return Ok(Some(false));
            }
            encoded = crate::storage::store::det_index::determinant_bytes(key, &buf.values, work)?;
            &encoded
        }
    };
    let ProbeBuffers { row, key, .. } = buf;
    let scratch = &key.words;
    let has_text = row.has_text();
    let mut hit = false;
    let mut visit_err: Option<Error> = None;
    projection.probe(projected, work, &mut |_id, bytes| {
        if visit_err.is_some() || hit {
            return Ok(false);
        }
        work.checkpoint()?;
        source.note_visits(1);
        if let Err(error) =
            crate::api::prepared::decode_row(row, fields, bytes, interner, work, false)
        {
            visit_err = Some(error);
            return Ok(false);
        }
        let matches = match key_spans_match(interner, key_words, row, scratch) {
            Ok(matched) => matched,
            Err(error) => {
                visit_err = Some(error);
                return Ok(false);
            }
        };
        if matches {
            if has_text
                && let Err(error) =
                    crate::api::prepared::decode_row(row, fields, bytes, interner, work, true)
            {
                visit_err = Some(error);
                return Ok(false);
            }
            hit = true;
            Ok(false)
        } else {
            Ok(true)
        }
    })?;
    if let Some(error) = visit_err {
        return Err(error);
    }
    Ok(Some(hit))
}

/// Exact scalar routing is already the column-word order, except Bool's
/// one-byte width and a fixed-byte field's final padded word. Validate the
/// same float domain as `value_of_words`, then copy into bounded stack space.
fn encode_exact_words(
    projection: &crate::schema::CompiledProjection,
    parts: &[KeyProbePart],
    words: &[u64],
    out: &mut [u8; 16],
) -> Option<()> {
    let mut written = 0;
    for (&position, field) in projection
        .scalar_positions
        .iter()
        .zip(&projection.scalar_fields)
    {
        let part = parts.get(position)?;
        let used = encode_exact_field(
            &field.value_type,
            words.get(part.words())?,
            &mut out[written..],
        )?;
        written += used;
    }
    (written == projection.encoding.routing_width()).then_some(())
}

fn encode_exact_field(ty: &ValueType, words: &[u64], out: &mut [u8]) -> Option<usize> {
    let len = match ty {
        ValueType::Bool => {
            *out.first_mut()? = u8::from(*words.first()? != 0);
            return Some(1);
        }
        ValueType::U64 | ValueType::I64 => 8,
        ValueType::F64 => {
            bumbledb_theory::F64::from_order_key(*words.first()?).ok()?;
            8
        }
        ValueType::Uuid => 16,
        ValueType::FixedBytes { len } => usize::from(*len),
        ValueType::String | ValueType::Interval { .. } | ValueType::FixedInterval { .. } => {
            return None;
        }
    };
    let target = out.get_mut(..len)?;
    if words.len() != len.div_ceil(8) {
        return None;
    }
    for (chunk, word) in target.chunks_mut(8).zip(words) {
        chunk.copy_from_slice(&word.to_be_bytes()[..chunk.len()]);
    }
    Some(len)
}

/// The bounded reference walk (heap sources, and the indexed path's exact
/// oracle): decode every candidate row in lookup-only text mode and compare
/// the determinant spans exactly.
fn probe_uniqueness_scan(
    plan: &KeyProbePlan,
    cx: ProbeCtx<'_>,
    fields: &[bumbledb_theory::schema::FieldDescriptor],
    key_words: &[KeyProbePart],
    buf: &mut ProbeBuffers,
) -> Result<bool> {
    let ProbeCtx {
        source,
        schema,
        interner,
        ..
    } = cx;
    let theory = schema
        .compiled_theory()
        .map_err(crate::api::prepared::source::compile_error)?;
    let witness = match &plan.kind {
        super::KeyProbeKind::Uniqueness { statement, .. } => theory
            .key_witness(*statement)
            .unwrap_or(crate::schema::CompiledTheory::full_row_witness()),
        super::KeyProbeKind::Membership { .. } => crate::schema::CompiledTheory::full_row_witness(),
    };
    let ProbeBuffers {
        row,
        key,
        walk_fields,
        walk_words,
        ..
    } = buf;
    let scratch = &key.words;
    walk_fields.clear();
    walk_fields.extend(key_words.iter().map(|part| part.field));
    walk_words.clear();
    walk_words.extend(
        key_words
            .iter()
            .map(|part| scratch[usize::from(part.start)]),
    );
    let has_text = row.has_text();
    let mut found = false;
    if let Some(_outcome) = source.consume_compiled_visits(
        schema,
        plan.relation,
        witness,
        walk_fields,
        walk_words,
        &mut |bytes| {
            crate::api::prepared::decode_row(row, fields, bytes, interner, source.work(), false)?;
            if key_spans_match(interner, key_words, row, scratch)? {
                if has_text {
                    crate::api::prepared::decode_row(
                        row,
                        fields,
                        bytes,
                        interner,
                        source.work(),
                        true,
                    )?;
                }
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
    key_words: &[KeyProbePart],
    probe: &RowWords,
    scratch: &[u64],
) -> Result<bool> {
    let eq = interner.text_eq();
    for part in key_words {
        let left = probe.span_words(part.field);
        let right = &scratch[part.words()];
        let same = if probe.field_is_string(part.field) {
            eq.tokens_equal(left[0], right[0])?
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
        crate::canonical::RowError::Work(_) | crate::canonical::RowError::Allocation => {
            Error::from(error)
        }
        _ => Error::Corruption(crate::error::CorruptionError::MalformedValue(
            "key-probe canonical reconstruction",
        )),
    }
}

#[cfg(test)]
mod exact_word_tests {
    use super::*;
    use bumbledb_theory::schema::FieldDescriptor;

    #[test]
    fn stack_routing_matches_the_value_codec_including_float_holes_and_padding() {
        let mut cases = vec![
            (ValueType::Bool, Value::Bool(false), vec![0]),
            (ValueType::Bool, Value::Bool(true), vec![17]),
            (ValueType::U64, Value::U64(u64::MAX), vec![u64::MAX]),
            (ValueType::I64, Value::I64(i64::MIN), vec![0]),
            (ValueType::I64, Value::I64(i64::MAX), vec![u64::MAX]),
        ];
        for bits in [
            0,
            1,
            0x8000_0000_0000_0001,
            0x7ff0_0000_0000_0000,
            0xfff0_0000_0000_0000,
            0x7ff8_0000_0000_0000,
        ] {
            let value = bumbledb_theory::F64::from_bits(bits);
            cases.push((
                ValueType::F64,
                Value::F64(value),
                vec![value.to_order_key()],
            ));
        }
        let uuid = bumbledb_theory::Uuid::from_bytes([0xab; 16]);
        cases.push((
            ValueType::Uuid,
            Value::Uuid(uuid),
            vec![0xabab_abab_abab_abab; 2],
        ));
        for len in [1u8, 7, 8, 9, 15, 16] {
            let bytes = vec![0x6d; usize::from(len)].into_boxed_slice();
            // Trailing engine-word padding is ignored by value_of_words;
            // this arm must route identically even with nonzero padding.
            let words = vec![0x6d6d_6d6d_6d6d_6d6d; usize::from(len).div_ceil(8)];
            cases.push((
                ValueType::FixedBytes { len: len.into() },
                Value::FixedBytes(bytes),
                words,
            ));
        }
        for (ty, value, words) in cases {
            let mut actual = [0u8; 16];
            let written = encode_exact_field(&ty, &words, &mut actual).expect("exact scalar");
            let expected = crate::schema::compiled::encode_scalar_group(
                &[value],
                &[FieldDescriptor {
                    name: "v".into(),
                    value_type: ty,
                }],
            )
            .expect("canonical exact scalar");
            assert_eq!(&actual[..written], expected.as_slice(), "{ty:?}");
        }
        for invalid in [0x7fff_ffff_ffff_ffff, 0xffff_ffff_ffff_ffff, 0] {
            assert!(bumbledb_theory::F64::from_order_key(invalid).is_err());
            assert!(encode_exact_field(&ValueType::F64, &[invalid], &mut [0; 16]).is_none());
        }
        assert!(encode_exact_field(&ValueType::Uuid, &[0], &mut [0; 16]).is_none());
        assert!(encode_exact_field(&ValueType::Uuid, &[0, 0], &mut [0; 8]).is_none());
    }
}
