//! Prepared queries, parameters and results: the reusable execution object.
//! `prepare` runs validate → normalize → statistics → plan → classify once. Plans
//! pin the statistics read at prepare time and are never invalidated by writes;
//! re-preparation is explicit. Text literals and params resolve in the shared cache
//! namespace; an unstored text is an ordinary unequal token.
use std::sync::Arc;

use crate::exec::colt::Colt;
use crate::exec::dispatch::KeyProbePlan;
use crate::exec::run::{Bindings, Executor};
use crate::exec::sink::{AggregateSink, FindSpec, ProjectionSink};
use crate::image::view::{Const, FilterPredicate};
use crate::ir::validate::Signature;
use crate::plan::fj::ValidatedPlan;
use crate::schema::Schema;
use bumbledb_theory::schema::ValueType;

mod answers;
mod bind;
mod build;
pub(crate) mod computed;
mod either_sink;
mod execute;
mod finalize;
mod introspect;
pub(crate) mod reach;
mod resolve_memo;
pub(crate) mod result;
mod run_join;
pub(crate) mod source;
mod text;
pub(crate) use self::text::{decode_row, owned_text};
mod view_memo;

#[cfg(test)]
mod tests;

pub(crate) use self::build::{prepare_on, prepare_owned};
pub use self::result::{
    CompleteResult, DeliveryTicket, ResultCursor, ResultIdentity, ResultPage, ResultRow,
};

/// One bound scalar payload: the bind surface's value vocabulary. Variable-width
/// payloads are **borrowed** — the engine only hashes and probes them
/// (a per-execution intern lookup), so owned payloads would buy
/// nothing; `&str` also makes non-UTF-8 string params unrepresentable
/// rather than checked. [`crate::ir::Value`] stays owned by decision:
/// IR literals are long-lived query data; only the bind surface
/// borrows .
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BindValue<'a> {
    Bool(bool),
    U64(u64),
    I64(i64),
    F64(bumbledb_theory::F64),
    Str(&'a str),
    /// A `bytes<N>` value: exactly the anchored field's N bytes (any
    /// other length is a bind-time type mismatch — the length is the
    /// type). Only hashed into column words at bind; never interned.
    FixedBytes(&'a [u8]),
    /// An application-owned 128-bit identity: sixteen exact bytes.
    Uuid(bumbledb_theory::Uuid),
    /// A half-open `[start, end)`.
    IntervalU64(u64, u64),
    /// A half-open `[start, end)`.
    IntervalI64(i64, i64),
    /// A checked dense-line interval: canonical endpoints, `start < end`
    /// by numeric order (the checked host type carries the proof).
    IntervalF64(bumbledb_theory::Interval<bumbledb_theory::F64>),
}

/// One positional execution argument
/// § facts and results): params are supplied by `ParamId` position —
/// scalars as [`BindValue`]s, param sets as slices. Bind checks count,
/// scalar-vs-set usage against what validation recorded, and element
/// types; set slices deduplicate into the prepared query's pooled
/// storage (sets are sets — . Set
/// elements stay [`crate::ir::Value`]: a set is long-lived host data
/// re-bound by reference, so its elements never re-box per bind.
#[derive(Debug, Clone)]
pub enum ParamArg<'a> {
    Scalar(BindValue<'a>),
    Set(&'a [crate::ir::Value]),
}

/// The one execute bind surface: scalar slices and mixed [`ParamArg`]
/// slices are the same entry, not twin methods.
pub trait BindArgs<'a> {
    /// Bind this argument list onto `prepared` (text params latch through
    /// the prepared query's interner under `work`).
    /// # Errors
    /// `ParamCountMismatch`/`ParamTypeMismatch` at bind time, plus the
    /// per-position set/scalar errors on mixed arguments.
    fn bind<S>(
        self,
        prepared: &mut PreparedQuery<S>,
        work: &crate::work::WorkContext,
    ) -> crate::error::Result<()>;
}

impl<'a> BindArgs<'a> for &'a [BindValue<'a>] {
    fn bind<S>(
        self,
        prepared: &mut PreparedQuery<S>,
        work: &crate::work::WorkContext,
    ) -> crate::error::Result<()> {
        prepared.bind_params(work, self)
    }
}

impl<'a, const N: usize> BindArgs<'a> for &'a [BindValue<'a>; N] {
    fn bind<S>(
        self,
        prepared: &mut PreparedQuery<S>,
        work: &crate::work::WorkContext,
    ) -> crate::error::Result<()> {
        prepared.bind_params(work, self)
    }
}

