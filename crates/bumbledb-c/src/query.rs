//! The query IR crossing and the prepared-query handle.
//!
//! The C view structs mirror `bumbledb::ir` 1:1 — relations, fields, and
//! interiors by numeric id (the host resolves names and sends ids; the
//! bridge never sees names here) — exactly the shape the Node bridge's
//! `marshal::query_in` reads off JS objects. The engine's IR validator
//! remains the trust boundary at `bdb_db_prepare`.

use std::sync::atomic::{AtomicBool, Ordering};

use bumbledb::{
    AllenMask, Atom, AtomSource, CmpOp, Comparison, ConditionTree, FieldId, FindTerm, FoldOp,
    HeadOp, HeadTerm, Interior, InteriorId, NonEmpty, ParamId, PreparedQuery, ProjectionRule,
    Query, Rec, RecRule, RecStep, RelationId, Rule, SchemaDescriptor, Term, VarId,
};

use crate::db::{Engine, OwnerToken, bdb_db};
use crate::error::{bdb_error, fail_busy, fail_engine, fail_shape};
use crate::value::{bdb_value, value_in};
use crate::{
    BridgeResult, Fail, bdb_status, box_in, box_out_to, c_tag, guard, ref_in, require_out,
    slice_in, tag_in,
};

// ---------------------------------------------------------------------------
// IR views
// ---------------------------------------------------------------------------

/// A term's tag (`bumbledb::ir::Term`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_term_kind {
    Var,
    Param,
    ParamSet,
    Literal,
    Measure,
}

c_tag!(bdb_term_kind {
    Var,
    Param,
    ParamSet,
    Literal,
    Measure,
});

/// One term. `var` is read for `Var`/`Measure`, `param` for
/// `Param`/`ParamSet`, `literal` for `Literal`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_term {
    pub kind: u32,
    pub var: u16,
    pub param: u16,
    pub literal: bdb_value,
}

/// One atom binding: `(field, term)`. Absence of a field is the
/// wildcard — bind only what the rule constrains.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_binding {
    pub field: u16,
    pub term: bdb_term,
}

/// An atom source's tag: a stored relation (`Edb`) or a derived table of
/// the same query (`Interior` — an interior or the rec).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_atom_source_kind {
    Edb,
    Interior,
}

c_tag!(bdb_atom_source_kind { Edb, Interior });

/// One atom. `relation` is read for `Edb`, `interior` for `Interior`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_atom {
    pub source_kind: u32,
    pub relation: u32,
    pub interior: u32,
    pub bindings: *const bdb_binding,
    pub binding_count: usize,
}

/// The var-free aggregate-op kind at a head position
/// (`bumbledb::ir::HeadOp`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_head_op {
    Sum,
    Min,
    Max,
    Count,
    Pack,
}

c_tag!(bdb_head_op {
    Sum,
    Min,
    Max,
    Count,
    Pack
});

/// One rule-scoped aggregate op.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_agg_op {
    pub kind: u32,
}

/// A find term's tag (`bumbledb::ir::FindTerm`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_find_term_kind {
    Var,
    Measure,
    Aggregate,
    AggregateMeasure,
    Count,
}

c_tag!(bdb_find_term_kind {
    Var,
    Measure,
    Aggregate,
    AggregateMeasure,
    Count,
});

/// One find term. `var` is read for `Var`/`Measure`; `op` plus `over` for
/// `Aggregate`/`AggregateMeasure` (folds always carry `over`); `Count` is
/// nullary and does not read `over`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_find_term {
    pub kind: u32,
    pub var: u16,
    pub op: bdb_agg_op,
    pub over: u16,
}

/// A head position's tag.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_head_term_kind {
    Var,
    Aggregate,
}

c_tag!(bdb_head_term_kind { Var, Aggregate });

/// One head position; `op` is read for `Aggregate`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_head_term {
    pub kind: u32,
    pub op: u32,
}

/// A comparison operator's tag (`bumbledb::ir::CmpOp`). For `PointIn`
/// the lhs is the INTERVAL term and the rhs the point term (the engine's
/// ordered lowering; the notation reads point-first).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_cmp_op_kind {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Allen,
    PointIn,
}

