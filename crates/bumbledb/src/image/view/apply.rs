//! The filter-application mechanism (docs/architecture/40-execution.md):
//! evaluates the per-atom conjunction over a warm image into a
//! survivor-position vector.

use std::sync::Arc;

use crate::image::{ColumnView, ColumnWidth, RelationImage};
use crate::ir::CmpOp;
use crate::schema::FieldId;

use super::{Const, FilterPredicate, MaskConst, ResolvedWordSource, View};

/// Resolves a filter constant through the bind-time param slice: `Param`
/// and `ParamSet` markers index it; everything else is already column form.
fn resolve<'a>(value: &'a Const, params: &'a [Const]) -> &'a Const {
    match value {
        Const::Param(param) | Const::ParamSet(param) => &params[usize::from(param.0)],
        other => other,
    }
}

/// The single column of a scalar field, through its span (the width
/// dispatch every field→column translation runs).
fn scalar_column(image: &RelationImage, field: FieldId) -> ColumnView<'_> {
    image.column(usize::from(image.span(field).first_column))
}

/// An interval field's two column words at one position: `(start, end)`.
///
/// # Panics
///
/// On a programmer-invariant violation: the field's span is not a word
/// pair (validation types every interval predicate over interval fields).
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

/// A scalar word field's column word at one position (interval point
/// operands: always word-typed by validation — interval elements are the
/// orderable scalars).
fn word_at(image: &RelationImage, field: FieldId, position: usize) -> u64 {
    match scalar_column(image, field) {
        ColumnView::Words(words) => words[position],
        ColumnView::Bytes(_) => unreachable!("validated: interval points are word-typed"),
    }
}

/// The resolved point word of a membership filter. A var-sourced point
/// never reaches the view evaluator: plan validation routes it into the
/// executor's membership probes (the point-membership scan runs inside
/// the join, once the variable is bound — `docs/architecture/
/// 40-execution.md`; the routing is [`ResolvedWordSource`]'s doc).
fn point_word(point: &ResolvedWordSource, params: &[Const]) -> u64 {
    match point {
        ResolvedWordSource::Word(word) => *word,
        ResolvedWordSource::Param(param) => match &params[usize::from(param.0)] {
            Const::Word(word) => *word,
            _ => unreachable!("validated: a point param resolves to a word"),
        },
        ResolvedWordSource::Var(_) => {
            unreachable!("var-sourced points are the executor's membership probes")
        }
    }
}

/// The resolved word set behind a set constant (sorted, deduplicated).
fn word_set<'a>(set: &'a Const, params: &'a [Const]) -> &'a [u64] {
    match resolve(set, params) {
        Const::WordSet(words) => words,
        _ => unreachable!("validated: a set resolves to a word set"),
    }
}

/// Point membership under the half-open interval: `start ≤ p AND p < end`
/// — `p == start` survives, `p == end` does not.
const fn point_in(start: u64, end: u64, point: u64) -> bool {
    start <= point && point < end
}

/// The resolved mask of an `Allen` shape: literal masks pass through;
/// param markers index the bind slice (a mask param resolves to its bits
/// as a `Word`), with the pre-encoded mirror applied after resolution
/// (`ConversedParam` — see [`MaskConst`]).
pub(crate) fn mask_of(mask: MaskConst, params: &[Const]) -> crate::allen::AllenMask {
    let param_bits = |param: crate::ir::ParamId| match &params[usize::from(param.0)] {
        Const::Word(word) => crate::allen::AllenMask::new(
            u16::try_from(*word).expect("bind stored 13-bit mask words"),
        )
        .expect("bind validated the mask"),
        _ => unreachable!("validated: a mask param resolves to a word"),
    };
    match mask {
        MaskConst::Mask(mask) => mask,
        MaskConst::Param(param) => param_bits(param),
        MaskConst::ConversedParam(param) => param_bits(param).converse(),
    }
}

