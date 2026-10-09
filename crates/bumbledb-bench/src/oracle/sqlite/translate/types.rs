use std::collections::BTreeSet;

use bumbledb::ir::{CmpOp, Comparison, Rule, Term};
use bumbledb::{ParamId, Schema, Value, VarId};

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

    fn is_scalar(&self, term: &Term) -> bool {
        match term {
            Term::Var(var) => self.scalar_vars.contains(var),
            Term::Param(param) => self.scalar_params.contains(param),

            Term::ParamSet(_) => true,
            Term::Literal(value) => {
                !matches!(value, Value::IntervalU64(..) | Value::IntervalI64(..))
            }
        }
    }

    fn mark_scalar(&mut self, term: &Term) -> bool {
        match term {
            Term::Var(var) => self.scalar_vars.insert(*var),
            Term::Param(param) => self.scalar_params.insert(*param),
            Term::ParamSet(_) | Term::Literal(_) => false,
        }
    }
}

pub(super) fn infer(rule: &Rule, schema: &Schema) -> TermTypes {
    let mut types = TermTypes::default();
    for atom in rule.atoms.iter().chain(&rule.negated) {
        for (field, term) in &atom.bindings {
            // interval-typed derived columns before any rule renders.
            let interval_field = match atom.source {
                bumbledb::AtomSource::Edb(relation) => schema.relation(relation).fields()
                    [usize::from(field.0)]
                .value_type
                .is_interval(),
                bumbledb::AtomSource::Interior(_) => false,
            };
            match term {
                Term::Var(_) | Term::Param(_) if !interval_field => {
                    types.mark_scalar(term);
                }

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
                CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge => {
                    changed |= types.mark_scalar(lhs);
                    changed |= types.mark_scalar(rhs);
                }

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
