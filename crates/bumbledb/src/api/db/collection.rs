//! The bridge crates' parse-once internal write representation, built once
//! at the boundary, carrying its own shape proof, consumed by borrow until
//! the rows enter a transaction.
//!
//! Construction IS the parse: [`CollectionBuilder`] performs the whole
//! shape judgment — arity per row, type-kind per cell against the sealed
//! roster — before any row is staged. `Send` by construction (owned arenas,
//! no borrow of the feeding thread): built on the caller's thread,
//! consumable on the transaction's.

use crate::error::{FactShapeError, Mismatch, Result};
use crate::ir::Value;
use bumbledb_theory::schema::{
    FieldDescriptor, FieldId, RelationId, ValueMismatch, ValueType, value_matches,
};
use bumbledb_theory::{F64, Id128, Interval};

pub(super) fn shape_mismatch(
    rel: RelationId,
    field: FieldId,
    mismatch: ValueMismatch,
) -> FactShapeError {
    match mismatch {
        ValueMismatch::Type => FactShapeError::TypeMismatch {
            relation: rel,
            field,
        },
    }
}

#[derive(Debug, Clone, Copy)]
enum Cell {
    Bool(bool),
    U64(u64),
    I64(i64),
    F64(F64),
    Id128(Id128),
    IntervalU64(Interval<u64>),
    IntervalI64(Interval<i64>),
    IntervalF64(Interval<F64>),

    FixedBytes { off: u32, len: u32 },

    Str { off: u32, len: u32 },
}

/// A shape-proved collection of dynamic rows for exactly one relation.
/// Built by [`CollectionBuilder`] / [`AcceptedCollection::from_value_rows`].
#[derive(Debug)]
pub struct AcceptedCollection {
    relation: RelationId,

    arity: u16,

    roster: Box<[ValueType]>,

    rows: u64,

    cells: Vec<Cell>,

    strings: String,

    bytes: Vec<u8>,
}

impl AcceptedCollection {
    /// # Errors
    /// Arity/type refusals, before any row is staged.
    pub fn from_value_rows(
        relation: RelationId,
        fields: &[FieldDescriptor],
        rows: impl IntoIterator<Item = impl AsRef<[Value]>>,
    ) -> Result<Self> {
        if fields.is_empty() {
            let mut count: u64 = 0;
            for row in rows {
                let row = row.as_ref();
                if !row.is_empty() {
                    return Err(FactShapeError::ArityMismatch {
                        relation,
                        mismatch: Mismatch {
                            witnessed: row.len(),
                            required: 0,
                        },
                    }
                    .into());
                }
                count += 1;
            }
            return CollectionBuilder::new(relation, fields).seal_nullary(count);
        }
        let mut builder = CollectionBuilder::new(relation, fields);
        for row in rows {
            builder.push_value_row(row.as_ref())?;
        }
        builder.seal()
    }

    #[must_use]
    pub fn relation(&self) -> RelationId {
        self.relation
    }

    #[must_use]
    pub fn rows(&self) -> u64 {
        self.rows
    }

    #[must_use]
    pub fn arity(&self) -> u16 {
        self.arity
    }

    pub(super) fn roster(&self) -> &[ValueType] {
        &self.roster
    }

    /// Materialize one row's canonical values into `out` (cleared first).
    /// # Panics
    /// `row` out of range (callers iterate `0..rows()`).
    pub(super) fn row_values_into(&self, row: u64, out: &mut Vec<Value>) {
        out.clear();
        let arity = usize::from(self.arity);
        let start = usize::try_from(row).expect("row index fits usize") * arity;
        for cell in &self.cells[start..start + arity] {
            out.push(match *cell {
                Cell::Bool(value) => Value::Bool(value),
                Cell::U64(value) => Value::U64(value),
                Cell::I64(value) => Value::I64(value),
                Cell::F64(value) => Value::F64(value),
                Cell::Id128(value) => Value::Id128(value),
                Cell::IntervalU64(interval) => Value::IntervalU64(interval),
                Cell::IntervalI64(interval) => Value::IntervalI64(interval),
                Cell::IntervalF64(interval) => Value::IntervalF64(interval),
                Cell::FixedBytes { off, len } => {
                    Value::FixedBytes(Box::from(&self.bytes[span(off, len)]))
                }
                Cell::Str { off, len } => Value::String(Box::from(&self.strings[span(off, len)])),
            });
        }
    }
}

fn span(off: u32, len: u32) -> std::ops::Range<usize> {
    off as usize..off as usize + len as usize
}

