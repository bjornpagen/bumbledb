//! Prepare-time row-count estimation: per-occurrence
//! input estimates for the join-order DP and the introspection/report
//! honesty numbers. Three sources, strongest first — schema structure
//! (free and exact), resident-image exact distinct counts, documented
//! constant floors. Prepare **never builds** an image for statistics
//! (the cache is peeked); a cold prepare degrades to bounds and floors,
use crate::image::ColumnWidth;
use crate::image::ImageBind;
#[cfg(test)]
use crate::image::LmdbSource;
#[cfg(test)]
use crate::image::cache::ImageCache;
use crate::image::view::{Const, FilterPredicate};
use crate::ir::WordCmp;
use crate::ir::normalize::Occurrence;
use crate::plan::fj::OccBind;
use crate::plan::fj::split_filters;
use crate::plan::planner::OccStats;
use crate::schema::Schema;
use crate::storage::catalog::CatalogRead;
#[cfg(test)]
use crate::storage::env::ReadTxn;
use bumbledb_theory::schema::FieldId;

/// A closed relation's rows ARE its sealed extension — the option is the kind
/// (`schema/relation.rs`) — and its stored `S` counter never exists (closed
/// relations are storage-virtual and write-refused), so a raw counter read
/// prices it at 0.
/// # Errors
pub(crate) fn relation_rows_on<C: CatalogRead>(
    catalog: &C,
    schema: &Schema,
    relation: bumbledb_theory::schema::RelationId,
) -> crate::error::Result<u64> {
    let rows = match schema.relation(relation).body().closed_rows() {
        Some(rows) => u64::try_from(rows.len()).expect("bounded extension"),
        None => catalog.row_count(relation)?,
    };
    crate::obs::event(
        crate::obs::names::RELATION_ROWS,
        crate::obs::TraceArgs::Pair(u64::from(relation.0), rows),
    );
    Ok(rows)
}

pub(crate) const DEFAULT_EQ_DISTINCT: u64 = 64;

pub(crate) const RANGE_KEEP_DEN: u64 = 4;

fn allen_keep(estimate: u64, mask: crate::image::view::MaskConst) -> u64 {
    (estimate.saturating_mul(u64::from(mask.popcount())) / 13).max(1)
}

pub(crate) const FIELDS_EQ_KEEP_DEN: u64 = 64;

pub(crate) const PARAM_SET_PLANNING_ROWS: u64 = 16;

pub(crate) const DELTA_PLANNING_ROWS: u64 = 1;

pub(crate) const ACCUMULATED_PLANNING_ROWS: u64 = 16;

/// # Errors
#[cfg(test)]
pub(crate) fn occurrence_stats(
    txn: &ReadTxn<'_>,
    cache: &ImageCache,
    schema: &Schema,
    occurrence: &Occurrence,
    rows: u64,
) -> crate::error::Result<OccStats> {
    occurrence_stats_on(
        &txn.catalog(),
        &LmdbSource::bind(txn, cache),
        schema,
        occurrence,
        rows,
    )
}

pub(crate) fn occurrence_stats_on<C: CatalogRead, I: ImageBind>(
    catalog: &C,
    images: &I,
    schema: &Schema,
    occurrence: &Occurrence,
    rows: u64,
) -> crate::error::Result<OccStats> {
    match OccBind::of_occurrence(occurrence) {
        OccBind::RecDelta(_) => {
            let floor = DELTA_PLANNING_ROWS.max(1);
            Ok(OccStats {
                occ_id: occurrence.occ_id,
                rows: floor,
                var_distincts: occurrence
                    .vars
                    .iter()
                    .map(|(_, var)| (*var, floor))
                    .collect(),
            })
        }
        OccBind::Finished(_) | OccBind::RecAcc(_) => {
            let floor = ACCUMULATED_PLANNING_ROWS.max(1);
            Ok(OccStats {
                occ_id: occurrence.occ_id,
                rows: floor,
                var_distincts: occurrence
                    .vars
                    .iter()
                    .map(|(_, var)| (*var, floor))
                    .collect(),
            })
        }
        OccBind::Edb(relation) => {
            let image = images.peek(schema, relation)?;
            let mut var_distincts = Vec::with_capacity(occurrence.vars.len());
            for (field, var) in &occurrence.vars {
                let distinct =
                    distinct_of(catalog, schema, relation, *field, image.as_deref(), rows)?;
                var_distincts.push((*var, distinct));
            }
            let estimate = occurrence_estimate(
                catalog,
                schema,
                occurrence,
                relation,
                image.as_deref(),
                rows,
            )?;
            Ok(OccStats {
                occ_id: occurrence.occ_id,
                rows: estimate,
                var_distincts,
            })
        }
    }
}

