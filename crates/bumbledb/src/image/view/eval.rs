//! One predicate walk over an operand-provider capability.
//!
//! Four interpreters used to spell the same algebra — image columns,
//! LMDB fact bytes, ray-probe slots, batch words — each copying
//! `point_in` / `resolve` / `const_interval`. One [`Operands`] trait,
//! one [`holds`] walk, four providers. Static dispatch, never `dyn`.

use crate::image::{ColumnView, ColumnWidth, RelationImage};
use crate::ir::WordCmp;
use bumbledb_theory::schema::FieldId;

use super::{
    Const, FilterPredicate, IntervalConst, MaskConst, SetConst, ViewWordSource, WordOrParam,
};

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

    /// One word of a variable: `offset` is [`IntervalWord::offset`].
    #[must_use]
    pub fn var_word(var: crate::ir::VarId, offset: usize) -> Self {
        Self {
            at: var.0,
            offset: u8::try_from(offset).expect("Start/End offset is 0 or 1"),
            width: 1,
        }
    }
}

/// One loaded operand. The walk matches on this; providers mint it
/// from [`Operands::word`] / [`Operands::pair`] / [`Operands::block`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Loaded {
    Word(u64),
    Byte(u8),
    Pair(u64, u64),
    Block { words: [u64; 8], count: u8 },
}

/// Operand-provider capability: where a predicate's words come from.
/// Monomorphized per source, like [`crate::exec::run::Sink`].
pub(crate) trait Operands {
    type Error;

    fn word(&self, at: OperandAddr) -> Result<u64, Self::Error>;
    fn pair(&self, at: OperandAddr) -> Result<(u64, u64), Self::Error>;
    #[allow(dead_code, reason = "bytes<N> currently loads via `loaded`")]
    fn block(&self, at: OperandAddr) -> Result<([u64; 8], u8), Self::Error>;

    /// Shape-aware load: [`FilterPredicate::FieldsCompare`] needs it because both sides'
    /// column kinds are the provider's, not the predicate's.
    fn loaded(&self, at: OperandAddr) -> Result<Loaded, Self::Error>;

    /// Resolve a still-pending string intern. Image rows never see one
    /// (bind latches first); fact rows look the dictionary up, a miss
    /// the never-minted sentinel.
    fn intern(&self, bytes: &[u8]) -> Result<u64, Self::Error> {
        let _ = bytes;
        unreachable!("validated: PendingIntern is latched before this provider")
    }
}

/// Image columns at one position.
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

/// Binding-slot words: `addr.slot()` is the first slot of the operand.
/// A pair reads two consecutive slots; a block reads `width` slots.
/// `word` is a slot reader so the ray probe can keep its `Fn(usize) -> u64`
/// callback without copying the binding row.
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
        // Missed intern: the never-minted sentinel, same as bind.
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

/// Resolves a filter constant through the bind-time param slice.
pub(crate) fn resolve<'a>(value: &'a Const, params: &'a [Const]) -> &'a Const {
    match value {
        Const::Param(param) | Const::ParamSet(param) => &params[usize::from(param.0)],
        other => other,
    }
}

/// One constant's encoded interval words.
pub(crate) fn const_interval(value: &IntervalConst, params: &[Const]) -> (u64, u64) {
    match value {
        IntervalConst::Interval { start, end } => (*start, *end),
        IntervalConst::Param(param) => match &params[usize::from(param.0)] {
            Const::Interval { start, end } => (*start, *end),
            _ => unreachable!("param slice: interval param resolves to an interval"),
        },
    }
}

/// Point membership under the half-open interval: `start ≤ p AND p < end`.
pub(crate) const fn point_in(start: u64, end: u64, point: u64) -> bool {
    start <= point && point < end
}

/// The resolved mask of an `Allen` shape.
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

