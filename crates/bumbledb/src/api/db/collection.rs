//! The accepted collection (`proposals/one-representation/20`): ONE
//! internal write representation, built once at the boundary, carrying
//! its own shape proof, consumed by borrow until the bytes enter the
//! delta.
//!
//! Construction IS the parse (King: a parser returns a type that carries
//! the proof). [`CollectionBuilder`] performs the whole shape judgment —
//! arity per row, type-kind per cell against the sealed roster (the one
//! rule set, `bumbledb_theory::schema::value_matches`), `bytes<N>` width,
//! interval nonemptiness (the checked [`Interval`] type) — and a sealed
//! [`AcceptedCollection`] *is* the proof: there is no unchecked
//! constructor and no builder that seals a partial row, so an
//! arity-illegal or type-illegal collection is unrepresentable, not
//! refused downstream. The physical form is pinned (R2): flat,
//! arity-strided, row-major cells — `(row r, field i)` at `r·arity + i`
//! — over one UTF-8 string arena and one bytes arena; no per-row
//! container exists anywhere between the constructor and the encoded
//! fact bytes.
//!
//! `pub` but `#[doc(hidden)]` — a transport type for the bridge crates,
//! not embedding API (the `introspect`/`profile` harness-surface
//! precedent). The spans are batch-local transport: they are never a
//! database identifier and nothing about them is observable through any
//! read surface.

use crate::encoding::{InternId, ValueRef};
use crate::error::{FactShapeError, Mismatch, Result};
use crate::ir::Value;
use bumbledb_theory::Interval;
use bumbledb_theory::schema::{
    FieldDescriptor, FieldId, RelationId, ValueMismatch, ValueType, value_matches,
};

use super::encode_dyn::shape_mismatch;

/// One accepted cell: fixed-width tagged, `Copy`, with variable-width
/// payloads as `(offset, len)` spans into the owning collection's arenas.
#[derive(Debug, Clone, Copy)]
enum Cell {
    Bool(bool),
    U64(u64),
    I64(i64),
    IntervalU64(Interval<u64>),
    IntervalI64(Interval<i64>),
    /// Span into [`AcceptedCollection::bytes`].
    FixedBytes {
        off: u32,
        len: u32,
    },
    /// Span into [`AcceptedCollection::strings`].
    Str {
        off: u32,
        len: u32,
    },
}