fn selection_matches(value: &Const) -> u64 {
    match value {
        Const::ParamSet(_) => PARAM_SET_PLANNING_ROWS,
        Const::WordSet(words) => u64::try_from(words.len()).expect("bounded set").max(1),
        _ => 1,
    }
}

fn occurrence_estimate<C: CatalogRead>(
    catalog: &C,
    schema: &Schema,
    occurrence: &Occurrence,
    relation: bumbledb_theory::schema::RelationId,
    image: Option<&crate::image::RelationImage>,
    rows: u64,
) -> crate::error::Result<u64> {
    let (selections, residuals) = split_filters(&occurrence.filters);
    let mut estimate = rows;
    for selection in &selections {
        let distinct = distinct_of(catalog, schema, relation, selection.field, image, rows)?;
        estimate =
            (estimate.saturating_mul(selection_matches(&selection.value)) / distinct.max(1)).max(1);
    }

    let mut folded_range_fields: Vec<FieldId> = Vec::new();
    for residual in &residuals {
        if let FilterPredicate::FieldsAllen { mask, .. }
        | FilterPredicate::FieldAllen { mask, .. } = residual
        {
            estimate = allen_keep(estimate, *mask);
            continue;
        }

        if let FilterPredicate::Compare {
            field,
            op: WordCmp::Eq,
            value,
        } = residual
        {
            let distinct = distinct_of(catalog, schema, relation, field.field(), image, rows)?;
            estimate = (estimate.saturating_mul(selection_matches(value)) / distinct.max(1)).max(1);
            continue;
        }
        if let FilterPredicate::Compare {
            field,
            op: WordCmp::Lt | WordCmp::Le | WordCmp::Gt | WordCmp::Ge,
            value: Const::Word(_),
        } = residual
        {
            if folded_range_fields.contains(&field.field()) {
                continue;
            }
            folded_range_fields.push(field.field());
            estimate = (estimate / RANGE_KEEP_DEN).max(1);
            continue;
        }
        let keep_den = match residual {
            FilterPredicate::Compare { op, .. } => match op {
                WordCmp::Lt | WordCmp::Le | WordCmp::Gt | WordCmp::Ge => RANGE_KEEP_DEN,
                WordCmp::Ne => 1,
                WordCmp::Eq => unreachable!("Eq residuals priced above"),
            },
            FilterPredicate::FieldsCompare { op, .. } => match op {
                WordCmp::Eq => FIELDS_EQ_KEEP_DEN,
                WordCmp::Lt | WordCmp::Le | WordCmp::Gt | WordCmp::Ge => RANGE_KEEP_DEN,
                WordCmp::Ne => 1,
            },

            FilterPredicate::PointIn { .. }
            | FilterPredicate::AnyPointIn { .. }
            | FilterPredicate::FieldsPointIn { .. }
            | FilterPredicate::FieldWithin { .. } => RANGE_KEEP_DEN,
            FilterPredicate::FieldsAllen { .. } | FilterPredicate::FieldAllen { .. } => {
                unreachable!("handled above")
            }
        };
        estimate = (estimate / keep_den).max(1);

        if matches!(residual, FilterPredicate::AnyPointIn { .. }) {
            estimate = estimate.saturating_mul(PARAM_SET_PLANNING_ROWS);
        }
    }
    Ok(estimate.clamp(1, rows.max(1)))
}

