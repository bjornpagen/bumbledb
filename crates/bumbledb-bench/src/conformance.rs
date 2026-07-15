//! The conformance lane (the covenant campaign, PRD 13): the Lean
//! denotation executes as the THIRD differential oracle.
//!
//! The dual-oracle blind spot: the engine and the naive model were
//! written from the same docs — a shared misreading passes every
//! two-way differential forever. The Lean tree
//! (`lean/Bumbledb/Query/Denotation.lean`) is derived from the
//! mathematics, and its executable half `evalList` is PROVED equal to
//! the set denotation (`eval_sound`), so evaluating it on real Tiny
//! worlds is a DENOTATION check, not a third implementation.
//!
//! This module is the Rust half of the lane:
//!
//! * the **serializer** — one hand-readable JSON document per case
//!   (`{ theory, instance, query, params, answers }`, tagged values,
//!   answers canonically sorted; format documented with an annotated
//!   example in `lean/conformance/README.md`);
//! * the **corpus builder** — seeded querygen cases (the valid arm
//!   behind the `Rng` seam, `Scale::Tiny`) plus the hand-picked shapes,
//!   written to `lean/conformance/cases/*.json` (checked in — the
//!   replay corpus);
//! * the **comparator** (`three_way_conformance_over_the_checked_in_corpus`)
//!   — per checked-in case, replayed from its recorded provenance: the
//!   engine fresh, the naive model fresh, byte-compare against the
//!   checked-in file, then `lake exe conformance` for the Lean side. Any disagreement names the case file. A DISAGREEMENT IS
//!   A TROPHY (engine bug / naive-model bug / spec bug — triage per the
//!   fuzzing charter, `docs/architecture/60-validation.md`); this test
//!   reports, it never fixes.
//!
//! The RECURSIVE arm ([`program`], `program-*.json`) rides the same
//! corpus and comparator: program cases the naive fixpoint and the
//! `SQLite` recursive lane agreed on, judged by the proved
//! `lean/Bumbledb/Exec/Fixpoint.lean: evalProgram` — the third oracle
//! wired for recursion before the engine can run one program.
//!
//! ## Scope fences (each counted in [`Report`], never silent)
//!
//! * Tiny scale, the valid querygen arm only — the hostile arm
//!   (`corpus_gen::irgen`) types nothing and stays with the fuzz lane.
//! * **Unresolved string literals excluded** (the latch): the model has
//!   no intern dictionary — the serializer interns per case, and a
//!   query/param string absent from the world's vocabulary is a
//!   recorded, principled exclusion.
//! * **Membership on negated atoms excluded**: the Lean anti-join
//!   quantifies `Matches` over binding terms, and point membership is a
//!   typing rule the serializer lowers to a fresh variable + `PointIn`
//!   condition — a lowering with no home inside a negated atom.
//! * **Element-typed param-set membership excluded**: the lowered
//!   `PointIn`-with-set comparison would violate the Lean shape
//!   discipline (`WellTyped`) that `eval_sound` names as its premise.
//! * **Runtime-error executions excluded** (`Overflow`, `MeasureOfRay`):
//!   the model reads a ray's measure as `none` where the engine raises —
//!   the recorded Level-0 narrowing; the lane compares answer sets on
//!   error-free executions only.
//! * **Slow and wide cases excluded by budget** (naive wall time / answer
//!   rows): the corpus is a per-push CI lane; the caps are counted and
//!   recorded, and shrink the case, never the model.
//!
//! ## The membership lowering (recorded)
//!
//! The engine's membership BINDING is a typing rule, not a syntax node
//! (`ir/validate/context.rs::resolve_bivalents`). The Lean matching
//! equation reads every binding as value selection, so the serializer
//! performs the same resolution the validator does: an element-typed
//! term on an interval field becomes a fresh interval variable plus a
//! `PointIn` condition — the exact predicate form the typing rule
//! licenses (`lean/Bumbledb/Query/Syntax.lean`, the membership note).
//! The engine executes the ORIGINAL query; Lean evaluates the lowered
//! one; agreement of the two is part of what the lane checks.
//!
//! No engine `pub` accessor was needed: `Answers` extraction via
//! `differential::engine_query` sufficed (recorded per the PRD).

pub mod judgment;
pub mod program;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use bumbledb::schema::ValueType;
use bumbledb::{
    AggOp, AllenMask, Atom, Basic, CmpOp, Comparison, ConditionTree, Db, FieldId, FindTerm,
    MaskTerm, ParamId, Query, RelationId, Rule, Term, Value, VarId,
};

use crate::corpus_gen::{GenConfig, Rng, Scale};
use crate::differential::{self, Answers};
use crate::naive::{Delta, NaiveDb, ParamValue, Tuple};
use crate::querygen::{self, ParamDraw, target};

/// The two Tiny world seeds the corpus alternates over — conformance's
/// own constants (the fuzz lane's cached worlds are its own).
pub const WORLD_SEEDS: [u64; 2] = [0x00C0_4F01, 0x00C0_4F02];

/// The seeded-case target (the PRD's N ≈ 200; hand cases ride on top).
pub const SEEDED_CASES: usize = 200;

/// Per-case seed base: `case_seed = CASE_SEED_BASE + attempt` — recorded
/// in each case's provenance, so the comparator replays the exact query
/// and draw through `Rng::new(case_seed)`.
pub const CASE_SEED_BASE: u64 = 0x0013_0000;

/// The naive-model wall-time budget per case. The Lean evaluator is the
/// same nested-loop cost class (`evalList` — join, filters, projection),
/// so the naive wall is the proxy that keeps the corpus run a per-push
/// lane; over-budget cases are counted (`excluded_slow`), never silent.
const NAIVE_BUDGET_MS: u128 = 25;

/// The answer-row cap per case (file size + Lean parse time); counted
/// (`excluded_wide`), never silent.
const MAX_ANSWER_ROWS: usize = 512;

/// The fragment-coverage report: cases expressible / generated, with
/// every exclusion named and counted (the crucible's no-silent-caps
/// rule).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    /// Candidate (query, draw) pairs attempted.
    pub attempted: u64,
    /// Cases written to the corpus.
    pub written: u64,
    /// A query/param string literal outside the world's vocabulary.
    pub excluded_unresolved: u64,
    /// A membership binding on a negated atom.
    pub excluded_negated_membership: u64,
    /// An element-typed param-set membership binding.
    pub excluded_set_membership: u64,
    /// The engine answered `Overflow` / `MeasureOfRay`.
    pub excluded_engine_error: u64,
    /// Naive wall time over [`NAIVE_BUDGET_MS`].
    pub excluded_slow: u64,
    /// Answer set over [`MAX_ANSWER_ROWS`].
    pub excluded_wide: u64,
}

impl Report {
    /// The coverage line the builder and comparator log.
    #[must_use]
    pub fn coverage_line(&self) -> String {
        format!(
            "conformance coverage: {}/{} expressible (excluded: {} unresolved-literal, \
             {} negated-membership, {} set-membership, {} engine-error, {} slow, {} wide)",
            self.written,
            self.attempted,
            self.excluded_unresolved,
            self.excluded_negated_membership,
            self.excluded_set_membership,
            self.excluded_engine_error,
            self.excluded_slow,
            self.excluded_wide,
        )
    }
}

/// Why one candidate case is outside the lane's fragment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Exclusion {
    UnresolvedLiteral,
    NegatedMembership,
    SetMembership,
}

/// One loaded Tiny world: the engine store, the naive model, and the
/// per-world intern dictionary (string → dense id, first-seen order
/// over the corpus streams — the ids Lean compares, engine-independent).
pub struct World {
    pub cfg: GenConfig,
    pub db: Db<target::Target>,
    pub naive: NaiveDb,
    dict: BTreeMap<Box<[u8]>, u64>,
    dict_order: Vec<Box<[u8]>>,
    _dir: ScratchDir,
}

