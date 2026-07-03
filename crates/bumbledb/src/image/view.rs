//! Filtered views (PRD 12): per-atom filter evaluation producing
//! survivor-position vectors over images. Views are query-local and never
//! cached (`docs/architecture/40-storage.md`); COLT roots iterate the view,
//! and view positions index the image.

use std::sync::Arc;

#[cfg(test)]
use crate::error::Result;
#[cfg(test)]
use crate::image::build;
use crate::image::{ColumnView, RelationImage};
use crate::ir::CmpOp;
use crate::schema::FieldId;
#[cfg(test)]
use crate::schema::{RelationId, Schema};
#[cfg(test)]
use crate::storage::env::ReadTxn;

/// The constant side of a lowered filter. `Word`/`Byte` are column form —
/// the byte-order-normalized word for 8-byte columns, the raw byte for
/// 1-byte columns. `Param` resolves at bind time through the evaluator's
/// param slice; `PendingIntern` is a raw String/Bytes literal resolved to
/// an intern-id word per execution (PRD 25 — a dictionary miss means the
/// query is empty on this snapshot, so the evaluator never sees one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Const {
    Word(u64),
    Byte(u8),
    /// Bind-time symbolic constant; the evaluator indexes the param slice.
    Param(crate::ir::ParamId),
    /// A raw String/Bytes literal awaiting per-execution intern resolution
    /// (`tag` is the dictionary type tag).
    PendingIntern {
        tag: u8,
        bytes: Box<[u8]>,
    },
}

/// One lowered per-atom filter (produced by PRD 15's normalization).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterPredicate {
    /// `field <op> constant`.
    Compare {
        field: FieldId,
        op: CmpOp,
        value: Const,
    },
    /// Same-fact equality between two fields of one atom (the lowering of a
    /// repeated in-atom variable). Both fields have the same structural
    /// type by validation, hence the same column kind.
    FieldsEqual { left: FieldId, right: FieldId },
}

/// A query-local view over an image: either every position (unfiltered) or
/// the filter's survivors. A two-variant representation, not a sentinel
/// vector.
#[derive(Debug)]
pub enum View {
    /// Every position `0..row_count`.
    All(Arc<RelationImage>),
    /// The survivor positions, in ascending order.
    Survivors {
        image: Arc<RelationImage>,
        positions: Vec<u32>,
    },
}

impl View {
    /// The underlying image.
    #[must_use]
    pub fn image(&self) -> &Arc<RelationImage> {
        match self {
            Self::All(image) | Self::Survivors { image, .. } => image,
        }
    }

    /// Number of positions the view exposes.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::All(image) => image.row_count(),
            Self::Survivors { positions, .. } => positions.len(),
        }
    }

    #[cfg(test)]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Iterates the view's image positions in ascending order.
    ///
    /// # Panics
    ///
    /// Only on a programmer-invariant violation: an image beyond the u32
    /// position space (the scale axiom sits orders of magnitude below).
    #[cfg(test)]
    pub fn positions(&self) -> impl Iterator<Item = u32> + '_ {
        // Chained empty arms keep one concrete iterator type without
        // boxing: exactly one arm is nonempty.
        let (all, survivors) = match self {
            Self::All(image) => (
                0..u32::try_from(image.row_count()).expect("row_count < u32::MAX"),
                [].iter(),
            ),
            Self::Survivors { positions, .. } => (0..0u32, positions.iter()),
        };
        all.chain(survivors.copied())
    }

    /// The image position at view index `idx` (reader: COLT root
    /// iteration, PRD 18).
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: `idx` out of the view's range.
    #[must_use]
    pub fn position_at(&self, idx: usize) -> u32 {
        match self {
            Self::All(_) => u32::try_from(idx).expect("positions fit u32"),
            Self::Survivors { positions, .. } => positions[idx],
        }
    }

    /// Reclaims the survivor buffer for reuse (the caller-owned storage
    /// discipline: buffers belong to the prepared query, PRD 25).
    #[must_use]
    pub fn recycle(self) -> Vec<u32> {
        match self {
            Self::All(_) => Vec::new(),
            Self::Survivors { positions, .. } => positions,
        }
    }
}

/// Evaluates the conjunction against one image position. `params` is the
/// bind-time resolution slice, indexed by `ParamId`, holding `Word`/`Byte`
/// constants only.
fn row_matches(
    image: &RelationImage,
    predicates: &[FilterPredicate],
    params: &[Const],
    position: usize,
) -> bool {
    predicates.iter().all(|predicate| match predicate {
        FilterPredicate::Compare { field, op, value } => {
            let value = match value {
                Const::Param(param) => &params[usize::from(param.0)],
                other => other,
            };
            match (image.column(usize::from(field.0)), value) {
                (ColumnView::Words(words), Const::Word(c)) => op.compare(&words[position], c),
                (ColumnView::Bytes(bytes), Const::Byte(c)) => op.compare(&bytes[position], c),
                // Width mismatches are unrepresentable through validation,
                // and PendingIntern constants are resolved before execution
                // (PRD 25) — a miss empties the query without reaching here.
                _ => unreachable!("validated, resolved filter constant"),
            }
        }
        FilterPredicate::FieldsEqual { left, right } => {
            match (
                image.column(usize::from(left.0)),
                image.column(usize::from(right.0)),
            ) {
                (ColumnView::Words(a), ColumnView::Words(b)) => a[position] == b[position],
                (ColumnView::Bytes(a), ColumnView::Bytes(b)) => a[position] == b[position],
                _ => unreachable!("same-fact equality joins same-typed fields"),
            }
        }
    })
}

