use super::{Cell, ResolveMemo, ResultBuffer, ResultValue, Row, ValueType};

use crate::error::Result;
use crate::interval::Interval;
use crate::schema::IntervalElement;
use crate::storage::env::ReadTxn;

impl ResultBuffer {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Empties the buffer, retaining capacity (the zero-alloc reuse path).
    pub fn clear(&mut self) {
        self.cells.clear();
        self.bytes.clear();
    }

    /// Number of result rows.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cells.len().checked_div(self.arity).unwrap_or(0)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Number of columns (find terms).
    #[must_use]
    pub fn arity(&self) -> usize {
        self.arity
    }

    /// The byte heap's length — memory observability (each distinct
    /// String value is stored once per buffer; bytes<N> cells copy their
    /// N bytes per row — docs/architecture/40-execution.md).
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.bytes.len()
    }

    /// The value at `(row, column)`.
    ///
    /// # Panics
    ///
    /// On out-of-range coordinates, and on a programmer-invariant violation
    /// (string cells are UTF-8-validated at materialization).
    #[must_use]
    pub fn get(&self, row: usize, column: usize) -> ResultValue<'_> {
        assert!(column < self.arity && row < self.len());
        match self.cells[row * self.arity + column] {
            Cell::Bool(v) => ResultValue::Bool(v),
            Cell::U64(v) => ResultValue::U64(v),
            Cell::I64(v) => ResultValue::I64(v),
            Cell::String { start, len } => ResultValue::String(
                std::str::from_utf8(&self.bytes[start..start + len])
                    .expect("validated at materialization"),
            ),
            Cell::FixedBytes { start, len } => {
                ResultValue::FixedBytes(&self.bytes[start..start + len])
            }
            Cell::IntervalU64(interval) => ResultValue::IntervalU64(interval),
            Cell::IntervalI64(interval) => ResultValue::IntervalI64(interval),
        }
    }

    /// Iterates the rows. Order is arbitrary (results are sets — the
    /// host sorts); the iterator exists so consumers stop hand-writing
    /// the index arithmetic around [`ResultBuffer::get`].
    pub fn rows(&self) -> impl Iterator<Item = Row<'_>> {
        (0..self.len()).map(move |row| Row { buffer: self, row })
    }

    /// Converts a fixed-width word to its cell — infallible by schema
    /// invariant (the all-words finalize path carries
    /// no `Result` and no dictionary plumbing per cell).
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
            ValueType::Interval { .. } => {
                unreachable!("interval finds take the two-word path (interval_cell)")
            }
        }
    }

    /// Materializes a `bytes<N>` find's padded slot words as one cell:
    /// the words' big-endian bytes, truncated to the declared N, copied
    /// into the byte heap (inline values — no dictionary, ever).
    pub(super) fn push_fixed_bytes(&mut self, len: u16, words: &[u64]) {
        let start = self.bytes.len();
        for word in words {
            self.bytes.extend_from_slice(&word.to_be_bytes());
        }
        self.bytes.truncate(start + usize::from(len));
        self.cells.push(Cell::FixedBytes {
            start,
            len: usize::from(len),
        });
    }

    /// Materializes an interval find's two slot words as one cell,
    /// re-encoded through the checked host type. The `expect` is a
    /// stored invariant, not a runtime hope: every stored interval was
    /// parsed through `Interval::new` at the write boundary (`start <
    /// end` — 10-data-model), the image columns carry the encoded words
    /// unchanged, and the executor and sinks move slot words whole — so
    /// bounds arriving here out of order name corruption, and panicking
    /// is the honest report.
    pub(super) fn interval_cell(element: IntervalElement, start: u64, end: u64) -> Cell {
        match element {
            IntervalElement::U64 => Cell::IntervalU64(
                Interval::<u64>::new(start, end).expect("stored invariant: start < end"),
            ),
            IntervalElement::I64 => {
                // Both words are the sign-flipped biased form (the
                // order-preserving I64 encoding) — decode each bound.
                let decode = |word: u64| (word ^ (1 << 63)).cast_signed();
                Cell::IntervalI64(
                    Interval::<i64>::new(decode(start), decode(end))
                        .expect("stored invariant: start < end"),
                )
            }
        }
    }

    pub(super) fn push_word(
        &mut self,
        txn: &ReadTxn<'_>,
        ty: &ValueType,
        word: u64,
        memo: &mut ResolveMemo,
    ) -> Result<()> {
        let cell = match ty {
            ValueType::Bool => Cell::Bool(word != 0),
            ValueType::U64 => Cell::U64(word),
            ValueType::I64 => Cell::I64((word ^ (1 << 63)).cast_signed()),
            ValueType::String => {
                let (start, len) = memo.resolve(txn, word, self)?;
                Cell::String { start, len }
            }
            ValueType::FixedBytes { .. } => {
                unreachable!("bytes<N> finds take the multi-word path (push_fixed_bytes)")
            }
            ValueType::Interval { .. } => {
                unreachable!("interval finds take the two-word path (interval_cell)")
            }
        };
        self.cells.push(cell);
        Ok(())
    }
}

impl<'a> Row<'a> {
    /// The value in `column` (a find-term index).
    ///
    /// # Panics
    ///
    /// On an out-of-range column.
    #[must_use]
    pub fn get(&self, column: usize) -> ResultValue<'a> {
        self.buffer.get(self.row, column)
    }
}
