//! Find-list rules: Datalog safety and the aggregate roster
//! query's signature derivation, the ONE place result-column types
//! come from.
use super::{AggKind, Context, RuleTyping, Signature, SignatureColumn};
use crate::error::{FindIndex, ValidationError};
use crate::ir::normalize::LoweredRule;
use crate::ir::{FindTerm, FoldOp, VarId};
use bumbledb_theory::schema::ValueType;
use std::collections::BTreeSet;

impl Signature {
    pub(super) fn derive(rule: &LoweredRule, typing: &RuleTyping) -> Self {
        let var_type = |var: &VarId| typing.var_types.get(var).copied().expect("typed var");
        let columns = rule
            .finds
            .iter()
            .map(|term| match term {
                FindTerm::Var(var) => SignatureColumn::Project { ty: var_type(var) },
                FindTerm::Count => SignatureColumn::Fold {
                    ty: ValueType::U64,
                    op: AggKind::Count,
                },
                FindTerm::Aggregate { op, over } => SignatureColumn::Fold {
                    ty: var_type(over),
                    op: AggKind::of(*op),
                },
                FindTerm::Pack { over } => SignatureColumn::Fold {
                    ty: var_type(over),
                    op: AggKind::Pack,
                },
            })
            .collect();
        Self { columns }
    }
}

impl AggKind {
    fn of(op: FoldOp) -> Self {
        match op {
            FoldOp::Sum => Self::Sum,
            FoldOp::Min => Self::Min,
            FoldOp::Max => Self::Max,
        }
    }
}

impl Context {
    pub(super) fn check_finds(
        &self,
        rule: &LoweredRule,
        group_key: &BTreeSet<VarId>,
    ) -> Result<(), ValidationError> {
        // one Pack per head (the multi-Pack product is refused with its

        let mut fold_seen = false;
        let mut pack_seen = false;
        for (find_idx, term) in rule.finds.iter().enumerate() {
            let find = FindIndex(find_idx);
            match term {
                FindTerm::Var(var) => {
                    if !self.atom_vars.contains(var) {
                        return Err(ValidationError::UnboundFindVariable { var: *var });
                    }
                }
                FindTerm::Count => {
                    fold_seen = true;
                    if pack_seen {
                        return Err(ValidationError::MixedPackAndFold { find });
                    }
                }
                FindTerm::Aggregate { op, over } => {
                    fold_seen = true;
                    if !self.atom_vars.contains(over) {
                        return Err(ValidationError::UnboundFindVariable { var: *over });
                    }
                    if group_key.contains(over) {
                        return Err(ValidationError::AggregateOverGroupKey { find });
                    }
                    let admitted = matches!(self.resolved_var_type(*over), ValueType::U64 | ValueType::I64)
                        || (*self.resolved_var_type(*over) == ValueType::F64
                            && matches!(op, FoldOp::Min | FoldOp::Max));
                    if !admitted {
                        return Err(ValidationError::AggregateInputType { find });
                    }
                    if self.closed_vars.contains_key(over) {
                        return Err(ValidationError::AggregateOverClosedReference { find });
                    }
                    if pack_seen {
                        return Err(ValidationError::MixedPackAndFold { find });
                    }
                }
                FindTerm::Pack { over } => {
                    if pack_seen {
                        return Err(ValidationError::MultiplePackTerms { find });
                    }
                    pack_seen = true;
                    if !self.atom_vars.contains(over) {
                        return Err(ValidationError::UnboundFindVariable { var: *over });
                    }
                    if group_key.contains(over) {
                        return Err(ValidationError::AggregateOverGroupKey { find });
                    }
                    if !self.resolved_var_type(*over).is_interval() {
                        return Err(ValidationError::PackInputType { find });
                    }
                    if fold_seen {
                        return Err(ValidationError::MixedPackAndFold { find });
                    }
                }
            }
        }
        Ok(())
    }
}