/// Evaluates the conjunction against one image position. `params` is the
/// bind-time resolution slice, indexed by `ParamId`: `Word`/`Byte` for
/// scalar params, `Interval` for interval params, `WordSet` for set
/// params.
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // one arm per filter kind, in kind order
fn row_matches(
    image: &RelationImage,
    predicates: &[FilterPredicate],
    params: &[Const],
    position: usize,
) -> bool {
    predicates.iter().all(|predicate| match predicate {
        FilterPredicate::Compare { field, op, value } => {
            match (
                scalar_or_pair(image, *field, position),
                resolve(value, params),
            ) {
                (Operand::Word(word), Const::Word(c)) => op.compare(&word, c),
                (Operand::Byte(byte), Const::Byte(c)) => op.compare(&byte, c),
                // Interval-vs-interval-constant: value equality only (a
                // binding's `Eq` on a negated occurrence — every
                // interval-pair *predicate* is an `Allen` kind).
                (Operand::Pair(s, e), Const::Interval { start, end }) => match op {
                    CmpOp::Eq => s == *start && e == *end,
                    _ => unreachable!("validated: interval constants compare under Eq only"),
                },
                // bytes<N>: word-wise identity — Eq/Ne only by validation.
                (Operand::Block { words, count }, Const::Words(c)) => match op {
                    CmpOp::Eq => words[..usize::from(count)] == **c,
                    CmpOp::Ne => words[..usize::from(count)] != **c,
                    _ => unreachable!("validated: bytes<N> compares under Eq/Ne only"),
                },
                // A bound set: `Eq` matches any element (validation admits
                // sets under `Eq` only).
                (Operand::Word(word), Const::WordSet(set)) => set.binary_search(&word).is_ok(),
                (Operand::Byte(byte), Const::WordSet(set)) => {
                    set.binary_search(&u64::from(byte)).is_ok()
                }
                // A multi-word element set: span-wise binary search over
                // the flat sorted rows.
                (Operand::Block { words, count }, Const::WordSet(set)) => {
                    let width = usize::from(count);
                    debug_assert_eq!(set.len() % width, 0, "flat element-major rows");
                    let value = &words[..width];
                    let mut lo = 0usize;
                    let mut hi = set.len() / width;
                    let mut hit = false;
                    while lo < hi {
                        let mid = usize::midpoint(lo, hi);
                        match set[mid * width..(mid + 1) * width].cmp(value) {
                            std::cmp::Ordering::Less => lo = mid + 1,
                            std::cmp::Ordering::Greater => hi = mid,
                            std::cmp::Ordering::Equal => {
                                hit = true;
                                break;
                            }
                        }
                    }
                    hit
                }
                // Width mismatches are unrepresentable through validation,
                // and PendingIntern constants are resolved before execution
                // (docs/architecture/40-execution.md) — a miss empties the query without reaching here.
                _ => unreachable!("validated, resolved filter constant"),
            }
        }
        FilterPredicate::FieldsCompare { left, right, op } => {
            match (
                scalar_or_pair(image, *left, position),
                scalar_or_pair(image, *right, position),
            ) {
                (Operand::Word(a), Operand::Word(b)) => op.compare(&a, &b),
                (Operand::Byte(a), Operand::Byte(b)) => op.compare(&a, &b),
                // Interval fields compare pairwise over their two-word
                // spans; validation admits `Eq`/`Ne` only (order operators
                // never apply to intervals).
                (Operand::Pair(a_s, a_e), Operand::Pair(b_s, b_e)) => match op {
                    CmpOp::Eq => a_s == b_s && a_e == b_e,
                    CmpOp::Ne => a_s != b_s || a_e != b_e,
                    _ => unreachable!("validated: no order comparison over intervals"),
                },
                // bytes<N> fields compare word-wise, Eq/Ne only.
                (Operand::Block { words: a, count }, Operand::Block { words: b, .. }) => match op {
                    CmpOp::Eq => a[..usize::from(count)] == b[..usize::from(count)],
                    CmpOp::Ne => a[..usize::from(count)] != b[..usize::from(count)],
                    _ => unreachable!("validated: bytes<N> compares under Eq/Ne only"),
                },
                _ => unreachable!("same-fact comparison joins same-typed fields"),
            }
        }
        FilterPredicate::PointIn { field, point } => {
            let (start, end) = interval_at(image, *field, position);
            point_in(start, end, point_word(point, params))
        }
        FilterPredicate::AnyPointIn { field, set } => {
            let (start, end) = interval_at(image, *field, position);
            // Sorted set: the smallest element ≥ start decides membership.
            let points = word_set(set, params);
            let idx = points.partition_point(|&p| p < start);
            idx < points.len() && points[idx] < end
        }
        // The Allen kinds: classify-then-test — the scalar fallback and
        // reference beside the configuration kernel (`kernel_scan` takes
        // the dense pivot; this loop refines non-pivot conjuncts).
        // Encoded words preserve value order, so classification over
        // column words equals classification over values.
        FilterPredicate::FieldsAllen { left, right, mask } => {
            let (l_start, l_end) = interval_at(image, *left, position);
            let (r_start, r_end) = interval_at(image, *right, position);
            mask_of(*mask, params).contains(crate::allen::classify_bounds(
                &l_start, &l_end, &r_start, &r_end,
            ))
        }
        FilterPredicate::FieldAllen { field, other, mask } => {
            let (f_start, f_end) = interval_at(image, *field, position);
            let Const::Interval { start, end } = resolve(other, params) else {
                unreachable!("validated: the Allen constant side is an interval")
            };
            mask_of(*mask, params)
                .contains(crate::allen::classify_bounds(&f_start, &f_end, start, end))
        }
        FilterPredicate::FieldsPointIn { interval, point } => {
            let (start, end) = interval_at(image, *interval, position);
            point_in(start, end, word_at(image, *point, position))
        }
        FilterPredicate::FieldWithin { field, outer } => {
            let Const::Interval { start, end } = resolve(outer, params) else {
                unreachable!("validated: the outer side is an interval constant")
            };
            match scalar_or_pair(image, *field, position) {
                // A scalar field: point membership in the outer interval
                // (the field is scalar by construction — an interval
                // field under a constant is `FieldAllen`).
                Operand::Word(word) => point_in(*start, *end, word),
                Operand::Pair(..) | Operand::Byte(_) | Operand::Block { .. } => {
                    unreachable!("validated: within-comparands are scalar words")
                }
            }
        }
        // The measure kinds never enter the infallible conjunction: they
        // evaluate last, over the other filters' survivors, on the
        // fallible refinement pass (`apply` — the filter-order law).
        FilterPredicate::DurationCompare { .. } | FilterPredicate::DurationFieldsCompare { .. } => {
            unreachable!("measure filters take the fallible refinement pass")
        }
    })
}

