//! One predicate walk over an operand-provider capability.
//! Callers construct an [`Operands`] provider — image columns, fact
//! bytes, or binding slots — and call [`holds`]. Measure-of-ray is
//! `None`; every other outcome is `Some`. There is no second walk.
//! Static dispatch, never `dyn`.
use crate::image::{ColumnView, ColumnWidth, RelationImage};
use crate::ir::WordCmp;
use crate::obs;
use crate::schema::Relation;
use crate::storage::catalog::CatalogRead;
use crate::storage::dict;
use bumbledb_theory::schema::{FieldId, IntervalElement, ValueType};

use super::{Const, FilterPredicate, IntervalConst, MaskConst, SetConst, ViewWordSource};

/// Address of one operand in a provider's space. Image and fact
/// providers interpret [`Self::at`] as a [`FieldId`]; binding and batch
/// providers interpret it as a variable id (residuals, at normalize) or
/// a binding slot (ray-probe verdicts). [`Self::offset`] selects Start/End
/// of an interval variable for word residuals; [`Self::width`] is 0 when
/// the provider decides the load shape (image/fact columns) and 1/2/N
/// when a slot provider must emit Word/Pair/Block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperandAddr {
    at: u16,
    offset: u8,
    width: u8,
}

impl From<FieldId> for OperandAddr {
    fn from(id: FieldId) -> Self {
        Self {
            at: id.0,
            offset: 0,
            width: 0,
        }
    }
}

impl From<crate::ir::VarId> for OperandAddr {
    fn from(id: crate::ir::VarId) -> Self {
        Self {
            at: id.0,
            offset: 0,
            width: 0,
        }
    }
}

impl PartialEq<FieldId> for OperandAddr {
    fn eq(&self, other: &FieldId) -> bool {
        self.at == other.0 && self.offset == 0
    }
}

impl PartialEq<OperandAddr> for FieldId {
    fn eq(&self, other: &OperandAddr) -> bool {
        other == self
    }
}

impl OperandAddr {
    #[must_use]
    pub const fn field(self) -> FieldId {
        FieldId(self.at)
    }

    #[must_use]
    pub const fn var(self) -> crate::ir::VarId {
        crate::ir::VarId(self.at)
    }

    #[must_use]
    pub const fn offset(self) -> usize {
        self.offset as usize
    }

    #[must_use]
    pub const fn width(self) -> u8 {
        self.width
    }

    #[must_use]
    pub fn slot(self) -> usize {
        usize::from(self.at) + usize::from(self.offset)
    }

    #[must_use]
    pub fn from_slot(slot: usize) -> Self {
        Self {
            at: u16::try_from(slot).expect("slot fits u16"),
            offset: 0,
            width: 1,
        }
    }

    #[must_use]
    pub fn from_span(slot: usize, width: usize) -> Self {
        Self {
            at: u16::try_from(slot).expect("slot fits u16"),
            offset: 0,
            width: u8::try_from(width).expect("span width fits u8"),
        }
    }

    #[must_use]
    pub fn var_word(var: crate::ir::VarId, offset: usize) -> Self {
        Self {
            at: var.0,
            offset: u8::try_from(offset).expect("Start/End offset is 0 or 1"),
            width: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Loaded {
    Word(u64),
    Byte(u8),
    Pair(u64, u64),
    Block { words: [u64; 8], count: u8 },
}

pub(crate) trait Operands {
    type Error;

    fn word(&self, at: OperandAddr) -> Result<u64, Self::Error>;
    fn pair(&self, at: OperandAddr) -> Result<(u64, u64), Self::Error>;
    #[allow(dead_code, reason = "bytes<N> currently loads via `loaded`")]
    fn block(&self, at: OperandAddr) -> Result<([u64; 8], u8), Self::Error>;

    fn loaded(&self, at: OperandAddr) -> Result<Loaded, Self::Error>;

    fn intern(&self, bytes: &[u8]) -> Result<u64, Self::Error> {
        let _ = bytes;
        unreachable!("validated: PendingIntern is latched before this provider")
    }
}

pub(crate) struct ImageRow<'a> {
    pub image: &'a RelationImage,
    pub position: usize,
}

impl Operands for ImageRow<'_> {
    type Error = std::convert::Infallible;

    fn word(&self, at: OperandAddr) -> Result<u64, Self::Error> {
        Ok(match self.loaded(at)? {
            Loaded::Word(w) => w,
            Loaded::Byte(b) => u64::from(b),
            Loaded::Pair(..) | Loaded::Block { .. } => {
                unreachable!("validated: word operands are scalar")
            }
        })
    }

    fn pair(&self, at: OperandAddr) -> Result<(u64, u64), Self::Error> {
        Ok(match self.loaded(at)? {
            Loaded::Pair(s, e) => (s, e),
            Loaded::Word(_) | Loaded::Byte(_) | Loaded::Block { .. } => {
                unreachable!("validated: interval predicates read interval fields")
            }
        })
    }

    fn block(&self, at: OperandAddr) -> Result<([u64; 8], u8), Self::Error> {
        Ok(match self.loaded(at)? {
            Loaded::Block { words, count } => (words, count),
            Loaded::Word(_) | Loaded::Byte(_) | Loaded::Pair(..) => {
                unreachable!("validated: block operands are bytes<N>")
            }
        })
    }

    fn loaded(&self, at: OperandAddr) -> Result<Loaded, Self::Error> {
        Ok(image_loaded(self.image, at.field(), self.position))
    }
}

pub(crate) struct SlotOps<'a, F: Fn(usize) -> u64 + ?Sized> {
    pub word: &'a F,
}

