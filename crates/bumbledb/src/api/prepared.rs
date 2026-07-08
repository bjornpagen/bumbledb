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

use crate::exec::colt::Colt;
use crate::exec::dispatch::ExecPlan;
use crate::exec::run::{Bindings, Executor};
use crate::exec::sink::{AggregateSink, FindSpec, ProjectionSink};
use crate::image::view::{Const, FilterPredicate};
use crate::schema::{Schema, ValueType};

mod bind;
mod build;
mod either_sink;
mod execute;
mod finalize;
mod introspect;
mod resolve_memo;
mod result_buffer;
mod run_join;
mod view_memo;

#[cfg(test)]
mod tests;

pub(crate) use self::build::prepare;

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

/// One result row, borrowed from a [`ResultBuffer`].
#[derive(Clone, Copy)]
pub struct Row<'a> {
    buffer: &'a ResultBuffer,
    row: usize,
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
