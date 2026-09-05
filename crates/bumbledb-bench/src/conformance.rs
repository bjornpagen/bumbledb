//! The Lean tree mathematics, and its executable half `evalList` is PROVED
//! equal to the set denotation (`eval_sound`), so evaluating it on real Tiny
//! (`lean/Bumbledb/Query/Denotation.lean`) is derived from the example in
//! `lean/conformance/README.md`); written to `lean/conformance/cases/*.json`
//! (checked in — the `lean/Bumbledb/Exec/Reach.lean: evalQueryList` — three-way
//! like (`lean/Bumbledb/Exec/Dedup.lean: membership_lowering_preserves_fold`).
//! licenses (`lean/Bumbledb/Query/Syntax.lean`, the membership note).
pub mod complete;
pub mod judgment;
pub mod reach;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use bumbledb::schema::ValueType;
use bumbledb::{
    AllenMask, Atom, AtomSource, Basic, CmpOp, Comparison, ConditionTree, Db, FieldId, FindTerm,
    FoldOp, HeadOp, HeadTerm, Interior, InteriorId, ParamId, Query, Rec, RecRule, RecStep,
    RelationId, Rule, Term, Value, VarId,
};

use crate::corpus_gen::{GenConfig, Rng, Scale};
use crate::differential::{self, Answers};
use crate::naive::{Delta, NaiveDb, ParamValue, Tuple};
use crate::querygen::{self, ParamDraw, target};

pub const WORLD_SEEDS: [u64; 2] = [0x00C0_4F01, 0x00C0_4F02];

pub const SEEDED_CASES: usize = 200;

pub const CASE_SEED_BASE: u64 = 0x0013_0000;

const NAIVE_BUDGET_MS: u128 = 25;

const MAX_ANSWER_ROWS: usize = 512;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    pub attempted: u64,

    pub written: u64,

    pub excluded_unresolved: u64,

    pub excluded_engine_error: u64,

    pub excluded_slow: u64,

    pub excluded_wide: u64,

    /// Cases with a computed head: the Lean case grammar has no compute
    /// kind, so such a query is inexpressible there — excluded, counted.
    pub excluded_compute: u64,

    /// Cases touching an `Id128` or dense-interval value the Lean value
    /// grammar cannot spell yet — excluded, counted.
    pub excluded_value: u64,
}

impl Report {
    #[must_use]
    pub fn coverage_line(&self) -> String {
        format!(
            "conformance coverage: {}/{} expressible (excluded: {} unresolved-literal, \
             {} engine-error, {} slow, {} wide, {} computed-head, \
             {} unrepresentable-value)",
            self.written,
            self.attempted,
            self.excluded_unresolved,
            self.excluded_engine_error,
            self.excluded_slow,
            self.excluded_wide,
            self.excluded_compute,
            self.excluded_value,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Exclusion {
    UnresolvedLiteral,

    /// A computed head — inexpressible in the Lean case grammar.
    ComputedHead,

    /// An `Id128` or dense `Interval<F64>` value — the Lean value grammar
    /// has no tag for either yet.
    UnrepresentableValue,
}

pub struct World {
    pub cfg: GenConfig,
    pub db: Db<target::Target>,
    pub naive: NaiveDb,
    dict: BTreeMap<Box<str>, u64>,
    dict_order: Vec<Box<str>>,
    _dir: ScratchDir,
}

struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new(tag: &str) -> Self {
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
        Self(path)
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// # Panics
#[must_use]
pub fn build_world(seed: u64) -> World {
    let cfg = GenConfig {
        seed,
        scale: Scale::Tiny,
    };
    let dir = ScratchDir::new(&format!("{seed:08x}"));
    let db = target::publish_admitted(&dir.0);
    let mut naive = NaiveDb::new(&target::descriptor());
    let mut delta = Delta::default();
    for rel in 0..target::TARGET_RELATIONS {
        let rel = RelationId(rel);
        match rel {
            target::ids::JOURNAL_ENTRY => load_du_cluster(&db, cfg),
            target::ids::IMPORT_BATCH => {}
            _ => {
                db.write(|tx| {
                    tx.insert_dyn(rel, target::corpus_relation_rows(cfg, rel))
                        .map(bumbledb::MutationReport::changed)
                })
                .expect("conformance target insert")
                .unwrap();
            }
        }
        for fact in target::corpus_relation_rows(cfg, rel) {
            delta.inserts.push((rel, fact));
        }
    }
    // The fixed-width Lane (`interval<i64, 5>`) sits after the closed

    db.write(|tx| {
        tx.insert_dyn(
            target::ids::LANE,
            target::corpus_relation_rows(cfg, target::ids::LANE),
        )
        .map(bumbledb::MutationReport::changed)
    })
    .expect("conformance lane insert")
    .unwrap();
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
                tx.insert_dyn(target::ids::JOURNAL_ENTRY, [&fact])?;
            }
            while next_batch < batches && target::import_batch_entry(next_batch) < end {
                let fact = target::corpus_row(cfg, &domains, target::ids::IMPORT_BATCH, next_batch);
                tx.insert_dyn(target::ids::IMPORT_BATCH, [&fact])?;
                next_batch += 1;
            }
            Ok(())
        })
        .expect("conformance DU cluster load")
        .unwrap();
        start = end;
    }
}

