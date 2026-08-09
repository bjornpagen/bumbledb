//! The query/program IR crossing (`TODO_CPP.md` §13) and the prepared-query
//! handle (§20).
//!
//! The C view structs mirror `bumbledb::ir` 1:1 — relations, fields, and
//! predicates by numeric id (the C++ layer resolves names at compile time
//! and sends ids; the bridge never sees names here) — exactly the shape
//! the Node bridge's `marshal::program_in` reads off JS objects. The
//! engine's IR validator remains the trust boundary at `bdb_db_prepare`.

use bumbledb::{
    AggOp, AllenMask, ArgKey, Atom, AtomSource, CmpOp, Comparison, ConditionTree, FieldId,
    FindTerm, HeadOp, HeadTerm, MaskTerm, ParamId, PredId, PredicateDef, PreparedQuery, Program,
    RelationId, Rule, SchemaDescriptor, Term, VarId,
};

use crate::db::{Engine, bdb_db};
use crate::error::{bdb_error, fail_engine, fail_shape};
use crate::value::{bdb_value, value_in};
use crate::{BridgeResult, bdb_status, box_in, box_out, guard, out, ref_in, slice_in};

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

/// One term. `var` is read for `Var`/`Measure`, `param` for
/// `Param`/`ParamSet`, `literal` for `Literal`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_term {
    pub kind: bdb_term_kind,
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

/// An atom source's tag: a stored relation (`Edb`) or a predicate of the
/// same program (`Idb`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_atom_source_kind {
    Edb,
    Idb,
}

/// One atom. `relation` is read for `Edb`, `pred` for `Idb`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_atom {
    pub source_kind: bdb_atom_source_kind,
    pub relation: u32,
    pub pred: u16,
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
    CountDistinct,
    ArgMax,
    ArgMin,
    Pack,
}

/// An Arg-restriction key position's tag.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_arg_key_kind {
    Var,
    Measure,
}

/// One rule-scoped aggregate op: the kind, plus the Arg key for
/// `ArgMax`/`ArgMin` (ignored for every other kind).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_agg_op {
    pub kind: bdb_head_op,
    pub arg_key_kind: bdb_arg_key_kind,
    pub arg_key_var: u16,
}

/// A find term's tag (`bumbledb::ir::FindTerm`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_find_term_kind {
    Var,
    Measure,
    Aggregate,
    AggregateMeasure,
}

/// One find term. `var` is read for `Var`/`Measure`; `op` plus
/// `has_over`/`over` for `Aggregate` (`has_over == false` is the nullary
/// `Count`); `op` plus `over` for `AggregateMeasure`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_find_term {
    pub kind: bdb_find_term_kind,
    pub var: u16,
    pub op: bdb_agg_op,
    pub has_over: bool,
    pub over: u16,
}

/// A head position's tag.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_head_term_kind {
    Var,
    Aggregate,
}

/// One head position; `op` is read for `Aggregate`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_head_term {
    pub kind: bdb_head_term_kind,
    pub op: bdb_head_op,
}

/// The Allen mask position's tag: a literal mask or a param resolved at
/// bind.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum bdb_mask_term_kind {
    Literal,
    Param,
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

/// One comparison operator; the mask fields are read for `Allen` only
/// (`mask` for a `Literal` mask term, `mask_param` for a `Param` one).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_cmp_op {
    pub kind: bdb_cmp_op_kind,
    pub mask_kind: bdb_mask_term_kind,
    pub mask: u16,
    pub mask_param: u16,
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

/// One condition-tree node. `cmp` is read for `Leaf`;
/// `children`/`child_count` for `And`/`Or`. Nesting past the engine's
/// `MAX_CONDITION_DEPTH` is refused at marshal (the engine's own bound,
/// re-checked here so the recursion is stack-safe on hostile input).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_condition {
    pub kind: bdb_condition_kind,
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

/// One predicate: the head shape its rules align against, and the rules.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_predicate {
    pub head: *const bdb_head_term,
    pub head_count: usize,
    pub rules: *const bdb_rule,
    pub rule_count: usize,
}

/// The whole program: predicates (`pred` = index) and the output
/// predicate. A query is the one-predicate program with `output == 0`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bdb_program {
    pub predicates: *const bdb_predicate,
    pub predicate_count: usize,
    pub output: u16,
}

// ---------------------------------------------------------------------------
// IR marshal (the ts bridge's program_in, off C views)
// ---------------------------------------------------------------------------

