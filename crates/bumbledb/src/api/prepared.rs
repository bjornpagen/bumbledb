//! Prepared queries, parameters, and results (docs/architecture/30-execution.md) — the reusable
//! execution object the allocation contract is written against
//! (`docs/architecture/20-query-ir.md`, `30-execution.md`, `60-api.md`).
//!
//! `prepare` runs the whole pipeline once: validate → normalize →
//! filtered-view statistics → plan → classify. **Plans pin the statistics
//! read at prepare time and are never invalidated by writes**; stale plans
//! are accepted at this scale and re-preparation is explicit. Execution
//! resolves `PendingIntern` constants per execution by read-only dictionary
//! lookup — a miss means the query is empty on this snapshot, never an
//! error, and a value interned by a later write is picked up on the next
//! execution.

use crate::api::stats::ExecutionStats;
use crate::error::{Error, Result};
use crate::exec::colt::Colt;
use crate::exec::dispatch::{classify, execute_guard, ExecPlan};
use crate::exec::explain::{CountingCounters, Report};
use crate::exec::run::{Bindings, Executor, NoopCounters, Sink};
use crate::exec::sink::{AggregateSink, FindSpec, ProjectionSink};
use crate::image::cache::ImageCache;
use crate::image::view::{apply, Const, FilterPredicate, View};
use crate::ir::normalize::normalize;
use crate::ir::validate::validate;
use crate::ir::{AggOp, FindTerm, ParamId, Query, Value};
use crate::obs;
use crate::plan::fj::{binary2fj, factor};
use crate::plan::planner::{plan as plan_order, OccStats};
use crate::schema::{Schema, ValueType};
use crate::storage::dict;
use crate::storage::env::ReadTxn;
use crate::storage::read;

/// One decoded output cell, borrowed from a [`ResultBuffer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultValue<'a> {
    Bool(bool),
    U64(u64),
    I64(i64),
    /// Declaration-order ordinal.
    Enum(u8),
    String(&'a str),
    Bytes(&'a [u8]),
}

/// One stored cell: fixed-width values inline, String/Bytes as ranges into
/// the buffer's byte heap.
#[derive(Debug, Clone, Copy)]
enum Cell {
    Bool(bool),
    U64(u64),
    I64(i64),
    Enum(u8),
    String { start: usize, len: usize },
    Bytes { start: usize, len: usize },
}

/// The caller-owned, reusable result buffer: columns are the find terms in
/// order; rows are unordered (results are sets — the host sorts). The byte
/// heap is the single sanctioned allocation site of a warm execution, and
/// `clear` retains every capacity.
#[derive(Debug, Default)]
pub struct ResultBuffer {
    arity: usize,
    cells: Vec<Cell>,
    bytes: Vec<u8>,
}

impl ResultBuffer {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Empties the buffer, retaining capacity (the zero-alloc reuse path).
    pub fn clear(&mut self) {
        self.cells.clear();
        self.bytes.clear();
    }

    /// Number of result rows.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cells.len().checked_div(self.arity).unwrap_or(0)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Number of columns (find terms).
    #[must_use]
    pub fn arity(&self) -> usize {
        self.arity
    }

    /// The value at `(row, column)`.
    ///
    /// # Panics
    ///
    /// On out-of-range coordinates, and on a programmer-invariant violation
    /// (string cells are UTF-8-validated at materialization).
    #[must_use]
    pub fn get(&self, row: usize, column: usize) -> ResultValue<'_> {
        assert!(column < self.arity && row < self.len());
        match self.cells[row * self.arity + column] {
            Cell::Bool(v) => ResultValue::Bool(v),
            Cell::U64(v) => ResultValue::U64(v),
            Cell::I64(v) => ResultValue::I64(v),
            Cell::Enum(v) => ResultValue::Enum(v),
            Cell::String { start, len } => ResultValue::String(
                std::str::from_utf8(&self.bytes[start..start + len])
                    .expect("validated at materialization"),
            ),
            Cell::Bytes { start, len } => ResultValue::Bytes(&self.bytes[start..start + len]),
        }
    }

    /// Iterates the rows. Order is arbitrary (results are sets — the
    /// host sorts); the iterator exists so consumers stop hand-writing
    /// the index arithmetic around [`ResultBuffer::get`].
    pub fn rows(&self) -> impl Iterator<Item = Row<'_>> {
        (0..self.len()).map(move |row| Row { buffer: self, row })
    }

    fn push_word(&mut self, txn: &ReadTxn<'_>, ty: &ValueType, word: u64) -> Result<()> {
        let cell = match ty {
            ValueType::Bool => Cell::Bool(word != 0),
            ValueType::Enum { .. } => Cell::Enum(
                // Programmer invariant, not corruption: image build
                // range-checked every stored ordinal against the schema.
                u8::try_from(word).expect("enum words fit u8"),
            ),
            ValueType::U64 => Cell::U64(word),
            ValueType::I64 => Cell::I64((word ^ (1 << 63)).cast_signed()),
            ValueType::String => {
                let raw = dict::resolve(txn, word, dict::TAG_STRING)?;
                std::str::from_utf8(raw).map_err(|_| {
                    Error::Corruption(crate::error::CorruptionError::NonUtf8Intern(word))
                })?;
                let start = self.bytes.len();
                self.bytes.extend_from_slice(raw);
                Cell::String {
                    start,
                    len: raw.len(),
                }
            }
            ValueType::Bytes => {
                let raw = dict::resolve(txn, word, dict::TAG_BYTES)?;
                let start = self.bytes.len();
                self.bytes.extend_from_slice(raw);
                Cell::Bytes {
                    start,
                    len: raw.len(),
                }
            }
        };
        self.cells.push(cell);
        Ok(())
    }
}

/// One result row, borrowed from a [`ResultBuffer`].
#[derive(Clone, Copy)]
pub struct Row<'a> {
    buffer: &'a ResultBuffer,
    row: usize,
}

impl<'a> Row<'a> {
    /// The value in `column` (a find-term index).
    ///
    /// # Panics
    ///
    /// On an out-of-range column.
    #[must_use]
    pub fn get(&self, column: usize) -> ResultValue<'a> {
        self.buffer.get(self.row, column)
    }
}