impl World {
    fn intern(&mut self, value: &Value) {
        if let Value::String(bytes) = value
            && !self.dict.contains_key(bytes)
        {
            let id = u64::try_from(self.dict_order.len()).expect("dictionary fits u64");
            self.dict.insert(bytes.clone(), id);
            self.dict_order.push(bytes.clone());
        }
    }

    fn resolve(&self, text: &str) -> Result<u64, Exclusion> {
        self.dict
            .get(text)
            .copied()
            .ok_or(Exclusion::UnresolvedLiteral)
    }
}

/// The Allen basic names, in `Basic::ALL` order — the mask spelling of the
/// interchange format (and of `lean/Bumbledb/Query/Syntax.lean`'s `AllenRel`).
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

fn push_value_typed(
    world: &World,
    used: &mut BTreeSet<u64>,
    out: &mut String,
    value: &Value,
    ty: Option<&ValueType>,
) -> Result<(), Exclusion> {
    if let Some(ValueType::FixedInterval { width: w, .. }) = ty {
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
        Value::F64(v) => {
            let _ = write!(out, "{{\"f64\":\"{:016x}\"}}", v.to_bits());
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
        Value::Id128(_) | Value::IntervalF64(_) => {
            return Err(Exclusion::UnrepresentableValue);
        }
    }
    Ok(())
}

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
    }
    Ok(())
}

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
            out.push_str(",\"mask\":");
            push_mask(out, mask);
        }
    }
    out.push_str(",\"lhs\":");
    push_term(world, used, out, &cmp.lhs)?;
    out.push_str(",\"rhs\":");
    push_term(world, used, out, &cmp.rhs)?;
    out.push_str("}}");
    Ok(())
}

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

fn push_find(out: &mut String, find: &FindTerm) -> Result<(), Exclusion> {
    match find {
        FindTerm::Var(v) => {
            let _ = write!(out, "{{\"var\":{}}}", v.0);
        }
        FindTerm::Compute(_) => return Err(Exclusion::ComputedHead),
        FindTerm::Count => out.push_str("{\"agg\":{\"op\":\"count\"}}"),
        FindTerm::Pack { over } => {
            let _ = write!(out, "{{\"agg\":{{\"op\":\"pack\",\"over\":{}}}}}", over.0);
        }
        FindTerm::Aggregate { op, over } => {
            let name = match op {
                FoldOp::Sum => "sum",
                FoldOp::Mean => "mean",
                FoldOp::Min => "min",
                FoldOp::Max => "max",
            };
            let _ = write!(out, "{{\"agg\":{{\"op\":\"{name}\",\"over\":{}}}}}", over.0);
        }
    }
    Ok(())
}

