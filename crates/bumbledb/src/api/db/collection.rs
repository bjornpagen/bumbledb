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

/// The one [`crate::schema::ValueMismatch`] → [`FactShapeError`] translation,
/// shared by every dynamic write/read surface (`insert_dyn`/`delete_dyn`/
/// `contains_dyn`/`get_dyn`, both transaction kinds).
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

/// One borrowed cell of an accepted row — the retired `ParsedCell`
/// vocabulary re-homed onto the arena spans: `Str`/`FixedBytes` resolve
/// to `&str`/`&[u8]` borrowed from the collection.
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
    /// it was built against, and arity alone re-anchors only the WIDTH:
    /// the [`Self::roster`] echo below is the wall's type half).
    arity: u16,
    /// The roster ECHO: the arity-long value-type row every cell was
    /// judged against, captured at seal. The echo law (the apply-side
    /// second wall, whole): sealed cells are proved against the echo, and
    /// `apply_accepted` proves the echo IS the target relation's own
    /// field value-types — so a collection built against a same-arity,
    /// type-different roster refuses typed at apply instead of encoding
    /// wrong-width fact bytes. O(arity) per collection, zero per-cell
    /// cost ([`ValueType`] is `Copy`).
    roster: Box<[ValueType]>,
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
    /// constructs without touching the roster. A fieldless roster counts
    /// its payload-witnessed empty slices — each row's emptiness IS the
    /// arity judgment a zero-width roster admits — and seals the count
    /// through the one fieldless constructor,
    /// [`CollectionBuilder::seal_nullary`] (the push lane never accrues
    /// nullary rows; N empty slices still submit exactly N).
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

    /// The value-type roster every cell was judged against — the echo the
    /// apply-side wall verifies against the target relation's own fields.
    pub(super) fn roster(&self) -> &[ValueType] {
        &self.roster
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
    /// `FactShape` as [`AcceptedCollection::from_value_rows`]; a
    /// fieldless roster refuses EVERY pushed row (the one-way ruling
    /// below).
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
            // The one-way ruling of fieldless rows (`proposals/
            // one-representation/20`): a fieldless collection IS its row
            // count, and the stated-count constructor
            // [`Self::seal_nullary`] is the ONE spelling — the push lane
            // never accrues nullary rows (a pushed count would be
            // silently replaced by the stated one), so `seal_nullary`'s
            // zero-rows precondition holds by construction. The refusal
            // is the zero-width roster's one push answer
            // ([`Self::expected`]): any push overflows the zero-width
            // row.
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
            Value::String(text) => self.land_str(text)?,
            Value::FixedBytes(raw) => self.land_bytes(raw)?,
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
        let cell = self.land_str(value)?;
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

    /// Arena offsets are u32 spans (R2's fixed-width cell): one
    /// collection's variable-width payload is bounded by the 4 GiB
    /// transport contract (a bridge collection is one call's facts). ETL
    /// input is data, so the bound is a typed refusal, never a panic
    /// (`docs/architecture/70-api.md`'s recorded ruling). UNTESTABLE as
    /// stated: witnessing the refusal takes a >4 GiB arena, which no CI
    /// harness can afford, and the tree holds no precedent for a
    /// test-scoped bound override — the typed arm stands on this comment
    /// and the [`FactShapeError::PayloadBound`] pin in `error.rs`.
    fn arena_span(&self, len: usize) -> Result<u32> {
        u32::try_from(len).map_err(|_| {
            FactShapeError::PayloadBound {
                relation: self.relation,
            }
            .into()
        })
    }

    /// The arity-0 seal (`proposals/one-representation/20`): a fieldless
    /// collection IS its row count — every row is the empty tuple (set
    /// semantics), so the count is the whole representation and the proof
    /// seals O(1) from the stated count, never through O(rows) empty
    /// pushes. On the bridge crossing the count is caller DATA (`rows` is
    /// a raw u64 the addon caller states; the `cells.len() == rows ×
    /// arity` wall is vacuously true at arity 0 for EVERY count — any
    /// count IS shape-lawful, N empty tuples), so a stated 2^63 must
    /// never buy 2^63 pushes from a 16-byte payload; the apply side
    /// collapses the same way (`apply_accepted`'s arity-0 arm: one judged
    /// apply, `submitted = rows` exact, `changed` the one effect).
    ///
    /// # Errors
    ///
    /// `FactShape` (`ArityMismatch`, witnessed 0) when the roster is not
    /// fieldless — a zero-width row against a widthful roster. The ONE
    /// refusal of this seal: on a fieldless roster no push can have
    /// succeeded (every push lane refuses the zero-width roster typed),
    /// so the seal below never fails.
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
        // Genuinely unreachable, kept as belt: rows accrue only through
        // the push lanes, and every push at a fieldless roster refuses
        // typed ([`Self::push_value_row`]'s one-way ruling;
        // [`Self::expected`]'s zero-width answer for the cell pushes) —
        // no public sequence can reach this seal with a pushed count for
        // the stated one to replace.
        debug_assert!(self.rows == 0, "the stated count IS the collection");
        self.rows = rows;
        self.seal()
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
            roster: self.fields.iter().map(|field| field.value_type).collect(),
            rows: self.rows,
            cells: self.cells,
            strings: self.strings,
            bytes: self.bytes,
        })
    }
}

/// Intern pending strings of row `row` and fill `refs` — the one
/// judged-row → [`ValueRef`] translation, collection and point-probe
/// lanes alike. `Ok(false)` = a resolve miss: the fact cannot exist.
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

/// THE single-row judgment of the point crossings (`contains_dyn` /
/// `contains_values` / the write path's `encode_dyn`), which are not
/// collections and keep their one-row form: parses `values` through the
/// one parse implementation ([`CollectionBuilder`], as a one-row
/// [`AcceptedCollection`]) and fills `refs` through the caller's resolve
/// discipline. The whole shape judgment lands before the first string
/// resolves — exactly the collection lane's parse-all-first order, so a
/// type-illegal cell refuses even when an earlier string would already
/// miss. `Ok(false)` = a resolve miss: the fact provably cannot exist.
pub(super) fn intern_value_row(
    rel: RelationId,
    fields: &[FieldDescriptor],
    values: &[Value],
    refs: &mut Vec<ValueRef>,
    resolve_str: impl FnMut(&str) -> Result<Option<InternId>>,
) -> Result<bool> {
    let row = AcceptedCollection::from_value_rows(rel, fields, std::iter::once(values))?;
    intern_accepted_row(&row, 0, refs, resolve_str)
}