/// One borrowed cell of an accepted row — the `ParsedCell` vocabulary
/// re-homed onto the arena spans: `Str`/`FixedBytes` resolve to
/// `&str`/`&[u8]` borrowed from the collection.
#[derive(Clone, Copy)]
pub(super) enum CellView<'c> {
    Bool(bool),
    U64(u64),
    I64(i64),
    IntervalU64(Interval<u64>),
    IntervalI64(Interval<i64>),
    FixedBytes(&'c [u8]),
    Str(&'c str),
}

/// A shape-proved collection of dynamic rows for exactly one relation:
/// the one representation a collection crosses the host→engine boundary
/// in. `Send` by construction (owned arenas, no borrow of the feeding
/// thread): built on the caller's thread, consumable on the
/// transaction's.
///
/// Fields are private — the proof is only mintable through
/// [`CollectionBuilder`] / [`AcceptedCollection::from_value_rows`].
#[derive(Debug)]
pub struct AcceptedCollection {
    /// The one relation every row belongs to, resolved by the feeder.
    relation: RelationId,
    /// Proved equal to the feeding roster's width at construction;
    /// re-anchored against the *target* roster at apply (the
    /// authoritative second wall — a collection may outlive the handle
    /// it was built against).
    arity: u16,
    /// Exact row count — `MutationReport::submitted`'s input.
    rows: u64,
    /// R2, pinned: flat, arity-strided, row-major. `rows · arity` cells.
    cells: Vec<Cell>,
    /// The one UTF-8 arena (`proposals/one-representation/30`): every
    /// string cell is one copy, landed here at parse.
    strings: String,
    /// The one `bytes<N>` arena, same span law.
    bytes: Vec<u8>,
}

impl AcceptedCollection {
    /// The dyn lane's one-call parse: per-row arity first, then the
    /// per-cell judgment — exactly the order (and the errors) of the
    /// retired `parse_dyn_row` collection loop. Empty is lawful and
    /// constructs without touching the roster.
    ///
    /// # Errors
    ///
    /// `FactShape` on an arity or type-kind mismatch, naming the
    /// relation (and field) exactly as the dyn write surface always has.
    pub fn from_value_rows(
        relation: RelationId,
        fields: &[FieldDescriptor],
        rows: impl IntoIterator<Item = impl AsRef<[Value]>>,
    ) -> Result<Self> {
        let mut builder = CollectionBuilder::new(relation, fields);
        for row in rows {
            builder.push_value_row(row.as_ref())?;
        }
        builder.seal()
    }

    /// The one relation every row was judged against.
    #[must_use]
    pub fn relation(&self) -> RelationId {
        self.relation
    }

    /// Exact row count. Zero seals lawfully.
    #[must_use]
    pub fn rows(&self) -> u64 {
        self.rows
    }

    /// The roster width the rows were judged against.
    #[must_use]
    pub fn arity(&self) -> u16 {
        self.arity
    }

    /// Row `row`'s `arity` cells, borrowed from the arenas — the
    /// engine's consumption surface: no per-row container is ever
    /// allocated between the constructor and the encoded fact bytes.
    pub(super) fn row_cells(&self, row: u64) -> impl Iterator<Item = CellView<'_>> {
        let arity = usize::from(self.arity);
        let start = usize::try_from(row).expect("row index fits usize") * arity;
        self.cells[start..start + arity]
            .iter()
            .map(|cell| self.view(cell))
    }

    fn view(&self, cell: &Cell) -> CellView<'_> {
        match *cell {
            Cell::Bool(value) => CellView::Bool(value),
            Cell::U64(value) => CellView::U64(value),
            Cell::I64(value) => CellView::I64(value),
            Cell::IntervalU64(interval) => CellView::IntervalU64(interval),
            Cell::IntervalI64(interval) => CellView::IntervalI64(interval),
            Cell::FixedBytes { off, len } => CellView::FixedBytes(&self.bytes[span(off, len)]),
            Cell::Str { off, len } => CellView::Str(&self.strings[span(off, len)]),
        }
    }
}

/// Arena span → slice range (64-bit only, `lib.rs` compile-errors on
/// narrower targets, so `u32 → usize` never truncates).
fn span(off: u32, len: u32) -> std::ops::Range<usize> {
    off as usize..off as usize + len as usize
}

/// THE one dynamic-row parse implementation: feeds cells positionally,
/// judges each against the sealed roster, and [`CollectionBuilder::seal`]s
/// into the proof-carrying [`AcceptedCollection`]. Two feeding surfaces —
/// [`CollectionBuilder::push_value`] for the engine dyn lane and the
/// typed pushes for the bridge — funnel through one positional cell
/// judgment, so the rules cannot fork.
pub struct CollectionBuilder<'s> {
    relation: RelationId,
    fields: &'s [FieldDescriptor],
    /// `fields.len()`, proved to fit `u16` once at construction (every
    /// `FieldId` this builder mints is a valid position).
    arity: u16,
    /// Completed rows so far (a row completes when its last cell lands).
    rows: u64,
    /// Cells of the currently open row: always `< fields.len()` between
    /// pushes; `seal` refuses a nonzero remainder as the arity mismatch
    /// it is.
    fill: u16,
    cells: Vec<Cell>,
    strings: String,
    bytes: Vec<u8>,
}

impl<'s> CollectionBuilder<'s> {
    /// Starts an empty collection for `relation`, judged against its
    /// sealed roster `fields` (borrowed — the roster is immutable per
    /// handle, never re-derived per row).
    ///
    /// # Panics
    ///
    /// On a roster wider than `u16::MAX` fields — schema validation
    /// makes such relations undeclarable.
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

    /// The positional expectation for the next cell. `None` only on a
    /// fieldless roster, where any pushed cell overflows the zero-width
    /// row.
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

