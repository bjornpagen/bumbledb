//! Prepare-time cardinality estimation (docs/architecture/30-execution.md): per-occurrence
//! input estimates for the join-order DP and the EXPLAIN/report
//! honesty numbers. Three sources, strongest first — schema structure
//! (free and exact), resident-image exact distinct counts, documented
//! constant floors. Prepare **never builds** an image for statistics
//! (the cache is peeked); a cold prepare degrades to bounds and floors,
//! and runtime cover choice (docs/architecture/30-execution.md) carries the load-bearing
//! decisions either way.

use crate::image::cache::ImageCache;
use crate::image::view::FilterPredicate;
use crate::ir::normalize::Occurrence;
use crate::ir::CmpOp;
use crate::plan::fj::split_filters;
use crate::plan::planner::OccStats;
use crate::schema::{ConstraintDescriptor, FieldId, Schema, ValueType};
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
/// the estimate only orders joins.
pub(crate) const RANGE_KEEP_DEN: u64 = 4;

/// A same-fact field equality (`FieldsCompare` under `Eq`, the repeated
/// in-atom variable) keeps `1/64` — same floor class as an Eq selection.
pub(crate) const FIELDS_EQ_KEEP_DEN: u64 = 64;

/// One occurrence's planner statistics: the cardinality estimate —
/// `rows` divided by each Eq selection's distinct count and each
/// residual's keep fraction, clamped to `[1, rows]` (`Ne` keeps
/// everything) — plus every bound variable's base-relation distinct
/// count for the join-step fanout model.
///
/// # Errors
///
/// `Lmdb` from counter reads (FK target row counts, the cache peek).
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
        estimate = (estimate / distinct.max(1)).max(1);
    }
    for residual in &residuals {
        let keep_den = match residual {
            FilterPredicate::Compare { op, .. } => match op {
                CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge => RANGE_KEEP_DEN,
                CmpOp::Ne => 1,
                CmpOp::Eq => unreachable!("split_filters routed Eq into selections"),
            },
            FilterPredicate::FieldsCompare { op, .. } => match op {
                CmpOp::Eq => FIELDS_EQ_KEEP_DEN,
                CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge => RANGE_KEEP_DEN,
                CmpOp::Ne => 1,
            },
        };
        estimate = (estimate / keep_den).max(1);
    }
    Ok(estimate.clamp(1, rows.max(1)))
}

/// The distinct-count ladder for one field, strongest source first:
/// 1. a single-field unique constraint ⇒ exactly `rows`;
/// 2. a resident image ⇒ the exact build-time count;
/// 3. schema bounds — a single-field FK is bounded by its target's row
///    count, an enum by its variant list, a bool by 2;
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
    let unique = descriptor.constraints().iter().any(
        |c| matches!(c, ConstraintDescriptor::Unique { fields, .. } if fields.as_ref() == [field]),
    );
    if unique {
        return Ok(rows.max(1));
    }
    if let Some(image) = image {
        return Ok(image.distinct(usize::from(field.0)).max(1));
    }
    for constraint in descriptor.constraints() {
        if let ConstraintDescriptor::ForeignKey {
            fields,
            target_relation,
            ..
        } = constraint
        {
            if fields.as_ref() == [field] {
                let target_rows = read::row_count(txn, *target_relation)?;
                return Ok(target_rows.min(rows).max(1));
            }
        }
    }
    Ok(match &descriptor.field(field).value_type {
        ValueType::Bool => 2,
        ValueType::Enum { variants } => u64::try_from(variants.len()).expect("small").max(1),
        _ => DEFAULT_EQ_DISTINCT,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{encode_fact, ValueRef};
    use crate::image::view::Const;
    use crate::ir::normalize::OccId;
    use crate::schema::{
        FieldDescriptor, Generation, RelationDescriptor, RelationId, SchemaDescriptor,
    };
    use crate::storage::commit::commit;
    use crate::storage::delta::WriteDelta;
    use crate::storage::env::Environment;
    use crate::testutil::TempDir;

    /// R(id u64 serial+unique, memo str, kind enum[4]);
    /// S(id u64 serial, r u64 fk -> R.id).
    fn schema() -> Schema {
        SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    name: "R".into(),
                    fields: vec![
                        FieldDescriptor {
                            name: "id".into(),
                            value_type: crate::schema::ValueType::U64,
                            generation: Generation::Serial,
                        },
                        FieldDescriptor {
                            name: "memo".into(),
                            value_type: crate::schema::ValueType::String,
                            generation: Generation::None,
                        },
                        FieldDescriptor {
                            name: "kind".into(),
                            value_type: crate::schema::ValueType::Enum {
                                variants: ["A", "B", "C", "D"]
                                    .iter()
                                    .map(|v| Box::from(*v))
                                    .collect(),
                            },
                            generation: Generation::None,
                        },
                    ],
                    constraints: vec![],
                },
                RelationDescriptor {
                    name: "S".into(),
                    fields: vec![
                        FieldDescriptor {
                            name: "id".into(),
                            value_type: crate::schema::ValueType::U64,
                            generation: Generation::Serial,
                        },
                        FieldDescriptor {
                            name: "r".into(),
                            value_type: crate::schema::ValueType::U64,
                            generation: Generation::None,
                        },
                    ],
                    constraints: vec![ConstraintDescriptor::ForeignKey {
                        name: "s_r".into(),
                        fields: Box::new([FieldId(1)]),
                        target_relation: RelationId(0),
                        target_constraint: crate::schema::ConstraintId(0),
                    }],
                },
            ],
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
            vars: vec![],
            filters: vec![FilterPredicate::Compare {
                field: FieldId(field),
                op: CmpOp::Eq,
                value: Const::Param(crate::ir::ParamId(0)),
            }],
        }
    }

    /// The ladder, rung by rung: unique ⇒ rows; resident image ⇒ exact;
    /// FK/enum schema bounds when cold; the floor for plain strings.
    #[test]
    fn the_distinct_ladder_resolves_strongest_first() {
        let dir = TempDir::new("selectivity-ladder");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        populate(&env, &schema);
        let txn = env.read_txn().expect("txn");
        let cache = ImageCache::new();

        // Unique (serial id): estimate = rows / rows = 1, cold or warm.
        let est = occurrence_stats(&txn, &cache, &schema, &eq_on(0, R), 64)
            .expect("estimate")
            .rows;
        assert_eq!(est, 1, "unique fields select one row");

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

        // FK field, cold: bounded by the target's row count (R has 64).
        let est = occurrence_stats(&txn, &cache, &schema, &eq_on(1, S), 1600)
            .expect("estimate")
            .rows;
        assert_eq!(est, 1600 / 64, "cold FK uses the target bound");

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
        let cache = ImageCache::new();

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
}