fn field_is_interval(atom: &Atom, field: FieldId) -> bool {
    match atom.source {
        AtomSource::Edb(relation) => target::schema()
            .relation(relation)
            .field(field)
            .value_type
            .is_interval(),
        AtomSource::Interior(_) => false,
    }
}

fn count_vars(rule: &Rule) -> u16 {
    fn see(count: &mut u16, var: VarId) {
        *count = (*count).max(var.0 + 1);
    }
    fn see_term(count: &mut u16, term: &Term) {
        if let Term::Var(var) = term {
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
            FindTerm::Var(var) => see(&mut count, *var),
            FindTerm::Aggregate { over, .. } | FindTerm::Pack { over } => see(&mut count, *over),
            FindTerm::Compute(expr) => {
                for var in expr.variables() {
                    see(&mut count, var);
                }
            }
            FindTerm::Count => {}
        }
    }
    count
}

fn scalar_anchors(rule: &Rule, var_count: u16) -> Vec<bool> {
    let mut anchored = vec![false; usize::from(var_count)];
    for atom in &rule.atoms {
        for (field, term) in &atom.bindings {
            if let Term::Var(var) = term
                && !field_is_interval(atom, *field)
            {
                anchored[usize::from(var.0)] = true;
            }
        }
    }
    anchored
}

fn membership(term: &Term, anchored: &[bool], params: &[ParamValue]) -> bool {
    match term {
        Term::Var(v) => anchored[usize::from(v.0)],
        Term::Literal(value) => matches!(value, Value::U64(_) | Value::I64(_)),
        Term::Param(p) => match &params[usize::from(p.0)] {
            ParamValue::Scalar(Value::U64(_) | Value::I64(_)) => true,
            ParamValue::Scalar(_) => false,
            ParamValue::Set(_) => unreachable!("validated: scalar use of a set param"),
        },
        Term::ParamSet(p) => match &params[usize::from(p.0)] {
            ParamValue::Set(values) => {
                matches!(values.first(), Some(Value::U64(_) | Value::I64(_)))
            }
            ParamValue::Scalar(_) => unreachable!("validated: set use of a scalar param"),
        },
    }
}

/// One rule after the membership lowering: rewritten positive atoms, negated
/// atoms left in SURFACE form (Lean `AntiProbe` / `surfaceMatchesB` reads
/// membership there), the original conditions plus the lowered `PointIn`
/// leaves, and the SURFACE WIDTH — the written rule's variable count, below the
/// fresh mints, so the Lean fold domain projects every mint away (finding 087,
/// discharged).
struct LoweredRule<'a> {
    finds: &'a [FindTerm],
    atoms: Vec<Atom>,
    negated: &'a [Atom],
    conditions: Vec<ConditionTree>,
    width: u16,
}

fn lower_rule<'a>(rule: &'a Rule, params: &[ParamValue]) -> LoweredRule<'a> {
    let var_count = count_vars(rule);
    let anchored = scalar_anchors(rule, var_count);
    let mut fresh = var_count;
    let mut atoms = Vec::with_capacity(rule.atoms.len());
    let mut conditions = rule.conditions.clone();
    for atom in &rule.atoms {
        let mut bindings = Vec::with_capacity(atom.bindings.len());
        for (field, term) in &atom.bindings {
            if field_is_interval(atom, *field) && membership(term, &anchored, params) {
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
            source: atom.source,
            bindings,
        });
    }
    LoweredRule {
        finds: &rule.finds,
        atoms,
        negated: &rule.negated,
        conditions,
        width: var_count,
    }
}