/// One field's value at a position, through its span: the scalar word or
/// byte, an interval field's `(start, end)` word pair, or a
/// `bytes<N > 8>` field's padded word block.
enum Operand {
    Word(u64),
    Byte(u8),
    Pair(u64, u64),
    Block { words: [u64; 8], count: u8 },
}

fn scalar_or_pair(image: &RelationImage, field: FieldId, position: usize) -> Operand {
    let span = image.span(field);
    match span.width {
        ColumnWidth::WordPair => {
            let (start, end) = interval_at(image, field, position);
            Operand::Pair(start, end)
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
            Operand::Block {
                words,
                count: u8::try_from(count).expect("at most 8 words"),
            }
        }
        ColumnWidth::Word | ColumnWidth::Byte => match scalar_column(image, field) {
            ColumnView::Words(words) => Operand::Word(words[position]),
            ColumnView::Bytes(bytes) => Operand::Byte(bytes[position]),
        },
    }
}

/// Applies the filter conjunction over a (warm) image, writing survivors
/// into `buf` (caller-owned, reused across executions — capacity is
/// retained). An empty predicate list yields the unfiltered [`View::All`].
///
/// **The filter-order law** (docs/architecture/20-query-ir.md, § the
/// measure): the measure kinds evaluate last, over the survivors of every
/// other predicate of the atom — an `Allen` ray guard or a bounded-end
/// filter on the same atom always runs before the subtraction, so a
/// guarded fact never reaches it. On the survivors, `end == MAX` raises
/// the typed [`crate::Error::MeasureOfRay`] — the engine's one runtime
/// type error.
///
/// # Errors
///
/// `MeasureOfRay` when a measure filter's subtraction reaches a ray.
///
/// # Panics
///
/// Only on programmer-invariant violations: an image beyond the u32
/// position space (the 10⁷ scale axiom sits orders of magnitude below).
pub fn apply(
    image: &Arc<RelationImage>,
    predicates: &[FilterPredicate],
    params: &[Const],
    buf: Vec<u32>,
) -> crate::error::Result<View> {
    let is_measure = |p: &FilterPredicate| {
        matches!(
            p,
            FilterPredicate::DurationCompare { .. } | FilterPredicate::DurationFieldsCompare { .. }
        )
    };
    if !predicates.iter().any(is_measure) {
        return Ok(apply_infallible(image, predicates, params, buf));
    }
    // The measure path (cold by shape, correct by order): the other
    // predicates run through the ordinary machinery first, then each
    // measure predicate refines their survivors — dense survivors take
    // the fused gather+subtract kernel; everything else is the scalar
    // subtraction (strided stays scalar until measured, the standing
    // rule).
    let others: Vec<FilterPredicate> = predicates
        .iter()
        .filter(|p| !is_measure(p))
        .cloned()
        .collect();
    let mut view = apply_infallible(image, &others, params, buf);
    for predicate in predicates.iter().filter(|p| is_measure(p)) {
        view = refine_measure(image, predicate, params, view)?;
    }
    Ok(view)
}