c_tag!(bdb_cmp_op_kind {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Allen,
    PointIn,
});

/// One comparison operator; `mask` is the literal 13-bit Allen mask,
/// read for `Allen` only.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_cmp_op {
    pub kind: u32,
    pub mask: u16,
}

/// One comparison condition.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_comparison {
    pub op: bdb_cmp_op,
    pub lhs: bdb_term,
    pub rhs: bdb_term,
}

/// A condition node's tag.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_condition_kind {
    Leaf,
    And,
    Or,
}

c_tag!(bdb_condition_kind { Leaf, And, Or });

/// One condition-tree node. `cmp` is read for `Leaf`;
/// `children`/`child_count` for `And`/`Or`. Nesting past the engine's
/// `MAX_CONDITION_DEPTH` is refused at marshal (the engine's own bound,
/// re-checked here so the recursion is stack-safe on hostile input).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_condition {
    pub kind: u32,
    pub cmp: bdb_comparison,
    pub children: *const bdb_condition,
    pub child_count: usize,
}

/// One rule: finds against the head, positive atoms, negated atoms,
/// conditions (conjoined).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_rule {
    pub finds: *const bdb_find_term,
    pub find_count: usize,
    pub atoms: *const bdb_atom,
    pub atom_count: usize,
    pub negated: *const bdb_atom,
    pub negated_count: usize,
    pub conditions: *const bdb_condition,
    pub condition_count: usize,
}

/// One named interior: a finite CQ (union of conjunctive rules).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_interior {
    pub head: *const bdb_head_term,
    pub head_count: usize,
    pub rules: *const bdb_rule,
    pub rule_count: usize,
}

/// One linear rec: base arms and rec arms.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_rec {
    pub head: *const bdb_head_term,
    pub head_count: usize,
    pub base: *const bdb_rule,
    pub base_count: usize,
    pub rec: *const bdb_rule,
    pub rec_count: usize,
}

/// Q1 discriminant: a CQ (no rec) or a Reach (rec by value).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_query_kind {
    Cq,
    Reach,
}

c_tag!(bdb_query_kind { Cq, Reach });

/// CQ payload: named interiors, then the main answer. No rec slot.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_cq {
    pub interiors: *const bdb_interior,
    pub interior_count: usize,
    pub head: *const bdb_head_term,
    pub head_count: usize,
    pub rules: *const bdb_rule,
    pub rule_count: usize,
}

/// Reach payload: named interiors, a required rec, then the main answer.
/// `rec` is the Reach arm's rec by value — not a nullable pointer.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_reach {
    pub interiors: *const bdb_interior,
    pub interior_count: usize,
    pub rec: bdb_rec,
    pub head: *const bdb_head_term,
    pub head_count: usize,
    pub rules: *const bdb_rule,
    pub rule_count: usize,
}

/// Live arm of [`bdb_query`]: CQ or Reach. Read only the arm `kind` names.
#[repr(C)]
#[derive(Clone, Copy)]
pub union bdb_query_payload {
    pub cq: bdb_cq,
    pub reach: bdb_reach,
}

/// The whole query: tagged encoding of Q1 (`Cq | Reach`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_query {
    pub kind: u32,
    pub payload: bdb_query_payload,
}

// ---------------------------------------------------------------------------
// IR marshal (the ts bridge's query_in, off C views)
// ---------------------------------------------------------------------------

fn term_in(view: &bdb_term) -> BridgeResult<Term> {
    Ok(match tag_in::<bdb_term_kind>(view.kind)? {
        bdb_term_kind::Var => Term::Var(VarId(view.var)),
        bdb_term_kind::Param => Term::Param(ParamId(view.param)),
        bdb_term_kind::ParamSet => Term::ParamSet(ParamId(view.param)),
        bdb_term_kind::Literal => Term::Literal(value_in(&view.literal)?),
        bdb_term_kind::Measure => Term::Measure(VarId(view.var)),
    })
}

