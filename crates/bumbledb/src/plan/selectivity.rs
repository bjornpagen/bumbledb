//! Prepare-time cardinality estimation (docs/architecture/40-execution.md): per-occurrence
//! input estimates for the join-order DP and the EXPLAIN/report
//! honesty numbers. Three sources, strongest first — schema structure
//! (free and exact), resident-image exact distinct counts, documented
//! constant floors. Prepare **never builds** an image for statistics
//! (the cache is peeked); a cold prepare degrades to bounds and floors,
//! and runtime cover choice (docs/architecture/40-execution.md) carries the load-bearing
//! decisions either way.

use crate::image::cache::ImageCache;
use crate::image::view::{Const, FilterPredicate};
use crate::image::ColumnWidth;
use crate::ir::normalize::Occurrence;
use crate::ir::CmpOp;
use crate::plan::fj::split_filters;
use crate::plan::planner::OccStats;
use crate::schema::{FieldId, Schema, StatementDescriptor};
use crate::storage::env::ReadTxn;
use crate::storage::read;

/// The distinct-count floor for an Eq selection on a field nothing else
/// describes (a plain string/int column with no resident image): keep
/// `rows / 64`. Chosen small enough that a selection always looks
/// selective to the DP, large enough that a genuinely low-cardinality
/// column (resident images tell the truth) still dominates it.
pub(crate) const DEFAULT_EQ_DISTINCT: u64 = 64;

/// A range residual (`Lt/Le/Gt/Ge` against a constant) keeps 1/4 of its
/// input — the classic textbook fraction; ranges are scans by design and
/// the estimate only orders joins. Membership predicates are fixed
/// word-range compositions over the start/end column pair
/// (docs/architecture/40-execution.md), so they take the same class.
pub(crate) const RANGE_KEEP_DEN: u64 = 4;

/// The Allen basics partition every interval pair (JEPD), so a mask
/// predicate's honest keep fraction is `popcount/13` — the mask's measure
/// in the coordinate system, no workload assumption needed — clamped to
/// the existing floor ladder exactly like every residual: never below one
/// row here, never outside `[1, rows]` at the end of the estimate. A
/// *param* mask is unmeasurable at prepare (the ladder's carve-out) and
/// takes the range class ([`RANGE_KEEP_DEN`]), like every other param.
fn allen_keep(estimate: u64, mask: crate::image::view::MaskConst) -> u64 {
    match mask {
        crate::image::view::MaskConst::Mask(mask) => {
            (estimate.saturating_mul(u64::from(mask.popcount())) / 13).max(1)
        }
        crate::image::view::MaskConst::Param(_)
        | crate::image::view::MaskConst::ConversedParam(_) => (estimate / RANGE_KEEP_DEN).max(1),
    }
}

/// A same-fact field equality (`FieldsCompare` under `Eq`, the repeated
/// in-atom variable) keeps `1/64` — same floor class as an Eq selection.
pub(crate) const FIELDS_EQ_KEEP_DEN: u64 = 64;

/// The assumed distinct-match count of a set-bound position. Params are
/// unmeasurable at prepare (the ladder's carve-out extends to sets), and
/// the prepared plan pins the documented **small-set assumption** —
/// |set| ≤ a few hundred, re-prepare or restructure beyond it
/// (`docs/architecture/20-query-ir.md`, § prepared queries). 16 is a
/// floor-style constant like the ladder's: small enough that a set-bound
/// selection still reads selective to the DP, large enough to price it
/// above a scalar equality.
pub(crate) const PARAM_SET_PLANNING_CARDINALITY: u64 = 16;

/// One occurrence's planner statistics: the cardinality estimate —
/// `rows` divided by each Eq selection's distinct count (times the
/// set-cardinality assumption for set-bound positions) and each
/// residual's keep fraction, clamped to `[1, rows]` (`Ne` keeps
/// everything) — plus every bound variable's base-relation distinct
/// count for the join-step fanout model.
///
/// # Errors
///
/// `Lmdb` from counter reads (containment target row counts, the cache
/// peek).
pub(crate) fn occurrence_stats(
    txn: &ReadTxn<'_>,
    cache: &ImageCache,
    schema: &Schema,
    occurrence: &Occurrence,
    rows: u64,
) -> crate::error::Result<OccStats> {
    let image = cache.peek(txn, occurrence.relation)?;
    let mut var_distincts = Vec::with_capacity(occurrence.vars.len());
    for (field, var) in &occurrence.vars {
        let distinct = distinct_of(
            txn,
            schema,
            occurrence.relation,
            *field,
            image.as_deref(),
            rows,
        )?;
        var_distincts.push((*var, distinct));
    }
    let estimate = occurrence_estimate(txn, schema, occurrence, image.as_deref(), rows)?;
    Ok(OccStats {
        occ_id: occurrence.occ_id,
        rows: estimate,
        var_distincts,
    })
}

