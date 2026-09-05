//! Canonical-row → column-word decoding: the one bridge from the stored
//! canonical row codec (`crate::canonical`, inline text, tagged fields) to
//! the execution engine's fixed 64-bit column words. Every consumer of a
//! committed row — image builds, key probes, the cursor fallback — decodes
//! through this walker, so the word conventions live in exactly one place:
//!
//! | Field type | Column words |
//! | --- | --- |
//! | `Bool` | one word 0/1 (byte column at the image layer) |
//! | `U64` | the value |
//! | `I64` | sign-biased order word (`v ^ 1<<63`) |
//! | `F64` | canonical total-order key |
//! | `String` | interner token ([`TextInterner`]); scans use lookup-only |
//! | `Bytes<N>` | ⌈N/8⌉ zero-padded big-endian words |
//! | `Id128` | two big-endian words (byte order = total order) |
//! | `Interval<U64/I64/F64>` | two order words (start, end) |
//! | `FixedInterval` | two order words (canonical rows carry both bounds) |
//!
//! Corrupt stored bytes refuse (typed corruption), never normalize: the
//! walker re-validates tags, widths, UTF-8, canonical float payloads and
//! interval bounds exactly as the strict parser would.

use bumbledb_theory::schema::{FieldDescriptor, ValueType};

use super::intern::{ResidentAdmit, SENTINEL_WORD, TextInterner};
use crate::canonical::field::{
    self, interval_f64_order_words, interval_i64_order_words, interval_u64_order_words,
};
use crate::error::{CorruptionError, Error, Result};
use crate::work::WorkContext;

/// How the walker turns text bytes into a word. The `Intern`/`Lookup`
/// arms hold the interner open across a whole scan (image builds); the
/// `Handle*` arms lock per text field (single-row probes and fallback
/// cursors, where holding a guard across caller-supplied closures would
/// invite deadlock).
pub(crate) enum TextWords<'i> {
    /// Mint (or find) the token — image builds and captured probe rows.
    Intern {
        interner: &'i mut TextInterner,
        work: &'i WorkContext,
        generation: &'i crate::work::GenerationHandle,
    },
    /// Find the token or the sentinel — scan-side comparisons where an
    /// un-interned text can equal no interned word.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the guard-holding lookup twin of `HandleLookup`; \
                      corruption/fixed-bytes suites drive it directly"
        )
    )]
    Lookup(&'i TextInterner),
    /// As `Intern`, locking per field through the shared handle.
    HandleIntern(&'i crate::image::intern::InternerHandle<'i>),
    /// As `Lookup`, locking per field through the shared handle.
    HandleLookup(&'i crate::image::intern::InternerHandle<'i>),
    /// Exact scratch-backed resolution for nonresident execution.
    Nonresident {
        store: &'i mut crate::image::NonresidentTextStore,
        work: &'i WorkContext,
    },
}

impl TextWords<'_> {
    fn word(&mut self, text: &str) -> Result<ResidentAdmit<u64>> {
        match self {
            Self::Intern {
                interner,
                work,
                generation,
            } => match interner.intern(text, work, generation.ledger()) {
                Ok(token) => Ok(ResidentAdmit::Ready(token)),
                Err(crate::image::intern::InternError::Cache(_))
                | Err(crate::image::intern::InternError::Allocation) => {
                    Ok(ResidentAdmit::BeyondMemory(
                        crate::image::ResidentTextExhausted::new((*generation).clone()),
                    ))
                }
                Err(error) => Err(Error::from(error)),
            },
            Self::Lookup(interner) => Ok(ResidentAdmit::Ready(interner.lookup_word(text))),
            Self::HandleIntern(handle) => handle.intern_or_spill(text),
            Self::HandleLookup(handle) => Ok(ResidentAdmit::Ready(handle.lookup_word(text))),
            Self::Nonresident { store, work } => store.intern(text, work).map(ResidentAdmit::Ready),
        }
    }
}

const fn corrupt(what: &'static str) -> Error {
    Error::Corruption(CorruptionError::MalformedValue(what))
}

fn row_error(error: crate::canonical::RowError) -> Error {
    Error::from_store(crate::storage::store::StoreError::Changes(
        crate::changes::ChangeError::Row(error),
    ))
}

struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        let (head, rest) = self
            .bytes
            .split_at_checked(len)
            .ok_or(corrupt("canonical row truncated"))?;
        self.bytes = rest;
        Ok(head)
    }

    fn word<const N: usize>(&mut self) -> Result<[u8; N]> {
        self.take(N)?
            .try_into()
            .map_err(|_| corrupt("canonical row truncated"))
    }

    fn blob(&mut self) -> Result<&'a [u8]> {
        let len = usize::try_from(u64::from_be_bytes(self.word()?))
            .map_err(|_| corrupt("canonical blob length"))?;
        self.take(len)
    }
}

pub(crate) const fn i64_word(value: i64) -> u64 {
    field::i64_word(value)
}