impl<'a> BindArgs<'a> for &'a [ParamArg<'a>] {
    fn bind<S>(
        self,
        prepared: &mut PreparedQuery<S>,
        work: &crate::work::WorkContext,
    ) -> crate::error::Result<()> {
        prepared.bind_param_args(work, self)
    }
}

impl<'a, const N: usize> BindArgs<'a> for &'a [ParamArg<'a>; N] {
    fn bind<S>(
        self,
        prepared: &mut PreparedQuery<S>,
        work: &crate::work::WorkContext,
    ) -> crate::error::Result<()> {
        prepared.bind_param_args(work, self)
    }
}

impl<'a> BindArgs<'a> for &'a Vec<BindValue<'a>> {
    fn bind<S>(
        self,
        prepared: &mut PreparedQuery<S>,
        work: &crate::work::WorkContext,
    ) -> crate::error::Result<()> {
        prepared.bind_params(work, self)
    }
}

impl<'a> BindArgs<'a> for &'a Vec<ParamArg<'a>> {
    fn bind<S>(
        self,
        prepared: &mut PreparedQuery<S>,
        work: &crate::work::WorkContext,
    ) -> crate::error::Result<()> {
        prepared.bind_param_args(work, self)
    }
}

/// One decoded answer cell, borrowed from [`Answers`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerValue<'a> {
    Bool(bool),
    U64(u64),
    I64(i64),
    F64(bumbledb_theory::F64),
    String(&'a str),
    /// A `bytes<N>` find: the value's N raw bytes.
    FixedBytes(&'a [u8]),
    /// An application-owned 128-bit identity find: sixteen exact bytes.
    Uuid(bumbledb_theory::Uuid),
    /// An interval find, rematerialized through the checked host type
    /// (the stored `start < end` invariant makes the re-parse
    /// infallible — the comment lives at the materialization site).
    IntervalU64(bumbledb_theory::Interval<u64>),
    IntervalI64(bumbledb_theory::Interval<i64>),
    /// A dense-line interval find: canonical F64 endpoints decoded from
    /// their order-key words (stored canonical or refused at decode).
    IntervalF64(bumbledb_theory::Interval<bumbledb_theory::F64>),
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
    F64(bumbledb_theory::F64),
    Uuid(bumbledb_theory::Uuid),
    String { start: usize, len: usize },
    FixedBytes { start: usize, len: usize },
    IntervalU64(bumbledb_theory::Interval<u64>),
    IntervalI64(bumbledb_theory::Interval<i64>),
    IntervalF64(bumbledb_theory::Interval<bumbledb_theory::F64>),
}

/// The caller-owned, reusable answer set: columns are the find terms in
/// order; answers are unordered (query denotations are sets — the host sorts). The two
/// byte heaps are the single sanctioned allocation site of a warm
/// execution, and `clear` retains every capacity.
#[derive(Debug, Default)]
pub struct Answers {
    arity: usize,
    cells: Vec<Cell>,
    /// The String cells' heap: whole UTF-8-validated strings appended
    /// end-to-end, so every cell range is a char boundary by
    /// construction — the type carries the materialization proof and
    /// `get` never re-validates (parse, don't validate).
    text: String,
    /// The `bytes<N>` cells' heap: raw payloads, no text contract.
    blob: Vec<u8>,
}

/// Per-finalize intern resolution. Text is copied only into the answer heap.
#[derive(Debug)]
struct ResolveMemo {
    /// word → packed `(start, len)` into this finalize's answer heap.
    ranges: crate::exec::wordmap::WordMap<(u32, u32)>,
    /// The last resolution: run-coherent columns skip even the map probe.
    last: Option<(u64, (usize, usize))>,
}

/// One query answer, borrowed from [`Answers`].
#[derive(Clone, Copy)]
pub struct Answer<'a> {
    buffer: &'a Answers,
    answer: usize,
}