/// A self-cleaning scratch directory for the engine store.
struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new(tag: &str) -> Self {
        // Unique per run (pid + wall-clock nanos): a fixed tag path lets
        // a concurrent or wedged prior run collide on the LMDB flock.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "bumbledb-conformance-{tag}-{}-{nanos}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create conformance scratch dir");
        Self(path)
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Builds one Tiny world: engine bulk loads in declaration order (the
/// DU cluster in joint chunks, as the verify harness loads it), the
/// naive model seeded from the descriptor and judged over the whole
/// corpus as one delta, and the intern dictionary collected from the
/// corpus streams and the sealed extensions.
///
/// # Panics
///
/// On tool-level failures (store creation, a corpus the theory
/// rejects) — never on a disagreement.
#[must_use]
pub fn build_world(seed: u64) -> World {
    let cfg = GenConfig {
        seed,
        scale: Scale::Tiny,
    };
    let dir = ScratchDir::new(&format!("{seed:08x}"));
    let db = Db::create(&dir.0, target::Target).expect("create conformance target store");
    let mut naive = NaiveDb::new(&target::descriptor());
    let mut delta = Delta::default();
    for rel in 0..target::TARGET_RELATIONS {
        let rel = RelationId(rel);
        match rel {
            target::ids::JOURNAL_ENTRY => load_du_cluster(&db, cfg),
            target::ids::IMPORT_BATCH => {} // loaded with its entries
            _ => {
                db.bulk_load_dyn(rel, target::corpus_relation_rows(cfg, rel))
                    .expect("conformance target bulk load");
            }
        }
        for fact in target::corpus_relation_rows(cfg, rel) {
            delta.inserts.push((rel, fact));
        }
    }
    // The fixed-width Lane (`interval<i64, 5>`) sits after the closed
    // vocabulary, so the `0..TARGET_RELATIONS` sweep skips it — its own
    // load here (statement-free payload, no draws: every earlier
    // relation's corpus stream is byte-stable).
    db.bulk_load_dyn(
        target::ids::LANE,
        target::corpus_relation_rows(cfg, target::ids::LANE),
    )
    .expect("conformance lane bulk load");
    for fact in target::corpus_relation_rows(cfg, target::ids::LANE) {
        delta.inserts.push((target::ids::LANE, fact));
    }
    naive
        .apply(&delta)
        .expect("the Tiny corpus satisfies the statements");
    let mut world = World {
        cfg,
        db,
        naive,
        dict: BTreeMap::new(),
        dict_order: Vec::new(),
        _dir: dir,
    };
    for rel in 0..target::TARGET_RELATIONS {
        for fact in target::corpus_relation_rows(cfg, RelationId(rel)) {
            for value in &fact {
                world.intern(value);
            }
        }
    }
    for relation in &target::descriptor().relations {
        if let Some(extension) = &relation.extension {
            for row in extension {
                for value in &row.values {
                    world.intern(value);
                }
            }
        }
    }
    world
}

/// The `JournalEntry == ImportBatch` cluster in joint chunks: the DU
/// `==` statement holds in neither one-relation prefix, so entries and
/// their import batches commit together.
fn load_du_cluster(db: &Db<target::Target>, cfg: GenConfig) {
    const CHUNK: u64 = 4096;
    let domains = target::Domains::of(cfg.scale);
    let entries = target::corpus_rows(&domains, target::ids::JOURNAL_ENTRY);
    let batches = target::corpus_rows(&domains, target::ids::IMPORT_BATCH);
    let mut next_batch = 0u64;
    let mut start = 0u64;
    while start < entries {
        let end = (start + CHUNK).min(entries);
        db.write(|tx| {
            for i in start..end {
                let fact = target::corpus_row(cfg, &domains, target::ids::JOURNAL_ENTRY, i);
                tx.insert_dyn(target::ids::JOURNAL_ENTRY, &fact)?;
            }
            while next_batch < batches && target::import_batch_entry(next_batch) < end {
                let fact = target::corpus_row(cfg, &domains, target::ids::IMPORT_BATCH, next_batch);
                tx.insert_dyn(target::ids::IMPORT_BATCH, &fact)?;
                next_batch += 1;
            }
            Ok(())
        })
        .expect("conformance DU cluster load");
        start = end;
    }
}

impl World {
    /// Adds a string value to the dictionary (first-seen order).
    fn intern(&mut self, value: &Value) {
        if let Value::String(bytes) = value
            && !self.dict.contains_key(bytes)
        {
            let id = u64::try_from(self.dict_order.len()).expect("dictionary fits u64");
            self.dict.insert(bytes.clone(), id);
            self.dict_order.push(bytes.clone());
        }
    }

    /// The dictionary id of a string, or the unresolved-literal
    /// exclusion.
    fn resolve(&self, bytes: &[u8]) -> Result<u64, Exclusion> {
        self.dict
            .get(bytes)
            .copied()
            .ok_or(Exclusion::UnresolvedLiteral)
    }
}

/// The Allen basic names, in `Basic::ALL` order — the mask spelling of
/// the interchange format (and of `lean/Bumbledb/Query/Syntax.lean`'s
/// `AllenRel`).
const BASIC_NAMES: [&str; 13] = [
    "before",
    "meets",
    "overlaps",
    "starts",
    "during",
    "finishes",
    "equals",
    "finished_by",
    "contains",
    "started_by",
    "overlapped_by",
    "met_by",
    "after",
];

/// Serializes an Allen mask as its admitted-relation name list.
fn push_mask(out: &mut String, mask: AllenMask) {
    out.push('[');
    let mut first = true;
    for (basic, name) in Basic::ALL.iter().zip(BASIC_NAMES) {
        if mask.contains(*basic) {
            if !first {
                out.push(',');
            }
            first = false;
            let _ = write!(out, "\"{name}\"");
        }
    }
    out.push(']');
}

/// [`push_value`] AT A FIELD'S TYPE: a fixed-width interval position
/// renders `[start, width]` under the family's own tag — the width is
/// the type, so the field re-derives the spelling and
/// `Conformance.lean: decodeValue` re-checks the Q2 bound decoding it.
/// Every other type falls through to the value's own spelling.
fn push_value_typed(
    world: &World,
    used: &mut BTreeSet<u64>,
    out: &mut String,
    value: &Value,
    ty: Option<&ValueType>,
) -> Result<(), Exclusion> {
    if let Some(ValueType::Interval { width: Some(w), .. }) = ty {
        match value {
            Value::IntervalU64(iv) => {
                debug_assert_eq!(iv.end() - iv.start(), *w, "typed writes checked the width");
                let _ = write!(out, "{{\"interval_u64_fixed\":[{},{w}]}}", iv.start());
                return Ok(());
            }
            Value::IntervalI64(iv) => {
                let _ = write!(out, "{{\"interval_i64_fixed\":[{},{w}]}}", iv.start());
                return Ok(());
            }
            _ => {}
        }
    }
    push_value(world, used, out, value)
}

/// Serializes one value in the tagged form; strings resolve through the
/// world dictionary (`used` collects the ids the case actually spends,
/// so the emitted dictionary stays small).
fn push_value(
    world: &World,
    used: &mut BTreeSet<u64>,
    out: &mut String,
    value: &Value,
) -> Result<(), Exclusion> {
    match value {
        Value::Bool(v) => {
            let _ = write!(out, "{{\"bool\":{v}}}");
        }
        Value::U64(v) => {
            let _ = write!(out, "{{\"u64\":{v}}}");
        }
        Value::I64(v) => {
            let _ = write!(out, "{{\"i64\":{v}}}");
        }
        Value::String(bytes) => {
            let id = world.resolve(bytes)?;
            used.insert(id);
            let _ = write!(out, "{{\"str\":{id}}}");
        }
        Value::FixedBytes(bytes) => {
            out.push_str("{\"bytes\":[");
            for (index, byte) in bytes.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                let _ = write!(out, "{byte}");
            }
            out.push_str("]}");
        }
        Value::IntervalU64(iv) => {
            let _ = write!(out, "{{\"interval_u64\":[{},{}]}}", iv.start(), iv.end());
        }
        Value::IntervalI64(iv) => {
            let _ = write!(out, "{{\"interval_i64\":[{},{}]}}", iv.start(), iv.end());
        }
        Value::AllenMask(mask) => {
            out.push_str("{\"mask\":");
            push_mask(out, *mask);
            out.push('}');
        }
    }
    Ok(())
}

/// Serializes one fact as a value array, at its relation's positional
/// types (`types` empty for answer rows — a fixed-width FIND is not a
/// case shape this lane carries: the engine's answer channel widens to
/// bounds, so the corpus keeps fixed values in instance columns and
/// scalar finds in heads).
fn push_fact(
    world: &World,
    used: &mut BTreeSet<u64>,
    out: &mut String,
    fact: &[Value],
    types: &[ValueType],
) -> Result<(), Exclusion> {
    out.push('[');
    for (index, value) in fact.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_value_typed(world, used, out, value, types.get(index))?;
    }
    out.push(']');
    Ok(())
}

