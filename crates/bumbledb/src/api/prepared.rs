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
use crate::plan::planner::plan as plan_order;
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

    /// The byte heap's length — memory observability (each distinct
    /// String/Bytes value is stored once per buffer, docs/architecture/30-execution.md).
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.bytes.len()
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

    /// Converts a fixed-width word to its cell — infallible by schema
    /// invariant (docs/perf/ PRD 08: the all-words finalize path carries
    /// no `Result` and no dictionary plumbing per cell).
    fn word_cell(ty: &ValueType, word: u64) -> Cell {
        match ty {
            ValueType::Bool => Cell::Bool(word != 0),
            ValueType::Enum { .. } => Cell::Enum(
                // Programmer invariant, not corruption: image build
                // range-checked every stored ordinal against the schema.
                u8::try_from(word).expect("enum words fit u8"),
            ),
            ValueType::U64 => Cell::U64(word),
            ValueType::I64 => Cell::I64((word ^ (1 << 63)).cast_signed()),
            ValueType::String | ValueType::Bytes => {
                unreachable!("interned finds take the resolving path")
            }
        }
    }

    fn push_word(
        &mut self,
        txn: &ReadTxn<'_>,
        ty: &ValueType,
        word: u64,
        memo: &mut ResolveMemo,
    ) -> Result<()> {
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
                let (start, len) = memo.resolve(txn, word, dict::TAG_STRING, self, true)?;
                Cell::String { start, len }
            }
            ValueType::Bytes => {
                let (start, len) = memo.resolve(txn, word, dict::TAG_BYTES, self, false)?;
                Cell::Bytes { start, len }
            }
        };
        self.cells.push(cell);
        Ok(())
    }
}

/// The per-finalize intern-resolution memo (docs/architecture/30-execution.md): each
/// distinct `(intern word, dictionary tag)` pair is resolved through
/// LMDB exactly once per finalize, and its bytes land in the output
/// buffer exactly once — K rows sharing one memo string cost one B-tree
/// lookup and one byte copy, not K. Cleared per finalize (the ranges
/// point into that call's buffer); capacity is retained, growing to the
/// distinct-string high-water like every other execution scratch.
///
/// The key carries the tag even though intern ids mint from one shared
/// counter (string and bytes words are numerically disjoint today) —
/// tag disambiguation must not depend on that allocation detail.
#[derive(Debug)]
struct ResolveMemo {
    /// `(word, tag)` → packed `(start, len)` into the buffer's bytes.
    ranges: crate::exec::wordmap::WordMap<(u32, u32)>,
    /// The last resolution (docs/perf/ PRD 08): run-coherent columns
    /// (few distinct interns, clustered rows) skip even the map probe.
    last: Option<((u64, u8), (usize, usize))>,
}

impl ResolveMemo {
    fn new() -> Self {
        Self {
            ranges: crate::exec::wordmap::WordMap::new(2),
            last: None,
        }
    }

    fn clear(&mut self) {
        self.ranges.clear();
        self.last = None;
    }