/// The reusable execution object. `!Sync` by construction (interior
/// scratch); executes from one thread at a time; owns its scratch.
///
/// Not shareable across threads:
///
/// ```compile_fail
/// fn require_sync<T: Sync>() {}
/// require_sync::<bumbledb::PreparedQuery<'static>>();
/// ```
pub struct PreparedQuery<'s> {
    schema: &'s Schema,
    plan: ExecPlan,
    /// The Free Join executor scratch (unused for guard probes).
    executor: Option<Executor>,
    bindings: Bindings,
    /// Per find term: the output spec and its result type.
    finds: Vec<(FindSpec, ValueType)>,
    /// Dense per-param expected types (validation rejects id gaps).
    param_types: Vec<ValueType>,
    /// Bind-time resolved constants, reused across executions.
    resolved_params: Vec<Const>,
    /// Per param: whether this execution's value missed the dictionary
    /// (String/Bytes only). A missed value under `Eq` short-circuits to an
    /// empty result; under `Ne` the sentinel word matches everything.
    missed_params: Vec<bool>,
    /// Per occurrence: filters with symbolic constants substituted, reused.
    resolved_filters: Vec<Vec<FilterPredicate>>,
    /// Recycled survivor buffers, one per occurrence.
    survivor_buffers: Vec<Vec<u32>>,
    /// Per occurrence: the generation whose image the COLT's current view
    /// was built from — when it matches the snapshot and the filters are
    /// unchanged, the view rebuild *and* the COLT reset are skipped
    /// entirely (forced tries are pure functions of the view).
    built_generation: Vec<Option<u64>>,
    /// Per occurrence: the resolved filters the current view was built
    /// with (capacity-retained; resolved filters carry no boxed constants).
    built_filters: Vec<Vec<FilterPredicate>>,
    /// COLT sources, one per occurrence, reset per execution with every
    /// pool capacity retained (self-joins rebuild their own views from the
    /// shared image — a duplicated filter pass, no shared buffer).
    colts: Vec<Colt>,
    /// The sink, reset per execution with capacities retained.
    sink: EitherSink,
    /// Aggregate-finalization row scratch.
    row_scratch: Vec<u64>,
    /// Guard-key byte scratch.
    guard_key: Vec<u8>,
    /// Marker: a prepared query is single-threaded scratch.
    _not_sync: std::marker::PhantomData<std::cell::Cell<()>>,
}

/// Prepares a query: the one-time pipeline, allocation-sanctioned.
///
/// # Errors
///
/// `Validation` at the IR boundary; planner caps; `Lmdb`/`Corruption` from
/// the statistics reads.
///
/// # Panics
///
/// Only on programmer-invariant violations (`binary2fj` + `factor`
/// construct valid plans by construction).
pub(crate) fn prepare<'s>(
    txn: &ReadTxn<'_>,
    cache: &ImageCache,
    schema: &'s Schema,
    query: &Query,
) -> Result<PreparedQuery<'s>> {
    let _prepare = obs::span(obs::names::PREPARE, obs::Category::Prepare);
    let witness = {
        let _s = obs::span(obs::names::VALIDATE, obs::Category::Prepare);
        validate(schema, query)?
    };
    let normalized = {
        let _s = obs::span(obs::names::NORMALIZE, obs::Category::Prepare);
        normalize(&witness)
    };

    // Classification first: a guard probe needs no statistics or planning.
    let classified = {
        let _s = obs::span(obs::names::CLASSIFY, obs::Category::Prepare);
        classify(&normalized, schema)
    };
    let exec_plan = if let Some(guard) = classified {
        ExecPlan::GuardProbe(guard)
    } else {
        // Filtered-view statistics: measured survivor counts for
        // occurrences whose filters are fully concrete; base row counts
        // otherwise (symbolic filters resolve per execution).
        let mut stats_span = obs::span(obs::names::STATS, obs::Category::Prepare);
        let mut measured = 0u64;
        let mut stats = Vec::with_capacity(normalized.occurrences.len());
        for occurrence in &normalized.occurrences {
            let concrete = !occurrence.filters.is_empty()
                && occurrence.filters.iter().all(|f| match f {
                    FilterPredicate::Compare { value, .. } => {
                        matches!(value, Const::Word(_) | Const::Byte(_))
                    }
                    FilterPredicate::FieldsCompare { .. } => true,
                });
            let rows = if concrete {
                measured += 1;
                let image = cache.get_or_build(txn, schema, occurrence.relation)?;
                apply(&image, &occurrence.filters, &[], Vec::new()).len() as u64
            } else {
                read::row_count(txn, occurrence.relation)?
            };
            stats.push(OccStats {
                occ_id: occurrence.occ_id,
                rows,
            });
        }
        stats_span.set_args(measured, 0);
        stats_span.end();
        let order = {
            let _s = obs::span(obs::names::PLAN_DP, obs::Category::Prepare);
            plan_order(&normalized, schema, &stats)
        };
        let lower_span = obs::span(obs::names::LOWER, obs::Category::Prepare);
        let mut fj = binary2fj(&normalized, &order);
        factor(&mut fj);
        let sink_vars = witness.group_key().clone();
        let validated = crate::plan::fj::validate(
            &fj,
            &normalized,
            schema,
            order.estimates.clone(),
            &sink_vars,
        )
        .expect("binary2fj + factor construct valid plans");
        lower_span.end();
        ExecPlan::FreeJoin(validated)
    };

    let finds = find_specs(query, &witness, &exec_plan);

    // Dense param typing for bind-time checks (validation rejected gaps,
    // so the id-ordered iteration is positional).
    let param_types: Vec<ValueType> = witness.param_types().map(|(_, ty)| ty.clone()).collect();

    let (executor, slot_count, occurrence_count) = match &exec_plan {
        ExecPlan::FreeJoin(plan) => (
            Some(Executor::new(plan)),
            plan.slots().len(),
            plan.occurrences().len(),
        ),
        ExecPlan::GuardProbe(guard) => (None, guard.vars.len(), 1),
    };

    let colts = {
        let _s = obs::span(obs::names::BUILD_COLTS, obs::Category::Prepare);
        build_colts(txn, cache, schema, &exec_plan)?
    };
    let sink = make_sink(&finds, slot_count, exec_plan.distinct_bindings());

    Ok(PreparedQuery {
        schema,
        plan: exec_plan,
        executor,
        bindings: Bindings::new(slot_count),
        finds,
        param_types,
        resolved_params: Vec::new(),
        missed_params: Vec::new(),
        resolved_filters: vec![Vec::new(); occurrence_count],
        survivor_buffers: (0..occurrence_count).map(|_| Vec::new()).collect(),
        built_generation: vec![None; occurrence_count],
        built_filters: vec![Vec::new(); occurrence_count],
        colts,
        sink,
        row_scratch: Vec::new(),
        guard_key: Vec::new(),
        _not_sync: std::marker::PhantomData,
    })
}