/// Serializes one term.
fn push_term(
    world: &World,
    used: &mut BTreeSet<u64>,
    out: &mut String,
    term: &Term,
) -> Result<(), Exclusion> {
    match term {
        Term::Var(v) => {
            let _ = write!(out, "{{\"var\":{}}}", v.0);
        }
        Term::Param(p) => {
            let _ = write!(out, "{{\"param\":{}}}", p.0);
        }
        Term::ParamSet(p) => {
            let _ = write!(out, "{{\"param_set\":{}}}", p.0);
        }
        Term::Literal(value) => {
            out.push_str("{\"lit\":");
            push_value(world, used, out, value)?;
            out.push('}');
        }
        Term::Measure(v) => {
            let _ = write!(out, "{{\"measure\":{}}}", v.0);
        }
    }
    Ok(())
}

/// Serializes one comparison leaf (the operator flattened into the
/// object; an Allen mask rides beside it).
fn push_comparison(
    world: &World,
    used: &mut BTreeSet<u64>,
    out: &mut String,
    cmp: &Comparison,
) -> Result<(), Exclusion> {
    out.push_str("{\"cmp\":{\"op\":");
    match cmp.op {
        CmpOp::Eq => out.push_str("\"eq\""),
        CmpOp::Ne => out.push_str("\"ne\""),
        CmpOp::Lt => out.push_str("\"lt\""),
        CmpOp::Le => out.push_str("\"le\""),
        CmpOp::Gt => out.push_str("\"gt\""),
        CmpOp::Ge => out.push_str("\"ge\""),
        CmpOp::PointIn => out.push_str("\"point_in\""),
        CmpOp::Allen { mask } => {
            out.push_str("\"allen\"");
            match mask {
                MaskTerm::Literal(mask) => {
                    out.push_str(",\"mask\":");
                    push_mask(out, mask);
                }
                MaskTerm::Param(p) => {
                    let _ = write!(out, ",\"mask_param\":{}", p.0);
                }
            }
        }
    }
    out.push_str(",\"lhs\":");
    push_term(world, used, out, &cmp.lhs)?;
    out.push_str(",\"rhs\":");
    push_term(world, used, out, &cmp.rhs)?;
    out.push_str("}}");
    Ok(())
}

/// Serializes one condition tree node.
fn push_condition(
    world: &World,
    used: &mut BTreeSet<u64>,
    out: &mut String,
    tree: &ConditionTree,
) -> Result<(), Exclusion> {
    match tree {
        ConditionTree::Leaf(cmp) => push_comparison(world, used, out, cmp),
        ConditionTree::And(children) | ConditionTree::Or(children) => {
            out.push_str(if matches!(tree, ConditionTree::And(_)) {
                "{\"and\":["
            } else {
                "{\"or\":["
            });
            for (index, child) in children.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                push_condition(world, used, out, child)?;
            }
            out.push_str("]}");
            Ok(())
        }
    }
}

/// Serializes one find term.
fn push_find(out: &mut String, find: &FindTerm) {
    match find {
        FindTerm::Var(v) => {
            let _ = write!(out, "{{\"var\":{}}}", v.0);
        }
        FindTerm::Measure(v) => {
            let _ = write!(out, "{{\"measure\":{}}}", v.0);
        }
        FindTerm::Aggregate { op, over } => match (op, over) {
            (AggOp::Count, None) => out.push_str("{\"agg\":{\"op\":\"count\"}}"),
            (AggOp::CountDistinct, Some(v)) => {
                let _ = write!(
                    out,
                    "{{\"agg\":{{\"op\":\"count_distinct\",\"over\":{}}}}}",
                    v.0
                );
            }
            (AggOp::Sum, Some(v)) => {
                let _ = write!(out, "{{\"agg\":{{\"op\":\"sum\",\"over\":{}}}}}", v.0);
            }
            (AggOp::Min, Some(v)) => {
                let _ = write!(out, "{{\"agg\":{{\"op\":\"min\",\"over\":{}}}}}", v.0);
            }
            (AggOp::Max, Some(v)) => {
                let _ = write!(out, "{{\"agg\":{{\"op\":\"max\",\"over\":{}}}}}", v.0);
            }
            (AggOp::Pack, Some(v)) => {
                let _ = write!(out, "{{\"agg\":{{\"op\":\"pack\",\"over\":{}}}}}", v.0);
            }
            (AggOp::ArgMax { key }, Some(v)) => {
                let _ = write!(
                    out,
                    "{{\"agg\":{{\"op\":\"arg_max\",\"over\":{},\"key\":{}}}}}",
                    v.0, key.0
                );
            }
            (AggOp::ArgMin { key }, Some(v)) => {
                let _ = write!(
                    out,
                    "{{\"agg\":{{\"op\":\"arg_min\",\"over\":{},\"key\":{}}}}}",
                    v.0, key.0
                );
            }
            other => unreachable!("validated: no such aggregate shape {other:?}"),
        },
        FindTerm::AggregateMeasure { op, over } => {
            let name = match op {
                AggOp::Sum => "sum",
                AggOp::Min => "min",
                AggOp::Max => "max",
                other => unreachable!("validated: measure folds are Sum/Min/Max, got {other:?}"),
            };
            let _ = write!(
                out,
                "{{\"agg_measure\":{{\"op\":\"{name}\",\"over\":{}}}}}",
                over.0
            );
        }
    }
}

/// Whether `(relation, field)` is interval-typed in the target schema.
fn field_is_interval(relation: RelationId, field: FieldId) -> bool {
    matches!(
        target::schema().relation(relation).field(field).value_type,
        ValueType::Interval { .. }
    )
}

/// One rule's variable count — the naive model's `count_vars`, walked
/// over every syntactic site (bindings, conditions, finds, Arg keys).
fn count_vars(rule: &Rule) -> u16 {
    fn see(count: &mut u16, var: VarId) {
        *count = (*count).max(var.0 + 1);
    }
    fn see_term(count: &mut u16, term: &Term) {
        if let Term::Var(var) | Term::Measure(var) = term {
            see(count, *var);
        }
    }
    fn see_tree(count: &mut u16, tree: &ConditionTree) {
        match tree {
            ConditionTree::Leaf(Comparison { lhs, rhs, .. }) => {
                see_term(count, lhs);
                see_term(count, rhs);
            }
            ConditionTree::And(children) | ConditionTree::Or(children) => {
                for child in children {
                    see_tree(count, child);
                }
            }
        }
    }
    let mut count = 0;
    for atom in rule.atoms.iter().chain(&rule.negated) {
        for (_, term) in &atom.bindings {
            see_term(&mut count, term);
        }
    }
    for tree in &rule.conditions {
        see_tree(&mut count, tree);
    }
    for find in &rule.finds {
        match find {
            FindTerm::Var(var) | FindTerm::Measure(var) => see(&mut count, *var),
            FindTerm::AggregateMeasure { over, .. } => see(&mut count, *over),
            FindTerm::Aggregate { op, over } => {
                if let Some(var) = over {
                    see(&mut count, *var);
                }
                if let AggOp::ArgMax { key } | AggOp::ArgMin { key } = op {
                    see(&mut count, *key);
                }
            }
        }
    }
    count
}

/// Which variables are scalar-anchored (bound on some non-interval
/// field of a positive atom) — the bivalent-resolution rule: an
/// anchored variable's interval-field occurrence is point membership;
/// an unanchored one is interval-typed and its occurrence is value
/// equality.
fn scalar_anchors(rule: &Rule, var_count: u16) -> Vec<bool> {
    let mut anchored = vec![false; usize::from(var_count)];
    for atom in &rule.atoms {
        for (field, term) in &atom.bindings {
            if let Term::Var(var) = term
                && !field_is_interval(atom.relation(), *field)
            {
                anchored[usize::from(var.0)] = true;
            }
        }
    }
    anchored
}

/// Whether one binding term on an interval field reads as point
/// membership (the typing rule), value equality, or an excluded shape.
fn membership(term: &Term, anchored: &[bool], params: &[ParamValue]) -> Result<bool, Exclusion> {
    Ok(match term {
        Term::Var(v) => anchored[usize::from(v.0)],
        Term::Literal(value) => matches!(value, Value::U64(_) | Value::I64(_)),
        Term::Param(p) => match &params[usize::from(p.0)] {
            ParamValue::Scalar(Value::U64(_) | Value::I64(_)) => true,
            ParamValue::Scalar(_) => false,
            ParamValue::Set(_) => unreachable!("validated: scalar use of a set param"),
        },
        Term::ParamSet(p) => match &params[usize::from(p.0)] {
            ParamValue::Set(values) => {
                if matches!(values.first(), Some(Value::U64(_) | Value::I64(_))) {
                    return Err(Exclusion::SetMembership);
                }
                false
            }
            ParamValue::Scalar(_) => unreachable!("validated: set use of a scalar param"),
        },
        Term::Measure(_) => unreachable!("validated: no measure in bindings"),
    })
}