/// One measure predicate over the current survivors. A full view takes
/// the fused dense kernel (subtract + range test + ray test in one
/// stride-1 pass); survivor views refine scalar, position by position.
fn refine_measure(
    image: &Arc<RelationImage>,
    predicate: &FilterPredicate,
    params: &[Const],
    view: View,
) -> crate::error::Result<View> {
    let ray = |start: u64, end: u64| crate::error::Error::MeasureOfRay { start, end };
    match predicate {
        FilterPredicate::DurationCompare { field, op, value } => {
            let Const::Word(bound) = resolve(value, params) else {
                unreachable!("validated: a measure compares against a u64 word")
            };
            // The order operator as an inclusive duration range — the
            // subtraction feeds the existing range machinery.
            let (lo, hi) = match op {
                CmpOp::Lt => match bound.checked_sub(1) {
                    Some(hi) => (0, hi),
                    None => (1, 0), // dur < 0: empty (lo > hi keeps nothing)
                },
                CmpOp::Le => (0, *bound),
                CmpOp::Gt => match bound.checked_add(1) {
                    Some(lo) => (lo, u64::MAX),
                    None => (1, 0), // dur > MAX: empty
                },
                CmpOp::Ge => (*bound, u64::MAX),
                _ => unreachable!("validated: measures compare under order operators"),
            };
            let (starts, ends) = interval_columns(image, *field);
            match view {
                View::All(_) => {
                    let mut positions = Vec::new();
                    crate::exec::kernel::filter_duration_range_u64(
                        starts,
                        ends,
                        lo,
                        hi,
                        &mut positions,
                    )
                    .map_err(|position| ray(starts[position], ends[position]))?;
                    Ok(View::Survivors {
                        image: Arc::clone(image),
                        positions,
                    })
                }
                View::Survivors {
                    image: view_image,
                    mut positions,
                } => {
                    let mut cursor = 0usize;
                    for read in 0..positions.len() {
                        let p = positions[read] as usize;
                        let (start, end) = (starts[p], ends[p]);
                        if end == u64::MAX {
                            return Err(ray(start, end));
                        }
                        positions[cursor] = positions[read];
                        cursor += usize::from(lo <= end - start && end - start <= hi);
                    }
                    positions.truncate(cursor);
                    Ok(View::Survivors {
                        image: view_image,
                        positions,
                    })
                }
                View::Unbound => unreachable!("apply binds the view it filters"),
            }
        }
        FilterPredicate::DurationFieldsCompare {
            interval,
            op,
            scalar,
        } => {
            // Two varying columns per position — no constant side, no
            // kernel shape (the `FieldsCompare` precedent): scalar over
            // whatever positions survive.
            let (starts, ends) = interval_columns(image, *interval);
            let scalars = match scalar_column(image, *scalar) {
                ColumnView::Words(words) => words,
                ColumnView::Bytes(_) => unreachable!("validated: the measure side is u64"),
            };
            let row_count = view.len();
            let mut positions = view.recycle();
            let survivors_input = !positions.is_empty() || row_count == 0;
            if !survivors_input {
                positions.extend(0..u32::try_from(row_count).expect("positions fit u32"));
            }
            let mut cursor = 0usize;
            for read in 0..positions.len() {
                let p = positions[read] as usize;
                let (start, end) = (starts[p], ends[p]);
                if end == u64::MAX {
                    return Err(ray(start, end));
                }
                positions[cursor] = positions[read];
                cursor += usize::from(op.compare(&(end - start), &scalars[p]));
            }
            positions.truncate(cursor);
            Ok(View::Survivors {
                image: Arc::clone(image),
                positions,
            })
        }
        _ => unreachable!("refine_measure takes the measure kinds"),
    }
}

