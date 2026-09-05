//! Prepared queries, parameters, and results — the reusable
//! execution object the allocation contract is written against.
//! `prepare` runs the whole pipeline once: validate → normalize →
//! filtered-view statistics → plan → classify. **Plans pin the statistics
//! read at prepare time and are never invalidated by writes**; stale plans
//! are accepted at this scale and re-preparation is explicit. Text
//! literals and params latch to tokens of the prepared query's own
//! append-only interner (`image/intern.rs`) — a latch is final, and a
//! text absent from every image is an ordinary unequal word, never an
//! error.
use std::num::NonZeroU32;
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
pub(crate) mod derived;
mod either_sink;
mod execute;
mod fallback;
mod finalize;
mod introspect;
pub(crate) mod reach;
mod resolve_memo;
pub(crate) mod result;
mod run_join;
pub(crate) mod source;
mod text;
pub(crate) use self::text::{decode_row, intern_admitted, owned_text, text_tokens_equal};
mod view_memo;

#[cfg(test)]
mod tests;

pub(crate) use self::build::{prepare_on, prepare_owned};
pub use self::result::{
    CompleteResult, DeliveryTicket, ResultCursor, ResultIdentity, ResultPage,
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
    Id128(bumbledb_theory::Id128),
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
    Id128(bumbledb_theory::Id128),
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
    Id128(bumbledb_theory::Id128),
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

/// Per-finalize intern resolution. Text is copied only into the answer
/// heap (result-charged). Scratch mappings are valid only while
/// [`PreparedQuery::nonresident`] is the store that minted them
/// (`scratch_epoch` is [`NonresidentTextStore::epoch`], the instance
/// owner id — not the 31-bit field packed into tokens).
#[derive(Debug)]
struct ResolveMemo {
    /// word → packed `(start, len)` into this finalize's answer heap.
    ranges: crate::exec::wordmap::WordMap<(u32, u32)>,
    /// The last resolution: run-coherent columns skip even the map probe.
    last: Option<(u64, (usize, usize))>,
    /// Owner id of the live scratch store, if any mappings named its tokens.
    scratch_epoch: Option<crate::image::TextStoreEpoch>,
}

/// One query answer, borrowed from [`Answers`].
#[derive(Clone, Copy)]
pub struct Answer<'a> {
    buffer: &'a Answers,
    answer: usize,
}

/// Pending intern literals vs the fully-latched fast path. The resolver
/// returns how many literals latched; this sum is the remaining debt —
/// not a counter that saturates when it distrusts itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Latch {
    Pending(NonZeroU32),
    Latched,
}

impl Latch {
    fn from_count(n: u32) -> Self {
        NonZeroU32::new(n).map_or(Self::Latched, Self::Pending)
    }

    fn is_latched(self) -> bool {
        matches!(self, Self::Latched)
    }

    fn credit(self, n: u32) -> Self {
        match self {
            Self::Latched => {
                debug_assert_eq!(n, 0, "a latched query has no remaining literals");
                Self::Latched
            }
            Self::Pending(remaining) => Self::from_count(
                remaining
                    .get()
                    .checked_sub(n)
                    .expect("latch credits cannot exceed pending literals"),
            ),
        }
    }

    #[cfg(test)]
    fn remaining(self) -> u32 {
        match self {
            Self::Pending(n) => n.get(),
            Self::Latched => 0,
        }
    }
}