/// One rule after the membership lowering: rewritten positive atoms,
/// untouched negated atoms (membership there is the recorded
/// exclusion), and the original conditions plus the lowered `PointIn`
/// leaves.
struct LoweredRule<'a> {
    finds: &'a [FindTerm],
    atoms: Vec<Atom>,
    negated: &'a [Atom],
    conditions: Vec<ConditionTree>,
}

/// Performs the bivalent resolution the validator owns: element-typed
/// terms on interval fields become fresh interval variables plus
/// `PointIn` conditions (module doc, "the membership lowering").
fn lower_rule<'a>(rule: &'a Rule, params: &[ParamValue]) -> Result<LoweredRule<'a>, Exclusion> {
    let var_count = count_vars(rule);
    let anchored = scalar_anchors(rule, var_count);
    let mut fresh = var_count;
    let mut atoms = Vec::with_capacity(rule.atoms.len());
    let mut conditions = rule.conditions.clone();
    for atom in &rule.atoms {
        let mut bindings = Vec::with_capacity(atom.bindings.len());
        for (field, term) in &atom.bindings {
            if field_is_interval(atom.relation(), *field) && membership(term, &anchored, params)? {
                let interval_var = VarId(fresh);
                fresh += 1;
                bindings.push((*field, Term::Var(interval_var)));
                conditions.push(ConditionTree::Leaf(Comparison {
                    op: CmpOp::PointIn,
                    lhs: Term::Var(interval_var),
                    rhs: term.clone(),
                }));
            } else {
                bindings.push((*field, term.clone()));
            }
        }
        atoms.push(Atom {
            source: bumbledb::AtomSource::Edb(atom.relation()),
            bindings,
        });
    }
    for atom in &rule.negated {
        for (field, term) in &atom.bindings {
            if field_is_interval(atom.relation(), *field) && membership(term, &anchored, params)? {
                return Err(Exclusion::NegatedMembership);
            }
        }
    }
    Ok(LoweredRule {
        finds: &rule.finds,
        atoms,
        negated: &rule.negated,
        conditions,
    })
}

/// The relations a lowered query mentions, positive and negated — the
/// serialized instance carries exactly these (`snapshot_single`: the
/// denotation reads nothing else).
fn mentioned(rules: &[LoweredRule<'_>]) -> BTreeSet<RelationId> {
    let mut set = BTreeSet::new();
    for rule in rules {
        for atom in rule.atoms.iter().chain(rule.negated.iter()) {
            set.insert(atom.relation());
        }
    }
    set
}

/// The type tag of one field, as the format spells it.
fn type_name(value_type: &ValueType) -> String {
    match value_type {
        ValueType::Bool => "bool".into(),
        ValueType::U64 => "u64".into(),
        ValueType::I64 => "i64".into(),
        ValueType::String => "str".into(),
        ValueType::FixedBytes { len } => format!("bytes<{len}>"),
        ValueType::Interval {
            element: bumbledb::schema::IntervalElement::U64,
            width: None,
        } => "interval_u64".into(),
        ValueType::Interval {
            element: bumbledb::schema::IntervalElement::I64,
            width: None,
        } => "interval_i64".into(),
        // The fixed-width family: the width is the type, so it rides
        // the spelling (`bytes<N>`'s precedent) — `Main.lean:
        // typeOfName` parses exactly this form.
        ValueType::Interval {
            element: bumbledb::schema::IntervalElement::U64,
            width: Some(w),
        } => format!("interval_u64_fixed<{w}>"),
        ValueType::Interval {
            element: bumbledb::schema::IntervalElement::I64,
            width: Some(w),
        } => format!("interval_i64_fixed<{w}>"),
    }
}

/// A closed relation's facts, exactly as the naive model seeds them:
/// `[row id, payload…]` in declaration order.
fn closed_facts(relation: RelationId) -> Vec<Vec<Value>> {
    let descriptor = target::descriptor();
    let extension = descriptor.relations[relation.0 as usize]
        .extension
        .as_ref()
        .expect("closed relations carry extensions");
    extension
        .iter()
        .enumerate()
        .map(|(row, axiom)| {
            let mut fact = vec![Value::U64(
                u64::try_from(row).expect("extension rows fit u64"),
            )];
            fact.extend(axiom.values.iter().cloned());
            fact
        })
        .collect()
}

/// Serializes one full case document, or the exclusion that keeps it
/// out of the corpus. The answers are canonically sorted: each row
/// rendered in the tagged compact form, rows in lexicographic byte
/// order of that rendering (the README's canonical-order rule).
fn render_case(
    world: &World,
    name: &str,
    provenance: &str,
    query: &Query,
    params: &[ParamValue],
    answers: &BTreeSet<Tuple>,
) -> Result<String, Exclusion> {
    let mut used = BTreeSet::new();
    let lowered: Vec<LoweredRule<'_>> = query
        .rules
        .iter()
        .map(|rule| lower_rule(rule, params))
        .collect::<Result<_, _>>()?;

    // The query block.
    let mut query_block = String::from("{\"rules\":[\n");
    for (index, rule) in lowered.iter().enumerate() {
        if index > 0 {
            query_block.push_str(",\n");
        }
        query_block.push_str("{\"finds\":[");
        for (position, find) in rule.finds.iter().enumerate() {
            if position > 0 {
                query_block.push(',');
            }
            push_find(&mut query_block, find);
        }
        query_block.push_str("],\n \"atoms\":[");
        for (position, atom) in rule.atoms.iter().enumerate() {
            if position > 0 {
                query_block.push(',');
            }
            push_atom(world, &mut used, &mut query_block, atom)?;
        }
        query_block.push_str("],\n \"negated\":[");
        for (position, atom) in rule.negated.iter().enumerate() {
            if position > 0 {
                query_block.push(',');
            }
            push_atom(world, &mut used, &mut query_block, atom)?;
        }
        query_block.push_str("],\n \"conditions\":[");
        for (position, tree) in rule.conditions.iter().enumerate() {
            if position > 0 {
                query_block.push(',');
            }
            push_condition(world, &mut used, &mut query_block, tree)?;
        }
        query_block.push_str("]}");
    }
    query_block.push_str("\n]}");

    // The params block.
    let mut params_block = String::from("[");
    for (index, param) in params.iter().enumerate() {
        if index > 0 {
            params_block.push(',');
        }
        match param {
            ParamValue::Scalar(Value::AllenMask(mask)) => {
                params_block.push_str("{\"mask\":");
                push_mask(&mut params_block, *mask);
                params_block.push('}');
            }
            ParamValue::Scalar(value) => {
                params_block.push_str("{\"scalar\":");
                push_value(world, &mut used, &mut params_block, value)?;
                params_block.push('}');
            }
            ParamValue::Set(values) => {
                params_block.push_str("{\"set\":[");
                for (position, value) in values.iter().enumerate() {
                    if position > 0 {
                        params_block.push(',');
                    }
                    push_value(world, &mut used, &mut params_block, value)?;
                }
                params_block.push_str("]}");
            }
        }
    }
    params_block.push(']');

    // The answers block, canonically sorted by rendered row.
    let mut rows: Vec<String> = Vec::with_capacity(answers.len());
    for tuple in answers {
        let mut row = String::new();
        push_fact(world, &mut used, &mut row, &tuple.0, &[])?;
        rows.push(row);
    }
    rows.sort_unstable();
    let answers_block = if rows.is_empty() {
        String::from("[]")
    } else {
        format!("[\n{}\n]", rows.join(",\n"))
    };

    // The theory + instance blocks (mentioned relations only —
    // `snapshot_single`: the denotation reads nothing else).
    let (relations_block, instance_block, axioms_block) =
        world_blocks(world, &mut used, mentioned(&lowered))?;

    // The used slice of the intern dictionary (hand-readability: the
    // ids Lean compares, with their texts beside them).
    let strings_block = strings_block(world, &used);

    Ok(format!(
        "{{\n\"case\":\"{name}\",\n\"provenance\":{provenance},\n\"strings\":{strings_block},\n\
         \"theory\":{{\"relations\":{relations_block},\n\"ground_axioms\":{axioms_block}}},\n\
         \"instance\":{instance_block},\n\"query\":{query_block},\n\"params\":{params_block},\n\
         \"answers\":{answers_block}\n}}\n"
    ))
}

/// The theory + instance + ground-axiom blocks for one
/// mentioned-relation set — the shared tail of the query and program
/// serializers (the recursive arm's cases carry the identical world
/// shape).
fn world_blocks(
    world: &World,
    used: &mut BTreeSet<u64>,
    mentioned: BTreeSet<RelationId>,
) -> Result<(String, String, String), Exclusion> {
    let schema = target::schema();
    let mut relations_block = String::from("[");
    let mut instance_block = String::from("[");
    let mut axioms_block = String::from("[");
    let mut open_count = 0usize;
    let mut closed_count = 0usize;
    for relation in mentioned {
        let descriptor = schema.relation(relation);
        if open_count + closed_count > 0 {
            relations_block.push_str(",\n");
        }
        let _ = write!(
            relations_block,
            "{{\"id\":{},\"name\":\"{}\",\"closed\":{},\"fields\":[",
            relation.0,
            descriptor.name(),
            descriptor.is_closed()
        );
        for (position, field) in descriptor.fields().iter().enumerate() {
            if position > 0 {
                relations_block.push(',');
            }
            let _ = write!(relations_block, "\"{}\"", type_name(&field.value_type));
        }
        relations_block.push_str("]}");
        // The sealed positional types (a closed relation's list opens
        // with the synthetic id, matching its id-prefixed facts) — the
        // fixed-width positions re-derive their `[start, width]`
        // spelling from these.
        let field_types: Vec<ValueType> = descriptor
            .fields()
            .iter()
            .map(|field| field.value_type.clone())
            .collect();
        let facts: Vec<Vec<Value>> = if descriptor.is_closed() {
            closed_facts(relation)
        } else {
            target::corpus_relation_rows(world.cfg, relation).collect()
        };
        let block = if descriptor.is_closed() {
            closed_count += 1;
            if closed_count > 1 {
                axioms_block.push_str(",\n");
            }
            &mut axioms_block
        } else {
            open_count += 1;
            if open_count > 1 {
                instance_block.push_str(",\n");
            }
            &mut instance_block
        };
        let _ = write!(block, "{{\"relation\":{},\"facts\":[", relation.0);
        block.push('\n');
        for (index, fact) in facts.iter().enumerate() {
            if index > 0 {
                block.push_str(",\n");
            }
            push_fact(world, used, block, fact, &field_types)?;
        }
        block.push_str("\n]}");
    }
    relations_block.push(']');
    instance_block.push(']');
    axioms_block.push(']');
    Ok((relations_block, instance_block, axioms_block))
}

/// The used slice of the intern dictionary (hand-readability: the ids
/// Lean compares, with their texts beside them).
fn strings_block(world: &World, used: &BTreeSet<u64>) -> String {
    let mut strings_block = String::from("[");
    for (index, id) in used.iter().enumerate() {
        if index > 0 {
            strings_block.push(',');
        }
        let text = std::str::from_utf8(&world.dict_order[usize::try_from(*id).expect("id fits")])
            .expect("corpus strings are UTF-8");
        let _ = write!(strings_block, "[{id},");
        crate::json::push_str_lit(&mut strings_block, text);
        strings_block.push(']');
    }
    strings_block.push(']');
    strings_block
}

/// Serializes one atom.
fn push_atom(
    world: &World,
    used: &mut BTreeSet<u64>,
    out: &mut String,
    atom: &Atom,
) -> Result<(), Exclusion> {
    let _ = write!(out, "{{\"relation\":{},\"bindings\":[", atom.relation().0);
    for (index, (field, term)) in atom.bindings.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let _ = write!(out, "[{},", field.0);
        push_term(world, used, out, term)?;
        out.push(']');
    }
    out.push_str("]}");
    Ok(())
}

