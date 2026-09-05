use super::{Answer, AnswerValue, Answers, Cell, ResolveMemo, ValueType};

use crate::error::Result;
use crate::image::intern::InternerHandle;
use crate::image::NonresidentTextStore;
use bumbledb_theory::Interval;
use bumbledb_theory::schema::IntervalElement;

impl Answers {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.cells.clear();
        self.text.clear();
        self.blob.clear();
    }

    pub(crate) fn begin(&mut self, arity: usize) {
        self.clear();
        self.arity = arity;
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.cells.len().checked_div(self.arity).unwrap_or(0)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub fn arity(&self) -> usize {
        self.arity
    }

    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.text.len() + self.blob.len()
    }

    /// # Panics
    #[must_use]
    pub fn get(&self, answer: usize, column: usize) -> AnswerValue<'_> {
        assert!(column < self.arity && answer < self.len());
        match self.cells[answer * self.arity + column] {
            Cell::Bool(v) => AnswerValue::Bool(v),
            Cell::U64(v) => AnswerValue::U64(v),
            Cell::I64(v) => AnswerValue::I64(v),
            Cell::F64(v) => AnswerValue::F64(v),
            Cell::Id128(v) => AnswerValue::Id128(v),

            Cell::String { start, len } => AnswerValue::String(&self.text[start..start + len]),
            Cell::FixedBytes { start, len } => {
                AnswerValue::FixedBytes(&self.blob[start..start + len])
            }
            Cell::IntervalU64(interval) => AnswerValue::IntervalU64(interval),
            Cell::IntervalI64(interval) => AnswerValue::IntervalI64(interval),
            Cell::IntervalF64(interval) => AnswerValue::IntervalF64(interval),
        }
    }

    pub fn answers(&self) -> impl Iterator<Item = Answer<'_>> {
        (0..self.len()).map(move |answer| Answer {
            buffer: self,
            answer,
        })
    }

    /// Append one decoded cell (the sealed-result copy path): text and
    /// byte payloads land in this buffer's own heaps.
    pub(crate) fn push_value(&mut self, value: &AnswerValue<'_>) {
        let cell = match value {
            AnswerValue::Bool(v) => Cell::Bool(*v),
            AnswerValue::U64(v) => Cell::U64(*v),
            AnswerValue::I64(v) => Cell::I64(*v),
            AnswerValue::F64(v) => Cell::F64(*v),
            AnswerValue::Id128(v) => Cell::Id128(*v),
            AnswerValue::String(text) => {
                let start = self.text.len();
                self.text.push_str(text);
                Cell::String {
                    start,
                    len: text.len(),
                }
            }
            AnswerValue::FixedBytes(raw) => {
                let start = self.blob.len();
                self.blob.extend_from_slice(raw);
                Cell::FixedBytes {
                    start,
                    len: raw.len(),
                }
            }
            AnswerValue::IntervalU64(interval) => Cell::IntervalU64(*interval),
            AnswerValue::IntervalI64(interval) => Cell::IntervalI64(*interval),
            AnswerValue::IntervalF64(interval) => Cell::IntervalF64(*interval),
        };
        self.cells.push(cell);
    }

    /// invariant. The point fast lane's per-cell decode; the finalize
    pub(super) fn word_cell(ty: &ValueType, word: u64) -> Result<Cell> {
        Ok(match ty {
            ValueType::Bool => Cell::Bool(word != 0),
            ValueType::U64 => Cell::U64(word),
            ValueType::I64 => Cell::I64((word ^ (1 << 63)).cast_signed()),
            ValueType::F64 => Cell::F64(crate::encoding::decode_f64(word.to_be_bytes())?),
            ValueType::String => {
                unreachable!("interned finds take the resolving path")
            }
            ValueType::FixedBytes { .. } => {
                unreachable!("bytes<N> finds take the multi-word path (push_fixed_bytes)")
            }
            ValueType::Id128 => {
                unreachable!("id128 finds take the two-word path (id128_cell)")
            }
            ValueType::Interval { .. } | ValueType::FixedInterval { .. } => {
                unreachable!("interval finds take the two-word path (interval_cell)")
            }
        })
    }

    /// An application-owned identity from its two big-endian column words
    /// (verbatim bytes: byte order IS the value's one total order).
    pub(super) fn id128_cell(hi: u64, lo: u64) -> Cell {
        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&hi.to_be_bytes());
        bytes[8..].copy_from_slice(&lo.to_be_bytes());
        Cell::Id128(bumbledb_theory::Id128::from_bytes(bytes))
    }

    pub(super) fn fixed_bytes_cell(&mut self, len: u16, words: &[u64]) -> Cell {
        let start = self.blob.len();
        for word in words {
            self.blob.extend_from_slice(&word.to_be_bytes());
        }
        self.blob.truncate(start + usize::from(len));
        Cell::FixedBytes {
            start,
            len: usize::from(len),
        }
    }

    pub(super) fn push_fixed_bytes(&mut self, len: u16, words: &[u64]) {
        let cell = self.fixed_bytes_cell(len, words);
        self.cells.push(cell);
    }

    /// stored invariant, not a runtime hope: every stored interval was
    pub(super) fn interval_cell(element: IntervalElement, start: u64, end: u64) -> Cell {
        match element {
            IntervalElement::U64 => Cell::IntervalU64(
                Interval::<u64>::new(start, end).expect("stored invariant: start < end"),
            ),
            IntervalElement::I64 => {
                let decode = |word: u64| (word ^ (1 << 63)).cast_signed();
                Cell::IntervalI64(
                    Interval::<i64>::new(decode(start), decode(end))
                        .expect("stored invariant: start < end"),
                )
            }
            IntervalElement::F64 => {
                // Dense-line words are F64 order keys; stored canonical
                // endpoints decode infallibly, and a corrupt hole was
                // already refused at image decode (`Decode::IntervalF64`).
                let decode = |word: u64| {
                    bumbledb_theory::F64::from_order_key(word)
                        .expect("stored invariant: canonical F64 endpoint")
                };
                Cell::IntervalF64(
                    Interval::new(decode(start), decode(end))
                        .expect("stored invariant: start < end"),
                )
            }
        }
    }

    pub(super) fn push_word(
        &mut self,
        interner: &InternerHandle<'_>,
        store: Option<&mut NonresidentTextStore>,
        ty: &ValueType,
        word: u64,
        memo: &mut ResolveMemo,
    ) -> Result<()> {
        let cell = match ty {
            ValueType::Bool => Cell::Bool(word != 0),
            ValueType::U64 => Cell::U64(word),
            ValueType::I64 => Cell::I64((word ^ (1 << 63)).cast_signed()),
            ValueType::F64 => Cell::F64(crate::encoding::decode_f64(word.to_be_bytes())?),
            ValueType::String => {
                let (start, len) = memo.resolve(interner, store, word, self)?;
                Cell::String { start, len }
            }
            ValueType::FixedBytes { .. } => {
                unreachable!("bytes<N> finds take the multi-word path (push_fixed_bytes)")
            }
            ValueType::Id128 => {
                unreachable!("id128 finds take the two-word path (id128_cell)")
            }
            ValueType::Interval { .. } | ValueType::FixedInterval { .. } => {
                unreachable!("interval finds take the two-word path (interval_cell)")
            }
        };
        self.cells.push(cell);
        Ok(())
    }
}

impl<'a> Answer<'a> {
    /// # Panics
    #[must_use]
    pub fn get(&self, column: usize) -> AnswerValue<'a> {
        self.buffer.get(self.answer, column)
    }
}