/// The reusable execution object. `!Sync` by construction (interior
/// scratch); executes from one thread at a time; owns its scratch.
/// Carries the preparing database's schema typestate `S`, so it executes
/// only against same-schema snapshots. A runtime identity check also
/// requires snapshots from the preparing environment.
/// Not shareable across threads:
/// ```compile_fail
/// fn require_sync<T: Sync>() {}
/// require_sync::<bumbledb::PreparedQuery<()>>();
/// ```
pub struct PreparedQuery<S> {
    program: Program,
    /// Interiors, then rec, then main. Main rules share one sink, reset once
    /// per execution; its seen set spanning rules is the union.
    pub(crate) pipeline: PreparedPipeline,
    bound: Bound,
    runtime: Runtime,
    /// A prepared query is single-threaded scratch (`Cell` makes it
    /// `!Sync`), pinned to schema `S` (`fn() -> S` keeps auto-traits
    /// independent of `S`).
    marker: std::marker::PhantomData<PreparedMarker<S>>,
}

/// What prepare sealed: fixed for the prepared query's lifetime.
struct Program {
    schema: Arc<Schema>,
    /// The preparing source's identity: execution against any other
    /// environment is `Error::ForeignPreparedQuery`, checked first.
    pinned: source::PinnedSource,
    /// The relation-image cache and the one text interner its images,
    /// binds and answers share.
    cache: Arc<crate::image::cache::ImageCache>,
    /// The query's output signature (a dead-main Cq still has an arity).
    signature: Signature,
    /// Dense per-param bind contracts.
    params: Vec<ParamSpec>,
    /// A direct probe whose row, parameters and results are text-free runs
    /// without acquiring a text generation.
    no_text_probe: bool,
    /// The query in rule notation, for introspection.
    rendered: String,
}

/// What the last bind resolved, reused across executions.
struct Bound {
    /// Resolved constants; a set param's `Const::WordSet` is rebound in
    /// place, keeping its capacity.
    resolved_params: Vec<Const>,
    /// The resolver every memoized word belongs to; a changed owner
    /// invalidates token-bearing views and resolutions.
    text_generation: Option<crate::work::cache::WeakGenerationHandle>,
    /// Per param slot: the last String resolution, valid within
    /// `text_generation`.
    param_word_memo: Vec<ParamWordMemo>,
    /// Per param: the value missed the dictionary (for a set, no element
    /// survived). Under `Eq` on a positive occurrence the rule is empty.
    missed_params: Vec<bool>,
}

/// Execution scratch, reset per execution with capacities retained.
struct Runtime {
    /// Heap executions count up; each tick is a fresh `ViewEpoch::Heap`.
    heap_tick: u64,
    /// Text retained by the recursive accumulator.
    execution_texts: crate::image::TextOwners,
    derived: crate::api::prepared::reach::DerivedImages,
    /// The main sink, aimed at each main rule in turn.
    sink: EitherSink,
    /// Rule-shared binding slots, resized at each rule's entry.
    bindings: Bindings,
    answer_scratch: Vec<u64>,
    resolve_memo: ResolveMemo,
    #[cfg(test)]
    last_visits: usize,
}

/// One named interior's prepared artifact: its rule loop and stage sink
/// — projection, aggregate, or computed, exactly like main. Evaluated once, in declaration
/// order, before rec and main; aggregate/computed stages FINALIZE before
/// sealing their table, so a required producer error fails the whole
/// query even if a consumer would discard the group (the stage error
/// boundary). A dead interior is the empty table.
pub(crate) struct PreparedInterior {
    pub(super) rules: Vec<PreparedRule>,
    pub(super) sink: EitherSink,
    pub(super) field_types: Vec<bumbledb_theory::schema::ValueType>,
    pub(super) units: usize,
}

/// One prepared pipeline. Interiors are data in Cq and Reach (the CQ
/// is the empty prefix). The point fast lane is its own arm, sealed at
/// build — not re-detected by empty interiors + a find-table `Option`.
/// Statically-dead main is `Cq { rules: [] }` — Empty is not a
/// variant; the empty fast path is the zero-iteration loop.
pub(crate) enum PreparedPipeline {
    /// No-interior CQ whose single main rule is a key probe with
    /// variable finds. The arm stores [`KeyProbeRule`] so a
    /// [`PreparedRule::FreeJoin`] is unrepresentable.
    PointProbe {
        rule: Box<KeyProbeRule>,
        finds: Vec<(bumbledb_theory::schema::FieldId, ValueType)>,
    },
    Cq {
        interiors: Vec<PreparedInterior>,
        rules: Vec<PreparedRule>,
    },
    Reach {
        interiors: Vec<PreparedInterior>,
        driver: Box<reach::ReachDriver>,
        main: Vec<PreparedRule>,
        rec_id: crate::ir::InteriorId,
        derived_count: u32,
    },
}