impl<F: Fn(usize) -> u64 + ?Sized> Operands for SlotOps<'_, F> {
    type Error = std::convert::Infallible;

    fn word(&self, at: OperandAddr) -> Result<u64, Self::Error> {
        Ok((self.word)(at.slot()))
    }

    fn pair(&self, at: OperandAddr) -> Result<(u64, u64), Self::Error> {
        let s = at.slot();
        Ok(((self.word)(s), (self.word)(s + 1)))
    }

    fn block(&self, at: OperandAddr) -> Result<([u64; 8], u8), Self::Error> {
        Ok(slot_block(self.word, at))
    }

    fn loaded(&self, at: OperandAddr) -> Result<Loaded, Self::Error> {
        Ok(match at.width() {
            0 | 1 => Loaded::Word((self.word)(at.slot())),
            2 => {
                let s = at.slot();
                Loaded::Pair((self.word)(s), (self.word)(s + 1))
            }
            n => {
                let (words, count) = slot_block(self.word, at);
                debug_assert_eq!(count, n);
                Loaded::Block { words, count }
            }
        })
    }

    fn intern(&self, _bytes: &[u8]) -> Result<u64, Self::Error> {
        Ok(crate::storage::dict::SENTINEL_ID)
    }
}

fn slot_block(word: &(impl Fn(usize) -> u64 + ?Sized), at: OperandAddr) -> ([u64; 8], u8) {
    let count = at.width();
    let s = at.slot();
    let mut words = [0u64; 8];
    for (i, slot) in words[..usize::from(count)].iter_mut().enumerate() {
        *slot = word(s + i);
    }
    (words, count)
}

pub(crate) fn resolve<'a>(value: &'a Const, params: &'a [Const]) -> &'a Const {
    match value {
        Const::Param(param) | Const::ParamSet(param) => &params[usize::from(param.0)],
        other => other,
    }
}

pub(crate) fn const_interval(value: &IntervalConst, params: &[Const]) -> (u64, u64) {
    match value {
        IntervalConst::Interval { start, end } => (*start, *end),
        IntervalConst::Param(param) => match &params[usize::from(param.0)] {
            Const::Interval { start, end } => (*start, *end),
            _ => unreachable!("param slice: interval param resolves to an interval"),
        },
    }
}

pub(crate) const fn point_in(start: u64, end: u64, point: u64) -> bool {
    start <= point && point < end
}

pub(crate) fn mask_of(mask: MaskConst, _params: &[Const]) -> bumbledb_theory::allen::AllenMask {
    mask
}

fn resolve_word(value: &ViewWordSource, params: &[Const]) -> u64 {
    match value {
        ViewWordSource::Word(word) => *word,
        ViewWordSource::Param(param) => match &params[usize::from(param.0)] {
            Const::Word(word) => *word,
            _ => unreachable!("param slice: word param resolves to a word"),
        },
    }
}

fn word_set<'a>(set: &'a SetConst, params: &'a [Const]) -> &'a [u64] {
    match set {
        SetConst::WordSet(words) => words,
        SetConst::ParamSet(param) => match &params[usize::from(param.0)] {
            Const::WordSet(words) => words,
            _ => unreachable!("param slice: set param resolves to a word set"),
        },
    }
}

fn span_in_set(words: &[u64], count: u8, set: &[u64]) -> bool {
    let width = usize::from(count);
    debug_assert_eq!(set.len() % width, 0, "flat element-major rows");
    let value = &words[..width];
    let mut lo = 0usize;
    let mut hi = set.len() / width;
    while lo < hi {
        let mid = usize::midpoint(lo, hi);
        match set[mid * width..(mid + 1) * width].cmp(value) {
            std::cmp::Ordering::Less => lo = mid + 1,
            std::cmp::Ordering::Greater => hi = mid,
            std::cmp::Ordering::Equal => return true,
        }
    }
    false
}