fn mentioned(rules: &[LoweredRule<'_>]) -> BTreeSet<RelationId> {
    let mut set = BTreeSet::new();
    for rule in rules {
        for atom in rule.atoms.iter().chain(rule.negated.iter()) {
            if let AtomSource::Edb(relation) = atom.source {
                set.insert(relation);
            }
        }
    }
    set
}

fn type_name(value_type: &ValueType) -> String {
    match value_type {
        ValueType::Bool => "bool".into(),
        ValueType::U64 => "u64".into(),
        ValueType::I64 => "i64".into(),
        ValueType::F64 => "f64".into(),
        ValueType::String => "str".into(),
        ValueType::FixedBytes { len } => format!("bytes<{len}>"),
        ValueType::Interval {
            element: bumbledb::schema::IntervalElement::U64,
        } => "interval_u64".into(),
        ValueType::Interval {
            element: bumbledb::schema::IntervalElement::I64,
        } => "interval_i64".into(),

        ValueType::Interval {
            element: bumbledb::schema::IntervalElement::F64,
        } => "interval_f64".into(),
        ValueType::Id128 => "id128".into(),

        ValueType::FixedInterval {
            element: bumbledb::schema::FixedIntervalElement::U64,
            width: w,
        } => format!("interval_u64_fixed<{w}>"),
        ValueType::FixedInterval {
            element: bumbledb::schema::FixedIntervalElement::I64,
            width: w,
        } => format!("interval_i64_fixed<{w}>"),
    }
}

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

fn render_case(
    world: &World,
    name: &str,
    provenance: &str,
    query: &Query,
    params: &[ParamValue],
    answers: &BTreeSet<Tuple>,
) -> Result<String, Exclusion> {
    let mut used = BTreeSet::new();
    let Query {
        interiors,
        head,
        rules,
        rec: None,
    } = query
    else {
        panic!("query-case renderer is the CQ arm");
    };
    let lowered: Vec<LoweredRule<'_>> = rules.iter().map(|rule| lower_rule(rule, params)).collect();

    let query_block = render_cq_lowered(world, &mut used, interiors, head, &lowered)?;

    let mut params_block = String::from("[");
    for (index, param) in params.iter().enumerate() {
        if index > 0 {
            params_block.push(',');
        }
        match param {
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

    let (relations_block, instance_block, axioms_block) =
        world_blocks(world, &mut used, mentioned(&lowered))?;

    let strings_block = strings_block(world, &used);

    Ok(format!(
        "{{\n\"case\":\"{name}\",\n\"provenance\":{provenance},\n\"strings\":{strings_block},\n\
         \"theory\":{{\"relations\":{relations_block},\n\"ground_axioms\":{axioms_block}}},\n\
         \"instance\":{instance_block},\n\"query\":{query_block},\n\"params\":{params_block},\n\
         \"answers\":{answers_block}\n}}\n"
    ))
}

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
            descriptor.body().closed_rows().is_some()
        );
        for (position, field) in descriptor.fields().iter().enumerate() {
            if position > 0 {
                relations_block.push(',');
            }
            let _ = write!(relations_block, "\"{}\"", type_name(&field.value_type));
        }
        relations_block.push_str("]}");

        let field_types: Vec<ValueType> = descriptor
            .fields()
            .iter()
            .map(|field| field.value_type)
            .collect();
        let facts: Vec<Vec<Value>> = if descriptor.body().closed_rows().is_some() {
            closed_facts(relation)
        } else {
            target::corpus_relation_rows(world.cfg, relation).collect()
        };
        let block = if descriptor.body().closed_rows().is_some() {
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

fn strings_block(world: &World, used: &BTreeSet<u64>) -> String {
    let mut strings_block = String::from("[");
    for (index, id) in used.iter().enumerate() {
        if index > 0 {
            strings_block.push(',');
        }
        let text = &world.dict_order[usize::try_from(*id).expect("id fits")];
        let _ = write!(strings_block, "[{id},");
        crate::json::push_str_lit(&mut strings_block, text);
        strings_block.push(']');
    }
    strings_block.push(']');
    strings_block
}

pub(super) fn render_reach_query_block(
    world: &World,
    used: &mut BTreeSet<u64>,
    query: &Query,
) -> Result<String, Exclusion> {
    match query {
        Query {
            interiors,
            head,
            rules,
            rec: None,
        } => {
            let mut main = String::new();
            push_reach_rules(world, used, &mut main, rules)?;
            render_cq_doc(world, used, interiors, head, &main)
        }
        Query {
            interiors,
            rec: Some(rec),
            head,
            rules,
        } => {
            let mut main = String::new();
            push_reach_rules(world, used, &mut main, rules)?;
            render_reach_doc(world, used, interiors, rec, head, &main)
        }
    }
}

fn render_cq_lowered(
    world: &World,
    used: &mut BTreeSet<u64>,
    interiors: &[Interior],
    head: &[HeadTerm],
    rules: &[LoweredRule<'_>],
) -> Result<String, Exclusion> {
    let mut main = String::new();
    push_lowered_rules(world, used, &mut main, rules)?;

    render_cq_doc(world, used, interiors, head, &main)
}

fn render_cq_doc(
    world: &World,
    used: &mut BTreeSet<u64>,
    interiors: &[Interior],
    head: &[HeadTerm],
    rules_json: &str,
) -> Result<String, Exclusion> {
    let mut out = String::new();
    let _ = write!(out, "{{\"cq\":{{\"interiors\":[");
    push_interiors(world, used, &mut out, interiors)?;
    out.push_str("],\"head\":");
    push_head(&mut out, head)?;
    out.push_str(",\"rules\":");
    out.push_str(rules_json);
    out.push_str("}}");
    Ok(out)
}

fn render_reach_doc(
    world: &World,
    used: &mut BTreeSet<u64>,
    interiors: &[Interior],
    rec: &Rec,
    head: &[HeadTerm],
    rules_json: &str,
) -> Result<String, Exclusion> {
    let mut out = String::new();
    let _ = write!(out, "{{\"reach\":{{\"interiors\":[");
    push_interiors(world, used, &mut out, interiors)?;
    out.push_str("],\"rec\":");
    push_reach_rec(world, used, &mut out, interiors.len(), rec)?;
    out.push_str(",\"head\":");
    push_head(&mut out, head)?;
    out.push_str(",\"rules\":");
    out.push_str(rules_json);
    out.push_str("}}");
    Ok(out)
}

fn push_interiors(
    world: &World,
    used: &mut BTreeSet<u64>,
    out: &mut String,
    interiors: &[Interior],
) -> Result<(), Exclusion> {
    for (index, interior) in interiors.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"head\":");
        push_head(out, &interior.head())?;
        out.push_str(",\"rules\":");
        push_reach_rules(world, used, out, &interior.rules)?;
        out.push('}');
    }
    Ok(())
}

fn push_head(out: &mut String, head: &[HeadTerm]) -> Result<(), Exclusion> {
    out.push('[');
    for (index, term) in head.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        match term {
            HeadTerm::Var => out.push_str("{\"kind\":\"var\"}"),
            HeadTerm::Compute => return Err(Exclusion::ComputedHead),
            HeadTerm::Aggregate(op) => {
                let name = match op {
                    HeadOp::Sum => "sum",
                    HeadOp::Mean => "mean",
                    HeadOp::Min => "min",
                    HeadOp::Max => "max",
                    HeadOp::Count => "count",
                    HeadOp::Pack => "pack",
                };
                let _ = write!(out, "{{\"kind\":\"aggregate\",\"op\":\"{name}\"}}");
            }
        }
    }
    out.push(']');
    Ok(())
}

fn push_lowered_rules(
    world: &World,
    used: &mut BTreeSet<u64>,
    out: &mut String,
    rules: &[LoweredRule<'_>],
) -> Result<(), Exclusion> {
    out.push('[');
    for (index, rule) in rules.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"finds\":[");
        for (position, find) in rule.finds.iter().enumerate() {
            if position > 0 {
                out.push(',');
            }
            push_find(out, find)?;
        }
        out.push_str("],\"atoms\":[");
        for (position, atom) in rule.atoms.iter().enumerate() {
            if position > 0 {
                out.push(',');
            }
            push_atom(world, used, out, atom)?;
        }
        out.push_str("],\"negated\":[");
        for (position, atom) in rule.negated.iter().enumerate() {
            if position > 0 {
                out.push(',');
            }
            push_atom(world, used, out, atom)?;
        }
        out.push_str("],\"conditions\":[");
        for (position, tree) in rule.conditions.iter().enumerate() {
            if position > 0 {
                out.push(',');
            }
            push_condition(world, used, out, tree)?;
        }
        let _ = write!(out, "],\"width\":{}}}", rule.width);
    }
    out.push(']');
    Ok(())
}