impl PreparedPipeline {
    pub(super) fn interiors(&self) -> &[PreparedInterior] {
        match self {
            Self::PointProbe { .. } => &[],
            Self::Cq { interiors, .. } | Self::Reach { interiors, .. } => interiors,
        }
    }

    pub(super) fn interiors_mut(&mut self) -> &mut Vec<PreparedInterior> {
        match self {
            Self::PointProbe { .. } => {
                unreachable!("PointProbe has no interiors")
            }
            Self::Cq { interiors, .. } | Self::Reach { interiors, .. } => interiors,
        }
    }

    /// Cq / Reach main rules. [`Self::PointProbe`] is a [`KeyProbeRule`], not a
    /// tagged [`PreparedRule`] — callers of the fast lane match that arm.
    pub(super) fn main_rules(&self) -> &[PreparedRule] {
        match self {
            Self::PointProbe { .. } => &[],
            Self::Cq { rules, .. } => rules,
            Self::Reach { main, .. } => main,
        }
    }

    pub(super) fn main_rules_mut(&mut self) -> &mut [PreparedRule] {
        match self {
            Self::PointProbe { .. } => &mut [],
            Self::Cq { rules, .. } => rules,
            Self::Reach { main, .. } => main,
        }
    }

    /// No interiors and no surviving main rules — the zero-iteration Cq.
    pub(super) fn is_empty_cq(&self) -> bool {
        matches!(
            self,
            Self::Cq { interiors, rules } if interiors.is_empty() && rules.is_empty()
        )
    }

    pub(super) fn has_derived(&self) -> bool {
        match self {
            Self::PointProbe { .. } => false,
            Self::Cq { interiors, .. } => !interiors.is_empty(),
            Self::Reach { .. } => true,
        }
    }
}

/// One rule's prepared artifact. Its kind carries exactly the scratch that
/// kind can consume. Rec arms are [`FreeJoinRule`], inhabitable only in
/// [`reach::ReachDriver::rec`].
#[expect(
    clippy::large_enum_variant,
    reason = "the decided representation keeps rule scratch inline; programs contain at most the validated rule cap"
)]
pub(crate) enum PreparedRule {
    FreeJoin(FreeJoinRule),
    KeyProbe(KeyProbeRule),
}

pub(crate) struct FreeJoinRule {
    plan: ValidatedPlan,
    executor: Executor,
    /// The rule's head projection: per head position, the output spec
    /// over this rule's binding-slot layout (result types live on the
    /// query — they are the head's, identical across rules).
    finds: Vec<FindSpec>,
    /// The rule's full slot array as `VarId`-ordered spans — the
    /// DNF-derived union regime's shared dedup key. Aggregate-bearing heads only; empty (and never read) for
    /// projection heads.
    dedup_spans: Box<[(usize, usize)]>,
    /// Per occurrence: residual filters with symbolic constants
    /// substituted, reused — in place, so a set-carrying filter's
    /// `WordSet` capacity survives re-binds (the allocation contract).
    resolved_filters: Vec<Vec<FilterPredicate>>,
    /// Per occurrence, per selection level: this execution's resolved key
    /// words —
    /// one word for a scalar constant, the encoded pair for an interval
    /// constant, k sorted deduplicated words for a set. Reused in place.
    resolved_selections: Vec<Vec<crate::image::view::ResolvedWords>>,
    /// This rule's resolved tables were fully written by a completed
    /// `resolve_filters` pass (a short-circuited pass leaves later
    /// slots unwritten and does not set it). Within one text generation,
    /// a parameter-free rule can reuse this completed resolution.
    resolution: ResolutionState,
    /// The view memo: per occurrence, the active binding
    /// (whose COLT the executor consumes) plus parked bindings under LRU.
    memo: ViewMemo,
}

pub(crate) struct KeyProbeRule {
    plan: KeyProbePlan,
    /// Shared by the direct and sink paths.
    probe: crate::exec::dispatch::ProbeBuffers,
    distinct_witness: Option<crate::plan::fj::DistinctWitness>,
    finds: Vec<FindSpec>,
    /// As [`FreeJoinRule::dedup_spans`], over this rule's key-probe layout.
    dedup_spans: Box<[(usize, usize)]>,
}