    /// The byte range for one intern word: memoized, or resolved through
    /// the dictionary (emitting `dict_resolve`), UTF-8-checked for
    /// strings, and appended to the buffer once.
    fn resolve(
        &mut self,
        txn: &ReadTxn<'_>,
        word: u64,
        tag: u8,
        buffer: &mut ResultBuffer,
        utf8: bool,
    ) -> Result<(usize, usize)> {
        if let Some((last_key, range)) = self.last {
            if last_key == (word, tag) {
                return Ok(range);
            }
        }
        let key = [word, u64::from(tag)];
        if let (range, false) = self.ranges.get_or_insert_with(&key, || (0, 0)) {
            let range = (range.0 as usize, range.1 as usize);
            self.last = Some(((word, tag), range));
            return Ok(range);
        }
        let raw = dict::resolve(txn, word, tag)?;
        obs::event(
            obs::names::DICT_RESOLVE,
            obs::Category::Storage,
            word,
            raw.len() as u64,
        );
        if utf8 {
            std::str::from_utf8(raw).map_err(|_| {
                Error::Corruption(crate::error::CorruptionError::NonUtf8Intern(word))
            })?;
        }
        let start = buffer.bytes.len();
        buffer.bytes.extend_from_slice(raw);
        // The byte heap's offsets are u32: a >4 GiB distinct-payload
        // result is absurd under the scale axiom but valid input — a
        // typed error, not a panic (finalize already threads Result).
        let range = (
            u32::try_from(start).map_err(|_| Error::ResultBytesOverflow)?,
            u32::try_from(raw.len()).map_err(|_| Error::ResultBytesOverflow)?,
        );
        let (slot, _) = self.ranges.get_or_insert_with(&key, || range);
        *slot = range;
        self.last = Some(((word, tag), (start, raw.len())));
        Ok((start, raw.len()))
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
    /// The preparing environment's process-unique identity: plan,
    /// statistics, and view memo all belong to it, so execution against
    /// any other environment's snapshot is `Error::ForeignPreparedQuery`
    /// — checked first at every execution entry.
    env_instance: u64,
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
    /// Per occurrence: residual filters with symbolic constants
    /// substituted, reused.
    resolved_filters: Vec<Vec<FilterPredicate>>,
    /// Per occurrence: this execution's resolved selection words, in
    /// selection-level order (docs/architecture/30-execution.md), reused.
    resolved_selections: Vec<Vec<u64>>,
    /// The view memo (docs/architecture/30-execution.md): per occurrence, the active binding
    /// (whose COLT the executor consumes) plus parked bindings under LRU.
    memo: ViewMemo,
    /// The sink, reset per execution with capacities retained.
    sink: EitherSink,
    /// Aggregate-finalization row scratch.
    row_scratch: Vec<u64>,
    /// No interned finds (docs/perf/ PRD 08): finalize takes the
    /// infallible all-words blit.
    all_words: bool,
    /// The guard fast lane's find table (docs/perf/ PRD 11): each output
    /// column's fact field and type, in find order. `Some` for guard
    /// plans whose finds are all plain variables; aggregate-find guards
    /// keep the sink path.
    guard_finds: Option<Vec<(crate::schema::FieldId, ValueType)>>,
    /// The per-finalize intern-resolution memo (docs/architecture/30-execution.md).
    resolve_memo: ResolveMemo,
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
        // Per-occurrence input estimates (docs/architecture/30-execution.md): row counters
        // shaped by the selectivity ladder — schema-exact uniques,
        // resident-image distinct counts (peek only: prepare never
        // builds an image for statistics), documented bounds and floors.
        let mut stats_span = obs::span(obs::names::STATS, obs::Category::Prepare);
        let mut stats = Vec::with_capacity(normalized.occurrences.len());
        for occurrence in &normalized.occurrences {
            let rows = read::row_count(txn, occurrence.relation)?;
            stats.push(crate::plan::selectivity::occurrence_stats(
                txn, cache, schema, occurrence, rows,
            )?);
        }
        stats_span.set_args(stats.len() as u64, 0);
        stats_span.end();
        let order = {
            let _s = obs::span(obs::names::PLAN_DP, obs::Category::Prepare);
            plan_order(&normalized, schema, &stats)
        };
        let lower_span = obs::span(obs::names::LOWER, obs::Category::Prepare);
        let mut fj = binary2fj(&normalized, &order);
        factor(&mut fj);
        // Group key for projections; every variable for aggregates —
        // skip-illegality under a fold is encoded in the bits themselves
        // (hardening PRD 05; `ValidatedQuery::sink_vars`).
        let sink_vars = witness.sink_vars();
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

    // BUILD_COLTS is pure column-schema construction since the unbound-
    // views cutover: prepare provably never touches an image (the stats
    // phase peeks, never builds), so a prepared query pins nothing.
    let memo = {
        let _s = obs::span(obs::names::BUILD_COLTS, obs::Category::Prepare);
        build_view_memo(&exec_plan)
    };
    // Sink presizing (docs/perf/ PRD 06): the last node's planner
    // estimate bounds the binding stream the sink consumes.
    let output_hint = match &exec_plan {
        ExecPlan::FreeJoin(plan) => {
            usize::try_from(plan.estimates().last().copied().unwrap_or(0).min(1 << 21))
                .expect("clamped")
        }
        ExecPlan::GuardProbe(_) => 1,
    };
    let sink = make_sink(
        &finds,
        slot_count,
        exec_plan.distinct_bindings(),
        output_hint,
    );

    let all_words = finds
        .iter()
        .all(|(_, ty)| !matches!(ty, ValueType::String | ValueType::Bytes));
    let guard_finds = guard_find_table(&exec_plan, &finds);
    Ok(PreparedQuery {
        schema,
        env_instance: txn.env_instance(),
        plan: exec_plan,
        executor,
        bindings: Bindings::new(slot_count),
        finds,
        param_types,
        resolved_params: Vec::new(),
        missed_params: Vec::new(),
        resolved_filters: vec![Vec::new(); occurrence_count],
        resolved_selections: vec![Vec::new(); occurrence_count],
        memo,
        sink,
        row_scratch: Vec::new(),
        all_words,
        guard_finds,
        resolve_memo: ResolveMemo::new(),
        guard_key: Vec::new(),
        _not_sync: std::marker::PhantomData,
    })
}

/// COLT sources with their fixed column schemas over [`View::Unbound`]:
/// prepare touches no image — the first execution binds every view via
/// the ordinary memo-miss path (a `None` generation never matches),
/// paying the image build exactly where a cold execution already pays
/// it. Pure column-schema construction; nothing here can fail.
fn build_view_memo(exec_plan: &ExecPlan) -> ViewMemo {
    let mut memo = ViewMemo {
        colts: Vec::new(),
        generation: Vec::new(),
        filters: Vec::new(),
        parked: Vec::new(),
        spare_buffers: Vec::new(),
        tick: 0,
    };
    let ExecPlan::FreeJoin(plan) = exec_plan else {
        return memo; // guard probes never touch views
    };
    for occurrence in plan.occurrences() {
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
        let selections: Vec<usize> = occurrence
            .selections
            .iter()
            .map(|s| usize::from(s.field.0))
            .collect();
        memo.colts
            .push(Colt::new(View::Unbound, &selections, columns));
        memo.generation.push(None);
        memo.filters.push(Vec::new());
        memo.parked.push((0..PARKED_SLOTS).map(|_| None).collect());
        memo.spare_buffers.push(Vec::new());
    }
    memo
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
        self.check_snapshot(txn)?;
        let mut execute_span = obs::span(obs::names::EXECUTE, obs::Category::Execute);
        out.clear();
        out.arity = self.finds.len();
        {
            let _s = obs::span(obs::names::BIND_PARAMS, obs::Category::Execute);
            self.bind_params(txn, params)?;
        }
        match &self.plan {
            ExecPlan::GuardProbe(guard) => {
                // The point fast lane (docs/perf/ PRD 11): one probe, one
                // fetch, cells decoded straight into the buffer — no
                // sink, no bindings, no finalize pass. Aggregate-find
                // guards (rare) keep the sink path below.
                if self.guard_finds.is_some() {
                    let result = self.execute_guard_direct(txn, out);
                    execute_span.set_args(out.len() as u64, 0);
                    return result;
                }
                self.sink.reset();
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
                self.sink.reset();
                let resolved = {
                    let _s = obs::span(obs::names::RESOLVE_FILTERS, obs::Category::Execute);
                    resolve_predicates(
                        txn,
                        plan,
                        &self.resolved_params,
                        &self.missed_params,
                        &mut self.resolved_filters,
                        &mut self.resolved_selections,
                    )?
                };
                if !resolved {
                    return Ok(()); // Eq-anchored dictionary miss: empty result
                }
                // Phase attribution engages only under an active obs
                // capture (docs/architecture/50-validation.md): timing
                // runs — even obs builds — monomorphize NoopCounters and
                // pay nothing.
                macro_rules! run_join_with {
                    ($counters:expr) => {
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
                            &self.resolved_selections,
                            &mut self.memo,
                            &mut self.sink,
                            $counters,
                        )
                    };
                }
                #[cfg(feature = "trace")]
                if obs::capturing() {
                    let mut timers = crate::exec::run::PhaseTimers::new();
                    run_join_with!(&mut timers)?;
                    timers.flush();
                } else {
                    run_join_with!(&mut NoopCounters)?;
                }
                #[cfg(not(feature = "trace"))]
                run_join_with!(&mut NoopCounters)?;
            }
        }
        let result = {
            let _s = obs::span(obs::names::FINALIZE, obs::Category::Execute);
            finalize(
                &self.sink,
                &mut self.row_scratch,
                &mut self.resolve_memo,
                txn,
                &self.finds,
                self.all_words,
                out,
            )
        };
        execute_span.set_args(out.len() as u64, 0);
        result
    }

    /// The point fast lane's body (docs/perf/ PRD 11): probe + fetch +
    /// direct cell decode, no sink machinery.
    fn execute_guard_direct(&mut self, txn: &ReadTxn<'_>, out: &mut ResultBuffer) -> Result<()> {
        let ExecPlan::GuardProbe(guard) = &self.plan else {
            unreachable!("guard_finds implies a guard plan")
        };
        let guard_finds = self.guard_finds.as_ref().expect("checked by the caller");
        self.resolve_memo.clear();
        let Some(fact) = crate::exec::dispatch::guard_probe_fact(
            guard,
            txn,
            self.schema,
            &self.resolved_params,
            &mut self.guard_key,
        )?
        else {
            return Ok(());
        };
        out.cells.reserve(guard_finds.len());
        for (field, ty) in guard_finds {
            let word = crate::exec::dispatch::fact_word(self.schema, guard, fact, *field);
            match ty {
                ValueType::String | ValueType::Bytes => {
                    out.push_word(txn, ty, word, &mut self.resolve_memo)?;
                }
                _ => out.cells.push(ResultBuffer::word_cell(ty, word)),
            }
        }
        Ok(())
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
        self.check_snapshot(txn)?;
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
                let short_circuit = !resolve_predicates(
                    txn,
                    plan,
                    &self.resolved_params,
                    &self.missed_params,
                    &mut self.resolved_filters,
                    &mut self.resolved_selections,
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
                    &self.resolved_selections,
                    &mut self.memo,
                    &mut self.sink,
                    &mut counters,
                )?;
                finalize(
                    &self.sink,
                    &mut self.row_scratch,
                    &mut self.resolve_memo,
                    txn,
                    &self.finds,
                    self.all_words,
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
    /// Whether every plan node binds a sink-relevant variable — the
    /// pipelined executor's eligibility (docs/perf/ PRD 09); `None` for
    /// guard plans (no join runs at all).
    #[must_use]
    pub fn skip_free(&self) -> Option<bool> {
        match &self.plan {
            ExecPlan::FreeJoin(plan) => Some(plan.skip_free()),
            ExecPlan::GuardProbe(_) => None,
        }
    }

    /// Whether the plan proved distinct bindings (the aggregate sink's
    /// seen-set elision, 30-execution) — the regime observable for the
    /// batch-fold fast path (docs/perf/ PRD 02).
    #[must_use]
    pub fn distinct_bindings(&self) -> bool {
        self.plan.distinct_bindings()
    }

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

    /// The identity check at every execution entry (`execute` and
    /// `profile`; `execute_collect` and `explain` route through them):
    /// a snapshot of any environment other than the preparing one is a
    /// typed error before anything else runs. One u64 compare — with the
    /// entry guarded, the view memo needs no environment epoch in its
    /// generation keys.
    fn check_snapshot(&self, txn: &ReadTxn<'_>) -> Result<()> {
        if txn.env_instance() == self.env_instance {
            Ok(())
        } else {
            Err(Error::ForeignPreparedQuery)
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

/// Resolves every occurrence's symbolic predicate constants for this
/// execution — residual filters into `out_filters`, selection words into
/// `out_selections`. `Ok(false)` = a dictionary miss under an `Eq`
/// predicate (filter or selection), which empties the whole conjunctive
/// query (the short-circuit is sound for `Eq` only — a missed value
/// under `Ne` resolves to the sentinel id and matches everything).
fn resolve_predicates(
    txn: &ReadTxn<'_>,
    plan: &crate::plan::fj::ValidatedPlan,
    params: &[Const],
    missed: &[bool],
    out_filters: &mut [Vec<FilterPredicate>],
    out_selections: &mut [Vec<u64>],
) -> Result<bool> {
    for (occ_idx, occurrence) in plan.occurrences().iter().enumerate() {
        out_filters[occ_idx].clear();
        for filter in &occurrence.filters {
            let Some(resolved) = resolve_filter(txn, filter, params, missed)? else {
                return Ok(false);
            };
            out_filters[occ_idx].push(resolved);
        }
        out_selections[occ_idx].clear();
        for selection in &occurrence.selections {
            let Some(word) = resolve_selection(txn, selection, params, missed)? else {
                return Ok(false);
            };
            out_selections[occ_idx].push(word);
        }
    }
    Ok(true)
}

/// Resolves one selection's constant to the column word its trie level
/// probes with. `Ok(None)` = a dictionary miss — the Eq short-circuit.
fn resolve_selection(
    txn: &ReadTxn<'_>,
    selection: &crate::plan::fj::Selection,
    params: &[Const],
    missed: &[bool],
) -> Result<Option<u64>> {
    let word_of = |constant: &Const| match constant {
        Const::Word(w) => *w,
        Const::Byte(b) => u64::from(*b),
        Const::Param(_) | Const::PendingIntern { .. } => {
            unreachable!("bind_param resolves params to column form")
        }
    };
    Ok(match &selection.value {
        Const::Word(w) => Some(*w),
        Const::Byte(b) => Some(u64::from(*b)),
        Const::Param(p) => {
            if missed[usize::from(p.0)] {
                None
            } else {
                Some(word_of(&params[usize::from(p.0)]))
            }
        }
        Const::PendingIntern { tag, bytes } => dict::lookup_tagged(txn, *tag, bytes)?,
    })
}

/// How many (generation, resolved residual filters) bindings each
/// occurrence memoizes: the active one plus [`PARKED_SLOTS`] parked.
/// Four covers the bench rotation and the handful of bindings real
/// workloads repeat; memory is bounded by four COLT high-waters per
/// occurrence per prepared query — the explicit trade (docs/architecture/30-execution.md).
const MEMO_SLOTS: usize = 4;
const PARKED_SLOTS: usize = MEMO_SLOTS - 1;

/// One parked view binding: a COLT (with its view and forced tries)
/// keyed by the (generation, resolved residual filters) it was built
/// for. Swapped — never cloned — with the active binding on a hit.
/// Parked bindings always carry a real generation: only executed
/// bindings park (prepare leaves every slot empty).
struct ParkedView {
    generation: u64,
    filters: Vec<FilterPredicate>,
    colt: Colt,
    last_used: u64,
}

/// The per-occurrence view memo (docs/architecture/30-execution.md): generational
/// immutability makes a memoized view provably valid for its whole
/// generation, so repeated residual bindings (range windows, Ne
/// constants) skip the rebuild scan entirely. Occurrences whose only
/// predicates are selections never park — their single binding hits on
/// generation alone (docs/architecture/30-execution.md).
struct ViewMemo {
    /// The executor-facing COLTs: each occurrence's *active* binding
    /// (over [`View::Unbound`] until the first execution — prepare pins
    /// no image).
    colts: Vec<Colt>,
    /// The active binding's generation, per occurrence (`None` =
    /// unbound).
    generation: Vec<Option<u64>>,
    /// The active binding's resolved residual filters, per occurrence.
    filters: Vec<Vec<FilterPredicate>>,
    /// Parked bindings, [`PARKED_SLOTS`] per occurrence, empty at
    /// prepare, LRU-evicted, stale-reaped at each bind (a below-current
    /// generation can never hit again — dropping it frees its COLT pools
    /// and its image Arc at the first post-commit execution).
    parked: Vec<Vec<Option<ParkedView>>>,
    /// Spare survivor buffers recycled through rebuilds.
    spare_buffers: Vec<Vec<u32>>,
    /// The LRU clock, ticked once per execution.
    tick: u64,
}

impl ViewMemo {
    /// Binds `occ`'s active slot to `(generation, filters)`: an active
    /// hit is free, a parked hit swaps in, and a miss parks the active
    /// binding (into an empty slot first, else the LRU victim) and
    /// reports `false` so the caller rebuilds in place.
    fn bind(&mut self, occ: usize, generation: u64, filters: &[FilterPredicate]) -> bool {
        // Stale reaping first: generations only advance, so a parked
        // binding below this one is provably unhittable — drop it, its
        // pools, and its image Arc. Fires only when the generation moved
        // (within a generation every parked entry is current), so the
        // zero-alloc/zero-dealloc discipline of the warm window holds.
        for slot in &mut self.parked[occ] {
            if slot
                .as_ref()
                .is_some_and(|parked| parked.generation < generation)
            {
                *slot = None;
            }
        }
        if self.generation[occ] == Some(generation) && self.filters[occ] == filters {
            return true;
        }
        if let Some(slot) = self.parked[occ].iter().position(|slot| {
            slot.as_ref()
                .is_some_and(|parked| parked.generation == generation && parked.filters == filters)
        }) {
            let parked = self.parked[occ][slot].as_mut().expect("matched Some above");
            std::mem::swap(&mut self.colts[occ], &mut parked.colt);
            std::mem::swap(&mut self.filters[occ], &mut parked.filters);
            // A parked entry exists only after a same-generation park, so
            // the outgoing active binding is bound (post-reap both sides
            // are at `generation`; the swap just rotates which is active).
            let outgoing = self.generation[occ]
                .replace(parked.generation)
                .expect("a parked hit implies an executed active binding");
            parked.generation = outgoing;
            parked.last_used = self.tick;
            return true;
        }
        // A current-generation active binding is still hittable — park it
        // into an empty slot (first park constructs the ParkedView inside
        // the sanctioned view-rebuild window), else over the LRU victim
        // (post-reap every survivor is current-generation, so LRU is the
        // whole policy). A stale or unbound active can never hit again:
        // rebuild it in place (zero-residual occurrences always land
        // here, so their parked slots stay empty forever).
        if self.generation[occ] == Some(generation) {
            if let Some(empty) = self.parked[occ].iter().position(Option::is_none) {
                let fresh = self.colts[occ].unbound_sibling();
                self.parked[occ][empty] = Some(ParkedView {
                    generation,
                    filters: std::mem::take(&mut self.filters[occ]),
                    colt: std::mem::replace(&mut self.colts[occ], fresh),
                    last_used: self.tick,
                });
                self.generation[occ] = None;
            } else if let Some(victim) = self.parked[occ]
                .iter_mut()
                .flatten()
                .min_by_key(|parked| parked.last_used)
            {
                std::mem::swap(&mut self.colts[occ], &mut victim.colt);
                std::mem::swap(&mut self.filters[occ], &mut victim.filters);
                victim.generation = generation;
                victim.last_used = self.tick;
            }
        }
        false
    }
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
    resolved_selections: &[Vec<u64>],
    memo: &mut ViewMemo,
    sink: &mut EitherSink,
    counters: &mut C,
) -> Result<()> {
    let views_span = obs::span(obs::names::VIEWS, obs::Category::Execute);
    let generation = txn.generation()?;
    memo.tick += 1;
    // Lowering routes every Eq-constant into selections; a leak here would
    // silently resurrect the per-param view scan (docs/architecture/30-execution.md).
    debug_assert!(
        resolved_filters.iter().flatten().all(|f| !matches!(
            f,
            FilterPredicate::Compare {
                op: crate::ir::CmpOp::Eq,
                ..
            }
        )),
        "Eq-constant predicates never reach view filters"
    );
    for (occ_idx, occurrence) in plan.occurrences().iter().enumerate() {
        // Warm fast path: an active or parked binding for this exact
        // (generation, resolved residual filters) pair — the COLT's view
        // is still exactly right, and so are its forced tries (selections
        // live in the trie, not the view, so param churn never lands
        // here). No cache lock, no filter scan, no re-force.
        if memo.bind(occ_idx, generation, &resolved_filters[occ_idx]) {
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
        let buffer = std::mem::take(&mut memo.spare_buffers[occ_idx]);
        let view = apply(&image, &resolved_filters[occ_idx], &[], buffer);
        build_span.set_args(occ_idx as u64, view.len() as u64);
        let old = memo.colts[occ_idx].reset(view);
        memo.spare_buffers[occ_idx] = old.recycle();
        memo.generation[occ_idx] = Some(generation);
        memo.filters[occ_idx].clone_from(&resolved_filters[occ_idx]);
    }
    views_span.end();
    // Selection probes (docs/architecture/30-execution.md): each occurrence's Eq constants
    // resolve to trie keys probed once per execution — a miss means no
    // fact matches, so the whole conjunctive query is empty and the join
    // never runs (the sink stays reset: a zero-emit execution).
    for (occ_idx, keys) in resolved_selections.iter().enumerate() {
        let hit = memo.colts[occ_idx].select(keys).is_some();
        obs::event(
            obs::names::SELECT_PROBE,
            obs::Category::Execute,
            occ_idx as u64,
            u64::from(hit),
        );
        if !hit {
            return Ok(());
        }
    }
    let _join = obs::span(obs::names::JOIN, obs::Category::Execute);
    // One match per execution: the executor monomorphizes per concrete
    // sink type — no per-emit enum branch on the hot path.
    match sink {
        EitherSink::Projection(s) => executor.execute(plan, &mut memo.colts, bindings, s, counters),
        EitherSink::Aggregate(s) => {
            executor.execute(plan, &mut memo.colts, bindings, s.as_mut(), counters);
        }
    }
    Ok(())
}

/// The guard fast lane's find table (docs/perf/ PRD 11): `Some` for
/// guard plans whose finds are all plain variables.
fn guard_find_table(
    exec_plan: &ExecPlan,
    finds: &[(FindSpec, ValueType)],
) -> Option<Vec<(crate::schema::FieldId, ValueType)>> {
    match exec_plan {
        ExecPlan::GuardProbe(guard) => finds
            .iter()
            .map(|(spec, ty)| match spec {
                FindSpec::Var { slot } => Some((guard.vars[*slot].0, ty.clone())),
                FindSpec::Agg { .. } => None, // aggregate guards keep the sink path
            })
            .collect::<Option<Vec<_>>>(),
        ExecPlan::FreeJoin(_) => None,
    }
}

/// Builds the sink matching the find shape (the variant is fixed per
/// prepared query — an enum, not `dyn`).
fn make_sink(
    finds: &[(FindSpec, ValueType)],
    slot_count: usize,
    distinct: bool,
    hint: usize,
) -> EitherSink {
    let has_aggregates = finds
        .iter()
        .any(|(spec, _)| matches!(spec, FindSpec::Agg { .. }));
    if has_aggregates {
        EitherSink::Aggregate(Box::new(AggregateSink::with_capacity_hint(
            finds.iter().map(|(spec, _)| *spec).collect(),
            slot_count,
            distinct,
            hint,
        )))
    } else {
        EitherSink::Projection(ProjectionSink::with_capacity_hint(
            finds
                .iter()
                .map(|(spec, _)| match spec {
                    FindSpec::Var { slot } => *slot,
                    FindSpec::Agg { .. } => unreachable!("no aggregates here"),
                })
                .collect(),
            hint,
        ))
    }
}

/// The two sink shapes behind one monomorphized dispatch (an enum, not
/// `dyn` — the variant is fixed per prepared query).
#[allow(clippy::large_enum_variant)] // Projection stays unboxed: it is
// the hot variant (per-item emit paths reach through it), one prepared
// query holds exactly one sink, and the pipeline scratch rows
// (docs/silicon/04) that tripped the lint are the working set itself.
enum EitherSink {
    Projection(ProjectionSink),
    /// Boxed: the batch-fold scratch (PRD 02) grew the sink past the
    /// variant-size lint; one prepared query holds one sink, and the
    /// indirection is paid once per batch, never per row.
    Aggregate(Box<AggregateSink>),
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

    fn emit_batch(
        &mut self,
        batch: &crate::exec::run::LeafBatch<'_>,
        stop_on_skip: bool,
    ) -> crate::exec::run::Flow {
        match self {
            Self::Projection(sink) => sink.emit_batch(batch, stop_on_skip),
            Self::Aggregate(sink) => sink.emit_batch(batch, stop_on_skip),
        }
    }

    fn may_skip(&self) -> bool {
        match self {
            Self::Projection(sink) => sink.may_skip(),
            Self::Aggregate(sink) => sink.may_skip(),
        }
    }

    fn begin_scan(&mut self, scan: &crate::exec::run::LeafScan<'_>) -> bool {
        match self {
            Self::Projection(sink) => sink.begin_scan(scan),
            Self::Aggregate(sink) => sink.begin_scan(scan),
        }
    }

    fn scan_run(
        &mut self,
        scan: &crate::exec::run::LeafScan<'_>,
        run: crate::exec::colt::SuffixRun<'_>,
    ) {
        match self {
            Self::Projection(sink) => sink.scan_run(scan, run),
            Self::Aggregate(sink) => sink.scan_run(scan, run),
        }
    }

    fn end_scan(&mut self, scan: &crate::exec::run::LeafScan<'_>) -> u64 {
        match self {
            Self::Projection(sink) => sink.end_scan(scan),
            Self::Aggregate(sink) => sink.end_scan(scan),
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
/// every stored value). The `Eq` arms here are unreachable through the
/// production pipeline — `split_filters` routes every Eq-constant into
/// selections — and stay as belt-and-braces for the same reason
/// `check_selections` exists: `PlanOccurrence` is plain data.
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

/// Drains the sink into the result buffer, decoding words by result type
/// (each distinct intern resolved once, docs/architecture/30-execution.md).
fn finalize(
    sink: &EitherSink,
    row_scratch: &mut Vec<u64>,
    memo: &mut ResolveMemo,
    txn: &ReadTxn<'_>,
    finds: &[(FindSpec, ValueType)],
    all_words: bool,
    out: &mut ResultBuffer,
) -> Result<()> {
    memo.clear();
    // The all-words fast path (docs/perf/ PRD 08): one reservation, then
    // infallible cell writes — no Result, no dictionary plumbing per
    // cell. Interned finds keep the resolving path (the per-cell memo
    // probe is the resolution semantics, softened by the run memo).
    match sink {
        EitherSink::Projection(sink) => {
            out.cells.reserve(sink.len() * finds.len());
            if all_words {
                for row in sink.rows() {
                    for (column, (_, ty)) in finds.iter().enumerate() {
                        out.cells.push(ResultBuffer::word_cell(ty, row[column]));
                    }
                }
                return Ok(());
            }
            for row in sink.rows() {
                for (column, (_, ty)) in finds.iter().enumerate() {
                    out.push_word(txn, ty, row[column], memo)?;
                }
            }
            Ok(())
        }
        EitherSink::Aggregate(sink) => {
            out.cells.reserve(sink.group_count() * finds.len());
            if all_words {
                return sink.finalize_into(row_scratch, |row| {
                    for (column, (_, ty)) in finds.iter().enumerate() {
                        out.cells.push(ResultBuffer::word_cell(ty, row[column]));
                    }
                    Ok(())
                });
            }
            sink.finalize_into(row_scratch, |row| {
                for (column, (_, ty)) in finds.iter().enumerate() {
                    out.push_word(txn, ty, row[column], memo)?;
                }
                Ok(())
            })
        }
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

    /// PRD 09 (docs/hardening): u64 ordered comparisons and cross-atom
    /// residuals — the generator's new constructs — each pinned against
    /// an independent nested-loop reference, no `SQLite` in sight.
    #[test]
    fn u64_ranges_and_cross_atom_residuals_match_nested_loops() {
        let dir = TempDir::new("prepared-new-construct-differential");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let rows: &[(u64, u64, &str, i64)] = &[
            (1, 3, "a", 10),
            (2, 3, "b", 25),
            (3, 7, "c", 25),
            (4, 7, "d", 40),
            (5, 9, "e", -5),
            (6, 9, "f", 40),
        ];
        insert_postings(&env, &schema, rows);
        let cache = ImageCache::new();
        let txn = env.read_txn().expect("txn");

        // Q(id) :- Posting(id, account = a), a >= 7 — a u64 ordered
        // comparison over the dense id domain.
        let range = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                relation: POSTING,
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            }],
            predicates: vec![Comparison {
                op: CmpOp::Ge,
                lhs: Term::Var(VarId(1)),
                rhs: Term::Literal(Value::U64(7)),
            }],
        };
        let mut prepared = prepare(&txn, &cache, &schema, &range).expect("prepare");
        let out = prepared
            .execute_collect(&txn, &cache, &[])
            .expect("execute");
        let mut got: Vec<u64> = (0..out.len())
            .map(|row| match out.get(row, 0) {
                ResultValue::U64(id) => id,
                other => panic!("column 0 is u64: {other:?}"),
            })
            .collect();
        got.sort_unstable();
        let mut expected: Vec<u64> = rows.iter().filter(|r| r.1 >= 7).map(|r| r.0).collect();
        expected.sort_unstable();
        assert_eq!(got, expected, "u64 ordered comparison");

        // Q(x, y) :- Posting(account = k, amount = x),
        //            Posting(account = k, amount = y), x < y — the
        // cross-atom residual, checked by nested loop.
        let spread = Query {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            atoms: vec![
                Atom {
                    relation: POSTING,
                    bindings: vec![
                        (FieldId(1), Term::Var(VarId(2))),
                        (FieldId(3), Term::Var(VarId(0))),
                    ],
                },
                Atom {
                    relation: POSTING,
                    bindings: vec![
                        (FieldId(1), Term::Var(VarId(2))),
                        (FieldId(3), Term::Var(VarId(1))),
                    ],
                },
            ],
            predicates: vec![Comparison {
                op: CmpOp::Lt,
                lhs: Term::Var(VarId(0)),
                rhs: Term::Var(VarId(1)),
            }],
        };
        let mut prepared = prepare(&txn, &cache, &schema, &spread).expect("prepare");
        let out = prepared
            .execute_collect(&txn, &cache, &[])
            .expect("execute");
        let mut got: Vec<(i64, i64)> = (0..out.len())
            .map(|row| match (out.get(row, 0), out.get(row, 1)) {
                (ResultValue::I64(x), ResultValue::I64(y)) => (x, y),
                other => panic!("two i64 columns: {other:?}"),
            })
            .collect();
        got.sort_unstable();
        let mut expected = std::collections::BTreeSet::new();
        for p1 in rows {
            for p2 in rows {
                if p1.1 == p2.1 && p1.3 < p2.3 {
                    expected.insert((p1.3, p2.3));
                }
            }
        }
        assert_eq!(
            got,
            expected.into_iter().collect::<Vec<_>>(),
            "cross-atom residual"
        );
    }

    /// PRD 11 (docs/perf/): the guard fast lane — hit, miss, and a
    /// param-type error, with an interned find exercising the resolving
    /// column beside the word blits.
    #[test]
    fn guard_fast_lane_hits_misses_and_type_errors() {
        let dir = TempDir::new("prepared-guard-lane");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        insert_postings(&env, &schema, &[(1, 7, "memo-a", 41), (2, 8, "memo-b", 42)]);
        // Q(account, memo, amount) :- Posting(id = ?0, account, memo, amount).
        let query = Query {
            finds: vec![
                FindTerm::Var(VarId(0)),
                FindTerm::Var(VarId(1)),
                FindTerm::Var(VarId(2)),
            ],
            atoms: vec![Atom {
                relation: POSTING,
                bindings: vec![
                    (FieldId(0), Term::Param(crate::ir::ParamId(0))),
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(2), Term::Var(VarId(1))),
                    (FieldId(3), Term::Var(VarId(2))),
                ],
            }],
            predicates: vec![],
        };
        let txn = env.read_txn().expect("txn");
        let cache = crate::image::cache::ImageCache::new();
        let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepares");
        assert!(
            prepared.guard_finds.is_some(),
            "plain-variable guard takes the fast lane"
        );
        let mut out = ResultBuffer::new();
        // Hit: every cell decoded straight from the fact.
        prepared
            .execute(&txn, &cache, &[crate::ir::Value::U64(2)], &mut out)
            .expect("hit");
        assert_eq!(out.len(), 1);
        assert_eq!(out.get(0, 0), ResultValue::U64(8));
        assert_eq!(out.get(0, 1), ResultValue::String("memo-b"));
        assert_eq!(out.get(0, 2), ResultValue::I64(42));
        // Miss: clean empty buffer.
        prepared
            .execute(&txn, &cache, &[crate::ir::Value::U64(999)], &mut out)
            .expect("miss is empty, not an error");
        assert_eq!(out.len(), 0);
        // Param-type error: typed, before any probe.
        let err = prepared
            .execute(&txn, &cache, &[crate::ir::Value::Bool(true)], &mut out)
            .expect_err("type mismatch");
        assert!(matches!(err, Error::ParamTypeMismatch { .. }), "{err:?}");
    }

    /// PRD 08 (docs/perf/): a finalize-time Overflow leaves the buffer
    /// discardable — the same prepared query re-executes cleanly into
    /// the same buffer (deterministic error), and a passing query then
    /// fills that buffer with exactly its own rows.
    #[test]
    fn overflow_errors_leave_the_buffer_reusable() {
        let dir = TempDir::new("prepared-overflow-reuse");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        insert_postings(
            &env,
            &schema,
            &[(1, 7, "a", i64::MAX), (2, 7, "b", 1), (3, 8, "c", 4)],
        );
        // Sum by account: account 7 overflows at finalize.
        let query = Query {
            finds: vec![
                FindTerm::Var(VarId(0)),
                FindTerm::Aggregate {
                    op: crate::ir::AggOp::Sum,
                    over: Some(VarId(1)),
                },
            ],
            atoms: vec![Atom {
                relation: POSTING,
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(3), Term::Var(VarId(1))),
                ],
            }],
            predicates: vec![],
        };
        let txn = env.read_txn().expect("txn");
        let cache = crate::image::cache::ImageCache::new();
        let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepares");
        let mut out = ResultBuffer::new();
        for _ in 0..2 {
            let err = prepared
                .execute(&txn, &cache, &[], &mut out)
                .expect_err("account 7 overflows");
            assert!(matches!(err, Error::Overflow { find: 1 }), "{err:?}");
        }
        // A passing query fills the same buffer with exactly its rows.
        let ok_query = Query {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            atoms: vec![Atom {
                relation: POSTING,
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(3), Term::Var(VarId(1))),
                ],
            }],
            predicates: vec![Comparison {
                op: CmpOp::Eq,
                lhs: Term::Var(VarId(0)),
                rhs: Term::Literal(crate::ir::Value::U64(8)),
            }],
        };
        let mut ok = prepare(&txn, &cache, &schema, &ok_query).expect("prepares");
        ok.execute(&txn, &cache, &[], &mut out).expect("executes");
        assert_eq!(out.len(), 1);
        assert_eq!(out.get(0, 0), ResultValue::U64(8));
        assert_eq!(out.get(0, 1), ResultValue::I64(4));
    }

    /// PRD 05 (docs/hardening): an aggregate whose body has a node
    /// binding only existential (non-projected, non-aggregated)
    /// variables folds every distinct full binding — pinned against an
    /// independent nested-loop reference. The plan's sink-relevance bits
    /// mark every variable-binding node relevant under aggregation, so
    /// no suffix skip can ever starve the fold.
    #[test]
    fn aggregates_fold_every_binding_of_existential_suffixes() {
        let dir = TempDir::new("prepared-agg-existential");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let rows: &[(u64, u64, &str, i64)] = &[
            (1, 7, "a", 10),
            (2, 7, "b", 10),
            (3, 7, "c", 20),
            (4, 8, "z", 5),
        ];
        insert_postings(&env, &schema, rows);

        // Q(x, Sum(y)) :- Posting(account = x, amount = y),
        //                 Posting(account = x, memo = m)
        // — m is existential; the self-join's second occurrence opens a
        // node binding only m.
        let query = Query {
            finds: vec![
                FindTerm::Var(VarId(0)),
                FindTerm::Aggregate {
                    op: crate::ir::AggOp::Sum,
                    over: Some(VarId(1)),
                },
            ],
            atoms: vec![
                Atom {
                    relation: POSTING,
                    bindings: vec![
                        (FieldId(1), Term::Var(VarId(0))),
                        (FieldId(3), Term::Var(VarId(1))),
                    ],
                },
                Atom {
                    relation: POSTING,
                    bindings: vec![
                        (FieldId(1), Term::Var(VarId(0))),
                        (FieldId(2), Term::Var(VarId(2))),
                    ],
                },
            ],
            predicates: vec![],
        };

        // The nested-loop reference over distinct (x, y, m) bindings.
        let mut bindings = std::collections::BTreeSet::new();
        for p1 in rows {
            for p2 in rows {
                if p1.1 == p2.1 {
                    bindings.insert((p1.1, p1.3, p2.2));
                }
            }
        }
        let mut expected = std::collections::BTreeMap::new();
        for (x, y, _) in &bindings {
            *expected.entry(*x).or_insert(0i64) += y;
        }

        let cache = ImageCache::new();
        let txn = env.read_txn().expect("txn");
        let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
        let out = prepared
            .execute_collect(&txn, &cache, &[])
            .expect("execute");
        let mut got: Vec<(u64, i64)> = (0..out.len())
            .map(|row| {
                let ResultValue::U64(account) = out.get(row, 0) else {
                    panic!("column 0 is u64");
                };
                let ResultValue::I64(sum) = out.get(row, 1) else {
                    panic!("column 1 is i64");
                };
                (account, sum)
            })
            .collect();
        got.sort_unstable();
        assert_eq!(got, expected.into_iter().collect::<Vec<_>>());
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

    /// Q(amount) :- Posting(memo = ?0, amount) — the selection shape
    /// (docs/architecture/30-execution.md): a param-Eq on a non-unique field.
    fn by_memo_query() -> Query {
        Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                relation: POSTING,
                bindings: vec![
                    (FieldId(2), Term::Param(crate::ir::ParamId(0))),
                    (FieldId(3), Term::Var(VarId(0))),
                ],
            }],
            predicates: vec![],
        }
    }

    fn memo_param(text: &str) -> Vec<Value> {
        vec![Value::String(Box::from(text.as_bytes()))]
    }

    fn amounts_of(buffer: &ResultBuffer) -> Vec<i64> {
        let mut amounts: Vec<i64> = (0..buffer.len())
            .map(|row| {
                let ResultValue::I64(amount) = buffer.get(row, 0) else {
                    panic!("column 0 is an i64");
                };
                amount
            })
            .collect();
        amounts.sort_unstable();
        amounts
    }

    /// The differential pin for the selection cutover (docs/architecture/30-execution.md):
    /// rotating Eq params across many executions, every result compared
    /// against a nested-loop filter over the inserted rows.
    #[test]
    fn selection_params_rotate_differentially() {
        let dir = TempDir::new("prepared-select-diff");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        // Seeded rows over 8 memo values, amounts unique per row.
        let mut state = 0xDEAD_BEEF_u64;
        let mut next = move || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            state >> 33
        };
        let rows: Vec<(u64, u64, String, i64)> = (0..200)
            .map(|id| {
                let memo = format!("m{}", next() % 8);
                let amount = i64::try_from(id).expect("fits") * 3 - 100;
                (id, next() % 5, memo, amount)
            })
            .collect();
        let borrowed: Vec<(u64, u64, &str, i64)> = rows
            .iter()
            .map(|(id, account, memo, amount)| (*id, *account, memo.as_str(), *amount))
            .collect();
        insert_postings(&env, &schema, &borrowed);
        let cache = ImageCache::new();
        let txn = env.read_txn().expect("txn");
        let mut prepared = prepare(&txn, &cache, &schema, &by_memo_query()).expect("prepare");
        for cycle in 0..3 {
            for m in 0..8 {
                let memo = format!("m{m}");
                let out = prepared
                    .execute_collect(&txn, &cache, &memo_param(&memo))
                    .expect("execute");
                let mut expected: Vec<i64> = rows
                    .iter()
                    .filter(|(_, _, row_memo, _)| *row_memo == memo)
                    .map(|(_, _, _, amount)| *amount)
                    .collect();
                expected.sort_unstable();
                expected.dedup();
                assert_eq!(
                    amounts_of(&out),
                    expected,
                    "cycle {cycle}, memo {memo} diverges from the nested loop"
                );
            }
        }
        // The never-interned miss stays the empty set.
        let out = prepared
            .execute_collect(&txn, &cache, &memo_param("never-stored"))
            .expect("execute");
        assert!(out.is_empty());
    }

    /// Counters pin (docs/architecture/30-execution.md): a selection's work is O(selected),
    /// never O(relation).
    #[test]
    fn selection_work_is_o_selected() {
        let dir = TempDir::new("prepared-select-counters");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        // 20 rows, exactly 4 carrying the hot memo, distinct amounts.
        let rows: Vec<(u64, u64, String, i64)> = (0..20)
            .map(|id| {
                let memo = if id % 5 == 0 {
                    "hot".to_owned()
                } else {
                    format!("cold-{id}")
                };
                (id, id % 3, memo, i64::try_from(id).expect("fits") * 7)
            })
            .collect();
        let borrowed: Vec<(u64, u64, &str, i64)> = rows
            .iter()
            .map(|(id, account, memo, amount)| (*id, *account, memo.as_str(), *amount))
            .collect();
        insert_postings(&env, &schema, &borrowed);
        let cache = ImageCache::new();
        let txn = env.read_txn().expect("txn");
        let mut prepared = prepare(&txn, &cache, &schema, &by_memo_query()).expect("prepare");
        let (out, stats) = prepared
            .profile(&txn, &cache, &memo_param("hot"))
            .expect("profile");
        assert_eq!(out.len(), 4);
        let drawn: u64 = stats.nodes.iter().map(|n| n.batch_entries).sum();
        assert_eq!(drawn, 4, "work is O(selected), not O(relation): {stats:?}");
    }

    /// Finalize resolves each distinct intern once per finalize and
    /// stores its bytes once per buffer (docs/architecture/30-execution.md).
    #[cfg(feature = "trace")]
    #[test]
    fn finalize_resolves_each_distinct_intern_once() {
        use crate::obs;

        let dir = TempDir::new("prepared-resolve-memo");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        // 64 rows sharing one memo (distinct amounts keep the rows
        // distinct under set semantics), plus 16 rows over 16 memos.
        let rows: Vec<(u64, u64, String, i64)> = (0..64)
            .map(|id| {
                (
                    id,
                    1,
                    "shared-memo".to_owned(),
                    i64::try_from(id).expect("fits"),
                )
            })
            .chain((0..16).map(|i| (64 + i, 2, format!("m{i}"), i64::try_from(i).expect("fits"))))
            .collect();
        let borrowed: Vec<(u64, u64, &str, i64)> = rows
            .iter()
            .map(|(id, account, memo, amount)| (*id, *account, memo.as_str(), *amount))
            .collect();
        insert_postings(&env, &schema, &borrowed);
        let cache = ImageCache::new();
        let txn = env.read_txn().expect("txn");
        let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");

        let resolves = |prepared: &mut PreparedQuery<'_>, account: u64| {
            obs::start_capture();
            let out = prepared
                .execute_collect(&txn, &cache, &[Value::U64(account), Value::I64(-1)])
                .expect("execute");
            let events = obs::finish_capture();
            let count = events
                .iter()
                .filter(|e| e.name == obs::names::DICT_RESOLVE)
                .count();
            (out, count)
        };

        // 64 rows, one distinct memo: one resolution, one byte copy.
        let (out, count) = resolves(&mut prepared, 1);
        assert_eq!(out.len(), 64);
        assert_eq!(count, 1, "one distinct intern, one resolution");
        assert_eq!(out.byte_len(), "shared-memo".len(), "bytes stored once");

        // 16 rows over 16 memos: sixteen resolutions.
        let (out, count) = resolves(&mut prepared, 2);
        assert_eq!(out.len(), 16);
        assert_eq!(count, 16);
        // A second execution memoizes per finalize, not across them.
        let (_, count) = resolves(&mut prepared, 2);
        assert_eq!(count, 16, "the memo clears per finalize");
    }

    /// PRD 02 (docs/hardening): prepare pins no image — the refcount
    /// proof. Executions bind views; a commit plus one execution at the
    /// new generation reaps every stale binding, releasing the old
    /// image entirely (only the test's own Arc survives).
    #[test]
    fn prepare_pins_no_images_and_reaping_releases_them() {
        let dir = TempDir::new("prepared-unbound-views");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        insert_postings(&env, &schema, &[(1, 7, "a", 10), (2, 7, "b", 20)]);
        let cache = ImageCache::new();
        let txn = env.read_txn().expect("txn");
        let held = cache
            .get_or_build(&txn, &schema, POSTING)
            .expect("generation-1 image");
        let baseline = std::sync::Arc::strong_count(&held);

        let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
        assert_eq!(
            std::sync::Arc::strong_count(&held),
            baseline,
            "prepare pinned an image"
        );

        // Two residual windows: the active and one parked binding both
        // hold views over the generation-1 image.
        for floor in [-100, 15] {
            prepared
                .execute_collect(&txn, &cache, &[Value::U64(7), Value::I64(floor)])
                .expect("execute");
        }
        assert!(
            std::sync::Arc::strong_count(&held) > baseline,
            "executions bind real views"
        );
        drop(txn);

        // Commit generation 2 and evict, exactly as Db::write does; the
        // first execution at the new generation reaps the stale parked
        // binding and rebuilds the active one.
        insert_postings(&env, &schema, &[(3, 7, "c", 30)]);
        cache.evict_older_than(2);
        let txn = env.read_txn().expect("txn");
        prepared
            .execute_collect(&txn, &cache, &[Value::U64(7), Value::I64(-100)])
            .expect("execute at generation 2");
        assert_eq!(
            std::sync::Arc::strong_count(&held),
            1,
            "the prepared query holds nothing of generation 1"
        );
    }

    /// PRD 02: prepare on a cold cache builds no images — zero
    /// `image_build`/`cache_hit` events; the first execution pays the
    /// build exactly where a cold execution always paid it.
    #[cfg(feature = "trace")]
    #[test]
    fn prepare_emits_no_image_events() {
        use crate::obs;

        let dir = TempDir::new("prepared-no-image-events");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        insert_postings(&env, &schema, &[(1, 7, "a", 10)]);
        let cache = ImageCache::new();
        let txn = env.read_txn().expect("txn");

        obs::start_capture();
        let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
        let events = obs::finish_capture();
        let names: Vec<&str> = events.iter().map(|e| e.name).collect();
        assert!(
            !names.contains(&obs::names::IMAGE_BUILD),
            "prepare built an image: {names:?}"
        );
        assert!(
            !names.contains(&obs::names::CACHE_HIT),
            "prepare touched the image cache: {names:?}"
        );

        obs::start_capture();
        prepared
            .execute_collect(&txn, &cache, &[Value::U64(7), Value::I64(-100)])
            .expect("execute");
        let events = obs::finish_capture();
        let names: Vec<&str> = events.iter().map(|e| e.name).collect();
        assert!(
            names.contains(&obs::names::IMAGE_BUILD),
            "the first execution pays the build: {names:?}"
        );
    }

    /// The view-memo LRU (docs/architecture/30-execution.md): four rotating residual bindings
    /// all memoize; a fifth evicts exactly the least recently used.
    #[cfg(feature = "trace")]
    #[test]
    fn residual_bindings_memoize_under_lru() {
        use crate::obs;

        let dir = TempDir::new("prepared-lru-trace");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        insert_postings(
            &env,
            &schema,
            &[
                (1, 7, "a", 10),
                (2, 7, "b", 20),
                (3, 7, "c", 30),
                (4, 7, "d", 40),
                (5, 7, "e", 50),
                (6, 7, "f", 60),
            ],
        );
        let cache = ImageCache::new();
        let txn = env.read_txn().expect("txn");
        let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
        let params = |floor: i64| vec![Value::U64(7), Value::I64(floor)];
        let windows = [-100, 15, 25, 35];

        let mut run = |floor: i64| -> (usize, usize, Vec<(String, i64)>) {
            obs::start_capture();
            let out = prepared
                .execute_collect(&txn, &cache, &params(floor))
                .expect("execute");
            let events = obs::finish_capture();
            let builds = events
                .iter()
                .filter(|e| e.name == obs::names::VIEW_BUILD)
                .count();
            let hits = events
                .iter()
                .filter(|e| e.name == obs::names::VIEW_MEMO_HIT)
                .count();
            (builds, hits, rows_of(&out))
        };
        let expected = |floor: i64| -> Vec<(String, i64)> {
            let rows = [
                ("a", 10),
                ("b", 20),
                ("c", 30),
                ("d", 40),
                ("e", 50),
                ("f", 60),
            ];
            let mut expected: Vec<(String, i64)> = rows
                .iter()
                .filter(|(_, amount)| *amount >= floor)
                .map(|(memo, amount)| ((*memo).to_owned(), *amount))
                .collect();
            expected.sort_unstable();
            expected
        };

        // First cycle: every window builds once (differentially checked).
        for floor in windows {
            let (builds, _, rows) = run(floor);
            assert_eq!(builds, 1, "first sight of window {floor} builds");
            assert_eq!(rows, expected(floor));
        }
        // Second cycle: every window hits — active or parked.
        for floor in windows {
            let (builds, hits, rows) = run(floor);
            assert_eq!(builds, 0, "window {floor} memoized");
            assert_eq!(hits, 1);
            assert_eq!(rows, expected(floor));
        }
        // A fifth window evicts the least recently used (floor -100).
        let (builds, _, rows) = run(45);
        assert_eq!(builds, 1, "the fifth binding builds");
        assert_eq!(rows, expected(45));
        // The most recent of the old four still hits...
        let (builds, hits, _) = run(35);
        assert_eq!((builds, hits), (0, 1), "most recent old binding kept");
        // ...and the least recent was the eviction victim.
        let (builds, _, rows) = run(-100);
        assert_eq!(builds, 1, "least recent binding was evicted");
        assert_eq!(rows, expected(-100));
    }

    /// A generation bump invalidates every memoized binding, and the
    /// rebuilt view reflects the new fact.
    #[cfg(feature = "trace")]
    #[test]
    fn a_generation_bump_invalidates_the_memo() {
        use crate::obs;

        let dir = TempDir::new("prepared-lru-generation");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        insert_postings(&env, &schema, &[(1, 7, "old", 10)]);
        let cache = ImageCache::new();
        let txn = env.read_txn().expect("txn");
        let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
        let params = vec![Value::U64(7), Value::I64(0)];
        let out = prepared
            .execute_collect(&txn, &cache, &params)
            .expect("execute");
        assert_eq!(out.len(), 1);
        drop(txn);

        insert_postings(&env, &schema, &[(2, 7, "new", 20)]);
        let txn = env.read_txn().expect("txn");
        obs::start_capture();
        let out = prepared
            .execute_collect(&txn, &cache, &params)
            .expect("execute");
        let events = obs::finish_capture();
        assert!(
            events.iter().any(|e| e.name == obs::names::VIEW_BUILD),
            "the stale binding rebuilds in place"
        );
        assert_eq!(
            rows_of(&out),
            vec![("new".to_owned(), 20), ("old".to_owned(), 10)],
            "the rebuilt view carries the new fact"
        );
    }

    /// The scan is dead (docs/architecture/30-execution.md): rotating Eq params build the view
    /// once per generation; every later execution memo-hits and probes.
    #[cfg(feature = "trace")]
    #[test]
    fn selection_params_rotate_without_view_rebuilds() {
        use crate::obs;

        let dir = TempDir::new("prepared-select-trace");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        insert_postings(
            &env,
            &schema,
            &[
                (1, 0, "m0", 10),
                (2, 0, "m1", 20),
                (3, 0, "m2", 30),
                (4, 0, "m0", 40),
            ],
        );
        let cache = ImageCache::new();
        let txn = env.read_txn().expect("txn");
        let mut prepared = prepare(&txn, &cache, &schema, &by_memo_query()).expect("prepare");

        let mut view_builds = 0;
        let mut memo_hits = 0;
        for _cycle in 0..3 {
            for m in ["m0", "m1", "m2"] {
                obs::start_capture();
                let out = prepared
                    .execute_collect(&txn, &cache, &memo_param(m))
                    .expect("execute");
                let events = obs::finish_capture();
                assert!(!out.is_empty());
                view_builds += events
                    .iter()
                    .filter(|e| e.name == obs::names::VIEW_BUILD)
                    .count();
                memo_hits += events
                    .iter()
                    .filter(|e| e.name == obs::names::VIEW_MEMO_HIT)
                    .count();
                let probe = events
                    .iter()
                    .find(|e| e.name == obs::names::SELECT_PROBE)
                    .expect("every execution probes");
                assert_eq!(probe.a1, 1, "present keys hit");
            }
        }
        assert_eq!(view_builds, 1, "one view build per generation");
        assert_eq!(memo_hits, 8, "every later execution memo-hits");

        // A never-interned param short-circuits at resolve: no view work,
        // no probe, no join — the empty set.
        obs::start_capture();
        let out = prepared
            .execute_collect(&txn, &cache, &memo_param("never-stored"))
            .expect("execute");
        let events = obs::finish_capture();
        assert!(out.is_empty());
        let names: Vec<&str> = events.iter().map(|e| e.name).collect();
        assert!(!names.contains(&obs::names::VIEW_BUILD), "{names:?}");
        assert!(!names.contains(&obs::names::SELECT_PROBE), "{names:?}");
        assert!(!names.contains(&obs::names::JOIN), "{names:?}");
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