fn term_in(view: &bdb_term) -> BridgeResult<Term> {
    Ok(match view.kind {
        bdb_term_kind::Var => Term::Var(VarId(view.var)),
        bdb_term_kind::Param => Term::Param(ParamId(view.param)),
        bdb_term_kind::ParamSet => Term::ParamSet(ParamId(view.param)),
        bdb_term_kind::Literal => Term::Literal(value_in(&view.literal)?),
        bdb_term_kind::Measure => Term::Measure(VarId(view.var)),
    })
}

fn atom_in(view: &bdb_atom) -> BridgeResult<Atom> {
    let source = match view.source_kind {
        bdb_atom_source_kind::Edb => AtomSource::Edb(RelationId(view.relation)),
        bdb_atom_source_kind::Idb => AtomSource::Idb(PredId(view.pred)),
    };
    let bindings = slice_in(view.bindings, view.binding_count)?
        .iter()
        .map(|binding| Ok((FieldId(binding.field), term_in(&binding.term)?)))
        .collect::<BridgeResult<Vec<_>>>()?;
    Ok(Atom { source, bindings })
}

/// The tag-to-op lift, exhaustive over [`bdb_head_op`]: a new engine
/// `HeadOp` variant grows the C enum and breaks THIS match at compile.
fn agg_op_in(view: &bdb_agg_op) -> AggOp {
    let arg_key = || match view.arg_key_kind {
        bdb_arg_key_kind::Var => ArgKey::Var(VarId(view.arg_key_var)),
        bdb_arg_key_kind::Measure => ArgKey::Measure(VarId(view.arg_key_var)),
    };
    match view.kind {
        bdb_head_op::Sum => AggOp::Sum,
        bdb_head_op::Min => AggOp::Min,
        bdb_head_op::Max => AggOp::Max,
        bdb_head_op::Count => AggOp::Count,
        bdb_head_op::CountDistinct => AggOp::CountDistinct,
        bdb_head_op::ArgMax => AggOp::ArgMax { key: arg_key() },
        bdb_head_op::ArgMin => AggOp::ArgMin { key: arg_key() },
        bdb_head_op::Pack => AggOp::Pack,
    }
}

fn head_op_in(op: bdb_head_op) -> HeadOp {
    match op {
        bdb_head_op::Sum => HeadOp::Sum,
        bdb_head_op::Min => HeadOp::Min,
        bdb_head_op::Max => HeadOp::Max,
        bdb_head_op::Count => HeadOp::Count,
        bdb_head_op::CountDistinct => HeadOp::CountDistinct,
        bdb_head_op::ArgMax => HeadOp::ArgMax,
        bdb_head_op::ArgMin => HeadOp::ArgMin,
        bdb_head_op::Pack => HeadOp::Pack,
    }
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "the uniform marshal-lane shape: every `*_in` reader returns \
              `BridgeResult` so call sites compose with `?` regardless of \
              which tags can currently fail"
)]
fn find_term_in(view: &bdb_find_term) -> BridgeResult<FindTerm> {
    Ok(match view.kind {
        bdb_find_term_kind::Var => FindTerm::Var(VarId(view.var)),
        bdb_find_term_kind::Measure => FindTerm::Measure(VarId(view.var)),
        bdb_find_term_kind::Aggregate => FindTerm::Aggregate {
            op: agg_op_in(&view.op),
            over: view.has_over.then_some(VarId(view.over)),
        },
        bdb_find_term_kind::AggregateMeasure => FindTerm::AggregateMeasure {
            op: agg_op_in(&view.op),
            over: VarId(view.over),
        },
    })
}

fn mask_term_in(op: &bdb_cmp_op) -> BridgeResult<MaskTerm> {
    Ok(match op.mask_kind {
        bdb_mask_term_kind::Literal => MaskTerm::Literal(AllenMask::new(op.mask).ok_or_else(
            || fail_shape(&format!("invalid allen mask bits {}", op.mask)),
        )?),
        bdb_mask_term_kind::Param => MaskTerm::Param(ParamId(op.mask_param)),
    })
}