/// The infallible conjunction — every non-measure predicate kind.
#[must_use]
fn apply_infallible(
    image: &Arc<RelationImage>,
    predicates: &[FilterPredicate],
    params: &[Const],
    mut buf: Vec<u32>,
) -> View {
    if predicates.is_empty() {
        return View::All(Arc::clone(image));
    }
    let row_count = image.row_count();
    debug_assert!(u32::try_from(row_count).is_ok(), "positions fit u32");
    buf.clear();

    // Kernel fast path: the *first kernel-compatible* predicate (not
    // blindly `predicates[0]` — a leading FieldsCompare or byte-column
    // `Ne` must not hide the SIMD path) produces the initial survivor
    // set; every other predicate refines it below.
    if let Some(pivot) = predicates
        .iter()
        .position(|p| kernel_scan(image, p, params, &mut buf))
    {
        let survivors_only = predicates.len() == 1;
        if survivors_only {
            return View::Survivors {
                image: Arc::clone(image),
                positions: buf,
            };
        }
        // Refine in place: evaluate the remaining conjunction per survivor
        // with the branchless cursor write.
        let mut cursor = 0usize;
        for read in 0..buf.len() {
            let position = buf[read] as usize;
            let mut keep = true;
            for (idx, predicate) in predicates.iter().enumerate() {
                if idx == pivot {
                    continue;
                }
                keep &= row_matches(image, std::slice::from_ref(predicate), params, position);
            }
            buf[cursor] = buf[read];
            cursor += usize::from(keep);
        }
        buf.truncate(cursor);
        return View::Survivors {
            image: Arc::clone(image),
            positions: buf,
        };
    }

    buf.resize(row_count, 0);
    let mut cursor = 0usize;
    // The scalar branchless survivor write (D4's compaction pattern):
    // unconditional store, conditional cursor advance — no `if` in this
    // loop body.
    for position in 0..row_count {
        let keep = row_matches(image, predicates, params, position);
        buf[cursor] = u32::try_from(position).expect("checked above");
        cursor += usize::from(keep);
    }
    buf.truncate(cursor);
    View::Survivors {
        image: Arc::clone(image),
        positions: buf,
    }
}

/// An interval field's two word-column slices — the operand shape of
/// every fused two-column composition.
fn interval_columns(image: &RelationImage, field: FieldId) -> (&[u64], &[u64]) {
    let span = image.span(field);
    debug_assert_eq!(span.width, ColumnWidth::WordPair);
    let first = usize::from(span.first_column);
    match (image.column(first), image.column(first + 1)) {
        (ColumnView::Words(starts), ColumnView::Words(ends)) => (starts, ends),
        _ => unreachable!("an interval span covers two word columns"),
    }
}

