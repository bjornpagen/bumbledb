use super::{Cell, ResolveMemo, ResultBuffer, ResultValue, Row, ValueType};

use crate::error::Result;
use crate::interval::Interval;
use crate::schema::IntervalElement;
use crate::storage::dict;
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
    /// String/Bytes value is stored once per buffer, docs/architecture/40-execution.md).
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
            Cell::Enum(v) => ResultValue::Enum(v),
            Cell::String { start, len } => ResultValue::String(
                std::str::from_utf8(&self.bytes[start..start + len])
                    .expect("validated at materialization"),
            ),
            Cell::Bytes { start, len } => ResultValue::Bytes(&self.bytes[start..start + len]),
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
    /// invariant (docs/perf/ PRD 08: the all-words finalize path carries
    /// no `Result` and no dictionary plumbing per cell).
    pub(super) fn word_cell(ty: &ValueType, word: u64) -> Cell {
        match ty {
            ValueType::Bool => Cell::Bool(word != 0),
            ValueType::Enum { .. } => Cell::Enum(
                // Programmer invariant, not corruption: image build
                // range-checked every stored ordinal against the schema.
                u8::try_from(word).expect("enum words fit u8"),
            ),
            ValueType::U64 => Cell::U64(word),
            ValueType::I64 => Cell::I64((word ^ (1 << 63)).cast_signed()),
            ValueType::String | ValueType::Bytes => {
                unreachable!("interned finds take the resolving path")
            }
            ValueType::Interval { .. } => {
                unreachable!("interval finds take the two-word path (interval_cell)")
            }
        }
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
            ValueType::Enum { .. } => Cell::Enum(
                // Programmer invariant, not corruption: image build
                // range-checked every stored ordinal against the schema.
                u8::try_from(word).expect("enum words fit u8"),
            ),
            ValueType::U64 => Cell::U64(word),
            ValueType::I64 => Cell::I64((word ^ (1 << 63)).cast_signed()),
            ValueType::String => {
                let (start, len) = memo.resolve(txn, word, dict::TAG_STRING, self, true)?;
                Cell::String { start, len }
            }
            ValueType::Bytes => {
                let (start, len) = memo.resolve(txn, word, dict::TAG_BYTES, self, false)?;
                Cell::Bytes { start, len }
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