/// The reusable execution object. `!Sync` by construction (interior
/// scratch); executes from one thread at a time; owns its scratch.
/// Carries the preparing database's schema typestate `S`, so it executes
/// only against same-schema snapshots (the same-environment check stays
/// a runtime identity check — [`source::PinnedSource`]).
/// Not shareable across threads:
/// ```compile_fail
/// fn require_sync<T: Sync>() {}
/// require_sync::<bumbledb::PreparedQuery<()>>();
/// ```
pub struct PreparedQuery<S> {
    schema: Arc<Schema>,
    /// The preparing source's identity: plan, statistics, view memo and
    /// interner tokens all belong to it, so execution against any other
    /// environment's snapshot is `Error::ForeignPreparedQuery` — checked
    /// first at every execution entry. Heap-prepared queries pin `Heap`
    /// and rebuild their images per execution (no durable identity).
    pinned: source::PinnedSource,
    /// The prepared query's relation-image cache plus the one text
    /// interner its images, binds, latches and answers share. Arc-shared
    /// so an execution can bind images while `&mut self` runs the rules.
    cache: Arc<crate::image::cache::ImageCache>,
    /// Heap executions count up; each tick is a fresh `ViewEpoch::Heap`,
    /// so no image or view memo can outlive the instance it was read from.
    heap_tick: u64,
    /// Route every Free Join rule through the cursor fallback: set by the
    /// test/diagnostic affordance, or for one bounded restart after the
    /// resident path's reservation refusal (chapter 12 §6 — never an
    /// endless replan loop).
    forced_fallback: bool,
    /// The main sink's RAM allowance before distinct state continues in
    /// the scratch map (`exec::scratch::DEFAULT_RAM_BYTES` by default).
    sink_ram: usize,
    /// Interiors then rec then main, as one pipeline sum: interiors
    /// live inside each arm, never as a sidecar. Dead main is
    /// `Cq { rules: [] }` — Empty is not a variant. Main rules share
    /// the ONE sink below
    /// rule loop): the sink resets once per execution, never per rule,
    /// and its seen-set spanning rules is the entire implementation of
    /// ∪ — no merge node, no concat-then-dedup pass exists.
    pub(crate) pipeline: PreparedPipeline,
    /// Derived-tuples budget. Judged after each interior and between
    /// rec rounds. Host-settable on every prepared query. The rounds
    /// axis lives on [`PreparedPipeline::Reach`].
    tuples_budget: u64,
    /// Finished derived images (interiors then rec) plus per-occurrence
    /// bind scratch for `run_join`'s Interior arm.
    derived: crate::api::prepared::reach::DerivedImages,
    /// The signature the query defines, sealed at validation and cloned
    /// here at prepare. It sits beside the pipeline because a dead-main
    /// Cq still has an arity and buffer types (the empty path's
    /// `out.arity` reads it).
    signature: Signature,
    /// Dense per-param bind contracts (validation rejects id gaps): one
    /// sum carries scalar/set/mask shape, element type, and point-domain
    /// status without parallel flags.
    params: Vec<ParamSpec>,
    /// Bind-time resolved constants, reused across executions — pooled
    /// storage: a set param's slot holds a [`Const::WordSet`] whose `Vec`
    /// is rebound in place (sorted, deduplicated words; capacity
    /// retained across differently-sized warm re-binds).
    resolved_params: Vec<Const>,
    /// `str` literals in the rules' templates still awaiting their
    /// dictionary word ([`Const::PendingIntern`]): decremented as each
    /// latches (`bind.rs`), and [`Latch::Latched`] — with no params of
    /// any shape — is the fully-latched fast path: `resolve_filters` is
    /// skipped entirely, the resolved tables having been written once
    /// and final.
    latch: Latch,
    /// Per param slot: the last successful String resolution — the
    /// bound text and its word (`bind.rs`). A resident intern HIT is
    /// final (append-only). A scratch word is bound to
    /// [`NonresidentTextStore::epoch`] and forgotten when the store is dropped.
    param_word_memo: Vec<ParamWordMemo>,
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
    /// The rule-shared binding-slot scratch (`40-execution.md` § the
    /// rule loop): written in place by each rule's recursion, re-sized
    /// to the rule's slot layout at rule entry — capacity is the
    /// high-water across all rules.
    bindings: Bindings,
    /// Aggregate-finalization answer scratch.
    answer_scratch: Vec<u64>,
    /// The per-finalize intern-resolution memo.
    resolve_memo: ResolveMemo,
    /// `KeyProbe` resolved-key word scratch.
    key_scratch: Vec<u64>,
    /// Scratch-backed text resolver. Opened only on
    /// [`crate::image::ResidentAdmit::BeyondMemory`] via
    /// [`crate::image::ResidentTextExhausted::open_nonresident`].
    nonresident: Option<crate::image::NonresidentTextStore>,
    /// Source-visit census of the last execute (D10).
    #[cfg(test)]
    last_visits: usize,
    #[cfg(test)]
    used_nonresident_text: bool,
    /// `Some(first computed find)` when any rule carries a computed
    /// scalar output: execution enters ONE [`NumericalGuard`] for the
    /// whole engine operation (chapter 11 §3 — never per tuple). The
    /// find index names the diagnostic position for an unsupported
    /// numerical platform.
    ///
    /// [`NumericalGuard`]: crate::exec::kernel::numeric::NumericalGuard
    numeric_outputs: Option<crate::error::FindIndex>,
    /// The query in the rule notation ([`crate::ir::render`]), rendered
    /// once at prepare — the introspection report's header and the
    /// [`Self::rendered_query`] diagnostic accessor. Cold data: read only
    /// on diagnostic surfaces, never on the warm path.
    rendered: String,
    /// Marker: a prepared query is single-threaded scratch (`Cell` makes
    /// it `!Sync`), pinned to schema `S` (`fn() -> S` keeps auto-traits
    /// independent of `S`).
    marker: std::marker::PhantomData<PreparedMarker<S>>,
}

