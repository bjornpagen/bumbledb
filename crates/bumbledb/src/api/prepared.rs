//! Prepared queries, parameters, and results (PRD 25) — the reusable
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

use std::sync::Arc;

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

    fn push_word(&mut self, txn: &ReadTxn<'_>, ty: &ValueType, word: u64) -> Result<()> {
        let cell = match ty {
            ValueType::Bool => Cell::Bool(word != 0),
            ValueType::Enum { .. } => Cell::Enum(u8::try_from(word).expect("enum words fit u8")),
            ValueType::U64 => Cell::U64(word),
            ValueType::I64 => Cell::I64((word ^ (1 << 63)).cast_signed()),
            ValueType::String => {
                let raw = dict::resolve(txn, word)?;
                std::str::from_utf8(raw).map_err(|_| {
                    Error::Corruption(crate::error::CorruptionError::DanglingInternId(word))
                })?;
                let start = self.bytes.len();
                self.bytes.extend_from_slice(raw);
                Cell::String {
                    start,
                    len: raw.len(),
                }
            }
            ValueType::Bytes => {
                let raw = dict::resolve(txn, word)?;
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

/// The reusable execution object. `!Sync` by construction (interior
/// scratch); executes from one thread at a time; owns its scratch.
pub struct PreparedQuery<'s> {
    schema: &'s Schema,
    plan: ExecPlan,
    /// The Free Join executor scratch (unused for guard probes).
    executor: Option<Executor>,
    bindings: Bindings,
    /// Per find term: the output spec and its result type.
    finds: Vec<(FindSpec, ValueType)>,
    /// Dense per-param expected types (`None` = the id never appears).
    param_types: Vec<Option<ValueType>>,
    /// Bind-time resolved constants, reused across executions.
    resolved_params: Vec<Const>,
    /// Per occurrence: filters with symbolic constants substituted, reused.
    resolved_filters: Vec<Vec<FilterPredicate>>,
    /// Recycled survivor buffers, one per occurrence.
    survivor_buffers: Vec<Vec<u32>>,
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
pub fn prepare<'s>(
    txn: &ReadTxn<'_>,
    schema: &'s Schema,
    query: &Query,
) -> Result<PreparedQuery<'s>> {
    let witness = validate(schema, query)?;
    let normalized = normalize(&witness);

    // Classification first: a guard probe needs no statistics or planning.
    let exec_plan = if let Some(guard) = classify(&normalized, schema) {
        ExecPlan::GuardProbe(guard)
    } else {
        // Filtered-view statistics: measured survivor counts for
        // occurrences whose filters are fully concrete; base row counts
        // otherwise (symbolic filters resolve per execution).
        let mut stats = Vec::with_capacity(normalized.occurrences.len());
        for occurrence in &normalized.occurrences {
            let concrete = !occurrence.filters.is_empty()
                && occurrence.filters.iter().all(|f| match f {
                    FilterPredicate::Compare { value, .. } => {
                        matches!(value, Const::Word(_) | Const::Byte(_))
                    }
                    FilterPredicate::FieldsEqual { .. } => true,
                });
            let rows = if concrete {
                let image = crate::image::build(txn, schema, occurrence.relation)?;
                apply(&image, &occurrence.filters, &[], Vec::new()).len() as u64
            } else {
                read::row_count(txn, occurrence.relation)?
            };
            stats.push(OccStats {
                occ_id: occurrence.occ_id,
                rows,
            });
        }
        let order = plan_order(&normalized, schema, &stats)?;
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
        ExecPlan::FreeJoin(validated)
    };

    let finds = find_specs(query, &witness, &exec_plan);

    // Dense param typing for bind-time checks.
    let max_param = witness
        .param_types()
        .map(|(p, _)| usize::from(p.0) + 1)
        .max()
        .unwrap_or(0);
    let mut param_types: Vec<Option<ValueType>> = vec![None; max_param];
    for (param, ty) in witness.param_types() {
        param_types[usize::from(param.0)] = Some(ty.clone());
    }

    let (executor, slot_count, occurrence_count) = match &exec_plan {
        ExecPlan::FreeJoin(plan) => (
            Some(Executor::new(plan)),
            plan.slots().len(),
            plan.occurrences().len(),
        ),
        ExecPlan::GuardProbe(guard) => (None, guard.vars.len(), 1),
    };

    Ok(PreparedQuery {
        schema,
        plan: exec_plan,
        executor,
        bindings: Bindings::new(slot_count),
        finds,
        param_types,
        resolved_params: Vec::new(),
        resolved_filters: vec![Vec::new(); occurrence_count],
        survivor_buffers: (0..occurrence_count).map(|_| Vec::new()).collect(),
        _not_sync: std::marker::PhantomData,
    })
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
    pub fn execute(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        params: &[Value],
        out: &mut ResultBuffer,
    ) -> Result<()> {
        out.clear();
        out.arity = self.finds.len();
        if !self.bind_params(txn, params)? {
            return Ok(()); // dictionary miss: empty result
        }
        match &self.plan {
            ExecPlan::GuardProbe(guard) => {
                let mut sink = make_sink(&self.finds, self.bindings.slot_count(), true);
                execute_guard(
                    guard,
                    txn,
                    self.schema,
                    &self.resolved_params,
                    &mut self.bindings,
                    &mut sink.0,
                )?;
                finalize(sink, txn, &self.finds, out)
            }
            ExecPlan::FreeJoin(plan) => {
                if !resolve_filters(txn, plan, &self.resolved_params, &mut self.resolved_filters)? {
                    return Ok(()); // intern miss: empty result
                }
                let mut sink = make_sink(
                    &self.finds,
                    self.bindings.slot_count(),
                    plan.distinct_bindings(),
                );
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
                    &mut sink.0,
                    &mut NoopCounters,
                )?;
                finalize(sink, txn, &self.finds, out)
            }
        }
    }

    /// Convenience path: a fresh buffer per call.
    ///
    /// # Errors
    ///
    /// As [`Self::execute`].
    pub fn execute_collect(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        params: &[Value],
    ) -> Result<ResultBuffer> {
        let mut out = ResultBuffer::new();
        self.execute(txn, cache, params, &mut out)?;
        Ok(out)
    }

    /// EXPLAIN (PRD 24): executes the query with counting instrumentation
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
    pub fn explain(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        params: &[Value],
    ) -> Result<(ResultBuffer, String)> {
        let mut out = ResultBuffer::new();
        out.arity = self.finds.len();
        if let ExecPlan::GuardProbe(guard) = &self.plan {
            let report = format!("{}", Report::GuardProbe { plan: guard });
            self.execute(txn, cache, params, &mut out)?;
            return Ok((out, report));
        }
        // Bind before borrowing the plan (bind_params takes &mut self).
        let bound = self.bind_params(txn, params)?;
        match &self.plan {
            ExecPlan::GuardProbe(_) => unreachable!("handled above"),
            ExecPlan::FreeJoin(plan) => {
                let mut counters = CountingCounters::new(plan);
                let short_circuit = !bound
                    || !resolve_filters(
                        txn,
                        plan,
                        &self.resolved_params,
                        &mut self.resolved_filters,
                    )?;
                if short_circuit {
                    let report = format!("{}", Report::FreeJoin { plan, counters });
                    return Ok((out, report));
                }
                let mut sink = make_sink(
                    &self.finds,
                    self.bindings.slot_count(),
                    plan.distinct_bindings(),
                );
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
                    &mut sink.0,
                    &mut counters,
                )?;
                let report = format!("{}", Report::FreeJoin { plan, counters });
                finalize(sink, txn, &self.finds, &mut out)?;
                Ok((out, report))
            }
        }
    }

    /// Binds and converts parameters; `Ok(false)` = a String/Bytes value
    /// that was never interned (the query is empty on this snapshot).
    fn bind_params(&mut self, txn: &ReadTxn<'_>, params: &[Value]) -> Result<bool> {
        if params.len() != self.param_types.len() {
            return Err(Error::ParamCountMismatch {
                expected: self.param_types.len(),
                supplied: params.len(),
            });
        }
        self.resolved_params.clear();
        for (idx, value) in params.iter().enumerate() {
            let Some(expected) = &self.param_types[idx] else {
                self.resolved_params.push(Const::Word(0)); // unused hole id
                continue;
            };
            let Some(resolved) = bind_param(txn, idx, value, expected)? else {
                return Ok(false);
            };
            self.resolved_params.push(resolved);
        }
        Ok(true)
    }
}

