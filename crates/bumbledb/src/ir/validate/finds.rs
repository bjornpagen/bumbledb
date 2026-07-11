//! Find-list rules: Datalog safety, the aggregate roster, and the Arg
//! discipline (`docs/architecture/20-query-ir.md` § aggregation).

use super::Context;
use crate::error::ValidationError;
use crate::ir::normalize::LoweredRule;
use crate::ir::{AggOp, FindTerm, VarId};
use crate::schema::ValueType;
use std::collections::BTreeSet;

impl Context {
    #[allow(clippy::too_many_lines)] // the aggregate roster, one arm per shape
    pub(super) fn check_finds(
        &self,
        rule: &LoweredRule,
        group_key: &BTreeSet<VarId>,
    ) -> Result<(), ValidationError> {
        // The Arg discipline: all Arg terms share one key variable and one
        // direction, and Arg terms and fold aggregates may not mix. Pack
        // extends the same rule — the relation-shaped aggregates admit no
        // companions but the group variables, and at most one Pack per
        // head (the multi-Pack product is refused with its trigger on the
        // error).
        let mut arg_spec: Option<(VarId, bool)> = None; // (key, is_max)
        let mut arg_seen = false;
        let mut fold_seen = false;
        let mut pack_seen = false;
        for (find_idx, term) in rule.finds.iter().enumerate() {
            match term {
                FindTerm::Var(var) => {
                    if !self.atom_vars.contains(var) {
                        return Err(ValidationError::UnboundFindVariable { var: *var });
                    }
                }
                // The measure positions (20-query-ir, § the measure):
                // both require an atom-bound, interval-resolved variable;
                // the fold form admits Sum/Min/Max only and folds a u64
                // input, so the aggregate-input type rule is satisfied by
                // construction.
                FindTerm::Duration(var) => {
                    if !self.atom_vars.contains(var) {
                        return Err(ValidationError::UnboundFindVariable { var: *var });
                    }
                    if !matches!(self.resolved_var_type(*var), ValueType::Interval { .. }) {
                        return Err(ValidationError::DurationOverNonInterval { var: *var });
                    }
                }
                FindTerm::AggregateDuration { op, over } => {
                    fold_seen = true;
                    if !matches!(op, AggOp::Sum | AggOp::Min | AggOp::Max) {
                        return Err(ValidationError::DurationAggregateOp { find: find_idx });
                    }
                    if !self.atom_vars.contains(over) {
                        return Err(ValidationError::UnboundFindVariable { var: *over });
                    }
                    if !matches!(self.resolved_var_type(*over), ValueType::Interval { .. }) {
                        return Err(ValidationError::DurationOverNonInterval { var: *over });
                    }
                    if group_key.contains(over) {
                        return Err(ValidationError::AggregateOverGroupKey { find: find_idx });
                    }
                    if arg_seen {
                        return Err(ValidationError::MixedArgAndFold { find: find_idx });
                    }
                    if pack_seen {
                        return Err(ValidationError::MixedPackAndFold { find: find_idx });
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
                            | AggOp::ArgMin { .. }
                            | AggOp::Pack,
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
                        // Pack: interval-typed input only, one Pack per
                        // head, and — like the Arg discipline — no fold
                        // companions (the pairwise checks run after the
                        // match, where every flag is current).
                        (AggOp::Pack, Some(var)) => {
                            if pack_seen {
                                return Err(ValidationError::MultiplePackTerms { find: find_idx });
                            }
                            pack_seen = true;
                            if !self.atom_vars.contains(var) {
                                return Err(ValidationError::UnboundFindVariable { var: *var });
                            }
                            if group_key.contains(var) {
                                return Err(ValidationError::AggregateOverGroupKey {
                                    find: find_idx,
                                });
                            }
                            if !matches!(self.resolved_var_type(*var), ValueType::Interval { .. }) {
                                return Err(ValidationError::PackInputType { find: find_idx });
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
                    if pack_seen && fold_seen {
                        return Err(ValidationError::MixedPackAndFold { find: find_idx });
                    }
                    if pack_seen && arg_seen {
                        return Err(ValidationError::MixedPackAndArg { find: find_idx });
                    }
                }
            }
        }
        Ok(())
    }
}