/// One randomized draw as positional [`ParamValue`]s (dense ids).
fn positional(draw: &ParamDraw) -> Vec<ParamValue> {
    let len = draw.scalars.len() + draw.sets.len();
    let mut out: Vec<ParamValue> = vec![ParamValue::Scalar(Value::Bool(false)); len];
    for (param, value) in &draw.scalars {
        out[usize::from(param.0)] = ParamValue::Scalar(value.clone());
    }
    for (param, values) in &draw.sets {
        out[usize::from(param.0)] = ParamValue::Set(values.clone());
    }
    out
}

/// One candidate through the pipeline: naive (timed, budgeted), engine
/// fresh, parity asserted (an engine≠naive disagreement here is a
/// TROPHY — this lane reports it and stops; triage per the fuzzing
/// charter), then the serialized document or the counted exclusion.
///
/// # Panics
///
/// On an engine-vs-naive disagreement — deliberately loud: the corpus
/// builder refuses to check in an already-disputed case.
fn one_case(
    world: &World,
    name: &str,
    provenance: &str,
    query: &Query,
    params: &[ParamValue],
    report: &mut Report,
) -> Option<String> {
    report.attempted += 1;
    let (answers, naive_ms) = execute_case(world, name, query, params);
    let Some(answers) = answers else {
        report.excluded_engine_error += 1;
        return None;
    };
    if naive_ms > NAIVE_BUDGET_MS {
        report.excluded_slow += 1;
        return None;
    }
    if answers.len() > MAX_ANSWER_ROWS {
        report.excluded_wide += 1;
        return None;
    }
    match render_case(world, name, provenance, query, params, &answers) {
        Ok(document) => {
            report.written += 1;
            Some(document)
        }
        Err(Exclusion::UnresolvedLiteral) => {
            report.excluded_unresolved += 1;
            None
        }
        Err(Exclusion::NegatedMembership) => {
            report.excluded_negated_membership += 1;
            None
        }
        Err(Exclusion::SetMembership) => {
            report.excluded_set_membership += 1;
            None
        }
    }
}

/// One case fresh through BOTH oracles: the naive model (timed — the
/// builder's budget proxy) and the engine, parity asserted. `None`
/// answers = a defined runtime error on both sides.
///
/// # Panics
///
/// On an engine-vs-naive disagreement — a TROPHY, reported loudly with
/// the case name; triage per the fuzzing charter.
fn execute_case(
    world: &World,
    name: &str,
    query: &Query,
    params: &[ParamValue],
) -> (Option<BTreeSet<Tuple>>, u128) {
    let started = Instant::now();
    let model = match world.naive.query(query, params) {
        Ok(rows) => Answers::Ok(rows),
        Err(crate::naive::query::QueryError::Overflow { .. }) => Answers::Overflow,
        Err(crate::naive::query::QueryError::MeasureOfRay) => Answers::MeasureOfRay,
    };
    let naive_ms = started.elapsed().as_millis();
    let engine = differential::engine_query(&world.db, query, params);
    assert_eq!(
        engine, model,
        "TROPHY (engine vs naive) on conformance case {name}: triage per the fuzzing \
         charter\n{query:#?}\nparams: {params:#?}"
    );
    match engine {
        Answers::Ok(answers) => (Some(answers), naive_ms),
        Answers::Overflow | Answers::MeasureOfRay | Answers::FixpointBudget => (None, naive_ms),
    }
}

/// One hand-picked case: a name, a query, and its params.
struct HandCase {
    name: &'static str,
    query: Query,
    params: Vec<ParamValue>,
}

fn rule(
    finds: Vec<FindTerm>,
    atoms: Vec<Atom>,
    negated: Vec<Atom>,
    conditions: Vec<ConditionTree>,
) -> Rule {
    Rule {
        finds,
        atoms,
        negated,
        conditions,
    }
}

fn atom(relation: RelationId, bindings: &[(FieldId, Term)]) -> Atom {
    Atom {
        source: bumbledb::AtomSource::Edb(relation),
        bindings: bindings.to_vec(),
    }
}

fn v(id: u16) -> Term {
    Term::Var(VarId(id))
}

fn fv(id: u16) -> FindTerm {
    FindTerm::Var(VarId(id))
}

fn agg(op: AggOp, over: u16) -> FindTerm {
    FindTerm::Aggregate {
        op,
        over: Some(VarId(over)),
    }
}