#[must_use]
pub(crate) fn is_prepare_resolvable(filter: &FilterPredicate) -> bool {
    let ordinary = |op: WordCmp| {
        matches!(
            op,
            WordCmp::Eq | WordCmp::Ne | WordCmp::Lt | WordCmp::Le | WordCmp::Gt | WordCmp::Ge
        )
    };
    match filter {
        FilterPredicate::AnyPointIn { .. } => false,
        FilterPredicate::Compare { op, value, .. } => match (op, value) {
            (WordCmp::Eq, Const::WordSet(_) | Const::Words(_) | Const::Interval { .. })
            | (WordCmp::Ne, Const::Words(_) | Const::Interval { .. })
            | (WordCmp::Eq | WordCmp::Ne, Const::Byte(_)) => true,
            (op, Const::Word(_)) if ordinary(*op) => true,
            _ => false,
        },
        FilterPredicate::PointIn { point, .. } => matches!(point, ViewWordSource::Word(_)),
        FilterPredicate::FieldAllen { other, .. } => {
            matches!(other, IntervalConst::Interval { .. })
        }
        FilterPredicate::FieldWithin { outer, .. } => {
            matches!(outer, IntervalConst::Interval { .. })
        }
        FilterPredicate::FieldsCompare { op, .. } => ordinary(*op),
        FilterPredicate::FieldsAllen { .. } | FilterPredicate::FieldsPointIn { .. } => true,
    }
}

pub(crate) fn holds<O: Operands>(
    predicate: &FilterPredicate,
    ops: &O,
    params: &[Const],
) -> std::result::Result<Option<bool>, O::Error> {
    Ok(match predicate {
        FilterPredicate::Compare { field, op, value } => Some(compare_loaded(
            ops,
            ops.loaded(*field)?,
            resolve(value, params),
            *op,
        )?),
        FilterPredicate::FieldsCompare { left, right, op } => {
            Some(fields_compare(ops.loaded(*left)?, ops.loaded(*right)?, *op))
        }
        FilterPredicate::PointIn { field, point } => {
            let (start, end) = ops.pair(*field)?;
            Some(point_in(start, end, resolve_word(point, params)))
        }
        FilterPredicate::AnyPointIn { field, set } => {
            let (start, end) = ops.pair(*field)?;
            let points = word_set(set, params);
            let idx = points.partition_point(|&p| p < start);
            Some(idx < points.len() && points[idx] < end)
        }
        FilterPredicate::FieldsAllen { left, right, mask } => {
            let (l_start, l_end) = ops.pair(*left)?;
            let (r_start, r_end) = ops.pair(*right)?;
            Some(
                mask_of(*mask, params).contains(crate::allen::classify_bounds(
                    &l_start, &l_end, &r_start, &r_end,
                )),
            )
        }
        FilterPredicate::FieldAllen { field, other, mask } => {
            let (f_start, f_end) = ops.pair(*field)?;
            let (start, end) = const_interval(other, params);
            Some(
                mask_of(*mask, params).contains(crate::allen::classify_bounds(
                    &f_start, &f_end, &start, &end,
                )),
            )
        }
        FilterPredicate::FieldsPointIn { interval, point } => {
            let (start, end) = ops.pair(*interval)?;
            Some(point_in(start, end, ops.word(*point)?))
        }
        FilterPredicate::FieldWithin { field, outer } => {
            let (start, end) = const_interval(outer, params);
            Some(match ops.loaded(*field)? {
                Loaded::Word(word) => point_in(start, end, word),
                Loaded::Byte(_) | Loaded::Pair(..) | Loaded::Block { .. } => {
                    unreachable!("validated: within-comparands are scalar words")
                }
            })
        }
    })
}

fn compare_loaded<O: Operands>(
    ops: &O,
    got: Loaded,
    value: &Const,
    op: WordCmp,
) -> Result<bool, O::Error> {
    Ok(match (got, value) {
        (Loaded::Word(word), Const::Word(c)) => op.compare(&word, c),
        (Loaded::Byte(byte), Const::Byte(c)) => op.compare(&byte, c),
        (Loaded::Pair(s, e), Const::Interval { start, end }) => match op {
            WordCmp::Eq => s == *start && e == *end,
            _ => unreachable!("validated: interval constants compare under Eq only"),
        },
        (Loaded::Block { words, count }, Const::Words(c)) => match op {
            WordCmp::Eq => words[..usize::from(count)] == **c,
            WordCmp::Ne => words[..usize::from(count)] != **c,
            _ => unreachable!("validated: bytes<N> compares under Eq/Ne only"),
        },
        (Loaded::Word(word), Const::WordSet(set)) => set.binary_search(&word).is_ok(),
        (Loaded::Byte(byte), Const::WordSet(set)) => set.binary_search(&u64::from(byte)).is_ok(),
        (Loaded::Block { words, count }, Const::WordSet(set)) => span_in_set(&words, count, set),

        (Loaded::Word(word), Const::Byte(c)) => op.compare(&word, &u64::from(*c)),
        (Loaded::Word(word), Const::PendingIntern { bytes }) => {
            op.compare(&word, &ops.intern(bytes)?)
        }
        _ => unreachable!("validated, resolved filter constant"),
    })
}