    /// One judged cell landed: advance the position; a completed row
    /// counts exactly once.
    fn advance(&mut self) {
        self.fill += 1;
        if usize::from(self.fill) == self.fields.len() {
            self.fill = 0;
            self.rows += 1;
        }
    }

    /// One dynamic row, whole: arity against the roster FIRST (the
    /// retired `parse_dyn_row`'s exact order and error), then each cell
    /// through the one positional judgment.
    ///
    /// # Errors
    ///
    /// `FactShape` as [`AcceptedCollection::from_value_rows`].
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
            // A fieldless relation's row has no cells; the row itself
            // still counts (`submitted` is exact).
            self.rows += 1;
            return Ok(());
        }
        for value in row {
            self.push_value(value)?;
        }
        Ok(())
    }

    /// One cell from a [`Value`] — the engine dyn lane's feed, judged by
    /// the one `Value` ↔ `ValueType` rule (`value_matches`), translated
    /// through the one [`shape_mismatch`] as every dynamic surface is.
    ///
    /// # Errors
    ///
    /// `FactShape` on a type-kind mismatch at this position.
    pub fn push_value(&mut self, value: &Value) -> Result<()> {
        let (field, expected) = self.expected()?;
        if let Err(mismatch) = value_matches(value, expected) {
            return Err(shape_mismatch(self.relation, field, mismatch).into());
        }
        let cell = match value {
            Value::Bool(value) => Cell::Bool(*value),
            Value::U64(value) => Cell::U64(*value),
            Value::I64(value) => Cell::I64(*value),
            Value::String(text) => self.land_str(text),
            Value::FixedBytes(raw) => self.land_bytes(raw),
            Value::IntervalU64(interval) => Cell::IntervalU64(*interval),
            Value::IntervalI64(interval) => Cell::IntervalI64(*interval),
        };
        self.cells.push(cell);
        self.advance();
        Ok(())
    }

    /// One `bool` cell — the bridge's typed feed (each typed push judges
    /// the same positional arm `value_matches` would, without buying an
    /// owned [`Value`] to be judged).
    ///
    /// # Errors
    ///
    /// `FactShape` on a type-kind mismatch at this position.
    pub fn push_bool(&mut self, value: bool) -> Result<()> {
        self.push_scalar(&Value::Bool(value), Cell::Bool(value))
    }

    /// One `u64` cell.
    ///
    /// # Errors
    ///
    /// As [`Self::push_bool`].
    pub fn push_u64(&mut self, value: u64) -> Result<()> {
        self.push_scalar(&Value::U64(value), Cell::U64(value))
    }

    /// One `i64` cell.
    ///
    /// # Errors
    ///
    /// As [`Self::push_bool`].
    pub fn push_i64(&mut self, value: i64) -> Result<()> {
        self.push_scalar(&Value::I64(value), Cell::I64(value))
    }

    /// One interval cell over `u64` — nonempty by the checked
    /// [`Interval`] type; the general-vs-fixed width and ray rules are
    /// `value_matches`' interval-family arms.
    ///
    /// # Errors
    ///
    /// As [`Self::push_bool`].
    pub fn push_interval_u64(&mut self, value: Interval<u64>) -> Result<()> {
        self.push_scalar(&Value::IntervalU64(value), Cell::IntervalU64(value))
    }

    /// One interval cell over `i64`.
    ///
    /// # Errors
    ///
    /// As [`Self::push_bool`].
    pub fn push_interval_i64(&mut self, value: Interval<i64>) -> Result<()> {
        self.push_scalar(&Value::IntervalI64(value), Cell::IntervalI64(value))
    }

    /// A fixed-width typed cell: judged through `value_matches` itself —
    /// these variants are `Copy` payloads, so the judging [`Value`] is a
    /// stack temporary, never a heap purchase.
    fn push_scalar(&mut self, judge: &Value, cell: Cell) -> Result<()> {
        let (field, expected) = self.expected()?;
        if let Err(mismatch) = value_matches(judge, expected) {
            return Err(shape_mismatch(self.relation, field, mismatch).into());
        }
        self.cells.push(cell);
        self.advance();
        Ok(())
    }

    /// One string cell, landed directly in the arena — the one copy
    /// (`proposals/one-representation/30`). UTF-8 is `&str`'s type;
    /// `value_matches`' `String` arm is bare kind equality, stated here
    /// directly so the borrowed cell never buys a `Box<str>` to be
    /// judged.
    ///
    /// # Errors
    ///
    /// As [`Self::push_bool`].
    pub fn push_str(&mut self, value: &str) -> Result<()> {
        let (field, expected) = self.expected()?;
        if !matches!(expected, ValueType::String) {
            return Err(shape_mismatch(self.relation, field, ValueMismatch::Type).into());
        }
        let cell = self.land_str(value);
        self.cells.push(cell);
        self.advance();
        Ok(())
    }

    /// One `bytes<N>` cell, landed in the bytes arena. The width rule is
    /// `value_matches`' `FixedBytes` arm verbatim: the length is the
    /// type, so any other width is a kind mismatch — stated here
    /// directly so the borrowed cell never buys a `Box<[u8]>` to be
    /// judged.
    ///
    /// # Errors
    ///
    /// As [`Self::push_bool`].
    pub fn push_bytes(&mut self, value: &[u8]) -> Result<()> {
        let (field, expected) = self.expected()?;
        if !matches!(expected, ValueType::FixedBytes { len } if value.len() == usize::from(*len)) {
            return Err(shape_mismatch(self.relation, field, ValueMismatch::Type).into());
        }
        let cell = self.land_bytes(value);
        self.cells.push(cell);
        self.advance();
        Ok(())
    }

    fn land_str(&mut self, value: &str) -> Cell {
        let off = arena_span(self.strings.len());
        let len = arena_span(value.len());
        self.strings.push_str(value);
        Cell::Str { off, len }
    }

    fn land_bytes(&mut self, value: &[u8]) -> Cell {
        let off = arena_span(self.bytes.len());
        let len = arena_span(value.len());
        self.bytes.extend_from_slice(value);
        Cell::FixedBytes { off, len }
    }

    /// Seals the proof: complete rows only (`cells == rows · arity`) —
    /// a dangling partial row is the arity mismatch it is. Zero rows
    /// seal lawfully.
    ///
    /// # Errors
    ///
    /// `FactShape` naming the incomplete row's witnessed width.
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
            rows: self.rows,
            cells: self.cells,
            strings: self.strings,
            bytes: self.bytes,
        })
    }
}