/// Resolves every occurrence's symbolic filter constants for this
/// execution; `Ok(false)` = a `PendingIntern` missed the dictionary.
fn resolve_filters(
    txn: &ReadTxn<'_>,
    plan: &crate::plan::fj::ValidatedPlan,
    params: &[Const],
    out: &mut [Vec<FilterPredicate>],
) -> Result<bool> {
    for (occ_idx, occurrence) in plan.occurrences().iter().enumerate() {
        out[occ_idx].clear();
        for filter in &occurrence.filters {
            let Some(resolved) = resolve_filter(txn, filter, params)? else {
                return Ok(false);
            };
            out[occ_idx].push(resolved);
        }
    }
    Ok(true)
}

/// Builds views and COLT roots, then runs the join into the sink.
#[allow(clippy::too_many_arguments)] // the prepared query's split borrows;
                                     // bundling them into a struct would only rename the same nine things
fn run_join<C: crate::exec::run::Counters>(
    plan: &crate::plan::fj::ValidatedPlan,
    schema: &Schema,
    txn: &ReadTxn<'_>,
    cache: &ImageCache,
    executor: &mut Executor,
    bindings: &mut Bindings,
    resolved_filters: &[Vec<FilterPredicate>],
    survivor_buffers: &mut [Vec<u32>],
    sink: &mut EitherSink,
    counters: &mut C,
) -> Result<()> {
    let views: Vec<Arc<View>> = plan
        .occurrences()
        .iter()
        .enumerate()
        .map(|(occ_idx, occurrence)| {
            let image = cache.get_or_build(txn, schema, occurrence.relation)?;
            let buffer = std::mem::take(&mut survivor_buffers[occ_idx]);
            Ok(Arc::new(apply(
                &image,
                &resolved_filters[occ_idx],
                &[],
                buffer,
            )))
        })
        .collect::<Result<_>>()?;
    let mut colts: Vec<Colt<'_>> = plan
        .occurrences()
        .iter()
        .enumerate()
        .map(|(occ_idx, occurrence)| {
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
            Colt::new(&views[occ_idx], columns)
        })
        .collect();
    executor.execute(plan, &mut colts, bindings, sink, counters);
    drop(colts);
    // Recycle the survivor buffers for the next execution.
    for (occ_idx, view) in views.into_iter().enumerate() {
        if let Ok(view) = Arc::try_unwrap(view) {
            survivor_buffers[occ_idx] = view.recycle();
        }
    }
    Ok(())
}