/// THE one dynamic-row parse implementation: feeds cells positionally,
/// judges each against the sealed roster, and [`CollectionBuilder::seal`]s
/// into the proof-carrying [`AcceptedCollection`]. Two feeding surfaces —
/// [`CollectionBuilder::push_value`] for the engine dyn lane and the typed
/// pushes for the bridge — share one judgment, so the rules cannot fork.
pub struct CollectionBuilder<'s> {
    relation: RelationId,
    fields: &'s [FieldDescriptor],

    arity: u16,

    rows: u64,

    fill: u16,
    cells: Vec<Cell>,
    strings: String,
    bytes: Vec<u8>,
}

impl<'s> CollectionBuilder<'s> {
    /// # Panics
    /// A sealed schema bounds field counts at `u16::MAX`.
    #[must_use]
    pub fn new(relation: RelationId, fields: &'s [FieldDescriptor]) -> Self {
        Self {
            relation,
            fields,
            arity: u16::try_from(fields.len()).expect("field count fits u16"),
            rows: 0,
            fill: 0,
            cells: Vec::new(),
            strings: String::new(),
            bytes: Vec::new(),
        }
    }

    fn expected(&self) -> Result<(FieldId, &'s ValueType)> {
        match self.fields.get(usize::from(self.fill)) {
            Some(field) => Ok((FieldId(self.fill), &field.value_type)),
            None => Err(FactShapeError::ArityMismatch {
                relation: self.relation,
                mismatch: Mismatch {
                    witnessed: usize::from(self.fill) + 1,
                    required: self.fields.len(),
                },
            }
            .into()),
        }
    }

    fn advance(&mut self) {
        self.fill += 1;
        if usize::from(self.fill) == self.fields.len() {
            self.fill = 0;
            self.rows += 1;
        }
    }

    /// # Errors
    /// Arity/type refusals.
    pub fn push_value_row(&mut self, row: &[Value]) -> Result<()> {
        if row.len() != self.fields.len() {
            return Err(FactShapeError::ArityMismatch {
                relation: self.relation,
                mismatch: Mismatch {
                    witnessed: row.len(),
                    required: self.fields.len(),
                },
            }
            .into());
        }
        if row.is_empty() {
            // Nullary rows go through `seal_nullary`; a pushed empty row
            // cannot advance the fill and is refused.
            return Err(FactShapeError::ArityMismatch {
                relation: self.relation,
                mismatch: Mismatch {
                    witnessed: 1,
                    required: 0,
                },
            }
            .into());
        }
        for value in row {
            self.push_value(value)?;
        }
        Ok(())
    }

    /// # Errors
    /// Arity/type refusals.
    pub fn push_value(&mut self, value: &Value) -> Result<()> {
        let (field, expected) = self.expected()?;
        if let Err(mismatch) = value_matches(value, expected) {
            return Err(shape_mismatch(self.relation, field, mismatch).into());
        }
        let cell = match value {
            Value::Bool(value) => Cell::Bool(*value),
            Value::U64(value) => Cell::U64(*value),
            Value::I64(value) => Cell::I64(*value),
            Value::F64(value) => Cell::F64(*value),
            Value::Id128(value) => Cell::Id128(*value),
            Value::String(text) => self.land_str(text)?,
            Value::FixedBytes(raw) => self.land_bytes(raw)?,
            Value::IntervalU64(interval) => Cell::IntervalU64(*interval),
            Value::IntervalI64(interval) => Cell::IntervalI64(*interval),
            Value::IntervalF64(interval) => Cell::IntervalF64(*interval),
        };
        self.cells.push(cell);
        self.advance();
        Ok(())
    }

    /// # Errors
    /// Arity/type refusals.
    pub fn push_bool(&mut self, value: bool) -> Result<()> {
        self.push_scalar(&Value::Bool(value), Cell::Bool(value))
    }

    /// # Errors
    /// Arity/type refusals.
    pub fn push_u64(&mut self, value: u64) -> Result<()> {
        self.push_scalar(&Value::U64(value), Cell::U64(value))
    }

    /// # Errors
    /// Arity/type refusals.
    pub fn push_i64(&mut self, value: i64) -> Result<()> {
        self.push_scalar(&Value::I64(value), Cell::I64(value))
    }

    /// # Errors
    /// Arity/type refusals.
    pub fn push_f64(&mut self, value: F64) -> Result<()> {
        self.push_scalar(&Value::F64(value), Cell::F64(value))
    }