/// Attempts the kernel fast path for one predicate. Scalar compares
/// against a resolved `Word`/`Byte` constant lower to the fixed-width
/// predicate scans; the membership kinds (`PointIn`, `AnyPointIn`,
/// `FieldWithin`) lower to compositions of that same shape over the
/// start/end column pair — two compare-and-mask passes `AND`ed, never a
/// new kernel shape (docs/architecture/40-execution.md, § access
/// paths); the Allen kinds take the configuration kernel over the dense
/// stride-1 column pairs (one branchless, flag-free kernel for every
/// mask — `exec/kernel/allen.rs`). A negated occurrence's view rides
/// this same path: its Allen filters classify identically and the probe
/// inverts at the hit, exactly like every other predicate class.
/// Returns whether the scan ran; `false` falls back to the scalar
/// `row_matches` loop.
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // one arm per kernel-shaped predicate kind
fn kernel_scan(
    image: &RelationImage,
    predicate: &FilterPredicate,
    params: &[Const],
    out: &mut Vec<u32>,
) -> bool {
    match predicate {
        FilterPredicate::Compare { .. } => {}
        FilterPredicate::PointIn { field, point } => {
            let (starts, ends) = interval_columns(image, *field);
            crate::exec::kernel::filter_point_in_u64(starts, ends, point_word(point, params), out);
            return true;
        }
        FilterPredicate::AnyPointIn { field, set } => {
            let (starts, ends) = interval_columns(image, *field);
            crate::exec::kernel::filter_any_point_in_u64(starts, ends, word_set(set, params), out);
            return true;
        }
        FilterPredicate::FieldWithin { field, outer } => {
            let Const::Interval { start, end } = resolve(outer, params) else {
                unreachable!("validated: the outer side is an interval constant")
            };
            let span = image.span(*field);
            // A scalar field within the constant interval: point
            // membership is the range scan `[start, end - 1]` (the
            // half-open bound; `end >= 1` because `start < end` and
            // word order is value order). Scalar by construction — an
            // interval field under a constant is `FieldAllen`.
            debug_assert_eq!(span.width, ColumnWidth::Word);
            let ColumnView::Words(words) = image.column(usize::from(span.first_column)) else {
                unreachable!("a word span covers a word column")
            };
            crate::exec::kernel::filter_range_u64(words, *start, *end - 1, out);
            return true;
        }
        // The Allen kinds: dense stride-1 endpoint columns through the
        // configuration kernel — codes via the 8 predicate lanes and the
        // 64-byte `tbl` nibble table, membership via the broadcast mask,
        // survivors via the branchless cursor-write.
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
            let Const::Interval { start, end } = resolve(other, params) else {
                unreachable!("validated: the Allen constant side is an interval")
            };
            crate::exec::kernel::allen_filter_columns_const(
                starts,
                ends,
                *start,
                *end,
                mask_of(*mask, params),
                out,
            );
            return true;
        }
        // Same-fact comparisons read two varying columns per position —
        // no constant side, no kernel shape; the scalar loop evaluates
        // them.
        FilterPredicate::FieldsCompare { .. } | FilterPredicate::FieldsPointIn { .. } => {
            return false;
        }
        // The measure kinds never reach the infallible machinery: they
        // evaluate on the fallible refinement pass (`apply`).
        FilterPredicate::DurationCompare { .. } | FilterPredicate::DurationFieldsCompare { .. } => {
            unreachable!("measure filters take the fallible refinement pass")
        }
    }
    let FilterPredicate::Compare { field, op, value } = predicate else {
        unreachable!("every other kind returned above")
    };
    let span = image.span(*field);
    let value = resolve(value, params);
    if span.width == ColumnWidth::WordPair {
        // Interval value equality (`Eq` on a negated occurrence's view)
        // has no fixed-width scan shape, like scalar `Ne`: scalar loop.
        return false;
    }
    if let ColumnWidth::Words { count } = span.width {
        // A multi-word bytes<N> Eq: the existing fixed-width Eq scan,
        // widened by word count — the first column's kernel pass seeds
        // the survivors, the remaining columns refine them word-wise
        // (no new NEON shapes). `Ne` has no scan shape, like scalar Ne.
        let (Const::Words(words), CmpOp::Eq) = (value, op) else {
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
                CmpOp::Eq => {
                    crate::exec::kernel::filter_eq_u64(words, *c, out);
                    return true;
                }
                CmpOp::Lt => {
                    let Some(hi) = c.checked_sub(1) else {
                        out.clear(); // x < 0 over unsigned words: empty
                        return true;
                    };
                    (0, hi)
                }
                CmpOp::Le => (0, *c),
                CmpOp::Gt => {
                    let Some(lo) = c.checked_add(1) else {
                        out.clear(); // x > MAX: empty
                        return true;
                    };
                    (lo, u64::MAX)
                }
                CmpOp::Ge => (*c, u64::MAX),
                // `Ne` has no fixed-width scan shape; the interval
                // operators never pair with a single-word constant
                // (normalization emits the interval filter kinds).
                CmpOp::Ne | CmpOp::Allen { .. } | CmpOp::PointIn => return false,
            };
            crate::exec::kernel::filter_range_u64(words, lo, hi, out);
            true
        }
        (ColumnView::Bytes(bytes), Const::Byte(c)) if *op == CmpOp::Eq => {
            crate::exec::kernel::filter_eq_u8(bytes, *c, out);
            true
        }
        _ => false,
    }
}