fn atom_in(view: &bdb_atom) -> BridgeResult<Atom> {
    let source = match tag_in::<bdb_atom_source_kind>(view.source_kind)? {
        bdb_atom_source_kind::Edb => AtomSource::Edb(RelationId(view.relation)),
        bdb_atom_source_kind::Interior => AtomSource::Interior(InteriorId(view.interior)),
    };
    let bindings = slice_in(view.bindings, view.binding_count)?
        .iter()
        .map(|binding| Ok((FieldId(binding.field), term_in(&binding.term)?)))
        .collect::<BridgeResult<Vec<_>>>()?;
    Ok(Atom { source, bindings })
}

/// The tag-to-op lift, exhaustive over [`bdb_head_op`]: a new engine
/// `HeadOp` variant grows the C enum and breaks THIS match at compile.
fn fold_op_in(view: bdb_agg_op) -> BridgeResult<FoldOp> {
    Ok(match tag_in::<bdb_head_op>(view.kind)? {
        bdb_head_op::Sum => FoldOp::Sum,
        bdb_head_op::Min => FoldOp::Min,
        bdb_head_op::Max => FoldOp::Max,
        bdb_head_op::Count => {
            return Err(fail_shape(
                "Count is BDB_FIND_TERM_KIND_COUNT, not AGGREGATE",
            ));
        }
        bdb_head_op::Pack => {
            return Err(fail_shape("Pack is a pack find, not a fold AGGREGATE"));
        }
    })
}

fn head_op_in(op: u32) -> BridgeResult<HeadOp> {
    Ok(match tag_in::<bdb_head_op>(op)? {
        bdb_head_op::Sum => HeadOp::Sum,
        bdb_head_op::Min => HeadOp::Min,
        bdb_head_op::Max => HeadOp::Max,
        bdb_head_op::Count => HeadOp::Count,
        bdb_head_op::Pack => HeadOp::Pack,
    })
}

fn find_term_in(view: &bdb_find_term) -> BridgeResult<FindTerm> {
    Ok(match tag_in::<bdb_find_term_kind>(view.kind)? {
        bdb_find_term_kind::Var => FindTerm::Var(VarId(view.var)),
        bdb_find_term_kind::Measure => FindTerm::Measure(VarId(view.var)),
        bdb_find_term_kind::Count => FindTerm::Count,
        bdb_find_term_kind::Aggregate => {
            let op = match tag_in::<bdb_head_op>(view.op.kind)? {
                bdb_head_op::Sum => FoldOp::Sum,
                bdb_head_op::Min => FoldOp::Min,
                bdb_head_op::Max => FoldOp::Max,
                bdb_head_op::Count => {
                    return Err(fail_shape(
                        "Count is BDB_FIND_TERM_KIND_COUNT, not AGGREGATE",
                    ));
                }
                bdb_head_op::Pack => {
                    return Ok(FindTerm::Pack {
                        over: VarId(view.over),
                    });
                }
            };
            FindTerm::Aggregate {
                op,
                over: VarId(view.over),
            }
        }
        bdb_find_term_kind::AggregateMeasure => FindTerm::AggregateMeasure {
            op: fold_op_in(view.op)?,
            over: VarId(view.over),
        },
    })
}

fn mask_in(op: bdb_cmp_op) -> BridgeResult<AllenMask> {
    AllenMask::new(op.mask)
        .ok_or_else(|| fail_shape(&format!("invalid allen mask bits {}", op.mask)))
}

fn comparison_in(view: &bdb_comparison) -> BridgeResult<Comparison> {
    let op = match tag_in::<bdb_cmp_op_kind>(view.op.kind)? {
        bdb_cmp_op_kind::Eq => CmpOp::Eq,
        bdb_cmp_op_kind::Ne => CmpOp::Ne,
        bdb_cmp_op_kind::Lt => CmpOp::Lt,
        bdb_cmp_op_kind::Le => CmpOp::Le,
        bdb_cmp_op_kind::Gt => CmpOp::Gt,
        bdb_cmp_op_kind::Ge => CmpOp::Ge,
        bdb_cmp_op_kind::Allen => CmpOp::Allen {
            mask: mask_in(view.op)?,
        },
        bdb_cmp_op_kind::PointIn => CmpOp::PointIn,
    };
    Ok(Comparison {
        op,
        lhs: term_in(&view.lhs)?,
        rhs: term_in(&view.rhs)?,
    })
}

