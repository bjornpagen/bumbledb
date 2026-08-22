use super::{Answer, AnswerValue, Answers, Cell, ResolveMemo, ValueType};

use crate::error::Result;
use crate::storage::catalog::CatalogRead;
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

            Cell::String { start, len } => AnswerValue::String(&self.text[start..start + len]),
            Cell::FixedBytes { start, len } => {
                AnswerValue::FixedBytes(&self.blob[start..start + len])
            }
            Cell::IntervalU64(interval) => AnswerValue::IntervalU64(interval),
            Cell::IntervalI64(interval) => AnswerValue::IntervalI64(interval),
        }
    }

    pub fn answers(&self) -> impl Iterator<Item = Answer<'_>> {
        (0..self.len()).map(move |answer| Answer {
            buffer: self,
            answer,
        })
    }

    /// invariant. The point fast lane's per-cell decode; the finalize

    pub(super) fn word_cell(ty: &ValueType, word: u64) -> Cell {
        match ty {
            ValueType::Bool => Cell::Bool(word != 0),
            ValueType::U64 => Cell::U64(word),
            ValueType::I64 => Cell::I64((word ^ (1 << 63)).cast_signed()),
            ValueType::String => {
                unreachable!("interned finds take the resolving path")
            }
            ValueType::FixedBytes { .. } => {
                unreachable!("bytes<N> finds take the multi-word path (push_fixed_bytes)")
            }
            ValueType::Interval { .. } | ValueType::FixedInterval { .. } => {
                unreachable!("interval finds take the two-word path (interval_cell)")
            }
        }
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
        }
    }

    pub(super) fn push_word<C: CatalogRead>(
        &mut self,
        catalog: &C,
        ty: &ValueType,
        word: u64,
        memo: &mut ResolveMemo,
    ) -> Result<()> {
        let cell = match ty {
            ValueType::Bool => Cell::Bool(word != 0),
            ValueType::U64 => Cell::U64(word),
            ValueType::I64 => Cell::I64((word ^ (1 << 63)).cast_signed()),
            ValueType::String => {
                let (start, len) = memo.resolve(catalog, word, self)?;
                Cell::String { start, len }
            }
            ValueType::FixedBytes { .. } => {
                unreachable!("bytes<N> finds take the multi-word path (push_fixed_bytes)")
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
