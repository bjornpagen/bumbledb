//! Term-type resolution for translation — a compact mirror of the
//! engine's bivalent-anchor rule (`ir::validate`; `docs/architecture/
//! 20-query-ir.md` § membership). The translator needs exactly one bit
//! per term — interval-typed or scalar — to choose between the membership
//! and half-equality forms. The rules are validation's: a monovalent
//! anchor (a scalar-field binding, an order comparison, a scalar side of
//! `Eq`/`Ne`) collapses a term to the scalar reading; a term anchored
//! only by interval positions resolves to the interval reading, exactly
//! as `resolve_bivalents` does. The translator evaluates queries the
//! engine's validation boundary has accepted, so conflicting anchors
//! cannot arise here.

use std::collections::BTreeSet;

use bumbledb::ir::{CmpOp, Comparison, Rule, Term};
use bumbledb::schema::ValueType;
use bumbledb::{ParamId, Schema, Value, VarId};

/// The resolved scalar terms; everything else reads as interval-typed
/// (the bivalent default).
#[derive(Debug, Default)]
pub(super) struct TermTypes {
    scalar_vars: BTreeSet<VarId>,
    scalar_params: BTreeSet<ParamId>,
}

impl TermTypes {
    pub(super) fn var_is_interval(&self, var: VarId) -> bool {
        !self.scalar_vars.contains(&var)
    }

    pub(super) fn param_is_interval(&self, param: ParamId) -> bool {
        !self.scalar_params.contains(&param)
    }

    /// Whether a term's scalar reading is already established.
    fn is_scalar(&self, term: &Term) -> bool {
        match term {
            Term::Var(var) => self.scalar_vars.contains(var),
            Term::Param(param) => self.scalar_params.contains(param),
            // A set holds points — always the element reading; the
            // measure is u64-valued (its variable keeps the interval
            // reading — never marked scalar through it).
            Term::ParamSet(_) | Term::Measure(_) => true,
            Term::Literal(value) => {
                !matches!(value, Value::IntervalU64(..) | Value::IntervalI64(..))
            }
        }
    }

    /// Marks a variable or param scalar; returns whether that changed
    /// anything (the fixpoint's progress signal).
    fn mark_scalar(&mut self, term: &Term) -> bool {
        match term {
            Term::Var(var) => self.scalar_vars.insert(*var),
            Term::Param(param) => self.scalar_params.insert(*param),
            Term::ParamSet(_) | Term::Literal(_) | Term::Measure(_) => false,
        }
    }
}

/// Resolves every variable and param of one validated rule (variables
/// are rule-scoped; the translator types each rule's core
/// independently, exactly as validation's per-rule fixpoint does).
/// Anchors flow exactly as in validation: field bindings first, then a
/// fixpoint over the predicates (comparison order cannot matter).
/// `Allen` operands anchor the interval reading — the default, so they
/// propagate nothing; `PointIn`'s right side is a point (the surviving
/// point form), so it anchors scalar.
pub(super) fn infer(rule: &Rule, schema: &Schema) -> TermTypes {
    let mut types = TermTypes::default();
    for atom in rule.atoms.iter().chain(&rule.negated) {
        let relation = schema.relation(atom.relation);
        for (field, term) in &atom.bindings {
            let interval_field = matches!(
                relation.fields()[usize::from(field.0)].value_type,
                ValueType::Interval { .. }
            );
            match term {
                Term::Var(_) | Term::Param(_) if !interval_field => {
                    types.mark_scalar(term);
                }
                // A set's interval-field position is membership per
                // element, never interval equality (`ir::validate`).
                Term::ParamSet(param) => {
                    types.scalar_params.insert(*param);
                }
                _ => {}
            }
        }
    }
    loop {
        let mut changed = false;
        for Comparison { op, lhs, rhs } in rule.conditions.iter().map(super::leaf) {
            match op {
                // Order operators are scalar-only by the type rules.
                CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge => {
                    changed |= types.mark_scalar(lhs);
                    changed |= types.mark_scalar(rhs);
                }
                // Same-type operators: a scalar side names the other.
                CmpOp::Eq | CmpOp::Ne => {
                    if types.is_scalar(lhs) {
                        changed |= types.mark_scalar(rhs);
                    }
                    if types.is_scalar(rhs) {
                        changed |= types.mark_scalar(lhs);
                    }
                }
                CmpOp::Allen { .. } => {}
                CmpOp::PointIn => {
                    changed |= types.mark_scalar(rhs);
                }
            }
        }
        if !changed {
            return types;
        }
    }
}