fn fields_compare(left: Loaded, right: Loaded, op: WordCmp) -> bool {
    match (left, right) {
        (Loaded::Word(a), Loaded::Word(b)) => op.compare(&a, &b),
        (Loaded::Byte(a), Loaded::Byte(b)) => op.compare(&a, &b),
        (Loaded::Pair(a_s, a_e), Loaded::Pair(b_s, b_e)) => match op {
            WordCmp::Eq => a_s == b_s && a_e == b_e,
            WordCmp::Ne => a_s != b_s || a_e != b_e,
            _ => unreachable!("validated: no order comparison over intervals"),
        },
        (Loaded::Block { words: a, count }, Loaded::Block { words: b, .. }) => match op {
            WordCmp::Eq => a[..usize::from(count)] == b[..usize::from(count)],
            WordCmp::Ne => a[..usize::from(count)] != b[..usize::from(count)],
            _ => unreachable!("validated: bytes<N> compares under Eq/Ne only"),
        },
        _ => unreachable!("same-fact comparison joins same-typed fields"),
    }
}

fn image_loaded(image: &RelationImage, field: FieldId, position: usize) -> Loaded {
    let span = image.span(field);
    match span.width {
        ColumnWidth::WordPair => {
            let (start, end) = interval_at(image, field, position);
            Loaded::Pair(start, end)
        }
        ColumnWidth::Words { count } => {
            let first = usize::from(span.first_column);
            let mut words = [0u64; 8];
            for (i, slot) in words[..usize::from(count)].iter_mut().enumerate() {
                let ColumnView::Words(column) = image.column(first + i) else {
                    unreachable!("a Words span covers word columns")
                };
                *slot = column[position];
            }
            Loaded::Block {
                words,
                count: u8::try_from(count).expect("at most 8 words"),
            }
        }
        ColumnWidth::Word | ColumnWidth::Byte => {
            match image.column(usize::from(span.first_column)) {
                ColumnView::Words(words) => Loaded::Word(words[position]),
                ColumnView::Bytes(bytes) => Loaded::Byte(bytes[position]),
            }
        }
    }
}

fn interval_at(image: &RelationImage, field: FieldId, position: usize) -> (u64, u64) {
    let span = image.span(field);
    assert_eq!(
        span.width,
        ColumnWidth::WordPair,
        "validated: interval predicates read interval fields"
    );
    let first = usize::from(span.first_column);
    match (image.column(first), image.column(first + 1)) {
        (ColumnView::Words(starts), ColumnView::Words(ends)) => (starts[position], ends[position]),
        _ => unreachable!("an interval span covers two word columns"),
    }
}

#[must_use]
pub(crate) fn row_holds(
    image: &RelationImage,
    predicates: &[FilterPredicate],
    params: &[Const],
    position: usize,
) -> bool {
    let ops = ImageRow { image, position };
    predicates.iter().all(|predicate| {
        holds(predicate, &ops, params)
            .unwrap_or_else(|e| match e {})
            .unwrap_or(false)
    })
}

fn interval_columns(image: &RelationImage, field: OperandAddr) -> (&[u64], &[u64]) {
    let span = image.span(field.field());
    debug_assert_eq!(span.width, ColumnWidth::WordPair);
    let first = usize::from(span.first_column);
    match (image.column(first), image.column(first + 1)) {
        (ColumnView::Words(starts), ColumnView::Words(ends)) => (starts, ends),
        _ => unreachable!("an interval span covers two word columns"),
    }
}

