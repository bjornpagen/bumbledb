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

/// One bound scalar payload (`docs/architecture/70-api.md` § facts and
/// results): the bind surface's value vocabulary. Variable-width
/// payloads are **borrowed** — the engine only hashes and probes them
/// (a per-execution intern lookup), so owned payloads would buy
/// nothing; `&str` also makes non-UTF-8 string params unrepresentable
/// rather than checked. [`crate::ir::Value`] stays owned by decision:
/// IR literals are long-lived query data; only the bind surface
/// borrows (`docs/architecture/20-query-ir.md`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BindValue<'a> {
    Bool(bool),
    U64(u64),
    I64(i64),
    /// Declaration-order ordinal.
    Enum(u8),
    Str(&'a str),
    /// A `bytes<N>` value: exactly the anchored field's N bytes (any
    /// other length is a bind-time type mismatch — the length is the
    /// type). Only hashed into column words at bind; never interned.
    FixedBytes(&'a [u8]),
    /// A half-open `[start, end)`.
    IntervalU64(u64, u64),
    /// A half-open `[start, end)`.
    IntervalI64(i64, i64),
    /// An Allen mask for an `Allen` comparison's mask param — the
    /// temporal relation as a bind-time argument (`crate::allen`). The
    /// vacuous ∅/full masks are rejected at bind with distinct typed
    /// errors, mirroring validation's literal-mask rules.
    AllenMask(crate::allen::AllenMask),
}

/// One positional execution argument (`docs/architecture/70-api.md`
/// § facts and results): params are supplied by `ParamId` position —
/// scalars as [`BindValue`]s, param sets as slices. Bind checks count,
/// scalar-vs-set usage against what validation recorded, and element
/// types; set slices deduplicate into the prepared query's pooled
/// storage (sets are sets — `docs/architecture/20-query-ir.md`). Set
/// elements stay [`crate::ir::Value`]: a set is long-lived host data
/// re-bound by reference, so its elements never re-box per bind.
#[derive(Debug, Clone)]
pub enum ParamArg<'a> {
    Scalar(BindValue<'a>),
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
    /// A `bytes<N>` find: the value's N raw bytes.
    FixedBytes(&'a [u8]),
    /// An interval find, rematerialized through the checked host type
    /// (the stored `start < end` invariant makes the re-parse
    /// infallible — the comment lives at the materialization site).
    IntervalU64(crate::interval::Interval<u64>),
    IntervalI64(crate::interval::Interval<i64>),
}

/// One stored cell: fixed-width values inline, String and `bytes<N>`
/// payloads as ranges into the buffer's byte heap. A multi-word find is
/// ONE cell (the buffer's arity counts find terms, not words — the slot
/// span collapses at materialization).
#[derive(Debug, Clone, Copy)]
enum Cell {
    Bool(bool),
    U64(u64),
    I64(i64),
    Enum(u8),
    String { start: usize, len: usize },
    FixedBytes { start: usize, len: usize },
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
/// distinct string intern word is resolved through LMDB exactly once per
/// finalize, and its bytes land in the output buffer exactly once — K
/// rows sharing one memo string cost one B-tree lookup and one byte
/// copy, not K. Cleared per finalize (the ranges point into that call's
/// buffer); capacity is retained, growing to the distinct-string
/// high-water like every other execution scratch. Keys are bare words:
/// strings are the one interned type, so the tag byte died with variable
/// bytes (docs/architecture/50-storage.md).
#[derive(Debug)]
struct ResolveMemo {
    /// word → packed `(start, len)` into the buffer's bytes.
    ranges: crate::exec::wordmap::WordMap<(u32, u32)>,
    /// The last resolution: run-coherent columns
    /// (few distinct interns, clustered rows) skip even the map probe.
    last: Option<(u64, (usize, usize))>,
}

/// One result row, borrowed from a [`ResultBuffer`].
#[derive(Clone, Copy)]
pub struct Row<'a> {
    buffer: &'a ResultBuffer,
    row: usize,
}

/// The reusable execution object. `!Sync` by construction (interior
/// scratch); executes from one thread at a time; owns its scratch.
/// Carries the preparing database's schema typestate `S`, so it executes
/// only against same-schema snapshots (the same-environment check stays
/// a runtime guard — `env_instance`).
///
/// Not shareable across threads:
///
/// ```compile_fail
/// fn require_sync<T: Sync>() {}
/// require_sync::<bumbledb::PreparedQuery<'static, ()>>();
/// ```
pub struct PreparedQuery<'s, S> {
    schema: &'s Schema,
    /// The preparing environment's process-distinct identity: plan,
    /// statistics, and view memo all belong to it, so execution against
    /// any other environment's snapshot is `Error::ForeignPreparedQuery`
    /// — checked first at every execution entry.
    env_instance: u64,
    /// The rule-disjointness proof (docs/architecture/40-execution.md
    /// § set semantics): `Some` iff the program's rules are provably
    /// pairwise disjoint, carrying the witness — the (relation, field)
    /// whose differing pinned literals forbid cross-rule head
    /// collisions. `None` for single-rule programs and unproven pairs.
    /// Readers: the sink configuration (built at prepare), EXPLAIN and
    /// the structured stats (an elision must name its proof).
    disjoint_rules: Option<crate::plan::fj::DisjointWitness>,
    /// The union elision, composed at prepare: disjoint rules ∧ per-rule
    /// distinct bindings ∧ heads reading every slot — the multi-rule
    /// aggregate seen-set is elided exactly when this holds
    /// (introspection's observable; the sink was built from it).
    union_elided: bool,
    /// The subsumption record (`plan/chase.rs`): rules deleted at
    /// prepare, each with its subsuming rule, in lowered-rule indices —
    /// `rules` below holds only the survivors, in order. Readers:
    /// EXPLAIN and the structured stats.
    subsumed: Vec<crate::api::stats::SubsumedRule>,
    /// Per rule, in rule order: the rule's validated plan plus its
    /// plan-shaped execution scratch — the whole plan pipeline ran per
    /// rule at prepare. Execution runs the rules **sequentially** into
    /// the ONE sink below (docs/architecture/40-execution.md § the rule
    /// loop): the sink resets once per execution, never per rule, and
    /// its seen-set spanning rules is the entire implementation of ∪ —
    /// no merge node, no concat-then-dedup pass exists.
    rules: Vec<PreparedRule>,
    /// Per head position: the result type (identical across rules — the
    /// head's positional alignment pins it at validation).
    column_types: Vec<ValueType>,
    /// Dense per-param expected shapes (validation rejects id gaps). A
    /// set param's entry carries its **element** type.
    param_types: Vec<ParamShape>,
    /// Dense per-param set-ness (`Term::ParamSet` anchors — a `ParamId`
    /// is scalar or set, never both; validation enforced it).
    param_is_set: Vec<bool>,
    /// Dense per-param point-ness: element-typed at an interval position
    /// (membership binding or `Contains` operand), so the bound value is
    /// a point and the domain ceiling is rejected — points are
    /// `MIN ..= MAX−1`; `MAX` is the ray's ∞ (the point-domain law,
    /// `docs/architecture/10-data-model.md`).
    param_is_point: Vec<bool>,
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
    /// The sink, reset once per execution with capacities retained —
    /// **one** sink configuration, owned by the head (its shape is the
    /// head's: projection vs aggregate, arity, distinctness). Its
    /// find-spec slot tables are re-aimed per rule as the rule loop
    /// switches plans (`run_rule`); the dedup keys are head-shaped —
    /// projected tuples, or head projections under the multi-rule
    /// aggregate regime — so the seen-set spanning rules is the union.
    sink: EitherSink,
    /// The rule-shared binding-slot scratch (docs/architecture/
    /// 40-execution.md § the rule loop): written in place by each rule's
    /// recursion, re-sized to the rule's slot layout at rule entry —
    /// capacity is the high-water across all rules.
    bindings: Bindings,
    /// Aggregate-finalization row scratch.
    row_scratch: Vec<u64>,
    /// No interned finds: finalize takes the
    /// infallible all-words blit.
    all_words: bool,
    /// The per-finalize intern-resolution memo (docs/architecture/40-execution.md).
    resolve_memo: ResolveMemo,
    /// Guard-key byte scratch.
    guard_key: Vec<u8>,
    /// Marker: a prepared query is single-threaded scratch (`Cell` makes
    /// it `!Sync`), pinned to schema `S` (`fn() -> S` keeps auto-traits
    /// independent of `S`).
    marker: std::marker::PhantomData<PreparedMarker<S>>,
}

/// One rule's prepared artifact: the validated plan the pipeline built
/// for it, plus every piece of execution scratch whose shape is the
/// plan's (slot layout, occurrence count, view memo). The prepared query
/// is a list of these — one per rule — under one head-owned sink.
struct PreparedRule {
    plan: ExecPlan,
    /// The Free Join executor scratch (unused for guard probes) — plan-
    /// shaped, so per-rule where the binding-slot scratch is shared.
    executor: Option<Executor>,
    /// The rule's head projection: per head position, the output spec
    /// over this rule's binding-slot layout (result types live on the
    /// query — they are the head's, identical across rules).
    finds: Vec<FindSpec>,
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
    /// The guard fast lane's find table: each output
    /// column's fact field and type, in find order. `Some` for guard
    /// plans whose finds are all plain variables; aggregate-find guards
    /// keep the sink path.
    guard_finds: Option<Vec<(crate::schema::FieldId, ValueType)>>,
    /// The staleness pin record (`staleness.rs`): per participating
    /// occurrence, the statistics the rule's plan was costed with. Cold
    /// data — written once at build, read only by
    /// [`PreparedQuery::staleness`] and the stats surface, never by
    /// execution. Empty for guard probes (classification precedes
    /// statistics; nothing is read).
    pinned: Box<[OccurrencePin]>,
}

/// [`PreparedQuery`]'s phantom payload: `!Sync` scratch pinned to `S`.
type PreparedMarker<S> = (std::cell::Cell<()>, fn() -> S);

/// What one param slot expects at bind: a data-model value of a type, or
/// an Allen mask (`Allen` mask positions — a mask is not a data-model
/// type, so it is not a [`ValueType`]; making the slot a two-variant sum
/// keeps the untyped placeholder unrepresentable).
#[derive(Debug, Clone, PartialEq, Eq)]
enum ParamShape {
    Value(ValueType),
    AllenMask,
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