fn push_reach_rec(
    world: &World,
    used: &mut BTreeSet<u64>,
    out: &mut String,
    rec_index: usize,
    rec: &Rec,
) -> Result<(), Exclusion> {
    out.push_str("{\"head\":");
    push_head(out, &rec.head())?;
    out.push_str(",\"base\":");
    let base: Vec<Rule> = rec.base.iter().map(RecRule::to_rule).collect();
    push_reach_rules(world, used, out, &base)?;
    out.push_str(",\"step\":");
    let rec_id = InteriorId(u32::try_from(rec_index).expect("interior id fits u32"));
    let step: Vec<Rule> = rec
        .rec
        .iter()
        .map(|arm| RecStep::to_written_rule(arm, rec_id))
        .collect();
    push_reach_rules(world, used, out, &step)?;
    out.push('}');
    Ok(())
}

fn push_reach_rules(
    world: &World,
    used: &mut BTreeSet<u64>,
    out: &mut String,
    rules: &[Rule],
) -> Result<(), Exclusion> {
    out.push('[');
    for (position, rule) in rules.iter().enumerate() {
        if position > 0 {
            out.push(',');
        }
        push_reach_rule(world, used, out, rule)?;
    }
    out.push(']');
    Ok(())
}

fn push_reach_rule(
    world: &World,
    used: &mut BTreeSet<u64>,
    out: &mut String,
    rule: &Rule,
) -> Result<(), Exclusion> {
    out.push_str("{\"finds\":[");
    for (position, find) in rule.finds.iter().enumerate() {
        if position > 0 {
            out.push(',');
        }
        match find {
            FindTerm::Var(var) => {
                let _ = write!(out, "{}", var.0);
            }
            other => unreachable!("fold-bearing queries are excluded before render: {other:?}"),
        }
    }
    out.push_str("],\"atoms\":[");
    for (position, atom) in rule.atoms.iter().enumerate() {
        if position > 0 {
            out.push(',');
        }
        push_atom(world, used, out, atom)?;
    }
    out.push_str("],\"negated\":[");
    for (position, atom) in rule.negated.iter().enumerate() {
        if position > 0 {
            out.push(',');
        }
        push_atom(world, used, out, atom)?;
    }
    out.push_str("],\"conditions\":[");
    for (position, tree) in rule.conditions.iter().enumerate() {
        if position > 0 {
            out.push(',');
        }
        push_condition(world, used, out, tree)?;
    }
    out.push_str("]}");
    Ok(())
}