/// Attempts the kernel fast path for one predicate. Returns whether the
/// scan ran; `false` falls back to the scalar [`row_holds`] loop.
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)]
pub(crate) fn kernel_scan(
    image: &RelationImage,
    predicate: &FilterPredicate,
    params: &[Const],
    out: &mut Vec<u32>,
) -> bool {
    match predicate {
        FilterPredicate::Compare { .. } => {}
        FilterPredicate::PointIn { field, point } => {
            let (starts, ends) = interval_columns(image, *field);
            crate::exec::kernel::filter_point_in_u64(
                starts,
                ends,
                resolve_word(point, params),
                out,
            );
            return true;
        }
        FilterPredicate::AnyPointIn { field, set } => {
            let (starts, ends) = interval_columns(image, *field);
            crate::exec::kernel::filter_any_point_in_u64(starts, ends, word_set(set, params), out);
            return true;
        }
        FilterPredicate::FieldWithin { field, outer } => {
            let (start, end) = const_interval(outer, params);
            let span = image.span(field.field());
            debug_assert_eq!(span.width, ColumnWidth::Word);
            let ColumnView::Words(words) = image.column(usize::from(span.first_column)) else {
                unreachable!("a word span covers a word column")
            };
            crate::exec::kernel::filter_range_u64(words, start, end - 1, out);
            return true;
        }
        FilterPredicate::FieldsAllen { left, right, mask } => {
            let (l_starts, l_ends) = interval_columns(image, *left);
            let (r_starts, r_ends) = interval_columns(image, *right);
            crate::exec::kernel::allen_filter_columns(
                l_starts,
                l_ends,
                r_starts,
                r_ends,
                mask_of(*mask, params),
                out,
            );
            return true;
        }
        FilterPredicate::FieldAllen { field, other, mask } => {
            let (starts, ends) = interval_columns(image, *field);
            let (start, end) = const_interval(other, params);
            crate::exec::kernel::allen_filter_columns_const(
                starts,
                ends,
                start,
                end,
                mask_of(*mask, params),
                out,
            );
            return true;
        }
        FilterPredicate::FieldsCompare { .. } | FilterPredicate::FieldsPointIn { .. } => {
            return false;
        }
    }
    let FilterPredicate::Compare { field, op, value } = predicate else {
        unreachable!("every other kind returned above")
    };
    let span = image.span(field.field());
    let value = resolve(value, params);
    if span.width == ColumnWidth::WordPair {
        return false;
    }
    if let ColumnWidth::Words { count } = span.width {
        let (Const::Words(words), WordCmp::Eq) = (value, op) else {
            return false;
        };
        debug_assert_eq!(words.len(), usize::from(count), "validated width");
        let first = usize::from(span.first_column);
        let ColumnView::Words(column0) = image.column(first) else {
            unreachable!("a Words span covers word columns")
        };
        crate::exec::kernel::filter_eq_u64(column0, words[0], out);
        for (i, expected) in words.iter().enumerate().skip(1) {
            let ColumnView::Words(column) = image.column(first + i) else {
                unreachable!("a Words span covers word columns")
            };
            let mut cursor = 0usize;
            for read in 0..out.len() {
                let position = out[read] as usize;
                out[cursor] = out[read];
                cursor += usize::from(column[position] == *expected);
            }
            out.truncate(cursor);
        }
        return true;
    }
    match (image.column(usize::from(span.first_column)), value) {
        (ColumnView::Words(words), Const::Word(c)) => {
            let (lo, hi) = match op {
                WordCmp::Eq => {
                    crate::exec::kernel::filter_eq_u64(words, *c, out);
                    return true;
                }
                WordCmp::Lt => {
                    let Some(hi) = c.checked_sub(1) else {
                        out.clear();
                        return true;
                    };
                    (0, hi)
                }
                WordCmp::Le => (0, *c),
                WordCmp::Gt => {
                    let Some(lo) = c.checked_add(1) else {
                        out.clear();
                        return true;
                    };
                    (lo, u64::MAX)
                }
                WordCmp::Ge => (*c, u64::MAX),
                WordCmp::Ne => return false,
            };
            crate::exec::kernel::filter_range_u64(words, lo, hi, out);
            true
        }
        (ColumnView::Bytes(bytes), Const::Byte(c)) if *op == WordCmp::Eq => {
            crate::exec::kernel::filter_eq_u8(bytes, *c, out);
            true
        }
        _ => false,
    }
}