/// COLT sources with their fixed column schemas; the initial views are
/// placeholders — every execution resets them against fresh images.
fn build_colts(
    txn: &ReadTxn<'_>,
    cache: &ImageCache,
    schema: &Schema,
    exec_plan: &ExecPlan,
) -> Result<Vec<Colt>> {
    match exec_plan {
        ExecPlan::GuardProbe(_) => Ok(Vec::new()),
        ExecPlan::FreeJoin(plan) => plan
            .occurrences()
            .iter()
            .map(|occurrence| {
                let image = cache.get_or_build(txn, schema, occurrence.relation)?;
                let columns: Vec<Vec<usize>> = occurrence
                    .trie_schema
                    .iter()
                    .map(|level| {
                        level
                            .iter()
                            .map(|var| {
                                let (field, _) = occurrence
                                    .vars
                                    .iter()
                                    .find(|(_, v)| v == var)
                                    .expect("plan vars come from the occurrence");
                                usize::from(field.0)
                            })
                            .collect()
                    })
                    .collect();
                Ok(Colt::new(View::All(image), columns))
            })
            .collect(),
    }
}

/// Derives per-find output specs (slots + result types) from the witness
/// and the classified plan.
fn find_specs(
    query: &Query,
    witness: &crate::ir::validate::ValidatedQuery,
    exec_plan: &ExecPlan,
) -> Vec<(FindSpec, ValueType)> {
    query
        .finds
        .iter()
        .map(|term| match term {
            FindTerm::Var(var) => (
                FindSpec::Var {
                    slot: exec_plan.slot_of(*var),
                },
                witness.var_type(*var).clone(),
            ),
            FindTerm::Aggregate { op, over } => {
                let (over_slot, ty) = match over {
                    Some(var) => (
                        Some(exec_plan.slot_of(*var)),
                        witness.var_type(*var).clone(),
                    ),
                    None => (None, ValueType::U64), // Count
                };
                (
                    FindSpec::Agg {
                        op: *op,
                        over_slot,
                        signed: matches!(ty, ValueType::I64),
                    },
                    if *op == AggOp::Count {
                        ValueType::U64
                    } else {
                        ty
                    },
                )
            }
        })
        .collect()
}