/// The hand-picked shapes the PRD names: exact partition (Pack over the
/// Mandate segment groups, rays included), aggregates (empty-global,
/// Arg, `CountDistinct`, union fold), Allen masks (composite literal and
/// the mask-param face), Pack, negation, unions, membership, measure,
/// param sets, and the closed-relation join.
#[expect(
    clippy::too_many_lines,
    reason = "one flat case roster, data not logic"
)]
fn hand_cases(cfg: GenConfig) -> Vec<HandCase> {
    use target::ids;
    let domains = target::Domains::of(cfg.scale);
    // A committed mandate interval and an instant inside it (real
    // corpus values — the interval-param and point-literal cases).
    let (m_account, _, (m_start, m_end)) = target::mandate(cfg, &domains, 0);
    let instant = m_start.midpoint(m_end);
    let full_i64 = Value::IntervalI64(
        bumbledb::Interval::<i64>::new(i64::MIN, i64::MAX - 1).expect("nonempty"),
    );
    vec![
        // Pack over the exactly-partitioned Mandate segment groups —
        // abutting segments coalesce, gaps survive, a packed ray is a
        // ray (the exact-partition shape).
        HandCase {
            name: "hand-pack-exact-partition",
            query: Query::single(rule(
                vec![fv(0), agg(AggOp::Pack, 1)],
                vec![atom(
                    ids::MANDATE,
                    &[(ids::mandate::ACCOUNT, v(0)), (ids::mandate::ACTIVE, v(1))],
                )],
                vec![],
                vec![],
            )),
            params: vec![],
        },
        // The empty-global aggregate: no bindings, no groups, the EMPTY
        // answer set — never a zero row (`empty_global_no_answer`).
        HandCase {
            name: "hand-empty-global-aggregates",
            query: Query::single(rule(
                vec![
                    FindTerm::Aggregate {
                        op: AggOp::Count,
                        over: None,
                    },
                    agg(AggOp::Sum, 1),
                ],
                vec![atom(
                    ids::POSTING,
                    &[
                        (ids::posting::ACCOUNT, Term::Literal(Value::U64(999_999))),
                        (ids::posting::AMOUNT, v(1)),
                    ],
                )],
                vec![],
                vec![],
            )),
            params: vec![],
        },
        // Arg-restriction over the tie-rich amount: every attaining row
        // survives (`argmax_ties_all_kept`).
        HandCase {
            name: "hand-arg-max-ties",
            query: Query::single(rule(
                vec![fv(1), agg(AggOp::ArgMax { key: VarId(2) }, 0)],
                vec![atom(
                    ids::POSTING,
                    &[
                        (ids::posting::ID, v(0)),
                        (ids::posting::ACCOUNT, v(1)),
                        (ids::posting::AMOUNT, v(2)),
                    ],
                )],
                vec![],
                vec![],
            )),
            params: vec![],
        },
        // A composite Allen mask between two mandate intervals of one
        // account (DISJOINT = before ∪ meets ∪ met-by ∪ after).
        HandCase {
            name: "hand-allen-mixed-width",
            // Q1's element-domain rule through the third oracle: a
            // FIXED-width lane (`interval<i64, 5>`) Allen-classified
            // against the GENERAL mandate interval of one account —
            // mixed widths, one element domain, classified over derived
            // bounds (`Query/Denotation.lean: classifyValue`'s fixed
            // arms). Scalar finds only: the answer channel widens fixed
            // values to bounds, so fixed values live in the instance
            // columns where the Lean side decodes the family's own tag.
            query: Query::single(rule(
                vec![fv(0), fv(3)],
                vec![
                    atom(
                        ids::MANDATE,
                        &[(ids::mandate::ACCOUNT, v(0)), (ids::mandate::ACTIVE, v(1))],
                    ),
                    atom(
                        ids::LANE,
                        &[
                            (ids::lane::ACCOUNT, v(0)),
                            (ids::lane::LANE, v(2)),
                            (ids::lane::TAG, v(3)),
                        ],
                    ),
                ],
                vec![],
                vec![ConditionTree::Leaf(Comparison {
                    op: CmpOp::Allen {
                        mask: MaskTerm::Literal(AllenMask::STARTS),
                    },
                    lhs: v(2),
                    rhs: v(1),
                })],
            )),
            params: vec![],
        },
        HandCase {
            name: "hand-allen-composite-mask",
            query: Query::single(rule(
                vec![fv(0), fv(1), fv(2)],
                vec![
                    atom(
                        ids::MANDATE,
                        &[(ids::mandate::ACCOUNT, v(0)), (ids::mandate::ACTIVE, v(1))],
                    ),
                    atom(
                        ids::MANDATE,
                        &[(ids::mandate::ACCOUNT, v(0)), (ids::mandate::ACTIVE, v(2))],
                    ),
                ],
                vec![],
                vec![ConditionTree::Leaf(Comparison {
                    op: CmpOp::Allen {
                        mask: MaskTerm::Literal(AllenMask::DISJOINT),
                    },
                    lhs: v(1),
                    rhs: v(2),
                })],
            )),
            params: vec![],
        },
        // The mask-param face: the same shape with the mask resolved at
        // bind (`ParamEnv.mask` on the Lean side).
        HandCase {
            name: "hand-allen-mask-param",
            query: Query::single(rule(
                vec![fv(0), fv(1), fv(2)],
                vec![
                    atom(
                        ids::MANDATE,
                        &[(ids::mandate::ACCOUNT, v(0)), (ids::mandate::ACTIVE, v(1))],
                    ),
                    atom(
                        ids::MANDATE,
                        &[(ids::mandate::ACCOUNT, v(0)), (ids::mandate::ACTIVE, v(2))],
                    ),
                ],
                vec![],
                vec![ConditionTree::Leaf(Comparison {
                    op: CmpOp::Allen {
                        mask: MaskTerm::Param(ParamId(0)),
                    },
                    lhs: v(1),
                    rhs: v(2),
                })],
            )),
            params: vec![ParamValue::Scalar(Value::AllenMask(AllenMask::MEETS))],
        },
        // Negation: postings no tag names — the plain anti-join.
        HandCase {
            name: "hand-negation-untagged",
            query: Query::single(rule(
                vec![fv(0)],
                vec![atom(
                    ids::POSTING,
                    &[
                        (ids::posting::ID, v(0)),
                        (ids::posting::ACCOUNT, Term::Literal(Value::U64(0))),
                    ],
                )],
                vec![atom(ids::POSTING_TAG, &[(ids::posting_tag::POSTING, v(0))])],
                vec![],
            )),
            params: vec![],
        },
        // Union with duplicate head answers across rules: one answer
        // (`union_idempotent` at the program level).
        HandCase {
            name: "hand-union-overlapping-rules",
            query: Query {
                head: vec![bumbledb::HeadTerm::Var],
                rules: vec![
                    rule(
                        vec![fv(0)],
                        vec![atom(
                            ids::POSTING,
                            &[
                                (ids::posting::ID, v(0)),
                                (ids::posting::ACCOUNT, Term::Literal(Value::U64(0))),
                                (ids::posting::RECONCILED, Term::Literal(Value::Bool(true))),
                            ],
                        )],
                        vec![],
                        vec![],
                    ),
                    rule(
                        vec![fv(0)],
                        vec![atom(
                            ids::POSTING,
                            &[
                                (ids::posting::ID, v(0)),
                                (ids::posting::ACCOUNT, Term::Literal(Value::U64(0))),
                            ],
                        )],
                        vec![],
                        vec![],
                    ),
                ],
            },
            params: vec![],
        },
        // The multi-rule aggregate head: the union fold.
        HandCase {
            name: "hand-union-aggregate-fold",
            query: Query {
                head: vec![
                    bumbledb::HeadTerm::Var,
                    bumbledb::HeadTerm::Aggregate(bumbledb::HeadOp::Count),
                ],
                rules: vec![
                    rule(
                        vec![
                            fv(0),
                            FindTerm::Aggregate {
                                op: AggOp::Count,
                                over: None,
                            },
                        ],
                        vec![atom(
                            ids::POSTING,
                            &[
                                (ids::posting::ACCOUNT, v(0)),
                                (ids::posting::RECONCILED, Term::Literal(Value::Bool(true))),
                            ],
                        )],
                        vec![],
                        vec![],
                    ),
                    rule(
                        vec![
                            fv(0),
                            FindTerm::Aggregate {
                                op: AggOp::Count,
                                over: None,
                            },
                        ],
                        vec![atom(ids::ORG_PARENT, &[(ids::org_parent::CHILD, v(0))])],
                        vec![],
                        vec![],
                    ),
                ],
            },
            params: vec![],
        },
        // CountDistinct over the interned-string column.
        HandCase {
            name: "hand-count-distinct-strings",
            query: Query::single(rule(
                vec![fv(0), agg(AggOp::CountDistinct, 1)],
                vec![atom(
                    ids::POSTING,
                    &[(ids::posting::ACCOUNT, v(0)), (ids::posting::MEMO, v(1))],
                )],
                vec![],
                vec![],
            )),
            params: vec![],
        },
        // The measure at a find position (Transfer windows sit below
        // the ray by construction — the total lane).
        HandCase {
            name: "hand-measure-find",
            query: Query::single(rule(
                vec![fv(0), FindTerm::Measure(VarId(1))],
                vec![atom(
                    ids::TRANSFER,
                    &[(ids::transfer::ID, v(0)), (ids::transfer::WINDOW, v(1))],
                )],
                vec![],
                vec![],
            )),
            params: vec![],
        },
        // A measure fold over ray-free mandate segments (the
        // COVERED_BY filter excludes every ray first — the documented
        // host pattern).
        HandCase {
            name: "hand-measure-fold-sum",
            query: Query::single(rule(
                vec![
                    fv(0),
                    FindTerm::AggregateMeasure {
                        op: AggOp::Sum,
                        over: VarId(1),
                    },
                ],
                vec![atom(
                    ids::MANDATE,
                    &[(ids::mandate::ACCOUNT, v(0)), (ids::mandate::ACTIVE, v(1))],
                )],
                vec![],
                vec![ConditionTree::Leaf(Comparison {
                    op: CmpOp::Allen {
                        mask: MaskTerm::Literal(AllenMask::COVERED_BY),
                    },
                    lhs: v(1),
                    rhs: Term::Literal(full_i64.clone()),
                })],
            )),
            params: vec![],
        },
        // The colliding-measure lock: two distinct intervals with EQUAL
        // measure land in ONE group under a `[Measure, Count]` head —
        // grouping fibers by the measure VALUE, not the interval, so
        // the answers merge (every collision group's parent interval
        // has width 256 by construction, and groups are disjoint, so
        // the collision is guaranteed data, not luck; the COVERED_BY
        // filter excludes the ray mandates first — the documented host
        // pattern).
        HandCase {
            name: "hand-measure-count-collision",
            query: Query::single(rule(
                vec![
                    FindTerm::Measure(VarId(0)),
                    FindTerm::Aggregate {
                        op: AggOp::Count,
                        over: None,
                    },
                ],
                vec![atom(ids::MANDATE, &[(ids::mandate::ACTIVE, v(0))])],
                vec![],
                vec![ConditionTree::Leaf(Comparison {
                    op: CmpOp::Allen {
                        mask: MaskTerm::Literal(AllenMask::COVERED_BY),
                    },
                    lhs: v(0),
                    rhs: Term::Literal(full_i64.clone()),
                })],
            )),
            params: vec![],
        },
        // The measure in a predicate.
        HandCase {
            name: "hand-measure-predicate",
            query: Query::single(rule(
                vec![fv(0)],
                vec![atom(
                    ids::TRANSFER,
                    &[(ids::transfer::ID, v(0)), (ids::transfer::WINDOW, v(1))],
                )],
                vec![],
                vec![ConditionTree::Leaf(Comparison {
                    op: CmpOp::Lt,
                    lhs: Term::Measure(VarId(1)),
                    rhs: Term::Literal(Value::U64(500)),
                })],
            )),
            params: vec![],
        },
        // Membership through a variable: the posting instant inside the
        // account's mandate interval (the bivalent lowering's flagship).
        HandCase {
            name: "hand-membership-var",
            query: Query::single(rule(
                vec![fv(0), fv(1), fv(2)],
                vec![
                    atom(
                        ids::POSTING,
                        &[(ids::posting::ACCOUNT, v(0)), (ids::posting::AT, v(1))],
                    ),
                    atom(
                        ids::MANDATE,
                        &[
                            (ids::mandate::ACCOUNT, v(0)),
                            (ids::mandate::ORG, v(2)),
                            (ids::mandate::ACTIVE, v(1)),
                        ],
                    ),
                ],
                vec![],
                vec![],
            )),
            params: vec![],
        },
        // Membership through a literal point (a real corpus instant).
        HandCase {
            name: "hand-membership-literal",
            query: Query::single(rule(
                vec![fv(0), fv(1)],
                vec![atom(
                    ids::MANDATE,
                    &[
                        (ids::mandate::ACCOUNT, v(0)),
                        (ids::mandate::ORG, v(1)),
                        (ids::mandate::ACTIVE, Term::Literal(Value::I64(instant))),
                    ],
                )],
                vec![],
                vec![],
            )),
            params: vec![],
        },
        // An interval-valued param compared for identity (no scalar
        // anchor — the equality face of the bivalent rule).
        HandCase {
            name: "hand-interval-param-equality",
            query: Query::single(rule(
                vec![fv(0)],
                vec![atom(
                    ids::MANDATE,
                    &[
                        (ids::mandate::ACCOUNT, v(0)),
                        (ids::mandate::ACTIVE, Term::Param(ParamId(0))),
                    ],
                )],
                vec![],
                vec![],
            )),
            params: vec![ParamValue::Scalar(Value::IntervalI64(
                bumbledb::Interval::<i64>::new(m_start, m_end).expect("corpus segments nonempty"),
            ))],
        },
        // A param set on a scalar field (the EqVarSet face).
        HandCase {
            name: "hand-param-set",
            query: Query::single(rule(
                vec![fv(0)],
                vec![atom(
                    ids::POSTING,
                    &[
                        (ids::posting::ID, v(0)),
                        (ids::posting::ACCOUNT, Term::ParamSet(ParamId(0))),
                    ],
                )],
                vec![],
                vec![],
            )),
            params: vec![ParamValue::Set(vec![
                Value::U64(0),
                Value::U64(2),
                Value::U64(m_account),
            ])],
        },
        // The closed-relation join: ground axioms are ordinary facts on
        // the Lean side too.
        HandCase {
            name: "hand-closed-join",
            query: Query::single(rule(
                vec![fv(0), fv(1), fv(2)],
                vec![
                    atom(
                        ids::ACCOUNT,
                        &[(ids::account::ID, v(0)), (ids::account::CURRENCY, v(1))],
                    ),
                    atom(
                        ids::CURRENCY,
                        &[
                            (ids::currency::ID, v(1)),
                            (ids::currency::MINOR_UNITS, v(2)),
                        ],
                    ),
                ],
                vec![],
                vec![],
            )),
            params: vec![],
        },
    ]
}