/// Substitutes one filter's symbolic constants into its resolved slot,
/// in place. `Ok(false)` = the positive-occurrence `Eq` short-circuit.
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)]
pub(crate) fn resolve_filter_into<C: CatalogRead>(
    catalog: &C,
    template: &mut FilterPredicate,
    params: &[Const],
    missed: &[bool],
    negated: bool,
    dst: &mut FilterPredicate,
    latched: &mut u32,
) -> crate::error::Result<bool> {
    match template {
        FilterPredicate::Compare { field, op, value } => {
            if let Const::PendingIntern { bytes } = value {
                match catalog.dict_lookup(bytes)? {
                    Some(id) => {
                        let word = Const::Word(id.raw());
                        *value = word;
                        *latched += 1;
                        obs::event(obs::names::LITERAL_LATCH, obs::TraceArgs::Count(id.raw()));
                    }
                    None if *op == WordCmp::Eq && !negated => return Ok(false),
                    None => {
                        write_compare(dst, *field, *op, Some(Const::Word(dict::SENTINEL_ID)));
                        return Ok(true);
                    }
                }
            }
            let resolved = match value {
                Const::Word(_) | Const::Byte(_) | Const::Interval { .. } => value.clone(),
                Const::Words(words) => {
                    write_compare(dst, *field, *op, None);
                    write_words_value(dst, words);
                    return Ok(true);
                }
                Const::Param(param) => {
                    if missed[usize::from(param.0)] && *op == WordCmp::Eq && !negated {
                        return Ok(false);
                    }
                    match &params[usize::from(param.0)] {
                        Const::Words(words) => {
                            write_compare(dst, *field, *op, None);
                            write_words_value(dst, words);
                            return Ok(true);
                        }
                        other => other.clone(),
                    }
                }
                Const::ParamSet(param) => {
                    debug_assert_eq!(*op, WordCmp::Eq, "validated: sets only under Eq");
                    if missed[usize::from(param.0)] && !negated {
                        return Ok(false);
                    }
                    let Const::WordSet(words) = &params[usize::from(param.0)] else {
                        unreachable!("validated: a set param resolves to a word set")
                    };
                    write_compare(dst, *field, *op, None);
                    write_word_set_value(dst, words);
                    return Ok(true);
                }
                Const::WordSet(words) => {
                    debug_assert_eq!(*op, WordCmp::Eq, "plan-constant sets ride Eq");
                    write_compare(dst, *field, *op, None);
                    write_word_set_value(dst, words);
                    return Ok(true);
                }
                Const::PendingIntern { .. } => unreachable!("latched or short-circuited above"),
            };
            write_compare(dst, *field, *op, Some(resolved));
        }
        FilterPredicate::PointIn { field, point } => {
            let word = match point {
                ViewWordSource::Word(word) => *word,
                ViewWordSource::Param(param) => match &params[usize::from(param.0)] {
                    Const::Word(word) => *word,
                    _ => unreachable!("param slice: a point param resolves to a word"),
                },
            };
            *dst = FilterPredicate::PointIn {
                field: *field,
                point: ViewWordSource::Word(word),
            };
        }
        FilterPredicate::AnyPointIn { field, set } => {
            let SetConst::ParamSet(param) = set else {
                unreachable!("templates carry ParamSet markers")
            };
            let Const::WordSet(words) = &params[usize::from(param.0)] else {
                unreachable!("param slice: a set param resolves to a word set")
            };
            if let FilterPredicate::AnyPointIn {
                field: dst_field,
                set: SetConst::WordSet(dst_words),
            } = dst
            {
                *dst_field = *field;
                dst_words.clear();
                dst_words.extend_from_slice(words);
            } else {
                *dst = FilterPredicate::AnyPointIn {
                    field: *field,
                    set: SetConst::WordSet(words.clone()),
                };
            }
        }
        FilterPredicate::FieldWithin { field, outer } => {
            let resolved = match outer {
                IntervalConst::Interval { .. } => outer.clone(),
                IntervalConst::Param(param) => match &params[usize::from(param.0)] {
                    Const::Interval { start, end } => IntervalConst::Interval {
                        start: *start,
                        end: *end,
                    },
                    _ => unreachable!("param slice: the outer side is an interval"),
                },
            };
            *dst = FilterPredicate::FieldWithin {
                field: *field,
                outer: resolved,
            };
        }
        FilterPredicate::FieldsAllen { left, right, mask } => {
            *dst = FilterPredicate::FieldsAllen {
                left: *left,
                right: *right,
                mask: mask_of(*mask, params),
            };
        }
        FilterPredicate::FieldAllen { field, other, mask } => {
            let resolved = match other {
                IntervalConst::Interval { .. } => other.clone(),
                IntervalConst::Param(param) => match &params[usize::from(param.0)] {
                    Const::Interval { start, end } => IntervalConst::Interval {
                        start: *start,
                        end: *end,
                    },
                    _ => unreachable!("param slice: the Allen constant side is an interval"),
                },
            };
            *dst = FilterPredicate::FieldAllen {
                field: *field,
                other: resolved,
                mask: mask_of(*mask, params),
            };
        }
        FilterPredicate::FieldsCompare { .. } | FilterPredicate::FieldsPointIn { .. } => {
            dst.clone_from(template);
        }
    }
    Ok(true)
}

fn write_compare(dst: &mut FilterPredicate, field: OperandAddr, op: WordCmp, value: Option<Const>) {
    if let FilterPredicate::Compare {
        field: dst_field,
        op: dst_op,
        value: dst_value,
    } = dst
    {
        *dst_field = field;
        *dst_op = op;
        if let Some(value) = value {
            *dst_value = value;
        }
        return;
    }
    *dst = FilterPredicate::Compare {
        field,
        op,
        value: value.unwrap_or(Const::WordSet(Vec::new())),
    };
}

fn write_word_set_value(dst: &mut FilterPredicate, words: &[u64]) {
    let FilterPredicate::Compare { value, .. } = dst else {
        unreachable!("write_compare just shaped the slot")
    };
    if let Const::WordSet(dst_words) = value {
        dst_words.clear();
        dst_words.extend_from_slice(words);
    } else {
        *value = Const::WordSet(words.to_vec());
    }
}