/// Builds the sink matching the find shape (the variant is fixed per
/// prepared query — an enum, not `dyn`).
fn make_sink(finds: &[(FindSpec, ValueType)], slot_count: usize, distinct: bool) -> SinkBox {
    let has_aggregates = finds
        .iter()
        .any(|(spec, _)| matches!(spec, FindSpec::Agg { .. }));
    if has_aggregates {
        SinkBox(EitherSink::Aggregate(AggregateSink::new(
            finds.iter().map(|(spec, _)| *spec).collect(),
            slot_count,
            distinct,
        )))
    } else {
        SinkBox(EitherSink::Projection(ProjectionSink::new(
            finds
                .iter()
                .map(|(spec, _)| match spec {
                    FindSpec::Var { slot } => *slot,
                    FindSpec::Agg { .. } => unreachable!("no aggregates here"),
                })
                .collect(),
        )))
    }
}

/// The two sink shapes behind one monomorphized dispatch (an enum, not
/// `dyn` — the variant is fixed per prepared query).
enum EitherSink {
    Projection(ProjectionSink),
    Aggregate(AggregateSink),
}

struct SinkBox(EitherSink);

impl Sink for EitherSink {
    fn emit(&mut self, bindings: &Bindings) -> crate::exec::run::Flow {
        match self {
            Self::Projection(sink) => sink.emit(bindings),
            Self::Aggregate(sink) => sink.emit(bindings),
        }
    }
}