impl PreparedQuery<'_> {
    /// Executes with the given parameters into the caller's buffer.
    ///
    /// # Errors
    ///
    /// `ParamCountMismatch`/`ParamTypeMismatch` at bind time; `Overflow`
    /// from aggregate finalization; `Lmdb`/`Corruption` from storage.
    ///
    /// # Panics
    ///
    /// Only on programmer-invariant violations (plan/executor pairing,
    /// validated id widths).
    pub(crate) fn execute(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        params: &[Value],
        out: &mut ResultBuffer,
    ) -> Result<()> {
        let mut execute_span = obs::span(obs::names::EXECUTE, obs::Category::Execute);
        out.clear();
        out.arity = self.finds.len();
        {
            let _s = obs::span(obs::names::BIND_PARAMS, obs::Category::Execute);
            self.bind_params(txn, params)?;
        }
        self.sink.reset();
        match &self.plan {
            ExecPlan::GuardProbe(guard) => {
                execute_guard(
                    guard,
                    txn,
                    self.schema,
                    &self.resolved_params,
                    &mut self.guard_key,
                    &mut self.bindings,
                    &mut self.sink,
                )?;
            }
            ExecPlan::FreeJoin(plan) => {
                let resolved = {
                    let _s = obs::span(obs::names::RESOLVE_FILTERS, obs::Category::Execute);
                    resolve_filters(
                        txn,
                        plan,
                        &self.resolved_params,
                        &self.missed_params,
                        &mut self.resolved_filters,
                    )?
                };
                if !resolved {
                    return Ok(()); // Eq-anchored dictionary miss: empty result
                }
                run_join(
                    plan,
                    self.schema,
                    txn,
                    cache,
                    self.executor
                        .as_mut()
                        .expect("free join plans carry executor scratch"),
                    &mut self.bindings,
                    &self.resolved_filters,
                    &mut self.survivor_buffers,
                    &mut self.built_generation,
                    &mut self.built_filters,
                    &mut self.colts,
                    &mut self.sink,
                    &mut NoopCounters,
                )?;
            }
        }
        let result = {
            let _s = obs::span(obs::names::FINALIZE, obs::Category::Execute);
            finalize(&self.sink, &mut self.row_scratch, txn, &self.finds, out)
        };
        execute_span.set_args(out.len() as u64, 0);
        result
    }

    /// Convenience path: a fresh buffer per call.
    ///
    /// # Errors
    ///
    /// As [`Self::execute`].
    pub(crate) fn execute_collect(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        params: &[Value],
    ) -> Result<ResultBuffer> {
        let mut out = ResultBuffer::new();
        self.execute(txn, cache, params, &mut out)?;
        Ok(out)
    }

    /// EXPLAIN (docs/architecture/30-execution.md): executes the query with counting instrumentation
    /// (ANALYZE semantics) and returns the rows alongside the rendered
    /// report.
    ///
    /// # Errors
    ///
    /// As [`Self::execute`].
    ///
    /// # Panics
    ///
    /// Only on programmer-invariant violations (plan/executor pairing).
    pub(crate) fn explain(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        params: &[Value],
    ) -> Result<(ResultBuffer, String)> {
        let (out, stats) = self.profile(txn, cache, params)?;
        let report = match &self.plan {
            ExecPlan::GuardProbe(guard) => format!("{}", Report::GuardProbe { plan: guard }),
            ExecPlan::FreeJoin(plan) => format!("{}", Report::FreeJoin { plan, stats }),
        };
        Ok((out, report))
    }

    /// ANALYZE with structured output: executes with counting
    /// instrumentation and returns the rows alongside [`ExecutionStats`]
    /// — the data `explain` renders. Allocation-sanctioned exactly like
    /// `explain`.
    ///
    /// # Errors
    ///
    /// As [`Self::execute`].
    ///
    /// # Panics
    ///
    /// Only on programmer-invariant violations (plan/executor pairing).
    pub(crate) fn profile(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        params: &[Value],
    ) -> Result<(ResultBuffer, ExecutionStats)> {
        let mut out = ResultBuffer::new();
        out.arity = self.finds.len();
        if matches!(&self.plan, ExecPlan::GuardProbe(_)) {
            self.execute(txn, cache, params, &mut out)?;
            let stats = ExecutionStats {
                nodes: Vec::new(),
                emits: out.len() as u64,
                guard: Some(crate::api::stats::GuardStats {
                    hit: !out.is_empty(),
                }),
            };
            return Ok((out, stats));
        }
        // Bind before borrowing the plan (bind_params takes &mut self).
        self.bind_params(txn, params)?;
        match &self.plan {
            ExecPlan::GuardProbe(_) => unreachable!("handled above"),
            ExecPlan::FreeJoin(plan) => {
                let mut counters = CountingCounters::new(plan);
                let short_circuit = !resolve_filters(
                    txn,
                    plan,
                    &self.resolved_params,
                    &self.missed_params,
                    &mut self.resolved_filters,
                )?;
                if short_circuit {
                    return Ok((out, counters.into_stats(plan)));
                }
                self.sink.reset();
                run_join(
                    plan,
                    self.schema,
                    txn,
                    cache,
                    self.executor
                        .as_mut()
                        .expect("free join plans carry executor scratch"),
                    &mut self.bindings,
                    &self.resolved_filters,
                    &mut self.survivor_buffers,
                    &mut self.built_generation,
                    &mut self.built_filters,
                    &mut self.colts,
                    &mut self.sink,
                    &mut counters,
                )?;
                finalize(
                    &self.sink,
                    &mut self.row_scratch,
                    txn,
                    &self.finds,
                    &mut out,
                )?;
                Ok((out, counters.into_stats(plan)))
            }
        }
    }

    /// The result column types, one per find term in `finds` order — the
    /// metadata a generic host needs to type an (even empty) result. The
    /// buffer itself stays typeless: stamping owned types per execution
    /// would allocate on the warm path.
    pub fn column_types(&self) -> impl Iterator<Item = &ValueType> {
        self.finds.iter().map(|(_, ty)| ty)
    }

    /// Rebuilds the executor scratch at a different batch size — the
    /// tuning/test surface for D4's measurement-owned constant. Allocation
    /// happens here, outside any measured window. A no-op for guard
    /// probes. Hidden: a measurement affordance, not a knob on the
    /// no-knobs surface (`docs/architecture/00-product.md`).
    #[doc(hidden)]
    pub fn set_batch_size(&mut self, batch: usize) {
        if let ExecPlan::FreeJoin(plan) = &self.plan {
            self.executor = Some(Executor::with_batch_size(plan, batch));
        }
    }

    /// Binds and converts parameters; `Ok(false)` = a String/Bytes value
    /// that was never interned (the query is empty on this snapshot).
    fn bind_params(&mut self, txn: &ReadTxn<'_>, params: &[Value]) -> Result<()> {
        if params.len() != self.param_types.len() {
            return Err(Error::ParamCountMismatch {
                expected: self.param_types.len(),
                supplied: params.len(),
            });
        }
        self.resolved_params.clear();
        self.missed_params.clear();
        for (idx, value) in params.iter().enumerate() {
            let (resolved, missed) = bind_param(txn, idx, value, &self.param_types[idx])?;
            self.resolved_params.push(resolved);
            self.missed_params.push(missed);
        }
        Ok(())
    }
}

/// Resolves every occurrence's symbolic filter constants for this
/// execution; `Ok(false)` = a dictionary miss under an `Eq` filter, which
/// empties the whole conjunctive query (the short-circuit is sound for
/// `Eq` only — a missed value under `Ne` resolves to the sentinel id and
/// matches everything).
fn resolve_filters(
    txn: &ReadTxn<'_>,
    plan: &crate::plan::fj::ValidatedPlan,
    params: &[Const],
    missed: &[bool],
    out: &mut [Vec<FilterPredicate>],
) -> Result<bool> {
    for (occ_idx, occurrence) in plan.occurrences().iter().enumerate() {
        out[occ_idx].clear();
        for filter in &occurrence.filters {
            let Some(resolved) = resolve_filter(txn, filter, params, missed)? else {
                return Ok(false);
            };
            out[occ_idx].push(resolved);
        }
    }
    Ok(true)
}

/// Resets the owned COLT sources against this execution's images and
/// views (buffer ping-pong: old survivor buffers recycle into the new
/// views), then runs the join into the sink.
#[allow(clippy::too_many_arguments)] // the prepared query's split borrows;
                                     // bundling them into a struct would only rename the same ten things
fn run_join<C: crate::exec::run::Counters>(
    plan: &crate::plan::fj::ValidatedPlan,
    schema: &Schema,
    txn: &ReadTxn<'_>,
    cache: &ImageCache,
    executor: &mut Executor,
    bindings: &mut Bindings,
    resolved_filters: &[Vec<FilterPredicate>],
    survivor_buffers: &mut [Vec<u32>],
    built_generation: &mut [Option<u64>],
    built_filters: &mut [Vec<FilterPredicate>],
    colts: &mut [Colt],
    sink: &mut EitherSink,
    counters: &mut C,
) -> Result<()> {
    let views_span = obs::span(obs::names::VIEWS, obs::Category::Execute);
    let generation = txn.generation()?;
    for (occ_idx, occurrence) in plan.occurrences().iter().enumerate() {
        // Warm fast path: same generation, same resolved filters — the
        // COLT's view is still exactly right, and so are its forced
        // tries. No cache lock, no filter scan, no re-force.
        if built_generation[occ_idx] == Some(generation)
            && built_filters[occ_idx] == resolved_filters[occ_idx]
        {
            obs::event(
                obs::names::VIEW_MEMO_HIT,
                obs::Category::Execute,
                occ_idx as u64,
                0,
            );
            continue;
        }
        let mut build_span = obs::span_args(
            obs::names::VIEW_BUILD,
            obs::Category::Execute,
            occ_idx as u64,
            0,
        );
        let image = cache.get_or_build(txn, schema, occurrence.relation)?;
        let buffer = std::mem::take(&mut survivor_buffers[occ_idx]);
        let view = apply(&image, &resolved_filters[occ_idx], &[], buffer);
        build_span.set_args(occ_idx as u64, view.len() as u64);
        let old = colts[occ_idx].reset(view);
        survivor_buffers[occ_idx] = old.recycle();
        built_generation[occ_idx] = Some(generation);
        built_filters[occ_idx].clone_from(&resolved_filters[occ_idx]);
    }
    views_span.end();
    let _join = obs::span(obs::names::JOIN, obs::Category::Execute);
    // One match per execution: the executor monomorphizes per concrete
    // sink type — no per-emit enum branch on the hot path.
    match sink {
        EitherSink::Projection(s) => executor.execute(plan, colts, bindings, s, counters),
        EitherSink::Aggregate(s) => executor.execute(plan, colts, bindings, s, counters),
    }
    Ok(())
}