fn distinct_of<C: CatalogRead>(
    catalog: &C,
    schema: &Schema,
    relation: bumbledb_theory::schema::RelationId,
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
        let distinct = rows.max(1);
        ladder_event(0, distinct);
        return Ok(distinct);
    }
    if let Some(image) = image {
        let span = image.span(field);
        let first = usize::from(span.first_column);
        let distinct = match span.width {
            ColumnWidth::Byte | ColumnWidth::Word => image.distinct_count(first),

            ColumnWidth::WordPair | ColumnWidth::Words { .. } => (first
                ..first + usize::from(span.width.column_count()))
                .map(|column| image.distinct_count(column))
                .max()
                .expect("at least one column"),
        };
        let distinct = distinct.max(1);
        ladder_event(1, distinct);
        return Ok(distinct);
    }

    let mut containment_bound: Option<u64> = None;
    for id in descriptor.outgoing() {
        let statement = schema.containment(*id);
        if statement.source.projection.as_ref() == [field] && statement.source.selection.is_empty()
        {
            let target_rows = relation_rows_on(catalog, schema, statement.target.relation)?;
            containment_bound =
                Some(containment_bound.map_or(target_rows, |bound| bound.min(target_rows)));
        }
    }
    if let Some(bound) = containment_bound {
        let distinct = bound.min(rows).max(1);
        ladder_event(2, distinct);
        return Ok(distinct);
    }
    let distinct = match &descriptor.field(field).value_type {
        bumbledb_theory::schema::ValueType::Bool => 2,
        _ => DEFAULT_EQ_DISTINCT,
    };
    ladder_event(3, distinct);
    Ok(distinct)
}