fn push_atom(
    world: &World,
    used: &mut BTreeSet<u64>,
    out: &mut String,
    atom: &Atom,
) -> Result<(), Exclusion> {
    match atom.source {
        AtomSource::Edb(relation) => {
            let _ = write!(out, "{{\"edb\":{},\"bindings\":[", relation.0);
        }
        AtomSource::Interior(InteriorId(id)) => {
            let _ = write!(out, "{{\"interior\":{id},\"bindings\":[");
        }
    }
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

/// # Panics
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
        Err(Exclusion::ComputedHead) => {
            report.excluded_compute += 1;
            None
        }
        Err(Exclusion::UnrepresentableValue) => {
            report.excluded_value += 1;
            None
        }
    }
}

/// # Panics
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
        Err(crate::naive::query::QueryError::Scalar { .. }) => Answers::Scalar,
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
        Answers::Overflow | Answers::Scalar | Answers::DerivedBudget => (None, naive_ms),
    }
}

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

fn agg(op: FoldOp, over: u16) -> FindTerm {
    FindTerm::Aggregate {
        op,
        over: VarId(over),
    }
}

fn pack(over: u16) -> FindTerm {
    FindTerm::Pack { over: VarId(over) }
}

#[expect(
    clippy::too_many_lines,
    reason = "one flat case roster, data not logic"
)]
fn hand_cases(cfg: GenConfig) -> Vec<HandCase> {
    use target::ids;
    let domains = target::Domains::of(cfg.scale);

    let (m_account, _, (m_start, m_end)) = target::mandate(cfg, &domains, 0);
    let instant = m_start.midpoint(m_end);
    let _full_i64 = Value::IntervalI64(
        bumbledb::Interval::<i64>::new(i64::MIN, i64::MAX - 1).expect("nonempty"),
    );
    vec![
        HandCase {
            name: "hand-pack-exact-partition",
            query: Query::single(rule(
                vec![fv(0), pack(1)],
                vec![atom(
                    ids::MANDATE,
                    &[(ids::mandate::ACCOUNT, v(0)), (ids::mandate::ACTIVE, v(1))],
                )],
                vec![],
                vec![],
            )),
            params: vec![],
        },
        HandCase {
            name: "hand-empty-global-aggregates",
            query: Query::single(rule(
                vec![FindTerm::Count, agg(FoldOp::Sum, 1)],
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
        HandCase {
            name: "hand-union-overlapping-rules",
            query: Query {
                interiors: vec![],
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
                rec: None,
            },
            params: vec![],
        },
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

/// # Panics
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
        let query = querygen::random_cq_query(&mut rng, world.cfg);
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
/// # Panics
#[must_use]
pub fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/bumbledb-bench sits two levels below the repository root")
        .join("lean/conformance/cases")
}

/// # Panics
#[must_use = "the coverage report is the recorded number"]
pub fn write_corpus(dir: &Path) -> Report {
    let (report, cases) = generate_corpus();
    let reach_world = build_world(WORLD_SEEDS[0]);
    let (reach_report, reach_cases) = reach::generate_reach_corpus(&reach_world);
    eprintln!("{}", reach_report.coverage_line());
    std::fs::create_dir_all(dir).expect("create the corpus directory");
    for entry in std::fs::read_dir(dir).expect("list the corpus directory") {
        let path = entry.expect("corpus dir entry").path();
        if path.extension().is_some_and(|ext| ext == "json")
            && !path
                .file_stem()
                .and_then(|name| name.to_str())
                .is_some_and(manually_authored_case)
        {
            std::fs::remove_file(&path).expect("clear a stale corpus case");
        }
    }
    for (name, document) in cases
        .iter()
        .chain(&judgment::generate_judgment_corpus())
        .chain(&complete::generate_complete_corpus())
        .chain(&reach_cases)
    {
        std::fs::write(dir.join(name), document).expect("write a corpus case");
    }
    report
}

// These existing Lean-only expression fixtures are authored independently of
// the random generator. Regeneration owns its outputs, not these witnesses.
fn manually_authored_case(name: &str) -> bool {
    matches!(
        name,
        "hand-measure-find"
            | "hand-measure-count-collision"
            | "hand-measure-predicate"
            | "hand-measure-fold-sum"
    )
}

/// # Panics
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

        // disk (do not edit cases/); skip engine+naive replay. Lean

        if manually_authored_case(&name) {
            continue;
        }
        let document = if name.starts_with("judgment-") {
            judgment::replay_judgment_case(&name)
        } else if name.starts_with("complete-") {
            complete::replay_complete_case(&name)
        } else if name.starts_with("reach-") {
            reach::replay_reach_case(&mut worlds, &name, &text)
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
            let query = querygen::random_cq_query(&mut rng, world.cfg);
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
    use bumbledb::FoldOp;

    #[test]
    fn membership_under_an_additive_fold_is_licensed_at_surface_width() {
        let membership_body = || {
            vec![Atom {
                source: bumbledb::AtomSource::Edb(target::ids::MANDATE),
                bindings: vec![
                    (target::ids::mandate::ACCOUNT, Term::Var(VarId(0))),
                    (target::ids::mandate::ACTIVE, Term::Literal(Value::I64(5))),
                ],
            }]
        };
        let rule = |finds: Vec<FindTerm>| Rule {
            finds,
            atoms: membership_body(),
            negated: vec![],
            conditions: vec![],
        };

        for finds in [
            vec![FindTerm::Var(VarId(0)), FindTerm::Count],
            vec![
                FindTerm::Var(VarId(0)),
                FindTerm::Aggregate {
                    op: FoldOp::Sum,
                    over: VarId(0),
                },
            ],
        ] {
            let additive = rule(finds);
            let lowered = lower_rule(&additive, &[]);
            assert_eq!(lowered.width, 1, "the surface width excludes the mint");
            assert_eq!(
                lowered.conditions.len(),
                1,
                "the lowering fired: one PointIn condition on the minted variable"
            );
        }

        let projection = rule(vec![FindTerm::Var(VarId(0))]);
        let lowered = lower_rule(&projection, &[]);
        assert_eq!(
            lowered.conditions.len(),
            1,
            "the lowering fired: one PointIn condition"
        );

        let max_rule = rule(vec![FindTerm::Aggregate {
            op: FoldOp::Max,
            over: VarId(0),
        }]);
        let max = lower_rule(&max_rule, &[]);
        assert_eq!(
            max.conditions.len(),
            1,
            "Max reads the value set — insensitive, not fenced"
        );

        let no_membership = Rule {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Count],
            atoms: vec![Atom {
                source: bumbledb::AtomSource::Edb(target::ids::MANDATE),
                bindings: vec![
                    (target::ids::mandate::ACCOUNT, Term::Var(VarId(0))),
                    (target::ids::mandate::ACTIVE, Term::Var(VarId(1))),
                ],
            }],
            negated: vec![],
            conditions: vec![],
        };
        assert!(
            lower_rule(&no_membership, &[]).conditions.is_empty(),
            "Count without a fired lowering stays expressible"
        );
    }

    /// Regenerates `lean/conformance/cases/` in place. Ignored: run it
    #[test]
    #[ignore = "regenerates the checked-in corpus; run deliberately"]
    fn regenerate_the_conformance_corpus() {
        let report = write_corpus(&corpus_dir());
        eprintln!("{}", report.coverage_line());
    }

    #[test]
    #[ignore = "regenerates the checked-in reach cases; run deliberately"]
    fn regenerate_the_recursive_conformance_corpus() {
        let report = reach::write_reach_corpus(&corpus_dir());
        eprintln!("{}", report.coverage_line());
    }

    #[test]
    #[ignore = "regenerates the checked-in judgment cases; run deliberately"]
    fn regenerate_the_judgment_conformance_corpus() {
        let dir = corpus_dir();
        for (name, document) in judgment::generate_judgment_corpus() {
            std::fs::write(dir.join(&name), document).expect("write a judgment case");
        }
    }

    #[test]
    #[ignore = "regenerates the checked-in complete-admission cases; run deliberately"]
    fn regenerate_the_complete_admission_corpus() {
        let dir = corpus_dir();
        for (name, document) in complete::generate_complete_corpus() {
            std::fs::write(dir.join(&name), document).expect("write a complete-admission case");
        }
    }

    #[test]
    fn the_corpus_replays_byte_identical_from_its_provenance() {
        let cases = replay_checked_in_corpus();
        eprintln!("conformance: {cases} checked-in cases replayed byte-identical");
    }

    /// Three-way (engine + naive + `lake exe conformance`) over the checked-in
    /// corpus. L19 removed cargo tests from `scripts/lean.sh`. This is L20
    /// qualification, not a Lean proof and not a G15 timing cell. Final
    /// qualification only:
    /// `cargo test -p bumbledb-bench three_way_conformance -- --ignored`.
    #[test]
    #[ignore = "needs elan/lake on PATH; L20 qualification, not scripts/lean.sh"]
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