/// Builds the sink matching the find shape (the variant is fixed per
/// prepared query — an enum, not `dyn`).
fn make_sink(finds: &[(FindSpec, ValueType)], slot_count: usize, distinct: bool) -> EitherSink {
    let has_aggregates = finds
        .iter()
        .any(|(spec, _)| matches!(spec, FindSpec::Agg { .. }));
    if has_aggregates {
        EitherSink::Aggregate(AggregateSink::new(
            finds.iter().map(|(spec, _)| *spec).collect(),
            slot_count,
            distinct,
        ))
    } else {
        EitherSink::Projection(ProjectionSink::new(
            finds
                .iter()
                .map(|(spec, _)| match spec {
                    FindSpec::Var { slot } => *slot,
                    FindSpec::Agg { .. } => unreachable!("no aggregates here"),
                })
                .collect(),
        ))
    }
}

/// The two sink shapes behind one monomorphized dispatch (an enum, not
/// `dyn` — the variant is fixed per prepared query).
enum EitherSink {
    Projection(ProjectionSink),
    Aggregate(AggregateSink),
}

impl EitherSink {
    fn reset(&mut self) {
        match self {
            Self::Projection(sink) => sink.reset(),
            Self::Aggregate(sink) => sink.reset(),
        }
    }
}

impl Sink for EitherSink {
    fn emit(&mut self, bindings: &Bindings) -> crate::exec::run::Flow {
        match self {
            Self::Projection(sink) => sink.emit(bindings),
            Self::Aggregate(sink) => sink.emit(bindings),
        }
    }
}

/// Converts a bound param value to column form. A String or Bytes value
/// that was never interned resolves to the sentinel intern id, flagged
/// `missed` so `Eq` uses can short-circuit to the empty result.
fn bind_param(
    txn: &ReadTxn<'_>,
    index: usize,
    value: &Value,
    expected: &ValueType,
) -> Result<(Const, bool)> {
    // The shared compatibility check (kind, enum range, UTF-8) — one rule
    // with validation and the dynamic write path.
    if crate::ir::value_matches(value, expected).is_err() {
        return Err(Error::ParamTypeMismatch {
            param: ParamId(u16::try_from(index).expect("param ids fit u16")),
            expected: expected.clone(),
        });
    }
    let resolved = match value {
        Value::Bool(v) => Const::Byte(u8::from(*v)),
        Value::Enum(ordinal) => Const::Byte(*ordinal),
        Value::U64(v) => Const::Word(*v),
        Value::I64(v) => Const::Word(u64::from_be_bytes(crate::encoding::encode_i64(*v))),
        Value::String(bytes) => {
            let text = std::str::from_utf8(bytes).expect("value_matches validated UTF-8");
            match dict::lookup_str(txn, text)? {
                Some(id) => Const::Word(id),
                None => return Ok((Const::Word(dict::SENTINEL_ID), true)),
            }
        }
        Value::Bytes(bytes) => match dict::lookup_bytes(txn, bytes)? {
            Some(id) => Const::Word(id),
            None => return Ok((Const::Word(dict::SENTINEL_ID), true)),
        },
    };
    Ok((resolved, false))
}

/// Substitutes symbolic constants into an executable filter. `Ok(None)` =
/// a dictionary miss under `Eq` (the whole-query empty short-circuit); a
/// miss under any other operator resolves to the sentinel intern id, whose
/// word comparison yields the correct per-operator semantics (`Ne` matches
/// every stored value).
fn resolve_filter(
    txn: &ReadTxn<'_>,
    filter: &FilterPredicate,
    params: &[Const],
    missed: &[bool],
) -> Result<Option<FilterPredicate>> {
    let FilterPredicate::Compare { field, op, value } = filter else {
        return Ok(Some(filter.clone()));
    };
    let resolved = match value {
        Const::Word(_) | Const::Byte(_) => value.clone(),
        Const::Param(p) => {
            if missed[usize::from(p.0)] && *op == crate::ir::CmpOp::Eq {
                return Ok(None);
            }
            params[usize::from(p.0)].clone()
        }
        Const::PendingIntern { tag, bytes } => match dict::lookup_tagged(txn, *tag, bytes)? {
            Some(id) => Const::Word(id),
            None if *op == crate::ir::CmpOp::Eq => return Ok(None),
            None => Const::Word(dict::SENTINEL_ID),
        },
    };
    Ok(Some(FilterPredicate::Compare {
        field: *field,
        op: *op,
        value: resolved,
    }))
}