/// One named interior's prepared artifact: its rule loop and stage sink
/// — projection, aggregate, or computed, exactly like main (chapter 12's
/// uniform nonrecursive composition). Evaluated once, in declaration
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
        rule: KeyProbeRule,
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
        rounds_budget: u32,
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
/// kind can consume. Rec arms are [`RecArm`], inhabitable only in
/// [`reach::ReachDriver::rec`].
#[expect(
    clippy::large_enum_variant,
    reason = "the decided representation keeps rule scratch inline; programs contain at most the validated rule cap"
)]
pub(crate) enum PreparedRule {
    FreeJoin(FreeJoinRule),
    KeyProbe(KeyProbeRule),
}

/// One rec arm: the unique positive self-atom is the delta occurrence.
/// Extra EDB / interior atoms are accumulated/EDB, never a second
/// delta. Inhabitable only in [`reach::ReachDriver::rec`].
pub(crate) struct RecArm {
    #[expect(
        dead_code,
        reason = "the rec arm's unique delta occurrence, recorded at prepare"
    )]
    delta: crate::ir::normalize::OccId,
    rule: FreeJoinRule,
}

pub(crate) struct FreeJoinRule {
    plan: ValidatedPlan,
    executor: Executor,
    /// The sealed cursor-fallback program (chapter 12 §3): the same rule
    /// over source cursors instead of images — used when forced (Q-FALLBACK)
    /// or after a resident reservation refusal (one bounded restart).
    fallback: fallback::FallbackRule,
    /// The rule's head projection: per head position, the output spec
    /// over this rule's binding-slot layout (result types live on the
    /// query — they are the head's, identical across rules).
    finds: Vec<FindSpec>,
    /// The rule's full slot array as `VarId`-ordered spans — the
    /// DNF-derived union regime's shared dedup key (ruled 2026-07-23,
    /// R2). Aggregate-bearing heads only; empty (and never read) for
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
    resolved_selections: Vec<Vec<Vec<u64>>>,
    /// This rule's resolved tables were fully written by a completed
    /// `resolve_filters` pass (a short-circuited pass leaves later
    /// slots unwritten and does not set it) — one leg of the
    /// fully-latched fast path.
    resolution: ResolutionState,
    /// The view memo : per occurrence, the active binding
    /// (whose COLT the executor consumes) plus parked bindings under LRU.
    memo: ViewMemo,
    /// Per participating occurrence, the statistics the rule's plan was
    /// costed with. Cold data — written once at build, read by the
    /// stats surface when a caller asks.
    #[expect(dead_code, reason = "prepare-time pins for the stats surface")]
    pinned: Box<[OccurrencePin]>,
}

/// One occurrence's pinned prepare-time statistics — the stats surface
/// renders ("estimated from (pinned rows at prepare)"). Participating
/// occurrences only: negated and grounding-eliminated occurrences enter
/// no DP state and earn no statistics read at prepare.
#[derive(Debug, Clone, Copy)]
#[expect(dead_code, reason = "prepare-time pins for the stats surface")]
pub(super) struct OccurrencePin {
    pub occ_id: crate::ir::normalize::OccId,
    pub relation: bumbledb_theory::schema::RelationId,
    pub rows: u64,
    pub survivors: Option<u64>,
}

pub(crate) struct KeyProbeRule {
    plan: KeyProbePlan,
    distinct_witness: Option<crate::plan::fj::DistinctWitness>,
    finds: Vec<FindSpec>,
    /// As [`FreeJoinRule::dedup_spans`] — the R2 shared-slot key over
    /// this rule's key-probe layout.
    dedup_spans: Box<[(usize, usize)]>,
}