fn comparison_in(view: &bdb_comparison) -> BridgeResult<Comparison> {
    let op = match view.op.kind {
        bdb_cmp_op_kind::Eq => CmpOp::Eq,
        bdb_cmp_op_kind::Ne => CmpOp::Ne,
        bdb_cmp_op_kind::Lt => CmpOp::Lt,
        bdb_cmp_op_kind::Le => CmpOp::Le,
        bdb_cmp_op_kind::Gt => CmpOp::Gt,
        bdb_cmp_op_kind::Ge => CmpOp::Ge,
        bdb_cmp_op_kind::Allen => CmpOp::Allen {
            mask: mask_term_in(&view.op)?,
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
/// hostile input — the ts bridge's rule, verbatim.
fn condition_in(view: &bdb_condition, depth: usize) -> BridgeResult<ConditionTree> {
    if depth > bumbledb::MAX_CONDITION_DEPTH {
        return Err(fail_shape(&format!(
            "condition tree deeper than {} (the engine's MAX_CONDITION_DEPTH)",
            bumbledb::MAX_CONDITION_DEPTH
        )));
    }
    Ok(match view.kind {
        bdb_condition_kind::Leaf => ConditionTree::Leaf(comparison_in(&view.cmp)?),
        bdb_condition_kind::And => ConditionTree::And(condition_children(view, depth)?),
        bdb_condition_kind::Or => ConditionTree::Or(condition_children(view, depth)?),
    })
}

fn condition_children(
    view: &bdb_condition,
    depth: usize,
) -> BridgeResult<Vec<ConditionTree>> {
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

/// The whole inbound program, copied into the engine's owned `Program`
/// before `prepare`.
pub(crate) fn program_in(view: &bdb_program) -> BridgeResult<Program> {
    let predicates = slice_in(view.predicates, view.predicate_count)?
        .iter()
        .map(|predicate| {
            Ok(PredicateDef {
                head: slice_in(predicate.head, predicate.head_count)?
                    .iter()
                    .map(|term| {
                        Ok(match term.kind {
                            bdb_head_term_kind::Var => HeadTerm::Var,
                            bdb_head_term_kind::Aggregate => {
                                HeadTerm::Aggregate(head_op_in(term.op))
                            }
                        })
                    })
                    .collect::<BridgeResult<Vec<_>>>()?,
                rules: slice_in(predicate.rules, predicate.rule_count)?
                    .iter()
                    .map(rule_in)
                    .collect::<BridgeResult<Vec<_>>>()?,
            })
        })
        .collect::<BridgeResult<Vec<_>>>()?;
    Ok(Program {
        predicates,
        output: PredId(view.output),
    })
}

// ---------------------------------------------------------------------------
// Prepared queries (TODO_CPP.md §20)
// ---------------------------------------------------------------------------

/// The opaque prepared-query handle. Field order is load-bearing: the
/// prepared value borrows the engine through the `Arc` and must drop
/// first (the Node bridge's `PreparedHandle`, verbatim).
pub struct bdb_prepared {
    pub(crate) prepared: PreparedQuery<'static, SchemaDescriptor>,
    _db: std::sync::Arc<Engine>,
}

/// Prepares a program against the database: the engine validates,
/// normalizes, reads statistics, and plans ONCE; the returned handle is
/// reusable across snapshots of this database (`&mut` per execution —
/// one execution at a time; the handle is not thread-shareable).
/// Validation (roster) failures are `BDB_ERROR_KIND_VALIDATION`.
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_db_prepare(
    db: *const bdb_db,
    program: *const bdb_program,
    out_prepared: *mut *mut bdb_prepared,
    out_error: *mut *mut bdb_error,
) -> bdb_status {
    guard(out_error, || {
        let handle = ref_in(db)?;
        let program = program_in(ref_in(program)?)?;
        let engine = std::sync::Arc::clone(&handle.db);
        let prepared = engine
            .prepare(&program)
            .map_err(|error| fail_engine(error, Some(&handle.descriptor)))?;
        // SAFETY of the lifetime erasure: the prepared query borrows
        // schema and cache data owned by the engine behind `engine` (an
        // `Arc` whose heap address is stable); `bdb_prepared` carries that
        // `Arc` and declares `prepared` first, so the borrow always drops
        // before its owner (the Node bridge's proven ownership argument).
        #[expect(
            unsafe_code,
            reason = "the self-referential handle (prepared query + its owning Arc) \
                      needs a lifetime erasure; the SAFETY comment above carries \
                      the drop-order argument"
        )]
        let prepared = unsafe {
            std::mem::transmute::<
                PreparedQuery<'_, SchemaDescriptor>,
                PreparedQuery<'static, SchemaDescriptor>,
            >(prepared)
        };
        out(
            out_prepared,
            box_out(bdb_prepared {
                prepared,
                _db: engine,
            }),
        )?;
        Ok(bdb_status::Ok)
    })
}

/// Releases a prepared query (its plan, memo, and engine reference).
#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "extern export: the unsafe(no_mangle) ABI attribute")]
pub extern "C" fn bdb_prepared_destroy(prepared: *mut bdb_prepared) -> bdb_status {
    guard(std::ptr::null_mut(), || {
        drop(box_in(prepared)?);
        Ok(bdb_status::Ok)
    })
}