impl<S> PreparedQuery<S> {
    /// Every prepared rule this query carries — interiors, rec base,
    /// then main. Rec arms are [`FreeJoinRule`], visited via
    /// [`Self::visit_rec_arms_mut`]. Cold surfaces only (the batch-size
    /// test affordance).
    fn visit_rules_mut(&mut self, mut visit: impl FnMut(&mut PreparedRule)) {
        match &mut self.pipeline {
            PreparedPipeline::PointProbe { .. } => {}
            PreparedPipeline::Cq { interiors, rules } => {
                for interior in interiors {
                    for rule in &mut interior.rules {
                        visit(rule);
                    }
                }
                for rule in rules {
                    visit(rule);
                }
            }
            PreparedPipeline::Reach {
                interiors,
                driver,
                main,
                ..
            } => {
                for interior in interiors {
                    for rule in &mut interior.rules {
                        visit(rule);
                    }
                }
                for rule in &mut driver.base {
                    visit(rule);
                }
                for rule in main {
                    visit(rule);
                }
            }
        }
    }

    fn visit_rec_arms_mut(&mut self, mut visit: impl FnMut(&mut FreeJoinRule)) {
        if let PreparedPipeline::Reach { driver, .. } = &mut self.pipeline {
            for arm in &mut driver.rec {
                visit(arm);
            }
        }
    }

    /// Release this query's execution buffers while keeping its compiled plan.
    /// Ordinary executions retain reusable capacity; this explicit operation
    /// drops it, so the next execution allocates scratch again. Shared database
    /// caches and independently owned results or snapshots are unaffected.
    pub fn release_memory(&mut self) {
        self.runtime.execution_texts = crate::image::TextOwners::default();
        self.runtime.derived = reach::DerivedImages::default();
        self.visit_rules_mut(|rule| match rule {
            PreparedRule::FreeJoin(rule) => rule.release_memory(),
            PreparedRule::KeyProbe(rule) => rule.probe.release_memory(),
        });
        self.visit_rec_arms_mut(FreeJoinRule::release_memory);
        match &mut self.pipeline {
            PreparedPipeline::PointProbe { rule, .. } => rule.probe.release_memory(),
            PreparedPipeline::Cq { interiors, .. } => {
                for interior in interiors {
                    interior.sink.release_memory();
                }
            }
            PreparedPipeline::Reach {
                interiors, driver, ..
            } => {
                for interior in interiors {
                    interior.sink.release_memory();
                }
                driver.sink.release_memory();
                driver.frontier = crate::image::TransientImage::default();
            }
        }
        self.runtime.sink.release_memory();
        self.runtime.bindings = Bindings::new(0);
        self.runtime.answer_scratch = Vec::new();
        self.runtime.resolve_memo = ResolveMemo::new();
        self.bound.resolved_params = Vec::new();
        self.bound.param_word_memo = Vec::new();
        self.bound.missed_params = Vec::new();
        self.bound.text_generation = None;
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn last_visits(&self) -> usize {
        self.runtime.last_visits
    }
}

impl FreeJoinRule {
    fn release_memory(&mut self) {
        self.memo.release_memory();
        self.executor.release_memory();
        for filters in &mut self.resolved_filters {
            *filters = Vec::new();
        }
        for selections in &mut self.resolved_selections {
            *selections = Vec::new();
        }
        self.resolution = ResolutionState::Pending;
    }
}

impl PreparedRule {
    fn finds(&self) -> &[FindSpec] {
        match self {
            Self::FreeJoin(rule) => &rule.finds,
            Self::KeyProbe(rule) => &rule.finds,
        }
    }

    fn slot_count(&self) -> usize {
        match self {
            Self::FreeJoin(rule) => rule.plan.slot_count(),
            Self::KeyProbe(rule) => rule.plan.slot_count(),
        }
    }

    fn distinct_witness(&self) -> Option<crate::plan::fj::DistinctWitness> {
        match self {
            Self::FreeJoin(rule) => rule.plan.distinct_witness(),
            Self::KeyProbe(rule) => rule.distinct_witness,
        }
    }

