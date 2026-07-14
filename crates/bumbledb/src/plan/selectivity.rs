//! Prepare-time cardinality estimation (docs/architecture/40-execution.md): per-occurrence
//! input estimates for the join-order DP and the EXPLAIN/report
//! honesty numbers. Three sources, strongest first — schema structure
//! (free and exact), resident-image exact distinct counts, documented
//! constant floors. Prepare **never builds** an image for statistics
//! (the cache is peeked); a cold prepare degrades to bounds and floors,
//! and runtime cover choice (docs/architecture/40-execution.md) carries the load-bearing
//! decisions either way.

use crate::image::ColumnWidth;
use crate::image::cache::ImageCache;
use crate::image::view::{Const, FilterPredicate};
use crate::ir::CmpOp;
use crate::ir::normalize::Occurrence;
use crate::plan::fj::split_filters;
use crate::plan::planner::OccStats;
use crate::schema::{FieldId, Schema};
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
/// the estimate only orders joins. Membership conditions are fixed
/// word-range compositions over the start/end column pair
/// (docs/architecture/40-execution.md), so they take the same class.
pub(crate) const RANGE_KEEP_DEN: u64 = 4;

/// The Allen basics partition every interval pair (JEPD), so a mask
/// condition's honest keep fraction is `popcount/13` — the mask's measure
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
    // Fields already charged for a folded constant range: the fold
    // (`ir/normalize/fold.rs`) collapsed each slot's constant order
    // filters into ONE `[lo, hi]` summary, emitted back as at most two
    // bounds — one summary is one range condition, so its keep fraction
    // applies once per field, never per constituent. This is the
    // double-counted-range selectivity fix (PRD 10): pre-fold, `x > a ∧
    // x < b` priced as 1/16 instead of 1/4. Param bounds never fold
    // (params are stage-3) and keep the per-filter fraction below.
    let mut folded_range_fields: Vec<FieldId> = Vec::new();
    for residual in &residuals {
        // The Allen kinds carry their own honest fraction ([`allen_keep`]:
        // popcount/13); everything else takes a constant denominator.
        if let FilterPredicate::FieldsAllen { mask, .. }
        | FilterPredicate::FieldAllen { mask, .. } = residual
        {
            estimate = allen_keep(estimate, *mask);
            continue;
        }
        if let FilterPredicate::Compare {
            field,
            op: CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge,
            value: Const::Word(_),
        } = residual
        {
            if folded_range_fields.contains(field) {
                continue;
            }
            folded_range_fields.push(*field);
            estimate = (estimate / RANGE_KEEP_DEN).max(1);
            continue;
        }
        let keep_den = match residual {
            FilterPredicate::Compare { op, .. } => match op {
                CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge => RANGE_KEEP_DEN,
                CmpOp::Ne => 1,
                CmpOp::Eq => unreachable!("split_filters routed Eq into selections"),
                CmpOp::Allen { .. } | CmpOp::PointIn => {
                    unreachable!("interval conditions lower to their fixed shapes")
                }
            },
            FilterPredicate::FieldsCompare { op, .. } => match op {
                CmpOp::Eq => FIELDS_EQ_KEEP_DEN,
                CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge => RANGE_KEEP_DEN,
                CmpOp::Ne => 1,
                CmpOp::Allen { .. } | CmpOp::PointIn => {
                    unreachable!("same-atom interval conditions lower to their fixed shapes")
                }
            },
            // The fixed membership compositions (word ranges over the
            // start/end pair), and the measure comparisons — a range
            // condition over the derived duration word, riding the
            // existing range keep-fraction floor unmodified (20-query-ir § the measure;
            // validation admits order operators only, so the range class
            // is exact, not a default).
            FilterPredicate::PointIn { .. }
            | FilterPredicate::AnyPointIn { .. }
            | FilterPredicate::FieldsPointIn { .. }
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
        .any(|id| schema.key(*id).projection.as_ref() == [field]);
    if keyed {
        return Ok(rows.max(1));
    }
    if let Some(image) = image {
        let span = image.span(field);
        let first = usize::from(span.first_column);
        let distinct = match span.width {
            ColumnWidth::Byte | ColumnWidth::Word => image.cardinality(first),
            // Multi-word fields: each column's distinct count lower-
            // bounds the tuple's, so the max is the tightest sound
            // estimate one-column counters give (exact tuple distincts
            // stay the sinks' k-word map job, not the planner's).
            ColumnWidth::WordPair | ColumnWidth::Words { .. } => (first
                ..first + usize::from(span.width.column_count()))
                .map(|column| image.cardinality(column))
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
        let statement = schema.containment(*id);
        if statement.source.projection.as_ref() == [field] && statement.source.selection.is_empty()
        {
            let target_rows = read::row_count(txn, statement.target.relation)?;
            containment_bound =
                Some(containment_bound.map_or(target_rows, |bound| bound.min(target_rows)));
        }
    }
    if let Some(bound) = containment_bound {
        return Ok(bound.min(rows).max(1));
    }
    Ok(match &descriptor.field(field).value_type {
        crate::schema::ValueType::Bool => 2,
        _ => DEFAULT_EQ_DISTINCT,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{ValueRef, encode_fact};
    use crate::image::view::Const;
    use crate::ir::normalize::{OccId, Role};
    use crate::schema::{
        FieldDescriptor, Generation, RelationDescriptor, RelationId, SchemaDescriptor, Side,
        StatementDescriptor, ValueType,
    };
    use crate::storage::commit::commit;
    use crate::storage::delta::WriteDelta;
    use crate::storage::env::Environment;
    use crate::testutil::TempDir;

    /// R(id u64 fresh — auto-key, memo str, kind u64 over 4 values);
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
                            value_type: ValueType::U64,
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
                    ValueRef::U64(i % 4),
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
    /// containment schema bounds when cold; the floor for plain
    /// strings and u64s.
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

        // Plain u64, cold: the same floor — no schema bound applies.
        let est = occurrence_stats(&txn, &cache, &schema, &eq_on(2, R), 6400)
            .expect("estimate")
            .rows;
        assert_eq!(est, 6400 / DEFAULT_EQ_DISTINCT, "cold u64 hits the floor");

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
        assert_eq!(est, 1600, "resident image: 4 distinct kinds, exact");
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

    /// A folded constant range counts ONCE: the fold
    /// (`ir/normalize/fold.rs`) collapsed the slot's constant bounds
    /// into one `[lo, hi]` summary, so the two emitted bounds are one
    /// range condition — 1/4, never 1/16 (the double-counted-range fix,
    /// PRD 10). Constant ranges on distinct fields still compose, and
    /// param bounds (which never fold) keep the per-filter fraction —
    /// `residual_fractions_compose_and_clamp` above pins that side.
    #[test]
    fn a_folded_constant_range_takes_the_keep_fraction_once() {
        let dir = TempDir::new("selectivity-folded-range");
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
                value: Const::Word(8),
            },
            FilterPredicate::Compare {
                field: FieldId(0),
                op: CmpOp::Le,
                value: Const::Word(19),
            },
        ];
        let est = occurrence_stats(&txn, &cache, &schema, &occ, 1600)
            .expect("estimate")
            .rows;
        assert_eq!(est, 400, "one summary, one 1/4 — not 1/16");

        // Distinct fields are distinct summaries and still compose.
        occ.filters.push(FilterPredicate::Compare {
            field: FieldId(2),
            op: CmpOp::Lt,
            value: Const::Word(3),
        });
        let est = occurrence_stats(&txn, &cache, &schema, &occ, 1600)
            .expect("estimate")
            .rows;
        assert_eq!(est, 100, "two fields, two fractions");
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

    const CYCLE_VOCAB: RelationId = RelationId(0);
    const CYCLE_A: RelationId = RelationId(1);
    const CYCLE_B: RelationId = RelationId(2);
    const CYCLE_C: RelationId = RelationId(3);

    fn cyclic_schema() -> Schema {
        use crate::schema::Row;

        let field = |name: &str| FieldDescriptor {
            name: name.into(),
            value_type: ValueType::U64,
            generation: Generation::None,
        };
        let side = |relation: RelationId, projection: &[u16]| Side {
            relation,
            projection: projection.iter().copied().map(FieldId).collect(),
            selection: Box::new([]),
        };
        SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: Some(Box::new([
                        Row {
                            handle: "X0".into(),
                            values: Box::new([]),
                        },
                        Row {
                            handle: "X1".into(),
                            values: Box::new([]),
                        },
                        Row {
                            handle: "X2".into(),
                            values: Box::new([]),
                        },
                    ])),
                    name: "X".into(),
                    fields: vec![],
                },
                RelationDescriptor {
                    extension: None,
                    name: "A".into(),
                    fields: vec![field("x"), field("y")],
                },
                RelationDescriptor {
                    extension: None,
                    name: "B".into(),
                    fields: vec![field("y"), field("z")],
                },
                RelationDescriptor {
                    extension: None,
                    name: "C".into(),
                    fields: vec![field("z"), field("x")],
                },
            ],
            statements: vec![
                StatementDescriptor::Containment {
                    source: side(CYCLE_A, &[0]),
                    target: side(CYCLE_VOCAB, &[0]),
                },
                StatementDescriptor::Containment {
                    source: side(CYCLE_C, &[1]),
                    target: side(CYCLE_VOCAB, &[0]),
                },
            ],
        }
        .validate()
        .expect("valid cyclic fixture")
    }

    fn populate_cycle(env: &Environment, schema: &Schema) {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(schema);
        {
            let mut insert = |relation: RelationId, values: &[u64]| {
                let values: Vec<ValueRef> = values.iter().copied().map(ValueRef::U64).collect();
                let mut bytes = Vec::new();
                encode_fact(&values, schema.relation(relation).layout(), &mut bytes);
                delta.insert(&view, relation, &bytes).expect("insert");
            };
            for x in 0..3 {
                for y in 0..8 {
                    insert(CYCLE_A, &[x, y]);
                }
            }
            for y in 0..8 {
                for z in 0..8 {
                    insert(CYCLE_B, &[y, z]);
                }
            }
            for z in 0..8 {
                for x in 0..3 {
                    insert(CYCLE_C, &[z, x]);
                }
            }
        }
        drop(view);
        commit(delta, env).expect("commit");
    }

    fn cyclic_query(finds: Vec<crate::ir::FindTerm>) -> crate::ir::Query {
        use crate::ir::{Atom, Query, Rule, Term, VarId};

        Query::single(Rule {
            finds,
            atoms: vec![
                Atom {
                    relation: CYCLE_A,
                    bindings: vec![
                        (FieldId(0), Term::Var(VarId(0))),
                        (FieldId(1), Term::Var(VarId(1))),
                    ],
                },
                Atom {
                    relation: CYCLE_B,
                    bindings: vec![
                        (FieldId(0), Term::Var(VarId(1))),
                        (FieldId(1), Term::Var(VarId(2))),
                    ],
                },
                Atom {
                    relation: CYCLE_C,
                    bindings: vec![
                        (FieldId(0), Term::Var(VarId(2))),
                        (FieldId(1), Term::Var(VarId(0))),
                    ],
                },
            ],
            negated: vec![],
            conditions: vec![],
        })
    }

    fn cyclic_profile(
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        schema: &Schema,
        finds: Vec<crate::ir::FindTerm>,
    ) -> crate::api::stats::ExecutionStats {
        use crate::api::prepared::{PreparedQuery, prepare};

        let mut prepared: PreparedQuery<'_, ()> =
            prepare(txn, cache, schema, &cyclic_query(finds)).expect("prepare cycle");
        prepared.profile(txn, cache, &[]).expect("profile cycle").1
    }

    /// P3 diagnosis: a three-edge cycle with exact resident distincts and a
    /// three-row closed vocabulary on `x`. The full head shows the inherent
    /// independence error at the closing two-variable probe; the narrow head
    /// additionally shows that EXPLAIN's final-node `actual` is emitted set
    /// witnesses after D2 cancellation, not the cycle's full binding count.
    /// P1 is pinned by the closed-domain fanout, and P2 is absent by
    /// construction (there are no range conditions).
    #[test]
    fn cyclic_estimate_diagnosis_is_p3_not_a_domain_or_range_defect() {
        use crate::ir::{FindTerm, VarId};

        let dir = TempDir::new("selectivity-p3-cycle");
        let schema = cyclic_schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        populate_cycle(&env, &schema);
        let cache = ImageCache::new(&schema);
        let txn = env.read_txn().expect("txn");
        for relation in [CYCLE_A, CYCLE_B, CYCLE_C] {
            cache
                .get_or_build(&txn, &schema, relation)
                .expect("resident exact distincts");
        }

        let full_stats = cyclic_profile(
            &txn,
            &cache,
            &schema,
            vec![
                FindTerm::Var(VarId(0)),
                FindTerm::Var(VarId(1)),
                FindTerm::Var(VarId(2)),
            ],
        );
        let full_pairs: Vec<_> = full_stats.rules[0]
            .nodes
            .iter()
            .map(|node| (node.estimate, node.actual))
            .collect();
        assert_eq!(
            full_pairs,
            vec![(24, 24), (192, 192), (576, 192)],
            "P3 cyclic-join independence: the closing two-variable probe uses its best one-column fanout 3 instead of pair fanout 1; P1's three-row closed domain is applied and P2 is absent"
        );

        let narrow_stats = cyclic_profile(&txn, &cache, &schema, vec![FindTerm::Var(VarId(0))]);
        let narrow_pairs: Vec<_> = narrow_stats.rules[0]
            .nodes
            .iter()
            .map(|node| (node.estimate, node.actual))
            .collect();
        assert_eq!(
            narrow_pairs,
            vec![(24, 24), (192, 24), (576, 24)],
            "P3 report population: D2 emits one set witness per root origin, so final est/actual is not a cardinality-accuracy bound"
        );
        assert_eq!(
            narrow_stats.rules[0].absorbed, 21,
            "24 emits collapse to 3 x rows"
        );
    }
}