/// The assumed distinct-match count of one selection: 1 for every scalar
/// constant; the documented small-set assumption for a set-bound
/// position (a bound `WordSet` carries its real, deduplicated size).
fn selection_matches(value: &Const) -> u64 {
    match value {
        Const::ParamSet(_) => PARAM_SET_PLANNING_CARDINALITY,
        Const::WordSet(words) => u64::try_from(words.len()).expect("bounded set").max(1),
        _ => 1,
    }
}

/// The estimate half of [`occurrence_stats`].
fn occurrence_estimate(
    txn: &ReadTxn<'_>,
    schema: &Schema,
    occurrence: &Occurrence,
    image: Option<&crate::image::RelationImage>,
    rows: u64,
) -> crate::error::Result<u64> {
    let (selections, residuals) = split_filters(&occurrence.filters);
    let mut estimate = rows;
    for selection in &selections {
        let distinct = distinct_of(
            txn,
            schema,
            occurrence.relation,
            selection.field,
            image,
            rows,
        )?;
        estimate =
            (estimate.saturating_mul(selection_matches(&selection.value)) / distinct.max(1)).max(1);
    }
    for residual in &residuals {
        // The Allen kinds carry their own honest fraction ([`allen_keep`]:
        // popcount/13); everything else takes a constant denominator.
        if let FilterPredicate::FieldsAllen { mask, .. }
        | FilterPredicate::FieldAllen { mask, .. } = residual
        {
            estimate = allen_keep(estimate, *mask);
            continue;
        }
        let keep_den = match residual {
            FilterPredicate::Compare { op, .. } => match op {
                CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge => RANGE_KEEP_DEN,
                CmpOp::Ne => 1,
                CmpOp::Eq => unreachable!("split_filters routed Eq into selections"),
                CmpOp::Allen { .. } | CmpOp::Contains => {
                    unreachable!("interval predicates lower to their fixed shapes")
                }
            },
            FilterPredicate::FieldsCompare { op, .. } => match op {
                CmpOp::Eq => FIELDS_EQ_KEEP_DEN,
                CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge => RANGE_KEEP_DEN,
                CmpOp::Ne => 1,
                CmpOp::Allen { .. } | CmpOp::Contains => {
                    unreachable!("same-atom interval predicates lower to their fixed shapes")
                }
            },
            // The fixed membership compositions (word ranges over the
            // start/end pair), and the measure comparisons — a range
            // predicate over the derived duration word, riding the
            // existing range keep-fraction floor unmodified (20-query-ir § the measure;
            // validation admits order operators only, so the range class
            // is exact, not a default).
            FilterPredicate::PointIn { .. }
            | FilterPredicate::AnyPointIn { .. }
            | FilterPredicate::FieldsContainPoint { .. }
            | FilterPredicate::FieldWithin { .. }
            | FilterPredicate::DurationCompare { .. }
            | FilterPredicate::DurationFieldsCompare { .. } => RANGE_KEEP_DEN,
            FilterPredicate::FieldsAllen { .. } | FilterPredicate::FieldAllen { .. } => {
                unreachable!("handled above")
            }
        };
        estimate = (estimate / keep_den).max(1);
        // A set-bound membership matches any of the set's elements —
        // the range fraction per element, the documented small-set
        // count of elements (unmeasurable at prepare, like every param).
        if matches!(residual, FilterPredicate::AnyPointIn { .. }) {
            estimate = estimate.saturating_mul(PARAM_SET_PLANNING_CARDINALITY);
        }
    }
    Ok(estimate.clamp(1, rows.max(1)))
}