fn write_words_value(dst: &mut FilterPredicate, words: &[u64]) {
    let FilterPredicate::Compare { value, .. } = dst else {
        unreachable!("write_compare just shaped the slot")
    };
    if let Const::Words(dst_words) = value
        && dst_words.len() == words.len()
    {
        dst_words.copy_from_slice(words);
    } else {
        *value = Const::Words(words.into());
    }
}

/// One prepare-resolved filter's picture (unresolvable shapes never
/// reach a folded occurrence's list).
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)]
pub(crate) fn render_filter(out: &mut String, relation: &Relation, filter: &FilterPredicate) {
    use crate::ir::normalize::{decoded_interval, decoded_scalar, render_const};
    use crate::ir::render::{literal, mask_names};
    let name = |field: &OperandAddr| relation.field(field.field()).name.as_ref();
    match filter {
        FilterPredicate::Compare { field, op, value } => {
            out.push_str(name(field));
            out.push_str(if matches!(value, Const::WordSet(_)) {
                " ∈ "
            } else {
                op_symbol(*op)
            });
            match value {
                Const::Word(word)
                    if *field == FieldId(0) && relation.body().closed_rows().is_some() =>
                {
                    push_handle(out, relation, *word);
                }
                Const::WordSet(words)
                    if *field == FieldId(0) && relation.body().closed_rows().is_some() =>
                {
                    out.push('{');
                    for (index, word) in words.iter().enumerate() {
                        if index > 0 {
                            out.push_str(", ");
                        }
                        push_handle(out, relation, *word);
                    }
                    out.push('}');
                }
                _ => render_const(out, &relation.field(field.field()).value_type, value),
            }
        }
        FilterPredicate::FieldsCompare { left, right, op } => {
            out.push_str(name(left));
            out.push_str(op_symbol(*op));
            out.push_str(name(right));
        }
        FilterPredicate::PointIn { field, point } => {
            let ViewWordSource::Word(point) = point else {
                render_unparsed_filter(out, filter);
                return;
            };
            literal(
                out,
                &decoded_scalar(
                    &element_type(&relation.field(field.field()).value_type),
                    *point,
                ),
            );
            out.push_str(" in ");
            out.push_str(name(field));
        }
        FilterPredicate::FieldsPointIn { interval, point } => {
            out.push_str(name(point));
            out.push_str(" in ");
            out.push_str(name(interval));
        }
        FilterPredicate::FieldWithin { field, outer } => {
            let IntervalConst::Interval { start, end } = outer else {
                render_unparsed_filter(out, filter);
                return;
            };
            out.push_str(name(field));
            out.push_str(" in ");
            let outer_type = ValueType::Interval {
                element: match relation.field(field.field()).value_type {
                    ValueType::I64 => IntervalElement::I64,
                    _ => IntervalElement::U64,
                },
            };
            literal(out, &decoded_interval(&outer_type, (*start, *end)));
        }
        FilterPredicate::FieldsAllen { left, right, mask } => {
            out.push_str("Allen(");
            out.push_str(name(left));
            out.push_str(", ");
            mask_names(out, *mask);
            out.push_str(", ");
            out.push_str(name(right));
            out.push(')');
        }
        FilterPredicate::FieldAllen { field, other, mask } => {
            let (mask, IntervalConst::Interval { start, end }) = (mask, other) else {
                render_unparsed_filter(out, filter);
                return;
            };
            out.push_str("Allen(");
            out.push_str(name(field));
            out.push_str(", ");
            mask_names(out, *mask);
            out.push_str(", ");
            literal(
                out,
                &decoded_interval(&relation.field(field.field()).value_type, (*start, *end)),
            );
            out.push(')');
        }
        FilterPredicate::AnyPointIn { .. } => {
            render_unparsed_filter(out, filter);
        }
    }
}

fn render_unparsed_filter(out: &mut String, filter: &FilterPredicate) {
    use std::fmt::Write as _;
    let _ = write!(out, "{filter:?}");
}

pub(crate) fn push_handle(out: &mut String, relation: &Relation, id: u64) {
    let row = relation
        .body()
        .closed_rows()
        .and_then(|rows| usize::try_from(id).ok().and_then(|index| rows.get(index)));
    if let Some(row) = row {
        out.push_str(&row.handle);
    } else {
        use std::fmt::Write as _;
        let _ = write!(out, "{}({id}?)", relation.name());
    }
}

fn element_type(value_type: &ValueType) -> ValueType {
    match value_type {
        ValueType::Interval {
            element: IntervalElement::I64,
        }
        | ValueType::FixedInterval {
            element: IntervalElement::I64,
            ..
        } => ValueType::I64,
        _ => ValueType::U64,
    }
}