/// One condition tree, marshaled with the engine's own depth ceiling
/// (`bumbledb::MAX_CONDITION_DEPTH`): the roster rejects deeper trees
/// anyway, and refusing at marshal keeps this recursion stack-safe on
/// hostile input — the ts bridge's rule, verbatim. Parse by kind: Leaf
/// reads `cmp` only; And/Or read `children` only. Leftover payloads of
/// the other arm are never read.
fn condition_in(view: &bdb_condition, depth: usize) -> BridgeResult<ConditionTree> {
    if depth > bumbledb::MAX_CONDITION_DEPTH {
        return Err(fail_shape(&format!(
            "condition tree deeper than {} (the engine's MAX_CONDITION_DEPTH)",
            bumbledb::MAX_CONDITION_DEPTH
        )));
    }
    Ok(match tag_in::<bdb_condition_kind>(view.kind)? {
        bdb_condition_kind::Leaf => ConditionTree::Leaf(comparison_in(&view.cmp)?),
        bdb_condition_kind::And => ConditionTree::And(condition_children(view, depth)?),
        bdb_condition_kind::Or => ConditionTree::Or(condition_children(view, depth)?),
    })
}

fn condition_children(view: &bdb_condition, depth: usize) -> BridgeResult<Vec<ConditionTree>> {
    slice_in(view.children, view.child_count)?
        .iter()
        .map(|child| condition_in(child, depth + 1))
        .collect()
}

fn rule_in(view: &bdb_rule) -> BridgeResult<Rule> {
    Ok(Rule {
        finds: slice_in(view.finds, view.find_count)?
            .iter()
            .map(find_term_in)
            .collect::<BridgeResult<Vec<_>>>()?,
        atoms: slice_in(view.atoms, view.atom_count)?
            .iter()
            .map(atom_in)
            .collect::<BridgeResult<Vec<_>>>()?,
        negated: slice_in(view.negated, view.negated_count)?
            .iter()
            .map(atom_in)
            .collect::<BridgeResult<Vec<_>>>()?,
        conditions: slice_in(view.conditions, view.condition_count)?
            .iter()
            .map(|condition| condition_in(condition, 1))
            .collect::<BridgeResult<Vec<_>>>()?,
    })
}

fn head_in(head: *const bdb_head_term, count: usize) -> BridgeResult<Vec<HeadTerm>> {
    slice_in(head, count)?
        .iter()
        .map(|term| {
            Ok(match tag_in::<bdb_head_term_kind>(term.kind)? {
                bdb_head_term_kind::Var => HeadTerm::Var,
                bdb_head_term_kind::Aggregate => HeadTerm::Aggregate(head_op_in(term.op)?),
            })
        })
        .collect()
}

fn rules_in(rules: *const bdb_rule, count: usize) -> BridgeResult<Vec<Rule>> {
    slice_in(rules, count)?.iter().map(rule_in).collect()
}

fn vars_only(finds: &[FindTerm]) -> BridgeResult<Vec<VarId>> {
    finds
        .iter()
        .map(|term| match term {
            FindTerm::Var(var) => Ok(*var),
            _ => Err(fail_shape("derived-table finds are variables only")),
        })
        .collect()
}

fn projection_rule_in(view: &bdb_rule) -> BridgeResult<ProjectionRule> {
    let rule = rule_in(view)?;
    Ok(ProjectionRule {
        finds: vars_only(&rule.finds)?,
        atoms: rule.atoms,
        negated: rule.negated,
        conditions: rule.conditions,
    })
}

fn rec_rule_in(view: &bdb_rule) -> BridgeResult<RecRule> {
    let rule = rule_in(view)?;
    if !rule.negated.is_empty() {
        return Err(fail_shape("negation is unrepresentable in rec"));
    }
    Ok(RecRule {
        finds: vars_only(&rule.finds)?,
        atoms: rule.atoms,
        conditions: rule.conditions,
    })
}