impl<S> PreparedQuery<S> {
    fn visit_rules(&self, mut visit: impl FnMut(&PreparedRule)) {
        match &self.pipeline {
            PreparedPipeline::PointProbe { .. } => {}
            PreparedPipeline::Cq { interiors, rules } => {
                for interior in interiors {
                    for rule in &interior.rules {
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
                    for rule in &interior.rules {
                        visit(rule);
                    }
                }
                for rule in &driver.base {
                    visit(rule);
                }
                for rule in main {
                    visit(rule);
                }
            }
        }
    }

    /// Every prepared rule this query carries — interiors, rec base,
    /// then main. Rec arms are [`RecArm`], visited via
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

    fn visit_rec_arms_mut(&mut self, mut visit: impl FnMut(&mut RecArm)) {
        if let PreparedPipeline::Reach { driver, .. } = &mut self.pipeline {
            for arm in &mut driver.rec {
                visit(arm);
            }
        }
    }

    /// Memory-pressure trim (Q-LIFETIME): drop every cached relation
    /// image, parked view binding and active COLT this prepared query
    /// retains. The next execution rebuilds what it touches — a trim can
    /// make the next query allocate or use disk; it never changes answers.
    /// Interner tokens stay (token stability is the cache's invariant;
    /// dropping the whole prepared query is the text trim unit).
    pub fn trim(&mut self) {
        self.cache.trim();
        self.derived = reach::DerivedImages::default();
        self.visit_rules_mut(|rule| {
            if let PreparedRule::FreeJoin(fj) = rule {
                fj.memo.trim();
            }
        });
        self.visit_rec_arms_mut(|arm| arm.rule.memo.trim());
    }

    /// Retained bytes across this prepared query's caches (images plus
    /// interner text) — a host budgeting figure, not an allocator
    /// measurement.
    #[must_use]
    pub fn retained_cache_bytes(&self) -> usize {
        self.cache.retained_bytes()
    }

    #[cfg(test)]
    pub(crate) fn open_from_exhausted(
        exhausted: &crate::image::ResidentTextExhausted,
        work: &crate::work::WorkContext,
    ) -> crate::error::Result<crate::image::NonresidentTextStore> {
        text::open_from_exhausted(exhausted, work)
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn last_visits(&self) -> usize {
        self.last_visits
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn used_nonresident_text(&self) -> bool {
        self.used_nonresident_text
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn uncharged_copy_bytes(&self) -> usize {
        self.resolve_memo.uncharged_copy_bytes()
    }

    fn visit_free_join(&self, mut visit: impl FnMut(&FreeJoinRule)) {
        self.visit_rules(|rule| {
            if let PreparedRule::FreeJoin(fj) = rule {
                visit(fj);
            }
        });
        if let PreparedPipeline::Reach { driver, .. } = &self.pipeline {
            for arm in &driver.rec {
                visit(&arm.rule);
            }
        }
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
    /// union regime's shared dedup key (R2); empty for projection heads.
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
/// Resident intern HITS are final. Scratch words carry
/// [`NonresidentTextStore::epoch`] (instance owner id) and must not
/// outlive that store.
#[derive(Debug, Default, Clone)]
struct ParamWordMemo {
    text: String,
    word: Option<u64>,
    /// `None` = resident intern. `Some` = the minting store's owner epoch.
    epoch: Option<crate::image::TextStoreEpoch>,
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

/// One executed binding: a real epoch plus the residual filters it was
/// built for. Active and parked slots move this shape; the COLT lives
/// on [`ViewMemo::colts`] (active) or [`Parked::colt`] (parked).
struct Bound {
    epoch: crate::image::ViewEpoch,
    filters: Vec<FilterPredicate>,
    last_used: u64,
}

/// The three proofs a parallel `None` used to conflate.
enum Binding {
    /// Never executed, or vacated after a park (the rebuild lands here).
    Unbound,
    /// Interior occurrence: lives outside the epoch-keyed memo.
    Derived,
    Bound(Bound),
}

/// A parked [`Bound`] plus the COLT it owns. The kernel only sees
/// [`ViewMemo::colts`]; this COLT is off the slice until unparked.
struct Parked {
    bound: Bound,
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
    /// One slot per occurrence: active [`Binding`], parked [`Bound`]s,
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
    reason = "boxing the hot sink would add indirection to every emit"
)] // Projection stays unboxed: it is
// the hot variant (per-item emit paths reach through it), one prepared
// query holds exactly one sink, and the pipeline scratch answers
// that tripped the lint are the working set itself.
pub(super) enum EitherSink {
    Computed(Box<computed::ComputedSink>),
    Projection(ProjectionSink),
    /// Boxed: the batch-fold scratch grew the sink past the
    /// variant-size lint; one prepared query holds one sink, and the
    /// indirection is paid once per batch, never per answer.
    Aggregate(Box<AggregateSink>),
}