fn op_symbol(op: WordCmp) -> &'static str {
    match op {
        WordCmp::Eq => " == ",
        WordCmp::Ne => " != ",
        WordCmp::Lt => " < ",
        WordCmp::Le => " <= ",
        WordCmp::Gt => " > ",
        WordCmp::Ge => " >= ",
    }
}

#[cfg(test)]
mod gate {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::{Path, PathBuf};

    const VARIANTS: &[&str] = &[
        "Compare",
        "FieldsCompare",
        "PointIn",
        "AnyPointIn",
        "FieldsAllen",
        "FieldAllen",
        "FieldsPointIn",
        "FieldWithin",
    ];

    fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).expect("src") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                if path.file_name().and_then(|s| s.to_str()) == Some("tests") {
                    continue;
                }
                rust_files(&path, out);
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs")
                && path.file_name().and_then(|s| s.to_str()) != Some("tests.rs")
            {
                out.push(path);
            }
        }
    }

    fn match_bodies(src: &str) -> Vec<&str> {
        let mut bodies = Vec::new();
        let mut i = 0;
        while let Some(rel) = src[i..].find("match") {
            let at = i + rel;
            let before = at == 0 || {
                let b = src.as_bytes()[at - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };
            let after = at + 5;
            let after_ok = after >= src.len() || {
                let a = src.as_bytes()[after];
                !a.is_ascii_alphanumeric() && a != b'_'
            };
            if before
                && after_ok
                && let Some(brace) = src[at..].find('{')
            {
                let start = at + brace;
                if let Some(end) = matching_brace(&src[start..]) {
                    bodies.push(&src[start + 1..start + end]);
                    i = start + end + 1;
                    continue;
                }
            }
            i = at + 5;
        }
        bodies
    }

    fn matching_brace(from: &str) -> Option<usize> {
        let mut depth = 0;
        let mut i = 0;
        let bytes = from.as_bytes();
        while i < bytes.len() {
            match bytes[i] {
                b'"' => {
                    i += 1;
                    while i < bytes.len() {
                        if bytes[i] == b'\\' {
                            i += 2;
                        } else if bytes[i] == b'"' {
                            i += 1;
                            break;
                        } else {
                            i += 1;
                        }
                    }
                }
                b'{' => {
                    depth += 1;
                    i += 1;
                }
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                    i += 1;
                }
                _ => i += 1,
            }
        }
        None
    }

    fn pattern_variants(body: &str) -> BTreeSet<String> {
        let mut found = BTreeSet::new();
        let mut depth = 0i32;
        let mut awaiting_arrow = true;
        let bytes = body.as_bytes();
        let mut i = 0;
        while i < body.len() {
            if !body.is_char_boundary(i) {
                i += 1;
                continue;
            }
            let rest = &body[i..];
            if rest.starts_with("FilterPredicate::") && depth == 0 && awaiting_arrow {
                let after = &rest["FilterPredicate::".len()..];
                if let Some(name) = VARIANTS.iter().find(|v| {
                    after.starts_with(*v)
                        && after
                            .as_bytes()
                            .get(v.len())
                            .is_none_or(|c| !c.is_ascii_alphanumeric() && *c != b'_')
                }) {
                    found.insert((*name).to_string());
                }
                i += "FilterPredicate::".len();
                continue;
            }
            match bytes[i] {
                b'{' => {
                    depth += 1;
                    i += 1;
                }
                b'}' => {
                    depth -= 1;
                    if depth == 0 && !awaiting_arrow {
                        awaiting_arrow = true;
                    }
                    i += 1;
                }
                b'=' if depth == 0 && rest.starts_with("=>") => {
                    awaiting_arrow = false;
                    i += 2;
                }
                b',' if depth == 0 && !awaiting_arrow => {
                    awaiting_arrow = true;
                    i += 1;
                }
                _ => i += 1,
            }
        }
        found
    }

    #[test]
    fn exhaustive_filter_predicate_matches_live_in_two_modules() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        rust_files(&src, &mut files);
        let mut exhaustive = Vec::new();
        for path in &files {
            let text = fs::read_to_string(path).expect("read");
            let mut names = BTreeSet::new();
            for body in match_bodies(&text) {
                names.extend(pattern_variants(body));
            }
            if VARIANTS.iter().all(|v| names.contains(*v)) {
                let rel = path
                    .strip_prefix(&src)
                    .expect("under src")
                    .to_string_lossy()
                    .replace('\\', "/");
                exhaustive.push(rel);
            }
        }
        exhaustive.sort();
        assert_eq!(
            exhaustive,
            ["image/view/eval.rs", "plan/selectivity.rs"],
            "exhaustive FilterPredicate matches belong in the evaluator and selectivity only"
        );
    }
}