/// Whether this predicate is prepare-evaluable (no bind-time params,
/// no measure). The grounding-evaluator's condition-2 parser — lives
/// beside the walk so a new kind is one module. The operator/constant
/// pairing matches the old `parse_resolvable` boundary: set inequality
/// and order against non-word constants refuse.
#[must_use]
pub(crate) fn is_prepare_resolvable(filter: &FilterPredicate) -> bool {
    let ordinary = |op: WordCmp| {
        matches!(
            op,
            WordCmp::Eq | WordCmp::Ne | WordCmp::Lt | WordCmp::Le | WordCmp::Gt | WordCmp::Ge
        )
    };
    match filter {
        FilterPredicate::DurationCompare { .. }
        | FilterPredicate::DurationFieldsCompare { .. }
        | FilterPredicate::AnyPointIn { .. } => false,
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

/// One predicate over one operand source. Measure kinds are unreachable
/// here — they take [`duration_holds`] (Kleene Ray) or the fallible
/// refinement pass (views).
pub(crate) fn holds<O: Operands>(
    predicate: &FilterPredicate,
    ops: &O,
    params: &[Const],
) -> Result<bool, O::Error> {
    Ok(match predicate {
        FilterPredicate::Compare { field, op, value } => {
            compare_loaded(ops, ops.loaded(*field)?, resolve(value, params), *op)?
        }
        FilterPredicate::FieldsCompare { left, right, op } => {
            fields_compare(ops.loaded(*left)?, ops.loaded(*right)?, *op)
        }
        FilterPredicate::PointIn { field, point } => {
            let (start, end) = ops.pair(*field)?;
            point_in(start, end, resolve_word(point, params))
        }
        FilterPredicate::AnyPointIn { field, set } => {
            let (start, end) = ops.pair(*field)?;
            let points = word_set(set, params);
            let idx = points.partition_point(|&p| p < start);
            idx < points.len() && points[idx] < end
        }
        FilterPredicate::FieldsAllen { left, right, mask } => {
            let (l_start, l_end) = ops.pair(*left)?;
            let (r_start, r_end) = ops.pair(*right)?;
            mask_of(*mask, params).contains(crate::allen::classify_bounds(
                &l_start, &l_end, &r_start, &r_end,
            ))
        }
        FilterPredicate::FieldAllen { field, other, mask } => {
            let (f_start, f_end) = ops.pair(*field)?;
            let (start, end) = const_interval(other, params);
            mask_of(*mask, params).contains(crate::allen::classify_bounds(
                &f_start, &f_end, &start, &end,
            ))
        }
        FilterPredicate::FieldsPointIn { interval, point } => {
            let (start, end) = ops.pair(*interval)?;
            point_in(start, end, ops.word(*point)?)
        }
        FilterPredicate::FieldWithin { field, outer } => {
            let (start, end) = const_interval(outer, params);
            match ops.loaded(*field)? {
                Loaded::Word(word) => point_in(start, end, word),
                Loaded::Byte(_) | Loaded::Pair(..) | Loaded::Block { .. } => {
                    unreachable!("validated: within-comparands are scalar words")
                }
            }
        }
        FilterPredicate::DurationCompare { .. } | FilterPredicate::DurationFieldsCompare { .. } => {
            unreachable!("measure filters take duration_holds or the refinement pass")
        }
    })
}

/// The measure comparison: `None` is Ray (`end == MAX`); `Some` is the
/// two-valued order on `(end − start)`.
pub(crate) fn duration_holds<O: Operands>(
    predicate: &FilterPredicate,
    ops: &O,
    params: &[Const],
) -> Result<Option<bool>, O::Error> {
    match predicate {
        FilterPredicate::DurationCompare { field, op, value } => {
            let (start, end) = ops.pair(*field)?;
            if end == u64::MAX {
                return Ok(None);
            }
            Ok(Some(op.compare(
                &(end - start),
                &resolve_duration_word(value, params),
            )))
        }
        FilterPredicate::DurationFieldsCompare {
            interval,
            op,
            scalar,
        } => {
            let (start, end) = ops.pair(*interval)?;
            if end == u64::MAX {
                return Ok(None);
            }
            Ok(Some(op.compare(&(end - start), &ops.word(*scalar)?)))
        }
        _ => unreachable!("duration_holds is the measure kinds"),
    }
}

fn resolve_duration_word(value: &WordOrParam, params: &[Const]) -> u64 {
    match value {
        WordOrParam::Word(word) => *word,
        WordOrParam::Param(param) => match &params[usize::from(param.0)] {
            Const::Word(word) => *word,
            Const::Byte(byte) => u64::from(*byte),
            _ => unreachable!("param slice: measure param resolves to a word"),
        },
    }
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
        // Fact providers widen bool/enum bytes to words; image keeps Byte.
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

/// Conjunction over an image row. Infallible: image decode already ran.
#[must_use]
pub(crate) fn row_holds(
    image: &RelationImage,
    predicates: &[FilterPredicate],
    params: &[Const],
    position: usize,
) -> bool {
    let ops = ImageRow { image, position };
    predicates.iter().all(|predicate| match predicate {
        FilterPredicate::DurationCompare { .. } | FilterPredicate::DurationFieldsCompare { .. } => {
            true
        }
        _ => holds(predicate, &ops, params).unwrap_or_else(|e| match e {}),
    })
}
