//! Find-list rules: Datalog safety and the aggregate roster
//! (`docs/architecture/20-query-ir.md` § aggregation) — and the
//! predicate's signature derivation, the ONE place result-column types
//! come from.

use super::{AggKind, Context, RuleTyping, Signature, SignatureColumn};
use crate::error::ValidationError;
use crate::ir::normalize::LoweredRule;
use crate::ir::{AggOp, FindTerm, VarId};
use bumbledb_theory::schema::ValueType;
use std::collections::BTreeSet;

impl Signature {
    /// Derives the signature from one rule's find terms and resolved
    /// typing — called exactly once, at validation, on rule 0 (the
    /// per-rule alignment already proved every rule derives the same
    /// predicate). No other derivation of the answer tuple exists.
    pub(super) fn derive(rule: &LoweredRule, typing: &RuleTyping) -> Self {
        let var_type = |var: &VarId| typing.var_types[var].clone();
        let columns = rule
            .finds
            .iter()
            .map(|term| match term {
                FindTerm::Var(var) => SignatureColumn {
                    ty: var_type(var),
                    op: None,
                },
                // The measure positions are u64 by definition (|[s, e)| =
                // e − s — 20-query-ir § the measure): projected plain,
                // folded under the fold's kind.
                FindTerm::Measure(_) => SignatureColumn {
                    ty: ValueType::U64,
                    op: None,
                },
                FindTerm::AggregateMeasure { op, .. } => SignatureColumn {
                    ty: ValueType::U64,
                    op: Some(AggKind::of(*op)),
                },
                FindTerm::Aggregate { op, over } => SignatureColumn {
                    ty: match op {
                        AggOp::Count => ValueType::U64,
                        AggOp::Sum | AggOp::Min | AggOp::Max | AggOp::Pack => {
                            var_type(&over.expect("validated: only Count is nullary"))
                        }
                    },
                    op: Some(AggKind::of(*op)),
                },
            })
            .collect();
        Self { columns }
    }
}

impl AggKind {
    fn of(op: AggOp) -> Self {
        match op {
            AggOp::Sum => Self::Sum,
            AggOp::Min => Self::Min,
            AggOp::Max => Self::Max,
            AggOp::Count => Self::Count,
            AggOp::Pack => Self::Pack,
        }
    }
}

impl Context {
    #[expect(
        clippy::too_many_lines,
        reason = "the linear table or protocol is clearer kept together"
    )] // the aggregate roster, one arm per shape
    pub(super) fn check_finds(
        &self,
        rule: &LoweredRule,
        group_key: &BTreeSet<VarId>,
    ) -> Result<(), ValidationError> {
        // Pack admits no companions but the group variables, and at most
        // one Pack per head (the multi-Pack product is refused with its
        // trigger on the error).
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
                FindTerm::Measure(var) => {
                    if !self.atom_vars.contains(var) {
                        return Err(ValidationError::UnboundFindVariable { var: *var });
                    }
                    if !matches!(self.resolved_var_type(*var), ValueType::Interval { .. }) {
                        return Err(ValidationError::DurationOverNonInterval { var: *var });
                    }
                }
                FindTerm::AggregateMeasure { op, over } => {
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
                        (AggOp::Sum | AggOp::Min | AggOp::Max | AggOp::Pack, None) => {
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
                            // The orderable roster (R3): Min/Max admit
                            // bool — false < true, so `Max` is Any and
                            // `Min` is All, the two quantifiers as the
                            // 0/1 encoding's extremes. Sum over bool
                            // stays refused: a quantifier is not an
                            // addition.
                            let admitted = match op {
                                AggOp::Sum => matches!(
                                    self.resolved_var_type(*var),
                                    ValueType::U64 | ValueType::I64
                                ),
                                _ => matches!(
                                    self.resolved_var_type(*var),
                                    ValueType::U64 | ValueType::I64 | ValueType::Bool
                                ),
                            };
                            if !admitted {
                                return Err(ValidationError::AggregateInputType { find: find_idx });
                            }
                            // The closed-reference wall (R4): folding a
                            // declaration-order accident is ordering it.
                            if self.closed_vars.contains_key(var) {
                                return Err(ValidationError::AggregateOverClosedReference {
                                    find: find_idx,
                                });
                            }
                        }
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
                    }
                    if pack_seen && fold_seen {
                        return Err(ValidationError::MixedPackAndFold { find: find_idx });
                    }
                }
            }
        }
        Ok(())
    }
}