/// Arena offsets are u32 spans (R2's fixed-width cell): one collection
/// is bounded far under 4 GiB of variable-width payload by the transport
/// contract (a bridge collection is one call's facts), so overflow is a
/// programmer-invariant violation, not host data.
fn arena_span(len: usize) -> u32 {
    u32::try_from(len).expect("arena span fits u32")
}

/// Intern pending strings of row `row` and fill `refs` — the accepted
/// twin of the point-probe lane's `intern_parsed_row`. `Ok(false)` = a
/// resolve miss: the fact cannot exist.
pub(super) fn intern_accepted_row(
    coll: &AcceptedCollection,
    row: u64,
    refs: &mut Vec<ValueRef>,
    mut resolve_str: impl FnMut(&str) -> Result<Option<InternId>>,
) -> Result<bool> {
    refs.clear();
    for cell in coll.row_cells(row) {
        let value_ref = match cell {
            CellView::Str(text) => {
                let Some(id) = resolve_str(text)? else {
                    return Ok(false);
                };
                ValueRef::String(id)
            }
            CellView::Bool(value) => ValueRef::Bool(value),
            CellView::U64(value) => ValueRef::U64(value),
            CellView::I64(value) => ValueRef::I64(value),
            CellView::IntervalU64(interval) => ValueRef::IntervalU64(interval),
            CellView::IntervalI64(interval) => ValueRef::IntervalI64(interval),
            CellView::FixedBytes(raw) => ValueRef::bytes(raw),
        };
        refs.push(value_ref);
    }
    Ok(true)
}