/// Converts a bound param value to column form. `Ok(None)` = a String or
/// Bytes value that was never interned: the query is empty on this
/// snapshot.
fn bind_param(
    txn: &ReadTxn<'_>,
    index: usize,
    value: &Value,
    expected: &ValueType,
) -> Result<Option<Const>> {
    let mismatch = || Error::ParamTypeMismatch {
        param: ParamId(u16::try_from(index).expect("param ids fit u16")),
    };
    let resolved = match (value, expected) {
        (Value::Bool(v), ValueType::Bool) => Const::Byte(u8::from(*v)),
        (Value::Enum(ordinal), ValueType::Enum { variants }) => {
            if usize::from(*ordinal) >= variants.len() {
                return Err(mismatch());
            }
            Const::Byte(*ordinal)
        }
        (Value::U64(v), ValueType::U64) => Const::Word(*v),
        (Value::I64(v), ValueType::I64) => {
            Const::Word(u64::from_be_bytes(crate::encoding::encode_i64(*v)))
        }
        (Value::String(bytes), ValueType::String) => {
            let text = std::str::from_utf8(bytes).map_err(|_| mismatch())?;
            match dict::lookup_str(txn, text)? {
                Some(id) => Const::Word(id),
                None => return Ok(None),
            }
        }
        (Value::Bytes(bytes), ValueType::Bytes) => match dict::lookup_bytes(txn, bytes)? {
            Some(id) => Const::Word(id),
            None => return Ok(None),
        },
        _ => return Err(mismatch()),
    };
    Ok(Some(resolved))
}

/// Substitutes symbolic constants into an executable filter. `Ok(None)` =
/// a `PendingIntern` missed the dictionary.
fn resolve_filter(
    txn: &ReadTxn<'_>,
    filter: &FilterPredicate,
    params: &[Const],
) -> Result<Option<FilterPredicate>> {
    let FilterPredicate::Compare { field, op, value } = filter else {
        return Ok(Some(filter.clone()));
    };
    let resolved = match value {
        Const::Word(_) | Const::Byte(_) => value.clone(),
        Const::Param(p) => params[usize::from(p.0)].clone(),
        Const::PendingIntern { tag, bytes } => match dict::lookup_tagged(txn, *tag, bytes)? {
            Some(id) => Const::Word(id),
            None => return Ok(None),
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
    sink: SinkBox,
    txn: &ReadTxn<'_>,
    finds: &[(FindSpec, ValueType)],
    out: &mut ResultBuffer,
) -> Result<()> {
    match sink.0 {
        EitherSink::Projection(sink) => {
            for row in sink.rows() {
                for (column, (_, ty)) in finds.iter().enumerate() {
                    out.push_word(txn, ty, row[column])?;
                }
            }
            Ok(())
        }
        EitherSink::Aggregate(sink) => {
            for row in sink.into_rows()? {
                for (column, (_, ty)) in finds.iter().enumerate() {
                    out.push_word(txn, ty, row[column])?;
                }
            }
            Ok(())
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
        let mut prepared = prepare(&txn, &schema, &by_account_query()).expect("prepare");
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
        let mut prepared = prepare(&txn, &schema, &by_account_query()).expect("prepare");
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
            matches!(err, Error::ParamTypeMismatch { param } if param.0 == 0),
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
        let mut prepared = prepare(&txn, &schema, &query).expect("prepare");
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

    #[test]
    fn results_decode_intern_ids_to_original_bytes() {
        let dir = TempDir::new("prepared-decode");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        insert_postings(&env, &schema, &[(1, 7, "a rather long memo text", 10)]);
        let cache = ImageCache::new();
        let txn = env.read_txn().expect("txn");
        let mut prepared = prepare(&txn, &schema, &by_account_query()).expect("prepare");
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
        let mut prepared = prepare(&txn, &schema, &by_account_query()).expect("prepare");
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
        let mut prepared = prepare(&txn, &schema, &by_account_query()).expect("prepare");
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
        let mut prepared = prepare(&txn, &schema, &query).expect("prepare");
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
        let mut prepared = prepare(&txn, &schema, &by_account_query()).expect("prepare");
        let (rows, report) = prepared
            .explain(&txn, &cache, &[Value::U64(7), Value::I64(0)])
            .expect("explain");
        assert_eq!(rows.len(), 2);
        assert!(report.contains("free join"));
        assert!(report.contains("emitted bindings: 2"));
    }
}