/// Decode one canonical row into flat column words (one word per image
/// column, `Bool` as 0/1), appended to `out`. The caller charges scan
/// work per row; this walker charges per text byte through the intern arm.
/// # Errors
/// Typed corruption for any malformed stored byte; work/allocation refusal
/// from interning.
#[expect(
    clippy::too_many_lines,
    reason = "the per-type decode arms are one linear wire table"
)]
pub(crate) fn row_words(
    fields: &[FieldDescriptor],
    bytes: &[u8],
    text: &mut TextWords<'_>,
    out: &mut Vec<u64>,
) -> Result<ResidentAdmit<()>> {
    let mut reader = Reader { bytes };
    if usize::from(u16::from_be_bytes(reader.word()?)) != fields.len() {
        return Err(corrupt("canonical row arity"));
    }
    for descriptor in fields {
        let tag = reader.word::<1>()?[0];
        match (tag, &descriptor.value_type) {
            (0, ValueType::Bool) => match reader.word::<1>()?[0] {
                bit @ (0 | 1) => out.push(u64::from(bit)),
                _ => return Err(corrupt("canonical bool payload")),
            },
            (1, ValueType::U64) => out.push(u64::from_be_bytes(reader.word()?)),
            (2, ValueType::I64) => {
                out.push(i64_word(i64::from_be_bytes(reader.word()?)));
            }
            (3, ValueType::F64) => {
                let value = bumbledb_theory::F64::from_canonical_be_bytes(reader.word()?)
                    .map_err(|_| corrupt("non-canonical stored F64"))?;
                out.push(value.to_order_key());
            }
            (4, ValueType::String) => {
                let blob = reader.blob()?;
                let text_str =
                    std::str::from_utf8(blob).map_err(|_| corrupt("non-UTF-8 stored text"))?;
                match text.word(text_str)? {
                    ResidentAdmit::Ready(token) => out.push(token),
                    ResidentAdmit::BeyondMemory(exhausted) => {
                        return Ok(ResidentAdmit::BeyondMemory(exhausted));
                    }
                }
            }
            (5, ValueType::FixedBytes { len }) => {
                let blob = reader.blob()?;
                if blob.len() != usize::from(*len) {
                    return Err(corrupt("stored bytes<N> width"));
                }
                for chunk_start in (0..blob.len()).step_by(8) {
                    let mut word = [0u8; 8];
                    let end = (chunk_start + 8).min(blob.len());
                    word[..end - chunk_start].copy_from_slice(&blob[chunk_start..end]);
                    out.push(u64::from_be_bytes(word));
                }
            }
            (6, _) if field::interval_tag_u64(descriptor) => {
                let (start, end) = interval_u64_order_words(
                    u64::from_be_bytes(reader.word()?),
                    u64::from_be_bytes(reader.word()?),
                    descriptor,
                )
                .map_err(row_error)?;
                out.extend([start, end]);
            }
            (7, _) if field::interval_tag_i64(descriptor) => {
                let (start, end) = interval_i64_order_words(
                    i64::from_be_bytes(reader.word()?),
                    i64::from_be_bytes(reader.word()?),
                    descriptor,
                )
                .map_err(row_error)?;
                out.extend([start, end]);
            }
            (8, ValueType::Id128) => {
                let raw: [u8; 16] = reader.word()?;
                out.push(u64::from_be_bytes(
                    raw[..8].try_into().expect("sixteen bytes"),
                ));
                out.push(u64::from_be_bytes(
                    raw[8..].try_into().expect("sixteen bytes"),
                ));
            }
            (9, _) if field::interval_tag_f64(descriptor) => {
                let start = bumbledb_theory::F64::from_canonical_be_bytes(reader.word()?)
                    .map_err(|_| corrupt("non-canonical stored F64 endpoint"))?;
                let end = bumbledb_theory::F64::from_canonical_be_bytes(reader.word()?)
                    .map_err(|_| corrupt("non-canonical stored F64 endpoint"))?;
                let (start, end) =
                    interval_f64_order_words(start, end, descriptor).map_err(row_error)?;
                if start == SENTINEL_WORD {
                    return Err(corrupt("stored dense interval bounds"));
                }
                out.extend([start, end]);
            }
            _ => return Err(corrupt("canonical row tag/schema disagreement")),
        }
    }
    if !reader.bytes.is_empty() {
        return Err(corrupt("canonical row trailing bytes"));
    }
    Ok(ResidentAdmit::Ready(()))
}

/// One decoded row's flat column words plus its field→column map — the
/// key-probe and cursor-fallback row shape ([`Self::operand`] mirrors the
/// image layer's span dispatch for single rows).
pub(crate) struct RowWords {
    spans: Box<[super::ColumnSpan]>,
    strings: Box<[bool]>,
    words: Vec<u64>,
}

impl RowWords {
    pub(crate) fn new(field_types: &[ValueType]) -> Self {
        Self {
            spans: super::column_spans(field_types),
            strings: field_types
                .iter()
                .map(|ty| matches!(ty, ValueType::String))
                .collect(),
            words: Vec::new(),
        }
    }