/// Applies the filter conjunction over a (warm) image, writing survivors
/// into `buf` (caller-owned, reused across executions — capacity is
/// retained). An empty predicate list yields the unfiltered [`View::All`].
///
/// # Panics
///
/// Only on programmer-invariant violations: an image beyond the u32
/// position space (the 10⁷ scale axiom sits orders of magnitude below).
#[must_use]
pub fn apply(
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

/// Attempts the kernel fast path for one predicate: a compare against a
/// resolved `Word`/`Byte` constant on a plain column lowers to a
/// fixed-width predicate scan. Returns whether the scan ran.
fn kernel_scan(
    image: &RelationImage,
    predicate: &FilterPredicate,
    params: &[Const],
    out: &mut Vec<u32>,
) -> bool {
    let FilterPredicate::Compare { field, op, value } = predicate else {
        return false;
    };
    let value = match value {
        Const::Param(param) => &params[usize::from(param.0)],
        other => other,
    };
    match (image.column(usize::from(field.0)), value) {
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
                CmpOp::Ne => return false, // no fixed-width scan shape
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

/// Cold dual-output build (`40-storage.md`): one storage scan produces both
/// the cacheable unfiltered image and the query-local survivor view. The
/// caller inserts the image into the cache.
///
/// The filter pass runs over the freshly decoded columns rather than being
/// interleaved into the decode loop — the one storage scan is the expensive
/// part, and sharing `apply`'s evaluator beats duplicating it inside the
/// builder (deliberate simplification of PRD 12's parenthetical).
///
/// # Errors
///
/// Build errors (`Lmdb`, `Corruption`) propagate.
#[cfg(test)]
pub fn build_with_filters(
    txn: &ReadTxn<'_>,
    schema: &Schema,
    rel: RelationId,
    predicates: &[FilterPredicate],
    params: &[Const],
    buf: Vec<u32>,
) -> Result<(Arc<RelationImage>, View)> {
    let image = build(txn, schema, rel)?;
    let view = apply(&image, predicates, params, buf);
    Ok((image, view))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{decode_field, encode_fact, encode_i64, encode_u64, ValueRef};
    use crate::error::Result as DbResult;
    use crate::schema::{
        FieldDescriptor, Generation, RelationDescriptor, Schema, SchemaDescriptor, ValueType,
    };
    use crate::storage::commit::commit;
    use crate::storage::delta::WriteDelta;
    use crate::storage::env::Environment;
    use crate::storage::read;
    use crate::testutil::TempDir;

    /// R(id u64, flag bool, a i64, b i64).
    fn schema() -> Schema {
        SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "R".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Serial,
                    },
                    FieldDescriptor {
                        name: "flag".into(),
                        value_type: ValueType::Bool,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "a".into(),
                        value_type: ValueType::I64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "b".into(),
                        value_type: ValueType::I64,
                        generation: Generation::None,
                    },
                ],
                constraints: vec![],
            }],
        }
        .validate()
        .expect("valid fixture")
    }

    const R: RelationId = RelationId(0);

    fn fact(schema: &Schema, id: u64, flag: bool, a: i64, b: i64) -> Vec<u8> {
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(id),
                ValueRef::Bool(flag),
                ValueRef::I64(a),
                ValueRef::I64(b),
            ],
            schema.relation(R).layout(),
            &mut bytes,
        );
        bytes
    }

    fn populated(dir: &TempDir, schema: &Schema) -> Environment {
        let env = Environment::create(dir.path(), schema).expect("create");
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(schema);
        for i in 0..50i64 {
            let id = i.cast_unsigned();
            // Every fifth row has a == b so the equality filter has matches.
            let b = if i % 5 == 0 { i - 25 } else { (i % 7) - 3 };
            delta
                .insert(&view, R, &fact(schema, id, i % 2 == 0, i - 25, b))
                .expect("insert");
        }
        drop(view);
        commit(delta, &env).expect("commit");
        env
    }

    /// The naive oracle: per-row decode via the fact codec, no images.
    fn oracle(
        env: &Environment,
        schema: &Schema,
        keep: impl Fn(u64, bool, i64, i64) -> bool,
    ) -> Vec<u64> {
        let txn = env.read_txn().expect("txn");
        let layout = schema.relation(R).layout();
        read::scan(&txn, schema, R)
            .expect("scan")
            .map(|entry| {
                let (_, bytes) = entry.expect("ok");
                let id = match decode_field(bytes, layout, 0).expect("decode") {
                    crate::encoding::ValueRef::U64(v) => v,
                    other => panic!("{other:?}"),
                };
                let flag = match decode_field(bytes, layout, 1).expect("decode") {
                    crate::encoding::ValueRef::Bool(v) => v,
                    other => panic!("{other:?}"),
                };
                let a = match decode_field(bytes, layout, 2).expect("decode") {
                    crate::encoding::ValueRef::I64(v) => v,
                    other => panic!("{other:?}"),
                };
                let b = match decode_field(bytes, layout, 3).expect("decode") {
                    crate::encoding::ValueRef::I64(v) => v,
                    other => panic!("{other:?}"),
                };
                (id, flag, a, b)
            })
            .filter(|(id, flag, a, b)| keep(*id, *flag, *a, *b))
            .map(|(id, ..)| id)
            .collect()
    }

    fn survivor_ids(view: &View) -> Vec<u64> {
        view.positions()
            .map(|p| view.image().column_words(0)[p as usize])
            .collect()
    }

    #[test]
    fn conjunction_over_mixed_width_fields_matches_the_naive_oracle() {
        let dir = TempDir::new("view-conjunction");
        let schema = schema();
        let env = populated(&dir, &schema);
        let txn = env.read_txn().expect("txn");
        let image = build(&txn, &schema, R).expect("build");

        // flag == true AND a >= -10 AND a < 15
        let predicates = vec![
            FilterPredicate::Compare {
                field: FieldId(1),
                op: CmpOp::Eq,
                value: Const::Byte(1),
            },
            FilterPredicate::Compare {
                field: FieldId(2),
                op: CmpOp::Ge,
                value: Const::Word(u64::from_be_bytes(encode_i64(-10))),
            },
            FilterPredicate::Compare {
                field: FieldId(2),
                op: CmpOp::Lt,
                value: Const::Word(u64::from_be_bytes(encode_i64(15))),
            },
        ];
        let view = apply(&image, &predicates, &[], Vec::new());
        let expected = oracle(&env, &schema, |_, flag, a, _| {
            flag && (-10..15).contains(&a)
        });
        assert_eq!(survivor_ids(&view), expected);
        assert!(!expected.is_empty(), "fixture exercises the filter");
    }

    #[test]
    fn same_fact_field_equality_pairs_work() {
        let dir = TempDir::new("view-fields-equal");
        let schema = schema();
        let env = populated(&dir, &schema);
        let txn = env.read_txn().expect("txn");
        let image = build(&txn, &schema, R).expect("build");
        let predicates = vec![FilterPredicate::FieldsEqual {
            left: FieldId(2),
            right: FieldId(3),
        }];
        let view = apply(&image, &predicates, &[], Vec::new());
        let expected = oracle(&env, &schema, |_, _, a, b| a == b);
        assert_eq!(survivor_ids(&view), expected);
        assert!(!expected.is_empty(), "fixture exercises the equality");
    }

    #[test]
    fn unsatisfiable_filter_yields_an_empty_survivor_set() {
        let dir = TempDir::new("view-empty");
        let schema = schema();
        let env = populated(&dir, &schema);
        let txn = env.read_txn().expect("txn");
        let image = build(&txn, &schema, R).expect("build");
        let predicates = vec![FilterPredicate::Compare {
            field: FieldId(0),
            op: CmpOp::Eq,
            value: Const::Word(u64::MAX),
        }];
        let view = apply(&image, &predicates, &[], Vec::new());
        assert_eq!(view.len(), 0);
        assert!(view.is_empty());
        assert_eq!(view.positions().count(), 0);
    }

    #[test]
    fn no_predicates_yield_the_all_variant() {
        let dir = TempDir::new("view-all");
        let schema = schema();
        let env = populated(&dir, &schema);
        let txn = env.read_txn().expect("txn");
        let image = build(&txn, &schema, R).expect("build");
        let view = apply(&image, &[], &[], Vec::new());
        assert!(matches!(view, View::All(_)));
        assert_eq!(view.len(), 50);
        let positions: Vec<u32> = view.positions().collect();
        assert_eq!(positions, (0..50).collect::<Vec<u32>>());
    }

    #[test]
    fn cold_dual_output_matches_separate_build_and_apply() -> DbResult<()> {
        let dir = TempDir::new("view-dual-output");
        let schema = schema();
        let env = populated(&dir, &schema);
        let txn = env.read_txn().expect("txn");
        let predicates = vec![FilterPredicate::Compare {
            field: FieldId(0),
            op: CmpOp::Ge,
            value: Const::Word(u64::from_be_bytes(encode_u64(40))),
        }];

        let (image, view) = build_with_filters(&txn, &schema, R, &predicates, &[], Vec::new())?;
        let reference = build(&txn, &schema, R)?;
        // Byte-identical columns (addresses differ; contents must not).
        assert_eq!(image.row_count(), reference.row_count());
        for field in 0..4 {
            assert_eq!(image.column(field), reference.column(field));
        }
        // ...and the view equals apply() over that image.
        let reapplied = apply(&image, &predicates, &[], Vec::new());
        assert_eq!(
            view.positions().collect::<Vec<_>>(),
            reapplied.positions().collect::<Vec<_>>()
        );
        assert_eq!(view.len(), 10);
        Ok(())
    }
}