#[inline]
fn ladder_event(rung: u64, distinct: u64) {
    crate::obs::event(
        crate::obs::names::DISTINCT_LADDER,
        crate::obs::TraceArgs::Pair(rung, distinct),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{ValueRef, encode_fact};
    use crate::image::view::Const;
    use crate::ir::normalize::{OccBind, OccId, Role};
    use crate::schema::ValidateDescriptor as _;
    use crate::storage::commit::commit;
    use crate::storage::delta::WriteDelta;
    use crate::storage::env::Environment;
    use crate::testutil::TempDir;
    use bumbledb_theory::schema::{
        FieldDescriptor, Generation, RelationDescriptor, RelationId, SchemaDescriptor, Side,
        StatementDescriptor, ValueType,
    };

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
        commit(delta, env).expect("commit").expect("admitted");
    }

    fn eq_on(field: u16, occ_relation: RelationId) -> Occurrence {
        Occurrence {
            occ_id: OccId(0),
            bind: OccBind::Edb(occ_relation),
            role: Role::Positive,
            vars: vec![],
            filters: vec![FilterPredicate::Compare {
                field: FieldId(field).into(),
                op: WordCmp::Eq,
                value: Const::Param(crate::ir::ParamId(0)),
            }],
            point_vars: vec![],
        }
    }

    #[test]
    fn the_distinct_ladder_resolves_strongest_first() {
        let dir = TempDir::new("selectivity-ladder");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        populate(&env, &schema);
        let txn = env.read_txn().expect("txn");
        let cache = ImageCache::new(&schema);

        let est = occurrence_stats(&txn, &cache, &schema, &eq_on(0, R), 64)
            .expect("estimate")
            .rows;
        assert_eq!(est, 1, "keyed fields select one row");

        let est = occurrence_stats(&txn, &cache, &schema, &eq_on(1, R), 6400)
            .expect("estimate")
            .rows;
        assert_eq!(
            est,
            6400 / DEFAULT_EQ_DISTINCT,
            "cold string hits the floor"
        );

        let est = occurrence_stats(&txn, &cache, &schema, &eq_on(2, R), 6400)
            .expect("estimate")
            .rows;
        assert_eq!(est, 6400 / DEFAULT_EQ_DISTINCT, "cold u64 hits the floor");

        let est = occurrence_stats(&txn, &cache, &schema, &eq_on(1, S), 1600)
            .expect("estimate")
            .rows;
        assert_eq!(est, 1600 / 64, "cold containment uses the target bound");

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
                field: FieldId(0).into(),
                op: WordCmp::Ge,
                value: Const::Param(crate::ir::ParamId(0)),
            },
            FilterPredicate::Compare {
                field: FieldId(0).into(),
                op: WordCmp::Lt,
                value: Const::Param(crate::ir::ParamId(1)),
            },
            FilterPredicate::Compare {
                field: FieldId(1).into(),
                op: WordCmp::Ne,
                value: Const::Param(crate::ir::ParamId(2)),
            },
        ];
        let est = occurrence_stats(&txn, &cache, &schema, &occ, 1600)
            .expect("estimate")
            .rows;
        assert_eq!(est, 100, "two ranges keep 1/16; Ne keeps everything");

        occ.filters = vec![FilterPredicate::FieldsCompare {
            left: FieldId(0).into(),
            right: FieldId(1).into(),
            op: WordCmp::Eq,
        }];
        let est = occurrence_stats(&txn, &cache, &schema, &occ, 128)
            .expect("estimate")
            .rows;
        assert_eq!(est, 2, "the repeated in-atom variable keeps 1/64");

        let est = occurrence_stats(&txn, &cache, &schema, &eq_on(1, R), 3)
            .expect("estimate")
            .rows;
        assert_eq!(est, 1);
    }

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
                field: FieldId(0).into(),
                op: WordCmp::Ge,
                value: Const::Word(8),
            },
            FilterPredicate::Compare {
                field: FieldId(0).into(),
                op: WordCmp::Le,
                value: Const::Word(19),
            },
        ];
        let est = occurrence_stats(&txn, &cache, &schema, &occ, 1600)
            .expect("estimate")
            .rows;
        assert_eq!(est, 400, "one summary, one 1/4 — not 1/16");

        occ.filters.push(FilterPredicate::Compare {
            field: FieldId(2).into(),
            op: WordCmp::Lt,
            value: Const::Word(3),
        });
        let est = occurrence_stats(&txn, &cache, &schema, &occ, 1600)
            .expect("estimate")
            .rows;
        assert_eq!(est, 100, "two fields, two fractions");
    }

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
        commit(delta, &env).expect("commit").expect("admitted");

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

    #[test]
    fn the_containment_rung_reads_a_closed_targets_sealed_extension() {
        let dir = TempDir::new("selectivity-closed-target");
        let schema = cyclic_schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let txn = env.read_txn().expect("txn");
        let cache = ImageCache::new(&schema);

        let est = occurrence_stats(&txn, &cache, &schema, &eq_on(0, CYCLE_A), 1500)
            .expect("estimate")
            .rows;
        assert_eq!(est, 500, "the sealed extension is the containment bound");
    }

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
            field: FieldId(0).into(),
            op: WordCmp::Eq,
            value: Const::ParamSet(crate::ir::ParamId(0)),
        }];
        let est = occurrence_stats(&txn, &cache, &schema, &occ, 6400)
            .expect("estimate")
            .rows;
        assert_eq!(
            est, PARAM_SET_PLANNING_ROWS,
            "keyed set-Eq: one row per assumed element"
        );

        let est = occurrence_stats(&txn, &cache, &schema, &eq_on(0, R), 6400)
            .expect("estimate")
            .rows;
        assert_eq!(est, 1);
    }

    const CYCLE_VOCAB: RelationId = RelationId(0);
    const CYCLE_A: RelationId = RelationId(1);
    const _CYCLE_B: RelationId = RelationId(2);
    const CYCLE_C: RelationId = RelationId(3);

    fn cyclic_schema() -> Schema {
        use bumbledb_theory::schema::Row;

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
}