    /// True when this field's column word is an intern/scratch text token.
    #[must_use]
    pub(crate) fn field_is_string(&self, field: bumbledb_theory::schema::FieldId) -> bool {
        self.strings
            .get(usize::from(field.0))
            .copied()
            .unwrap_or(false)
    }

    /// Decode `bytes` into this row's words (replacing the previous row).
    /// # Errors
    /// As [`row_words`].
    pub(crate) fn decode(
        &mut self,
        fields: &[FieldDescriptor],
        bytes: &[u8],
        text: &mut TextWords<'_>,
    ) -> Result<ResidentAdmit<()>> {
        self.words.clear();
        row_words(fields, bytes, text, &mut self.words)
    }

    pub(crate) fn span_words(&self, field: bumbledb_theory::schema::FieldId) -> &[u64] {
        let span = self.spans[usize::from(field.0)];
        let first = usize::from(span.first_column);
        &self.words[first..first + usize::from(span.width.column_count())]
    }

    pub(crate) fn operand(
        &self,
        field: bumbledb_theory::schema::FieldId,
    ) -> crate::exec::dispatch::FactOperand {
        let span = self.spans[usize::from(span_field(field))];
        let first = usize::from(span.first_column);
        match span.width {
            super::ColumnWidth::Byte | super::ColumnWidth::Word => {
                crate::exec::dispatch::FactOperand::Word(self.words[first])
            }
            super::ColumnWidth::WordPair => {
                crate::exec::dispatch::FactOperand::Pair(self.words[first], self.words[first + 1])
            }
            super::ColumnWidth::Words { count } => {
                let count = usize::from(count);
                let mut words = [0u64; 8];
                words[..count].copy_from_slice(&self.words[first..first + count]);
                crate::exec::dispatch::FactOperand::Block {
                    words,
                    count: u8::try_from(count).expect("bytes width is at most 8 words"),
                }
            }
        }
    }
}

const fn span_field(field: bumbledb_theory::schema::FieldId) -> u16 {
    field.0
}

/// Field-space filter operands over one decoded row — resolved templates
/// only (a `PendingIntern` latches before any provider is consulted).
impl crate::image::view::Operands for RowWords {
    type Error = crate::error::Error;

    fn word(&self, at: crate::image::view::OperandAddr) -> Result<u64> {
        Ok(match self.operand(at.field()) {
            crate::exec::dispatch::FactOperand::Word(w) => w,
            crate::exec::dispatch::FactOperand::Pair(..)
            | crate::exec::dispatch::FactOperand::Block { .. } => {
                unreachable!("validated: word operands are scalar fields")
            }
        })
    }

    fn pair(&self, at: crate::image::view::OperandAddr) -> Result<(u64, u64)> {
        Ok(match self.operand(at.field()) {
            crate::exec::dispatch::FactOperand::Pair(s, e) => (s, e),
            crate::exec::dispatch::FactOperand::Word(_)
            | crate::exec::dispatch::FactOperand::Block { .. } => {
                unreachable!("validated: interval predicates read interval fields")
            }
        })
    }

    fn block(&self, at: crate::image::view::OperandAddr) -> Result<([u64; 8], u8)> {
        Ok(match self.operand(at.field()) {
            crate::exec::dispatch::FactOperand::Block { words, count } => (words, count),
            crate::exec::dispatch::FactOperand::Word(_)
            | crate::exec::dispatch::FactOperand::Pair(..) => {
                unreachable!("validated: block operands are bytes<N>")
            }
        })
    }

    fn loaded(&self, at: crate::image::view::OperandAddr) -> Result<crate::image::view::Loaded> {
        Ok(match self.operand(at.field()) {
            crate::exec::dispatch::FactOperand::Word(w) => crate::image::view::Loaded::Word(w),
            crate::exec::dispatch::FactOperand::Pair(s, e) => {
                crate::image::view::Loaded::Pair(s, e)
            }
            crate::exec::dispatch::FactOperand::Block { words, count } => {
                crate::image::view::Loaded::Block { words, count }
            }
        })
    }

    fn string_field(&self, at: crate::image::view::OperandAddr) -> bool {
        self.field_is_string(at.field())
    }
}

#[cfg(test)]
mod string_field_tests {
    use super::*;
    use crate::image::view::{OperandAddr, Operands};
    use bumbledb_theory::schema::FieldId;

    /// Probe/fallback `RowWords` must mark String columns so `holds` uses
    /// [`crate::image::TextEq`] instead of raw word identity.
    #[test]
    fn d02_row_words_string_field_marks_string_columns() {
        let row = RowWords::new(&[ValueType::U64, ValueType::String, ValueType::I64]);
        assert!(!Operands::string_field(
            &row,
            OperandAddr::from(FieldId(0))
        ));
        assert!(Operands::string_field(
            &row,
            OperandAddr::from(FieldId(1))
        ));
        assert!(!Operands::string_field(
            &row,
            OperandAddr::from(FieldId(2))
        ));
        assert!(row.field_is_string(FieldId(1)));
        assert!(!row.field_is_string(FieldId(0)));
    }
}