/// The whole corpus, deterministically: the hand cases, then seeded
/// querygen cases (query and draw replayed from `Rng::new(case_seed)`,
/// the seed recorded in each file's provenance) until [`SEEDED_CASES`]
/// are expressible. Returns the coverage report and the `(file name,
/// document)` pairs in corpus order.
///
/// # Panics
///
/// On an engine-vs-naive disagreement (a trophy — see [`one_case`]).
#[must_use]
pub fn generate_corpus() -> (Report, Vec<(String, String)>) {
    let mut report = Report::default();
    let mut cases: Vec<(String, String)> = Vec::new();
    let worlds: Vec<World> = WORLD_SEEDS.iter().map(|seed| build_world(*seed)).collect();

    for case in hand_cases(worlds[0].cfg) {
        let provenance = format!(
            "{{\"hand\":\"{}\",\"world_seed\":{}}}",
            case.name, WORLD_SEEDS[0]
        );
        if let Some(document) = one_case(
            &worlds[0],
            case.name,
            &provenance,
            &case.query,
            &case.params,
            &mut report,
        ) {
            cases.push((format!("{}.json", case.name), document));
        } else {
            panic!("hand case {} must be expressible", case.name);
        }
    }

    let mut attempt = 0u64;
    let mut written = 0usize;
    while written < SEEDED_CASES {
        let world = &worlds[usize::try_from(attempt).expect("attempts fit usize") % worlds.len()];
        let case_seed = CASE_SEED_BASE + attempt;
        attempt += 1;
        let mut rng = Rng::new(case_seed);
        let query = querygen::random_query(&mut rng, world.cfg);
        let draws = querygen::params_for(&query, &mut rng, world.cfg);
        let draw = usize::try_from(case_seed).expect("seed fits usize") % draws.len();
        let params = positional(&draws[draw]);
        let name = format!("seeded-{written:04}");
        let provenance = format!(
            "{{\"world_seed\":{},\"case_seed\":{case_seed},\"draw\":{draw}}}",
            world.cfg.seed
        );
        if let Some(document) = one_case(world, &name, &provenance, &query, &params, &mut report) {
            cases.push((format!("{name}.json"), document));
            written += 1;
        }
    }
    (report, cases)
}

/// The checked-in corpus directory (`lean/conformance/cases`).
///
/// # Panics
///
/// Never in practice (the manifest path is UTF-8 and has two parents).
#[must_use]
pub fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/bumbledb-bench sits two levels below the repository root")
        .join("lean/conformance/cases")
}