    /// # Errors
    /// Arity/type refusals.
    pub fn push_id128(&mut self, value: Id128) -> Result<()> {
        self.push_scalar(&Value::Id128(value), Cell::Id128(value))
    }

    /// # Errors
    /// Arity/type refusals.
    pub fn push_interval_u64(&mut self, value: Interval<u64>) -> Result<()> {
        self.push_scalar(&Value::IntervalU64(value), Cell::IntervalU64(value))
    }

    /// # Errors
    /// Arity/type refusals.
    pub fn push_interval_i64(&mut self, value: Interval<i64>) -> Result<()> {
        self.push_scalar(&Value::IntervalI64(value), Cell::IntervalI64(value))
    }

    /// # Errors
    /// Arity/type refusals.
    pub fn push_interval_f64(&mut self, value: Interval<F64>) -> Result<()> {
        self.push_scalar(&Value::IntervalF64(value), Cell::IntervalF64(value))
    }

    fn push_scalar(&mut self, judge: &Value, cell: Cell) -> Result<()> {
        let (field, expected) = self.expected()?;
        if let Err(mismatch) = value_matches(judge, expected) {
            return Err(shape_mismatch(self.relation, field, mismatch).into());
        }
        self.cells.push(cell);
        self.advance();
        Ok(())
    }

    /// # Errors
    /// Arity/type refusals.
    pub fn push_str(&mut self, value: &str) -> Result<()> {
        let (field, expected) = self.expected()?;
        if !matches!(expected, ValueType::String) {
            return Err(shape_mismatch(self.relation, field, ValueMismatch::Type).into());
        }
        let cell = self.land_str(value)?;
        self.cells.push(cell);
        self.advance();
        Ok(())
    }

    /// # Errors
    /// Arity/type refusals.
    pub fn push_bytes(&mut self, value: &[u8]) -> Result<()> {
        let (field, expected) = self.expected()?;
        if !matches!(expected, ValueType::FixedBytes { len } if value.len() == usize::from(*len)) {
            return Err(shape_mismatch(self.relation, field, ValueMismatch::Type).into());
        }
        let cell = self.land_bytes(value)?;
        self.cells.push(cell);
        self.advance();
        Ok(())
    }

    fn land_str(&mut self, value: &str) -> Result<Cell> {
        let off = self.arena_span(self.strings.len())?;
        let len = self.arena_span(value.len())?;
        self.strings.push_str(value);
        Ok(Cell::Str { off, len })
    }

    fn land_bytes(&mut self, value: &[u8]) -> Result<Cell> {
        let off = self.arena_span(self.bytes.len())?;
        let len = self.arena_span(value.len())?;
        self.bytes.extend_from_slice(value);
        Ok(Cell::FixedBytes { off, len })
    }

    /// The arena bound is a typed refusal, never a panic.
    fn arena_span(&self, len: usize) -> Result<u32> {
        u32::try_from(len).map_err(|_| {
            FactShapeError::PayloadBound {
                relation: self.relation,
            }
            .into()
        })
    }

    /// # Errors
    /// A nonempty roster refuses this seal: on a fieldless roster no push
    /// can have run, so the stated count IS the collection.
    pub fn seal_nullary(mut self, rows: u64) -> Result<AcceptedCollection> {
        if !self.fields.is_empty() {
            return Err(FactShapeError::ArityMismatch {
                relation: self.relation,
                mismatch: Mismatch {
                    witnessed: 0,
                    required: self.fields.len(),
                },
            }
            .into());
        }

        debug_assert!(self.rows == 0, "the stated count IS the collection");
        self.rows = rows;
        self.seal()
    }

    /// # Errors
    /// A partially filled final row refuses the seal.
    pub fn seal(self) -> Result<AcceptedCollection> {
        if self.fill != 0 {
            return Err(FactShapeError::ArityMismatch {
                relation: self.relation,
                mismatch: Mismatch {
                    witnessed: usize::from(self.fill),
                    required: self.fields.len(),
                },
            }
            .into());
        }
        debug_assert!(
            u64::try_from(self.cells.len()) == Ok(self.rows * u64::from(self.arity)),
            "complete rows by construction"
        );
        Ok(AcceptedCollection {
            relation: self.relation,
            arity: self.arity,
            roster: self.fields.iter().map(|field| field.value_type).collect(),
            rows: self.rows,
            cells: self.cells,
            strings: self.strings,
            bytes: self.bytes,
        })
    }
}