/// The distinct-count ladder for one field, strongest source first:
/// 1. a single-field key (a `Functionality` statement whose whole
///    projection is this field) ⇒ exactly `rows` — sound for a pointwise
///    single-field key too: equal interval values overlap, so the
///    statement forces value-distinctness exactly like a scalar key;
/// 2. a resident image ⇒ the exact build-time count, through the
///    field→column span map (an interval field lower-bounds its pair
///    distincts by the larger of its two word columns — underestimating
///    distincts overestimates rows, the safe direction);
/// 3. schema bounds — a `Containment` whose unselected source projection
///    is exactly this field is bounded by its target relation's row
///    count (the containment domain), an enum by its variant list, a
///    bool by 2;
/// 4. the documented floor.
fn distinct_of(
    txn: &ReadTxn<'_>,
    schema: &Schema,
    relation: crate::schema::RelationId,
    field: FieldId,
    image: Option<&crate::image::RelationImage>,
    rows: u64,
) -> crate::error::Result<u64> {
    let descriptor = schema.relation(relation);
    let keyed = descriptor
        .keys()
        .iter()
        .any(|id| schema.key_projection(*id) == [field]);
    if keyed {
        return Ok(rows.max(1));
    }
    if let Some(image) = image {
        let span = image.span(field);
        let first = usize::from(span.first_column);
        let distinct = match span.width {
            ColumnWidth::Byte | ColumnWidth::Word => image.distinct(first),
            // Multi-word fields: each column's distinct count lower-
            // bounds the tuple's, so the max is the tightest sound
            // estimate one-column counters give (exact tuple distincts
            // stay the sinks' k-word map job, not the planner's).
            ColumnWidth::WordPair | ColumnWidth::Words { .. } => (first
                ..first + usize::from(span.width.column_count()))
                .map(|column| image.distinct(column))
                .max()
                .expect("at least one column"),
        };
        return Ok(distinct.max(1));
    }
    // A field under several unconditional containments is bounded by
    // each target's row count — fold to the tightest (the min), never
    // the first statement's.
    let mut containment_bound: Option<u64> = None;
    for id in descriptor.outgoing() {
        if let StatementDescriptor::Containment { source, target } =
            &schema.statement(*id).descriptor
        {
            if source.projection.as_ref() == [field] && source.selection.is_empty() {
                let target_rows = read::row_count(txn, target.relation)?;
                containment_bound =
                    Some(containment_bound.map_or(target_rows, |bound| bound.min(target_rows)));
            }
        }
    }
    if let Some(bound) = containment_bound {
        return Ok(bound.min(rows).max(1));
    }
    Ok(match &descriptor.field(field).value_type {
        crate::schema::ValueType::Bool => 2,
        crate::schema::ValueType::Enum { variants } => {
            u64::try_from(variants.len()).expect("small").max(1)
        }
        _ => DEFAULT_EQ_DISTINCT,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{encode_fact, ValueRef};
    use crate::image::view::Const;
    use crate::ir::normalize::{OccId, Role};
    use crate::schema::{
        FieldDescriptor, Generation, RelationDescriptor, RelationId, SchemaDescriptor, Side,
        ValueType,
    };
    use crate::storage::commit::commit;
    use crate::storage::delta::WriteDelta;
    use crate::storage::env::Environment;
    use crate::testutil::TempDir;

    /// R(id u64 fresh — auto-key, memo str, kind enum[4]);
    /// S(id u64 fresh, r u64) with the containment S(r) <= R(id).
    fn schema() -> Schema {
        SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "R".into(),
                    fields: vec![
                        FieldDescriptor {
                            name: "id".into(),
                            value_type: ValueType::U64,
                            generation: Generation::Fresh,
                        },
                        FieldDescriptor {
                            name: "memo".into(),
                            value_type: ValueType::String,
                            generation: Generation::None,
                        },
                        FieldDescriptor {
                            name: "kind".into(),
                            value_type: ValueType::Enum {
                                variants: ["A", "B", "C", "D"]
                                    .iter()
                                    .map(|v| Box::from(*v))
                                    .collect(),
                            },
                            generation: Generation::None,
                        },
                    ],
                },
                RelationDescriptor {
                    extension: None,
                    name: "S".into(),
                    fields: vec![
                        FieldDescriptor {
                            name: "id".into(),
                            value_type: ValueType::U64,
                            generation: Generation::Fresh,
                        },
                        FieldDescriptor {
                            name: "r".into(),
                            value_type: ValueType::U64,
                            generation: Generation::None,
                        },
                    ],
                },
            ],
            statements: vec![StatementDescriptor::Containment {
                source: Side {
                    relation: RelationId(1),
                    projection: Box::new([FieldId(1)]),
                    selection: Box::new([]),
                },
                target: Side {
                    relation: RelationId(0),
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([]),
                },
            }],
        }
        .validate()
        .expect("valid fixture")
    }

    const R: RelationId = RelationId(0);
    const S: RelationId = RelationId(1);

    fn populate(env: &Environment, schema: &Schema) {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(schema);
        for i in 0..64u64 {
            let memo = delta
                .intern_str(&view, &format!("m{}", i % 8))
                .expect("intern");
            let mut bytes = Vec::new();
            encode_fact(
                &[
                    ValueRef::U64(i),
                    ValueRef::String(memo),
                    ValueRef::Enum(u8::try_from(i % 4).expect("small")),
                ],
                schema.relation(R).layout(),
                &mut bytes,
            );
            delta.insert(&view, R, &bytes).expect("insert");
        }
        for i in 0..16u64 {
            let mut bytes = Vec::new();
            encode_fact(
                &[ValueRef::U64(i), ValueRef::U64(i % 64)],
                schema.relation(S).layout(),
                &mut bytes,
            );
            delta.insert(&view, S, &bytes).expect("insert");
        }
        drop(view);
        commit(delta, env).expect("commit");
    }

    fn eq_on(field: u16, occ_relation: RelationId) -> Occurrence {
        Occurrence {
            occ_id: OccId(0),
            relation: occ_relation,
            role: Role::Positive,
            vars: vec![],
            filters: vec![FilterPredicate::Compare {
                field: FieldId(field),
                op: CmpOp::Eq,
                value: Const::Param(crate::ir::ParamId(0)),
            }],
        }
    }

    /// The ladder, rung by rung: key ⇒ rows; resident image ⇒ exact;
    /// containment/enum schema bounds when cold; the floor for plain
    /// strings.
    #[test]
    fn the_distinct_ladder_resolves_strongest_first() {
        let dir = TempDir::new("selectivity-ladder");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        populate(&env, &schema);
        let txn = env.read_txn().expect("txn");
        let cache = ImageCache::new(&schema);

        // Keyed (fresh id): estimate = rows / rows = 1, cold or warm.
        let est = occurrence_stats(&txn, &cache, &schema, &eq_on(0, R), 64)
            .expect("estimate")
            .rows;
        assert_eq!(est, 1, "keyed fields select one row");

        // Plain string, cold cache: the floor (64 / 64 = 1)… use more
        // rows to see it: pretend 6400 rows.
        let est = occurrence_stats(&txn, &cache, &schema, &eq_on(1, R), 6400)
            .expect("estimate")
            .rows;
        assert_eq!(
            est,
            6400 / DEFAULT_EQ_DISTINCT,
            "cold string hits the floor"
        );

        // Enum, cold: the variant bound (4).
        let est = occurrence_stats(&txn, &cache, &schema, &eq_on(2, R), 6400)
            .expect("estimate")
            .rows;
        assert_eq!(est, 1600, "cold enum uses the variant bound");

        // Containment source field, cold: bounded by the target's row
        // count (R has 64).
        let est = occurrence_stats(&txn, &cache, &schema, &eq_on(1, S), 1600)
            .expect("estimate")
            .rows;
        assert_eq!(est, 1600 / 64, "cold containment uses the target bound");

        // Warm the cache: exact image distincts displace bounds/floors.
        cache.get_or_build(&txn, &schema, R).expect("build");
        let est = occurrence_stats(&txn, &cache, &schema, &eq_on(1, R), 6400)
            .expect("estimate")
            .rows;
        assert_eq!(est, 6400 / 8, "resident image: 8 distinct memos, exact");
        let est = occurrence_stats(&txn, &cache, &schema, &eq_on(2, R), 6400)
            .expect("estimate")
            .rows;
        assert_eq!(est, 1600, "resident enum count matches the bound here");
    }

    /// Residual fractions and clamping: ranges keep 1/4 each, Ne keeps
    /// all, the repeated in-atom variable keeps 1/64, floor at 1.
    #[test]
    fn residual_fractions_compose_and_clamp() {
        let dir = TempDir::new("selectivity-residuals");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        populate(&env, &schema);
        let txn = env.read_txn().expect("txn");
        let cache = ImageCache::new(&schema);

        let mut occ = eq_on(0, R);
        occ.filters = vec![
            FilterPredicate::Compare {
                field: FieldId(0),
                op: CmpOp::Ge,
                value: Const::Param(crate::ir::ParamId(0)),
            },
            FilterPredicate::Compare {
                field: FieldId(0),
                op: CmpOp::Lt,
                value: Const::Param(crate::ir::ParamId(1)),
            },
            FilterPredicate::Compare {
                field: FieldId(1),
                op: CmpOp::Ne,
                value: Const::Param(crate::ir::ParamId(2)),
            },
        ];
        let est = occurrence_stats(&txn, &cache, &schema, &occ, 1600)
            .expect("estimate")
            .rows;
        assert_eq!(est, 100, "two ranges keep 1/16; Ne keeps everything");

        occ.filters = vec![FilterPredicate::FieldsCompare {
            left: FieldId(0),
            right: FieldId(1),
            op: CmpOp::Eq,
        }];
        let est = occurrence_stats(&txn, &cache, &schema, &occ, 128)
            .expect("estimate")
            .rows;
        assert_eq!(est, 2, "the repeated in-atom variable keeps 1/64");

        // Clamp: estimates never reach zero.
        let est = occurrence_stats(&txn, &cache, &schema, &eq_on(1, R), 3)
            .expect("estimate")
            .rows;
        assert_eq!(est, 1);
    }

    /// A field under TWO unconditional containments takes the tightest
    /// target bound — the min over target row counts, never the first
    /// statement's. Big (64 rows) is declared first; Small (16 rows)
    /// must still win.
    #[test]
    fn the_containment_rung_takes_the_tightest_target_bound() {
        const BIG: RelationId = RelationId(0);
        const SMALL: RelationId = RelationId(1);
        const SRC: RelationId = RelationId(2);
        let fresh_id = || FieldDescriptor {
            name: "id".into(),
            value_type: ValueType::U64,
            generation: Generation::Fresh,
        };
        let side = |relation: u32, field: u16| Side {
            relation: RelationId(relation),
            projection: Box::new([FieldId(field)]),
            selection: Box::new([]),
        };
        let schema = SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "Big".into(),
                    fields: vec![fresh_id()],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Small".into(),
                    fields: vec![fresh_id()],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Src".into(),
                    fields: vec![
                        fresh_id(),
                        FieldDescriptor {
                            name: "r".into(),
                            value_type: ValueType::U64,
                            generation: Generation::None,
                        },
                    ],
                },
            ],
            statements: vec![
                StatementDescriptor::Containment {
                    source: side(2, 1),
                    target: side(0, 0),
                },
                StatementDescriptor::Containment {
                    source: side(2, 1),
                    target: side(1, 0),
                },
            ],
        }
        .validate()
        .expect("valid fixture");

        let dir = TempDir::new("selectivity-tightest-containment");
        let env = Environment::create(dir.path(), &schema).expect("create");
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        let mut put = |rel: RelationId, values: &[ValueRef]| {
            let mut bytes = Vec::new();
            encode_fact(values, schema.relation(rel).layout(), &mut bytes);
            delta.insert(&view, rel, &bytes).expect("insert");
        };
        for i in 0..64u64 {
            put(BIG, &[ValueRef::U64(i)]);
        }
        for i in 0..16u64 {
            put(SMALL, &[ValueRef::U64(i)]);
        }
        for i in 0..8u64 {
            put(SRC, &[ValueRef::U64(i), ValueRef::U64(i)]);
        }
        drop(view);
        commit(delta, &env).expect("commit");

        let txn = env.read_txn().expect("txn");
        let cache = ImageCache::new(&schema);
        let est = occurrence_stats(&txn, &cache, &schema, &eq_on(1, SRC), 1600)
            .expect("estimate")
            .rows;
        assert_eq!(
            est,
            1600 / 16,
            "the min target bound (Small, 16 rows) wins over the first (Big, 64)"
        );
    }

    /// A set-bound position plans as `PARAM_SET_PLANNING_CARDINALITY`
    /// distinct matches instead of one — the small-set assumption
    /// (`docs/architecture/20-query-ir.md`, § prepared queries): a set-Eq
    /// on a keyed field prices at the assumed element count, not 1.
    #[test]
    fn a_set_bound_selection_plans_on_the_small_set_assumption() {
        let dir = TempDir::new("selectivity-paramset");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        populate(&env, &schema);
        let txn = env.read_txn().expect("txn");
        let cache = ImageCache::new(&schema);

        let mut occ = eq_on(0, R);
        occ.filters = vec![FilterPredicate::Compare {
            field: FieldId(0),
            op: CmpOp::Eq,
            value: Const::ParamSet(crate::ir::ParamId(0)),
        }];
        let est = occurrence_stats(&txn, &cache, &schema, &occ, 6400)
            .expect("estimate")
            .rows;
        assert_eq!(
            est, PARAM_SET_PLANNING_CARDINALITY,
            "keyed set-Eq: one row per assumed element"
        );

        // Scalar control: the same position with a scalar param is 1.
        let est = occurrence_stats(&txn, &cache, &schema, &eq_on(0, R), 6400)
            .expect("estimate")
            .rows;
        assert_eq!(est, 1);
    }
}
