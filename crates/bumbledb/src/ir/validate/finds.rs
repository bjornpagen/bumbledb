//! Find-list rules: Datalog safety, the aggregate roster, and the Arg
//! discipline (`docs/architecture/20-query-ir.md` § aggregation).

use super::Context;
use crate::error::ValidationError;
use crate::ir::{AggOp, FindTerm, Rule, VarId};
use crate::schema::ValueType;
use std::collections::BTreeSet;

impl Context {
    pub(super) fn check_finds(
        &self,
        rule: &Rule,
        group_key: &BTreeSet<VarId>,
    ) -> Result<(), ValidationError> {
        // The Arg discipline: all Arg terms share one key variable and one
        // direction, and Arg terms and fold aggregates may not mix.
        let mut arg_spec: Option<(VarId, bool)> = None; // (key, is_max)
        let mut arg_seen = false;
        let mut fold_seen = false;
        for (find_idx, term) in rule.finds.iter().enumerate() {
            match term {
                FindTerm::Var(var) => {
                    if !self.atom_vars.contains(var) {
                        return Err(ValidationError::UnboundFindVariable { var: *var });
                    }
                }
                FindTerm::Aggregate { op, over } => {
                    match (op, over) {
                        (AggOp::Count, Some(_)) => {
                            return Err(ValidationError::CountWithVariable { find: find_idx });
                        }
                        (AggOp::Count, None) => {
                            fold_seen = true;
                        }
                        (
                            AggOp::Sum
                            | AggOp::Min
                            | AggOp::Max
                            | AggOp::CountDistinct
                            | AggOp::ArgMax { .. }
                            | AggOp::ArgMin { .. },
                            None,
                        ) => {
                            return Err(ValidationError::AggregateWithoutVariable {
                                find: find_idx,
                            });
                        }
                        (AggOp::Sum | AggOp::Min | AggOp::Max, Some(var)) => {
                            fold_seen = true;
                            if !self.atom_vars.contains(var) {
                                return Err(ValidationError::UnboundFindVariable { var: *var });
                            }
                            if group_key.contains(var) {
                                return Err(ValidationError::AggregateOverGroupKey {
                                    find: find_idx,
                                });
                            }
                            if !matches!(
                                self.resolved_var_type(*var),
                                ValueType::U64 | ValueType::I64
                            ) {
                                return Err(ValidationError::AggregateInputType { find: find_idx });
                            }
                        }
                        // CountDistinct is legal over every type — equality
                        // is all it needs.
                        (AggOp::CountDistinct, Some(var)) => {
                            fold_seen = true;
                            if !self.atom_vars.contains(var) {
                                return Err(ValidationError::UnboundFindVariable { var: *var });
                            }
                            if group_key.contains(var) {
                                return Err(ValidationError::AggregateOverGroupKey {
                                    find: find_idx,
                                });
                            }
                        }
                        // Arg-restriction: `over` is the carried variable
                        // (it may equal the key); the key rides in the op,
                        // must be orderable, and may itself be projected.
                        (AggOp::ArgMax { key } | AggOp::ArgMin { key }, Some(carry)) => {
                            arg_seen = true;
                            if !self.atom_vars.contains(carry) {
                                return Err(ValidationError::UnboundFindVariable { var: *carry });
                            }
                            if !self.atom_vars.contains(key) {
                                return Err(ValidationError::UnboundFindVariable { var: *key });
                            }
                            if group_key.contains(carry) {
                                return Err(ValidationError::AggregateOverGroupKey {
                                    find: find_idx,
                                });
                            }
                            if !matches!(
                                self.resolved_var_type(*key),
                                ValueType::U64 | ValueType::I64
                            ) {
                                return Err(ValidationError::NonOrderableArgKey { find: find_idx });
                            }
                            let is_max = matches!(op, AggOp::ArgMax { .. });
                            match arg_spec {
                                None => arg_spec = Some((*key, is_max)),
                                Some((shared_key, shared_dir)) => {
                                    if shared_key != *key || shared_dir != is_max {
                                        return Err(ValidationError::ArgKeyMismatch {
                                            find: find_idx,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    if arg_seen && fold_seen {
                        return Err(ValidationError::MixedArgAndFold { find: find_idx });
                    }
                }
            }
        }
        Ok(())
    }
}