fn rec_step_in(view: &bdb_rule, rec_id: InteriorId) -> BridgeResult<RecStep> {
    let rule = rule_in(view)?;
    if !rule.negated.is_empty() {
        return Err(fail_shape("negation is unrepresentable in rec"));
    }
    let mut self_bindings = None;
    let mut atoms = Vec::new();
    for atom in rule.atoms {
        if atom.source.interior() == Some(rec_id) {
            if self_bindings.is_some() {
                return Err(fail_shape("rec step has two self-atoms"));
            }
            self_bindings = Some(atom.bindings);
        } else {
            atoms.push(atom);
        }
    }
    Ok(RecStep {
        finds: vars_only(&rule.finds)?,
        self_bindings: self_bindings.ok_or_else(|| fail_shape("rec step missing self-atom"))?,
        atoms,
        conditions: rule.conditions,
    })
}

fn nonempty<T>(items: Vec<T>, what: &str) -> BridgeResult<NonEmpty<T>> {
    NonEmpty::from_vec(items).ok_or_else(|| fail_shape(&format!("empty {what}")))
}

fn interior_in(view: &bdb_interior) -> BridgeResult<Interior> {
    let _ = head_in(view.head, view.head_count)?;
    Ok(Interior {
        rules: slice_in(view.rules, view.rule_count)?
            .iter()
            .map(projection_rule_in)
            .collect::<BridgeResult<Vec<_>>>()?,
    })
}

fn rec_in(view: &bdb_rec, rec_id: InteriorId) -> BridgeResult<Rec> {
    let _ = head_in(view.head, view.head_count)?;
    let base = slice_in(view.base, view.base_count)?
        .iter()
        .map(rec_rule_in)
        .collect::<BridgeResult<Vec<_>>>()?;
    let rec = slice_in(view.rec, view.rec_count)?
        .iter()
        .map(|rule| rec_step_in(rule, rec_id))
        .collect::<BridgeResult<Vec<_>>>()?;
    Ok(Rec {
        base: nonempty(base, "rec base")?,
        rec: nonempty(rec, "rec step")?,
    })
}

fn query_from_cq(
    interiors: *const bdb_interior,
    interior_count: usize,
    head: *const bdb_head_term,
    head_count: usize,
    rules: *const bdb_rule,
    rule_count: usize,
) -> BridgeResult<Query> {
    Ok(Query {
        interiors: slice_in(interiors, interior_count)?
            .iter()
            .map(interior_in)
            .collect::<BridgeResult<Vec<_>>>()?,
        head: head_in(head, head_count)?,
        rules: rules_in(rules, rule_count)?,
        rec: None,
    })
}

fn query_from_reach(
    interiors: *const bdb_interior,
    interior_count: usize,
    rec: &bdb_rec,
    head: *const bdb_head_term,
    head_count: usize,
    rules: *const bdb_rule,
    rule_count: usize,
) -> BridgeResult<Query> {
    Ok(Query {
        interiors: slice_in(interiors, interior_count)?
            .iter()
            .map(interior_in)
            .collect::<BridgeResult<Vec<_>>>()?,
        rec: Some(rec_in(
            rec,
            InteriorId(u32::try_from(interior_count).map_err(|_| fail_shape("interior count"))?),
        )?),
        head: head_in(head, head_count)?,
        rules: rules_in(rules, rule_count)?,
    })
}