/// Drains the sink into the result buffer, decoding words by result type.
fn finalize(
    sink: &EitherSink,
    row_scratch: &mut Vec<u64>,
    txn: &ReadTxn<'_>,
    finds: &[(FindSpec, ValueType)],
    out: &mut ResultBuffer,
) -> Result<()> {
    match sink {
        EitherSink::Projection(sink) => {
            for row in sink.rows() {
                for (column, (_, ty)) in finds.iter().enumerate() {
                    out.push_word(txn, ty, row[column])?;
                }
            }
            Ok(())
        }
        EitherSink::Aggregate(sink) => sink.finalize_into(row_scratch, |row| {
            for (column, (_, ty)) in finds.iter().enumerate() {
                out.push_word(txn, ty, row[column])?;
            }
            Ok(())
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::encoding::{encode_fact, ValueRef};
    use crate::ir::{Atom, CmpOp, Comparison, FindTerm, Term, VarId};
    use crate::schema::{
        FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor,
    };
    use crate::storage::commit::commit;
    use crate::storage::delta::WriteDelta;
    use crate::storage::env::Environment;
    use crate::testutil::TempDir;

    /// Posting(id serial u64, account u64, memo string, amount i64).
    fn schema() -> Schema {
        SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "Posting".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Serial,
                    },
                    FieldDescriptor {
                        name: "account".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "memo".into(),
                        value_type: ValueType::String,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "amount".into(),
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

    const POSTING: RelationId = RelationId(0);

    fn insert_postings(env: &Environment, schema: &Schema, rows: &[(u64, u64, &str, i64)]) {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(schema);
        for (id, account, memo, amount) in rows {
            let memo_id = delta.intern_str(&view, memo).expect("intern");
            let mut bytes = Vec::new();
            encode_fact(
                &[
                    ValueRef::U64(*id),
                    ValueRef::U64(*account),
                    ValueRef::String(memo_id),
                    ValueRef::I64(*amount),
                ],
                schema.relation(POSTING).layout(),
                &mut bytes,
            );
            delta.insert(&view, POSTING, &bytes).expect("insert");
        }
        drop(view);
        commit(delta, env).expect("commit");
    }

    /// Q(memo, amount) :- Posting(account = ?0, memo, amount), amount >= ?1.
    fn by_account_query() -> Query {
        Query {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            atoms: vec![Atom {
                relation: POSTING,
                bindings: vec![
                    (FieldId(1), Term::Param(crate::ir::ParamId(0))),
                    (FieldId(2), Term::Var(VarId(0))),
                    (FieldId(3), Term::Var(VarId(1))),
                ],
            }],
            predicates: vec![Comparison {
                op: CmpOp::Ge,
                lhs: Term::Var(VarId(1)),
                rhs: Term::Param(crate::ir::ParamId(1)),
            }],
        }
    }

    fn rows_of(buffer: &ResultBuffer) -> Vec<(String, i64)> {
        let mut rows: Vec<(String, i64)> = (0..buffer.len())
            .map(|row| {
                let ResultValue::String(memo) = buffer.get(row, 0) else {
                    panic!("column 0 is a string");
                };
                let ResultValue::I64(amount) = buffer.get(row, 1) else {
                    panic!("column 1 is an i64");
                };
                (memo.to_owned(), amount)
            })
            .collect();
        rows.sort();
        rows
    }

    #[test]
    fn prepare_once_execute_many_with_varying_params() {
        let dir = TempDir::new("prepared-many");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        insert_postings(
            &env,
            &schema,
            &[
                (1, 7, "rent", -1200),
                (2, 7, "salary", 5000),
                (3, 8, "coffee", -4),
            ],
        );
        let cache = ImageCache::new();
        let txn = env.read_txn().expect("txn");
        let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
        let mut out = ResultBuffer::new();

        prepared
            .execute(&txn, &cache, &[Value::U64(7), Value::I64(0)], &mut out)
            .expect("execute");
        assert_eq!(rows_of(&out), vec![("salary".to_owned(), 5000)]);

        prepared
            .execute(
                &txn,
                &cache,
                &[Value::U64(7), Value::I64(i64::MIN)],
                &mut out,
            )
            .expect("execute");
        assert_eq!(
            rows_of(&out),
            vec![("rent".to_owned(), -1200), ("salary".to_owned(), 5000)]
        );

        prepared
            .execute(
                &txn,
                &cache,
                &[Value::U64(8), Value::I64(i64::MIN)],
                &mut out,
            )
            .expect("execute");
        assert_eq!(rows_of(&out), vec![("coffee".to_owned(), -4)]);
    }

    #[test]
    fn bind_time_checks_reject_bad_params() {
        let dir = TempDir::new("prepared-bind-errors");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let cache = ImageCache::new();
        let txn = env.read_txn().expect("txn");
        let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
        let mut out = ResultBuffer::new();

        let err = prepared
            .execute(&txn, &cache, &[Value::U64(7)], &mut out)
            .unwrap_err();
        assert!(
            matches!(
                err,
                Error::ParamCountMismatch {
                    expected: 2,
                    supplied: 1
                }
            ),
            "{err:?}"
        );

        let err = prepared
            .execute(&txn, &cache, &[Value::I64(7), Value::I64(0)], &mut out)
            .unwrap_err();
        assert!(
            matches!(err, Error::ParamTypeMismatch { param, .. } if param.0 == 0),
            "{err:?}"
        );
    }

    #[test]
    fn string_params_resolve_per_execution() {
        let dir = TempDir::new("prepared-string-param");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        insert_postings(&env, &schema, &[(1, 7, "rent", -1200)]);
        let cache = ImageCache::new();

        // Q(amount) :- Posting(memo = ?0, amount).
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                relation: POSTING,
                bindings: vec![
                    (FieldId(2), Term::Param(crate::ir::ParamId(0))),
                    (FieldId(3), Term::Var(VarId(0))),
                ],
            }],
            predicates: vec![],
        };
        let txn = env.read_txn().expect("txn");
        let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
        let mut out = ResultBuffer::new();

        // Never-interned value: empty, not an error.
        prepared
            .execute(
                &txn,
                &cache,
                &[Value::String(Box::from(&b"groceries"[..]))],
                &mut out,
            )
            .expect("execute");
        assert!(out.is_empty());
        drop(txn);

        // A later commit interns it; the SAME prepared query now finds rows
        // (per-execution resolution — no stale-resolution trap).
        insert_postings(&env, &schema, &[(2, 9, "groceries", -55)]);
        let txn = env.read_txn().expect("txn");
        prepared
            .execute(
                &txn,
                &cache,
                &[Value::String(Box::from(&b"groceries"[..]))],
                &mut out,
            )
            .expect("execute");
        assert_eq!(out.len(), 1);
        assert_eq!(out.get(0, 0), ResultValue::I64(-55));
    }

    /// Regression for the `Ne`-miss semantics
    /// (docs/architecture/20-query-ir.md): a never-interned value under
    /// `Ne` matches every stored row — the miss resolves to the sentinel
    /// intern id, not to an empty result. The old blanket "miss ⇒ empty"
    /// rule silently returned nothing here.
    #[test]
    fn ne_against_a_never_interned_string_matches_everything() {
        let dir = TempDir::new("prepared-ne-miss");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        insert_postings(&env, &schema, &[(1, 7, "rent", -1200), (2, 9, "food", -55)]);
        let cache = ImageCache::new();

        // Literal path: Q(amount) :- Posting(memo = m, amount), m != "ghost".
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                relation: POSTING,
                bindings: vec![
                    (FieldId(2), Term::Var(VarId(1))),
                    (FieldId(3), Term::Var(VarId(0))),
                ],
            }],
            predicates: vec![Comparison {
                op: CmpOp::Ne,
                lhs: Term::Var(VarId(1)),
                rhs: Term::Literal(Value::String(Box::from(&b"ghost"[..]))),
            }],
        };
        let txn = env.read_txn().expect("txn");
        let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
        let out = prepared
            .execute_collect(&txn, &cache, &[])
            .expect("execute");
        assert_eq!(out.len(), 2, "no stored memo equals a never-interned value");

        // Param path: Q(amount) :- Posting(memo = m, amount), m != ?0.
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                relation: POSTING,
                bindings: vec![
                    (FieldId(2), Term::Var(VarId(1))),
                    (FieldId(3), Term::Var(VarId(0))),
                ],
            }],
            predicates: vec![Comparison {
                op: CmpOp::Ne,
                lhs: Term::Var(VarId(1)),
                rhs: Term::Param(crate::ir::ParamId(0)),
            }],
        };
        let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
        let out = prepared
            .execute_collect(&txn, &cache, &[Value::String(Box::from(&b"ghost"[..]))])
            .expect("execute");
        assert_eq!(out.len(), 2);
        // An interned value under Ne excludes exactly its rows.
        let out = prepared
            .execute_collect(&txn, &cache, &[Value::String(Box::from(&b"rent"[..]))])
            .expect("execute");
        assert_eq!(out.len(), 1);
        assert_eq!(out.get(0, 0), ResultValue::I64(-55));
    }

    #[test]
    fn results_decode_intern_ids_to_original_bytes() {
        let dir = TempDir::new("prepared-decode");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        insert_postings(&env, &schema, &[(1, 7, "a rather long memo text", 10)]);
        let cache = ImageCache::new();
        let txn = env.read_txn().expect("txn");
        let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
        let out = prepared
            .execute_collect(&txn, &cache, &[Value::U64(7), Value::I64(0)])
            .expect("execute");
        assert_eq!(
            out.get(0, 0),
            ResultValue::String("a rather long memo text")
        );
    }

    #[test]
    fn buffer_reuse_retains_capacity_and_results_stay_identical() {
        let dir = TempDir::new("prepared-buffer-reuse");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        insert_postings(
            &env,
            &schema,
            &[(1, 7, "one", 1), (2, 7, "two", 2), (3, 7, "three", 3)],
        );
        let cache = ImageCache::new();
        let txn = env.read_txn().expect("txn");
        let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
        let mut out = ResultBuffer::new();
        let params = [Value::U64(7), Value::I64(0)];

        prepared
            .execute(&txn, &cache, &params, &mut out)
            .expect("execute");
        let first = rows_of(&out);
        let (cells_cap, bytes_cap) = (out.cells.capacity(), out.bytes.capacity());
        assert!(cells_cap > 0 && bytes_cap > 0);

        prepared
            .execute(&txn, &cache, &params, &mut out)
            .expect("execute");
        assert_eq!(rows_of(&out), first);
        // Capacity is retained across reuse (the zero-alloc path).
        assert!(out.cells.capacity() >= cells_cap);
        assert!(out.bytes.capacity() >= bytes_cap);
        assert_eq!(first.len(), 3);
    }

    #[test]
    fn pinned_plan_reads_fresh_data_at_newer_generations() {
        let dir = TempDir::new("prepared-fresh-data");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        insert_postings(&env, &schema, &[(1, 7, "old", 1)]);
        let cache = ImageCache::new();
        let txn = env.read_txn().expect("txn");
        let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
        let mut out = ResultBuffer::new();
        prepared
            .execute(&txn, &cache, &[Value::U64(7), Value::I64(0)], &mut out)
            .expect("execute");
        assert_eq!(out.len(), 1);
        drop(txn);

        // New commit, new snapshot: the pinned *plan* runs over fresh data.
        insert_postings(&env, &schema, &[(2, 7, "new", 2)]);
        let txn = env.read_txn().expect("txn");
        prepared
            .execute(&txn, &cache, &[Value::U64(7), Value::I64(0)], &mut out)
            .expect("execute");
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn guard_probe_queries_flow_through_the_same_surface() {
        let dir = TempDir::new("prepared-guard");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        insert_postings(&env, &schema, &[(5, 7, "found", 42)]);
        let cache = ImageCache::new();
        // Q(amount) :- Posting(id = 5, amount) — the serial key: guard probe.
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                relation: POSTING,
                bindings: vec![
                    (FieldId(0), Term::Literal(Value::U64(5))),
                    (FieldId(3), Term::Var(VarId(0))),
                ],
            }],
            predicates: vec![],
        };
        let txn = env.read_txn().expect("txn");
        let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
        assert!(matches!(prepared.plan, ExecPlan::GuardProbe(_)));
        let out = prepared
            .execute_collect(&txn, &cache, &[])
            .expect("execute");
        assert_eq!(out.len(), 1);
        assert_eq!(out.get(0, 0), ResultValue::I64(42));

        // EXPLAIN reports the classification alongside the rows.
        let (rows, report) = prepared.explain(&txn, &cache, &[]).expect("explain");
        assert_eq!(rows.len(), 1);
        assert!(report.contains("guard probe"));
    }

    #[test]
    fn explain_reports_the_join_plan_with_actuals() {
        let dir = TempDir::new("prepared-explain");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        insert_postings(&env, &schema, &[(1, 7, "a", 1), (2, 7, "b", 2)]);
        let cache = ImageCache::new();
        let txn = env.read_txn().expect("txn");
        let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
        let (rows, report) = prepared
            .explain(&txn, &cache, &[Value::U64(7), Value::I64(0)])
            .expect("explain");
        assert_eq!(rows.len(), 2);
        assert!(report.contains("free join"));
        assert!(report.contains("emitted bindings: 2"));
    }

    /// PRD 03's read-path capture contract (feature `trace`).
    #[cfg(feature = "trace")]
    #[test]
    fn read_path_traces_phases_memo_hits_and_guard() {
        use crate::obs;

        let dir = TempDir::new("prepared-trace-read");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        insert_postings(&env, &schema, &[(1, 7, "rent", -1200), (2, 7, "food", -55)]);
        let cache = ImageCache::new();
        let txn = env.read_txn().expect("txn");

        let names = |events: &[obs::TraceEvent]| -> Vec<&'static str> {
            events.iter().map(|e| e.name).collect()
        };

        // Prepare: the phase spans, exactly.
        obs::start_capture();
        let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
        let events = obs::finish_capture();
        let got = names(&events);
        for expected in [
            obs::names::VALIDATE,
            obs::names::NORMALIZE,
            obs::names::CLASSIFY,
            obs::names::STATS,
            obs::names::PLAN_DP,
            obs::names::LOWER,
            obs::names::BUILD_COLTS,
            obs::names::PREPARE,
        ] {
            assert!(got.contains(&expected), "missing {expected} in {got:?}");
        }
        // Containment: every phase inside the outer prepare span.
        let outer = events
            .iter()
            .find(|e| e.name == obs::names::PREPARE)
            .expect("outer");
        for e in &events {
            assert!(e.start_ns >= outer.start_ns);
            assert!(e.start_ns + e.dur_ns <= outer.start_ns + outer.dur_ns);
        }

        // First execute: builds views, no memo hits, row count in a0.
        obs::start_capture();
        let out = prepared
            .execute_collect(&txn, &cache, &[Value::U64(7), Value::I64(-100_000)])
            .expect("execute");
        let first = obs::finish_capture();
        assert_eq!(out.len(), 2);
        let first_names = names(&first);
        assert!(
            first_names.contains(&obs::names::VIEW_BUILD),
            "{first_names:?}"
        );
        assert!(!first_names.contains(&obs::names::VIEW_MEMO_HIT));
        let exec = first
            .iter()
            .find(|e| e.name == obs::names::EXECUTE)
            .expect("execute span");
        assert_eq!(exec.a0, 2, "execute a0 carries the row count");

        // Second execute, same snapshot + params: memo hits only.
        obs::start_capture();
        prepared
            .execute_collect(&txn, &cache, &[Value::U64(7), Value::I64(-100_000)])
            .expect("execute");
        let second = obs::finish_capture();
        let second_names = names(&second);
        assert!(
            second_names.contains(&obs::names::VIEW_MEMO_HIT),
            "{second_names:?}"
        );
        assert!(!second_names.contains(&obs::names::VIEW_BUILD));
        assert!(!second_names.contains(&obs::names::IMAGE_BUILD));

        // A guard-shaped query: guard_probe, never join.
        let guard_query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                relation: POSTING,
                bindings: vec![
                    (FieldId(0), Term::Param(crate::ir::ParamId(0))),
                    (FieldId(3), Term::Var(VarId(0))),
                ],
            }],
            predicates: vec![],
        };
        let mut guard = prepare(&txn, &cache, &schema, &guard_query).expect("prepare");
        obs::start_capture();
        guard
            .execute_collect(&txn, &cache, &[Value::U64(1)])
            .expect("execute");
        let guard_events = obs::finish_capture();
        let guard_names = names(&guard_events);
        assert!(
            guard_names.contains(&obs::names::GUARD_PROBE),
            "{guard_names:?}"
        );
        assert!(!guard_names.contains(&obs::names::JOIN));
        let probe = guard_events
            .iter()
            .find(|e| e.name == obs::names::GUARD_PROBE)
            .expect("probe");
        assert_eq!(probe.a0, 1, "hit flag");

        // Nothing records without capture.
        prepared
            .execute_collect(&txn, &cache, &[Value::U64(7), Value::I64(-100_000)])
            .expect("execute");
        obs::start_capture();
        assert!(obs::finish_capture().is_empty());
    }

    #[test]
    fn profile_returns_structured_stats_matching_the_execution() {
        let dir = TempDir::new("prepared-profile");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        insert_postings(
            &env,
            &schema,
            &[
                (1, 7, "rent", -1200),
                (2, 7, "food", -55),
                (3, 9, "gym", -80),
            ],
        );
        let cache = ImageCache::new();
        let txn = env.read_txn().expect("txn");

        let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
        let (rows, stats) = prepared
            .profile(&txn, &cache, &[Value::U64(7), Value::I64(-100_000)])
            .expect("profile");
        assert_eq!(rows.len(), 2);
        assert_eq!(stats.emits, 2);
        assert!(stats.guard.is_none());
        assert!(!stats.nodes.is_empty());
        let last = stats.nodes.last().expect("nodes");
        assert_eq!(last.actual, stats.emits, "last node's actual = emits");
        assert!(
            stats.nodes[0].batches >= 1 && stats.nodes[0].batch_entries >= stats.nodes[0].batches,
            "batching counters populated: {stats:?}"
        );

        // The rendered explain is built from the same struct — spot-pin
        // the format so the golden contract holds.
        let (_, report) = prepared
            .explain(&txn, &cache, &[Value::U64(7), Value::I64(-100_000)])
            .expect("explain");
        assert!(report.contains("access path: free join"), "{report}");
        assert!(report.contains("emitted bindings: 2"), "{report}");

        // Guard profile: no nodes, a hit flag.
        let guard_query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                relation: POSTING,
                bindings: vec![
                    (FieldId(0), Term::Param(crate::ir::ParamId(0))),
                    (FieldId(3), Term::Var(VarId(0))),
                ],
            }],
            predicates: vec![],
        };
        let mut guard = prepare(&txn, &cache, &schema, &guard_query).expect("prepare");
        let (rows, stats) = guard
            .profile(&txn, &cache, &[Value::U64(1)])
            .expect("profile");
        assert_eq!(rows.len(), 1);
        assert!(stats.nodes.is_empty());
        assert_eq!(
            stats.guard,
            Some(crate::api::stats::GuardStats { hit: true })
        );
        let (_, stats) = guard
            .profile(&txn, &cache, &[Value::U64(999)])
            .expect("profile");
        assert_eq!(
            stats.guard,
            Some(crate::api::stats::GuardStats { hit: false })
        );
    }
}