/// Regenerates the checked-in corpus on disk — the query cases AND the
/// judgment cases (the write-side lane, [`judgment`]) — (the builder
/// half; the `regenerate_the_conformance_corpus` test wraps it).
///
/// # Panics
///
/// On filesystem failures, or an engine-vs-naive trophy.
#[must_use = "the coverage report is the recorded number"]
pub fn write_corpus(dir: &Path) -> Report {
    let (report, cases) = generate_corpus();
    let program_world = build_world(WORLD_SEEDS[0]);
    let (program_report, program_cases) = program::generate_program_corpus(&program_world);
    eprintln!("{}", program_report.coverage_line());
    std::fs::create_dir_all(dir).expect("create the corpus directory");
    for entry in std::fs::read_dir(dir).expect("list the corpus directory") {
        let path = entry.expect("corpus dir entry").path();
        if path.extension().is_some_and(|ext| ext == "json") {
            std::fs::remove_file(&path).expect("clear a stale corpus case");
        }
    }
    for (name, document) in cases
        .iter()
        .chain(&judgment::generate_judgment_corpus())
        .chain(&program_cases)
    {
        std::fs::write(dir.join(name), document).expect("write a corpus case");
    }
    report
}

/// Replays every checked-in case FROM ITS PROVENANCE — the comparator's
/// engine+naive half. Per file: rebuild the query and params (the hand
/// roster by name, or `Rng::new(case_seed)` + the recorded draw), run
/// the engine and the naive model fresh (parity asserted — a trophy
/// panics with the case name), re-serialize, and hold the file to byte
/// equality. Provenance-driven on purpose: the builder's slow/wide
/// budgets are wall-clock measurements taken once at build time; the
/// comparator replays exactly what was checked in, so its verdict is
/// deterministic under any machine load.
///
/// # Panics
///
/// On a byte mismatch (names the case file), an engine/naive trophy,
/// unreadable provenance, or an empty corpus directory.
#[must_use = "the case count is the comparator's evidence line"]
pub fn replay_checked_in_corpus() -> usize {
    let dir = corpus_dir();
    let mut worlds: BTreeMap<u64, World> = BTreeMap::new();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("list the corpus directory (regenerate the corpus first)")
        .map(|entry| entry.expect("corpus dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort();
    assert!(
        !files.is_empty(),
        "no checked-in conformance cases under {}",
        dir.display()
    );
    for path in &files {
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .expect("corpus names are UTF-8")
            .to_owned();
        let text = std::fs::read_to_string(path).expect("read a corpus case");
        let document = if name.starts_with("judgment-") {
            judgment::replay_judgment_case(&name)
        } else if name.starts_with("program-") {
            program::replay_program_case(&mut worlds, &name, &text)
        } else {
            replay_case(&mut worlds, &name, &text)
        };
        assert!(
            text == document,
            "conformance case {name}: the checked-in file differs from the fresh \
             engine+naive replay of its provenance — a trophy or a stale corpus; \
             triage per the fuzzing charter, regenerate only if the generator changed"
        );
    }
    files.len()
}

/// One case's fresh document from its recorded provenance.
fn replay_case(worlds: &mut BTreeMap<u64, World>, name: &str, text: &str) -> String {
    let parsed = crate::json::parse(text).expect("a corpus case parses as JSON");
    let provenance = parsed
        .get("provenance")
        .expect("a corpus case records provenance");
    let world_seed = read_u64(provenance, "world_seed");
    let world = worlds
        .entry(world_seed)
        .or_insert_with(|| build_world(world_seed));
    let (query, params, provenance_line) =
        if provenance.get("hand").and_then(crate::json::Value::as_str) == Some(name) {
            let case = hand_cases(world.cfg)
                .into_iter()
                .find(|case| case.name == name)
                .unwrap_or_else(|| panic!("unknown hand case {name}: stale corpus"));
            let line = format!("{{\"hand\":\"{name}\",\"world_seed\":{world_seed}}}");
            (case.query, case.params, line)
        } else {
            let case_seed = read_u64(provenance, "case_seed");
            let draw = usize::try_from(read_u64(provenance, "draw")).expect("draw fits");
            let mut rng = Rng::new(case_seed);
            let query = querygen::random_query(&mut rng, world.cfg);
            let draws = querygen::params_for(&query, &mut rng, world.cfg);
            let params = positional(&draws[draw]);
            let line = format!(
                "{{\"world_seed\":{world_seed},\"case_seed\":{case_seed},\"draw\":{draw}}}"
            );
            (query, params, line)
        };
    let (answers, _) = execute_case(world, name, &query, &params);
    let answers = answers.unwrap_or_else(|| {
        panic!("conformance case {name}: a runtime error on replay — stale corpus or trophy")
    });
    render_case(world, name, &provenance_line, &query, &params, &answers).unwrap_or_else(|why| {
        panic!("conformance case {name}: inexpressible on replay ({why:?}) — stale corpus")
    })
}

/// A `u64` field of a hand-rolled-JSON object (corpus provenance values
/// are small and exact in the parser's `f64` carrier).
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "provenance integers are small; exactness is asserted right below"
)]
fn read_u64(value: &crate::json::Value, key: &str) -> u64 {
    let number = value
        .get(key)
        .and_then(crate::json::Value::as_f64)
        .unwrap_or_else(|| panic!("provenance field {key} missing"));
    let converted = number as u64;
    assert!(
        (converted as f64 - number).abs() < f64::EPSILON,
        "provenance field {key} is not an exact integer"
    );
    converted
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    /// Regenerates `lean/conformance/cases/` in place. Ignored: run it
    /// deliberately (`cargo test -p bumbledb-bench regenerate_the_conformance_corpus
    /// -- --ignored --nocapture`) when the generator, the corpus seeds,
    /// or the format change — the checked-in files are the replay
    /// corpus, and the comparator holds every run to their bytes. Run
    /// it on a quiet machine: the slow/wide budgets are wall-clock
    /// measurements taken here, once (the comparator replays what was
    /// written and never re-measures).
    #[test]
    #[ignore = "regenerates the checked-in corpus; run deliberately"]
    fn regenerate_the_conformance_corpus() {
        let report = write_corpus(&corpus_dir());
        eprintln!("{}", report.coverage_line());
    }

    /// Regenerates the RECURSIVE arm's `program-*.json` cases only —
    /// the query and judgment cases keep their bytes (their wall-clock
    /// budgets were measured at their own build time and never
    /// re-measure on replay).
    #[test]
    #[ignore = "regenerates the checked-in program cases; run deliberately"]
    fn regenerate_the_recursive_conformance_corpus() {
        let report = program::write_program_corpus(&corpus_dir());
        eprintln!("{}", report.coverage_line());
    }

    /// The engine+naive half of the comparator, no Lean toolchain
    /// needed: replay every checked-in case from its provenance (fresh
    /// worlds, fresh engine executions, naive parity asserted inside)
    /// and hold the file to byte equality — any drift names the case
    /// file.
    #[test]
    fn the_corpus_replays_byte_identical_from_its_provenance() {
        let cases = replay_checked_in_corpus();
        eprintln!("conformance: {cases} checked-in cases replayed byte-identical");
    }

    /// THE three-way comparator (the PRD's test): for each corpus case,
    /// the engine fresh and the naive model fresh (byte-held to the
    /// checked-in file), then `lake exe conformance` — the Lean
    /// denotation, `evalList` under `eval_sound` — over the same files.
    /// Any disagreement names the case file. Ignored in the plain
    /// workspace run because it needs the Lean toolchain;
    /// `scripts/lean.sh` runs it with `--ignored` after the corpus
    /// replay — the Lean-dependent lane owns the Lean-dependent test,
    /// so the three-way comparator gates every lean.sh run while
    /// check.sh stays toolchain-independent.
    #[test]
    #[ignore = "needs the Lean toolchain (elan/lake) on PATH; scripts/lean.sh runs it"]
    fn three_way_conformance_over_the_checked_in_corpus() {
        let engine_started = Instant::now();
        let cases = replay_checked_in_corpus();
        eprintln!(
            "conformance: {cases} cases — engine+naive replay + byte comparison: {} ms",
            engine_started.elapsed().as_millis()
        );

        let lean_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("repository root")
            .join("lean");
        let lean_started = Instant::now();
        let output = Command::new("lake")
            .arg("exe")
            .arg("conformance")
            .arg("conformance/cases")
            .current_dir(&lean_dir)
            .output()
            .expect("run `lake exe conformance` (install elan / the pinned Lean toolchain)");
        eprintln!(
            "conformance: lake exe conformance: {} ms\n{}{}",
            lean_started.elapsed().as_millis(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        assert!(
            output.status.success(),
            "the Lean denotation disagrees with the checked-in corpus (see the named case \
             files above) — a trophy; triage per the fuzzing charter"
        );
    }
}