    /// The rule's `VarId`-ordered full slot spans — the DNF-derived
    /// union regime's shared dedup key; empty for projection heads.
    fn dedup_spans(&self) -> &[(usize, usize)] {
        match self {
            Self::FreeJoin(rule) => &rule.dedup_spans,
            Self::KeyProbe(rule) => &rule.dedup_spans,
        }
    }
}

/// [`PreparedQuery`]'s phantom payload: `!Sync` scratch pinned to `S`.
type PreparedMarker<S> = (std::cell::Cell<()>, fn() -> S);

/// One param slot's complete bind-time contract — dense by `ParamId`,
/// sealed at prepare from validation's recording.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ParamSpec {
    /// A scalar slot. `point` marks an element-typed interval position,
    /// whose domain ceiling is not a point.
    Scalar { ty: ValueType, point: bool },
    /// A set slot. `elem` is the element type, and `point` applies to
    /// each element.
    Set { elem: ValueType, point: bool },
}

/// One scalar param slot's memoized String resolution
/// ([`PreparedQuery::param_word_memo`]): the bound text and its word.
/// Words are scoped by `PreparedQuery::text_generation`; the memo pins its text.
#[derive(Debug, Default, Clone)]
struct ParamWordMemo {
    text: Option<std::sync::Arc<str>>,
    word: Option<u64>,
}

/// Whether every symbolic filter/selection slot was written by a complete
/// resolution pass. Only `Complete` licenses the warm resolution skip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResolutionState {
    Pending,
    Complete,
}

/// How many (generation, resolved residual filters) bindings each
/// occurrence memoizes: the active one plus [`PARKED_SLOTS`] parked.
/// Four covers the bench rotation and the handful of bindings real
/// workloads repeat; memory is bounded by four COLT high-waters per
/// occurrence per prepared query — the explicit trade .
const MEMO_SLOTS: usize = 4;
const PARKED_SLOTS: usize = MEMO_SLOTS - 1;

/// One executed binding: a real epoch, residual filters and image coverage.
/// Active and parked slots move this shape; the COLT lives
/// on [`ViewMemo::colts`] (active) or [`Parked::colt`] (parked).
struct BoundView {
    epoch: crate::image::ViewEpoch,
    filters: Vec<FilterPredicate>,
    /// None covers the whole relation; Some covers only this occurrence's
    /// resolved selections. Partial images never enter the shared cache.
    selections: Option<Vec<crate::image::view::ResolvedWords>>,
    last_used: u64,
}

/// The three proofs a parallel `None` used to conflate.
enum Binding {
    /// Never executed, or vacated after a park (the rebuild lands here).
    Unbound,
    /// Interior occurrence: lives outside the epoch-keyed memo.
    Derived,
    Bound(BoundView),
}

/// A parked [`BoundView`] plus the COLT it owns. The kernel only sees
/// [`ViewMemo::colts`]; this COLT is off the slice until unparked.
struct Parked {
    bound: BoundView,
    colt: Colt,
}

/// Per-occurrence memo slot: the active binding, its parked twins, and
/// the spare survivor buffer. [`Binding::Derived`] has no park arm.
struct OccMemo {
    active: Binding,
    parked: [Option<Parked>; PARKED_SLOTS],
    spare: Vec<u32>,
}

/// The per-occurrence view memo :
/// an epoch-stable source makes a memoized view provably valid for
/// its whole epoch, so repeated residual bindings (range windows, Ne
/// constants) skip the rebuild scan entirely. Occurrences whose only
/// conditions are selections never park — their single binding hits on
/// epoch alone .
struct ViewMemo {
    /// The executor-facing COLTs: each occurrence's *active* binding
    /// (over [`View::Unbound`] until the first execution — prepare pins
    /// no image). The kernel takes `&mut [Colt]`; this vector stays.
    colts: Vec<Colt>,
    /// One slot per occurrence: active [`Binding`], parked [`BoundView`]s,
    /// spare survivor buffer.
    occs: Vec<OccMemo>,
    /// The LRU clock, ticked once per execution.
    tick: u64,
}

/// The two sink shapes behind one monomorphized dispatch (an enum, not
/// `dyn` — the variant is fixed per prepared query). `pub(super)` because
/// [`PreparedInterior::sink`] is reachable at that visibility; the type
/// never crosses the `api` boundary.
#[expect(
    clippy::large_enum_variant,
    reason = "Projection is the hot variant: boxing it would add indirection to every emit"
)]
pub(super) enum EitherSink {
    Computed(Box<computed::ComputedSink>),
    Projection(ProjectionSink),
    /// Boxed: the batch-fold scratch grew the sink past the
    /// variant-size lint; one prepared query holds one sink, and the
    /// indirection is paid once per batch, never per answer.
    Aggregate(Box<AggregateSink>),
}