/// The whole inbound query, copied into the engine's owned `Query`
/// before `prepare`. Reads only the live arm named by `kind`.
pub(crate) fn query_in(view: &bdb_query) -> BridgeResult<Query> {
    match tag_in::<bdb_query_kind>(view.kind)? {
        bdb_query_kind::Cq => {
            // SAFETY: `kind` selected the CQ arm; the header contract
            // keeps that payload initialized for the call.
            #[expect(
                unsafe_code,
                reason = "union arm: CQ kind names payload.cq (header contract)"
            )]
            let cq = unsafe { view.payload.cq };
            query_from_cq(
                cq.interiors,
                cq.interior_count,
                cq.head,
                cq.head_count,
                cq.rules,
                cq.rule_count,
            )
        }
        bdb_query_kind::Reach => {
            // SAFETY: `kind` selected the Reach arm; the header contract
            // keeps that payload initialized for the call.
            #[expect(
                unsafe_code,
                reason = "union arm: Reach kind names payload.reach (header contract)"
            )]
            let reach = unsafe { view.payload.reach };
            query_from_reach(
                reach.interiors,
                reach.interior_count,
                &reach.rec,
                reach.head,
                reach.head_count,
                reach.rules,
                reach.rule_count,
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Prepared queries
// ---------------------------------------------------------------------------

/// The opaque prepared-query handle. Field order is load-bearing: the
/// prepared value drops before the optional store `Arc`. Heap-prepared
/// queries hold `None`.
pub struct bdb_prepared {
    pub(crate) prepared: PreparedQuery<SchemaDescriptor>,
    pub(crate) _keep: Option<std::sync::Arc<Engine>>,
    pub(crate) owner: OwnerToken,
    pub(crate) in_execute: AtomicBool,
}

pub(crate) struct InExecuteReset<'a>(&'a AtomicBool);

impl Drop for InExecuteReset<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

pub(crate) fn enter_execute(flag: &AtomicBool) -> BridgeResult<InExecuteReset<'_>> {
    if flag.swap(true, Ordering::AcqRel) {
        return Err(fail_busy(
            "re-entrant or concurrent execute on this prepared handle (one \
             execution at a time)",
        ));
    }
    Ok(InExecuteReset(flag))
}

/// The execute-exclusion flag, claimed before forming `&mut bdb_prepared`.
pub(crate) fn prepared_execute_flag<'a>(
    prepared: *mut bdb_prepared,
) -> BridgeResult<&'a AtomicBool> {
    if prepared.is_null() {
        return Err(Fail::Misuse);
    }
    #[expect(
        unsafe_code,
        reason = "claim in_execute on the raw handle before forming &mut; \
                  the flag is AtomicBool and destroy checks it before from_raw"
    )]
    // SAFETY: non-null was just checked; we only touch `in_execute`
    // (an AtomicBool) until we win exclusive. Concurrent destroy
    // loads the same flag before `from_raw` (best-effort, analogous
    // to enter_write). The returned borrow is the handle's field; the
    // caller holds the handle for the enclosing `bdb_instance_execute`.
    unsafe {
        Ok(&(*prepared).in_execute)
    }
}

/// Prepares a query against the database: the engine validates,
/// normalizes, reads statistics, and plans ONCE; the returned handle is
/// reusable across reads of this database (`&mut` per execution —
/// one execution at a time; the handle is not thread-shareable).
/// Validation (roster) failures are `BDB_ERROR_KIND_VALIDATION`.
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_db_prepare(
    db: *const bdb_db,
    query: *const bdb_query,
    out_prepared: *mut *mut bdb_prepared,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        require_out(out_prepared)?;
        let handle = ref_in(db)?;
        let query = query_in(ref_in(query)?)?;
        let engine = std::sync::Arc::clone(&handle.db);
        let prepared = engine.prepare(&query).map_err(fail_engine)?;
        box_out_to(
            out_prepared,
            bdb_prepared {
                prepared,
                owner: OwnerToken::Store(std::sync::Arc::as_ptr(&engine)),
                _keep: Some(engine),
                in_execute: AtomicBool::new(false),
            },
        )?;
        Ok(bdb_status::Ok)
    })
}

/// Releases a prepared query (its plan, memo, and engine reference).
#[unsafe(no_mangle)]
#[expect(
    unsafe_code,
    reason = "extern export: the unsafe(no_mangle) ABI attribute"
)]
pub extern "C" fn bdb_prepared_destroy(prepared: *mut bdb_prepared) -> bdb_status {
    crate::guard_statusless(|| {
        let handle = ref_in(prepared)?;
        if handle.in_execute.load(Ordering::Acquire) {
            return Err(Fail::Misuse);
        }
        drop(box_in(prepared)?);
        Ok(bdb_status::Ok)
    })
}
