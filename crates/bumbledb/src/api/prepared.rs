//! Prepared queries, parameters, and results (docs/architecture/40-execution.md) — the reusable
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
mod staleness;
mod view_memo;

#[cfg(test)]
mod tests;

pub(crate) use self::build::prepare;
use self::staleness::OccurrencePin;
pub use self::staleness::{OccurrenceDrift, Staleness};

/// One positional execution argument (`docs/architecture/70-api.md`
/// § facts and results): params are supplied by `ParamId` position —
/// scalars as values, param sets as slices. Bind checks count, scalar-
/// vs-set usage against what validation recorded, and element types;
/// set slices deduplicate into the prepared query's pooled storage
/// (sets are sets — `docs/architecture/20-query-ir.md`).
#[derive(Debug, Clone)]
pub enum ParamArg<'a> {
    Scalar(crate::ir::Value),
    Set(&'a [crate::ir::Value]),
}

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
    /// An interval find, rematerialized through the checked host type
    /// (the stored `start < end` invariant makes the re-parse
    /// infallible — the comment lives at the materialization site).
    IntervalU64(crate::interval::Interval<u64>),
    IntervalI64(crate::interval::Interval<i64>),
}

/// One stored cell: fixed-width values inline, String/Bytes as ranges into
/// the buffer's byte heap. An interval find is ONE cell (the buffer's
/// arity counts find terms, not words — the two-word slot span collapses
/// at materialization).
#[derive(Debug, Clone, Copy)]
enum Cell {
    Bool(bool),
    U64(u64),
    I64(i64),
    Enum(u8),
    String { start: usize, len: usize },
    Bytes { start: usize, len: usize },
    IntervalU64(crate::interval::Interval<u64>),
    IntervalI64(crate::interval::Interval<i64>),
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

/// The per-finalize intern-resolution memo (docs/architecture/40-execution.md): each
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
    /// The last resolution: run-coherent columns
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
    /// The preparing environment's process-distinct identity: plan,
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
    /// Dense per-param expected types (validation rejects id gaps). A set
    /// param's type is its **element** type.
    param_types: Vec<ValueType>,
    /// Dense per-param set-ness (`Term::ParamSet` anchors — a `ParamId`
    /// is scalar or set, never both; validation enforced it).
    param_is_set: Vec<bool>,
    /// Bind-time resolved constants, reused across executions — pooled
    /// storage: a set param's slot holds a [`Const::WordSet`] whose `Vec`
    /// is rebound in place (sorted, deduplicated words; capacity
    /// retained across differently-sized warm re-binds).
    resolved_params: Vec<Const>,
    /// Per param: whether this execution's value missed the dictionary
    /// (String/Bytes only; for a set, whether NO element survived — the
    /// empty set rides the same short-circuit machinery). A missed value
    /// under `Eq` on a positive occurrence short-circuits to an empty
    /// result; under `Ne` the sentinel word matches everything; on a
    /// negated occurrence it just matches nothing.
    missed_params: Vec<bool>,
    /// Per occurrence: residual filters with symbolic constants
    /// substituted, reused — in place, so a set-carrying filter's
    /// `WordSet` capacity survives re-binds (the allocation contract).
    resolved_filters: Vec<Vec<FilterPredicate>>,
    /// Per occurrence, per selection level: this execution's resolved key
    /// words (docs/architecture/40-execution.md, § selection levels) —
    /// one word for a scalar constant, the encoded pair for an interval
    /// constant, k sorted deduplicated words for a set. Reused in place.
    resolved_selections: Vec<Vec<Vec<u64>>>,
    /// The view memo (docs/architecture/40-execution.md): per occurrence, the active binding
    /// (whose COLT the executor consumes) plus parked bindings under LRU.
    memo: ViewMemo,
    /// The sink, reset per execution with capacities retained.
    sink: EitherSink,
    /// Aggregate-finalization row scratch.
    row_scratch: Vec<u64>,
    /// No interned finds: finalize takes the
    /// infallible all-words blit.
    all_words: bool,
    /// The guard fast lane's find table: each output
    /// column's fact field and type, in find order. `Some` for guard
    /// plans whose finds are all plain variables; aggregate-find guards
    /// keep the sink path.
    guard_finds: Option<Vec<(crate::schema::FieldId, ValueType)>>,
    /// The per-finalize intern-resolution memo (docs/architecture/40-execution.md).
    resolve_memo: ResolveMemo,
    /// Guard-key byte scratch.
    guard_key: Vec<u8>,
    /// The staleness pin record (`staleness.rs`): per participating
    /// occurrence, the statistics the plan was costed with. Cold data —
    /// written once at build, read only by [`PreparedQuery::staleness`]
    /// and the stats surface, never by execution. Empty for guard
    /// probes (classification precedes statistics; nothing is read).
    pinned: Box<[OccurrencePin]>,
    /// Marker: a prepared query is single-threaded scratch.
    _not_sync: std::marker::PhantomData<std::cell::Cell<()>>,
}

/// How many (generation, resolved residual filters) bindings each
/// occurrence memoizes: the active one plus [`PARKED_SLOTS`] parked.
/// Four covers the bench rotation and the handful of bindings real
/// workloads repeat; memory is bounded by four COLT high-waters per
/// occurrence per prepared query — the explicit trade (docs/architecture/40-execution.md).
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

/// The per-occurrence view memo (docs/architecture/40-execution.md): generational
/// immutability makes a memoized view provably valid for its whole
/// generation, so repeated residual bindings (range windows, Ne
/// constants) skip the rebuild scan entirely. Occurrences whose only
/// predicates are selections never park — their single binding hits on
/// generation alone (docs/architecture/40-execution.md).
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
                                     // that tripped the lint are the working set itself.
enum EitherSink {
    Projection(ProjectionSink),
    /// Boxed: the batch-fold scratch grew the sink past the
    /// variant-size lint; one prepared query holds one sink, and the
    /// indirection is paid once per batch, never per row.
    Aggregate(Box<AggregateSink>),
}
