//! Find-list rules: Datalog safety and the aggregate roster
//! query's signature derivation, the ONE place result-column types
//! come from.
use super::{AggKind, Context, QueryType, RuleTyping, Signature, SignatureColumn};
use crate::error::{FindIndex, ValidationError};
use crate::ir::normalize::LoweredRule;
use crate::ir::{FindTerm, FoldOp, VarId};
use bumbledb_theory::schema::ValueType;
use std::collections::BTreeSet;

impl Signature {
    /// Retain only refinements shared by every arm. Recursive feedback keeps
    /// exact types; nonrecursive interval outputs may forget a fixed width.
    pub(super) fn meet(&mut self, other: &Self, widen: bool) -> Result<(), usize> {
        for (position, (left, right)) in self.columns.iter_mut().zip(&other.columns).enumerate() {
            if left.op() != right.op() {
                return Err(position);
            }
            if matches!(
                (&*left, right),
                (SignatureColumn::ProjectObservation(_), SignatureColumn::ProjectObservation(_)) if left.binding_type() == right.binding_type()
            ) {
                continue;
            }
            if matches!(
                (&*left, right),
                (SignatureColumn::Probability, SignatureColumn::Probability)
                    | (SignatureColumn::Expectation, SignatureColumn::Expectation)
            ) {
                continue;
            }
            let (Some(left_ty), Some(right_ty)) = (left.ty(), right.ty()) else {
                return Err(position);
            };
            let ty = if left_ty == right_ty {
                *left_ty
            } else if widen
                && let Some(element) = left_ty.interval_element()
                && right_ty.interval_element() == Some(element)
            {
                ValueType::Interval { element }
            } else {
                return Err(position);
            };
            match left {
                SignatureColumn::Project { ty: current }
                | SignatureColumn::Fold { ty: current, .. } => *current = ty,
                SignatureColumn::Probability
                | SignatureColumn::Expectation
                | SignatureColumn::ProjectObservation(_) => {
                    unreachable!("matched above")
                }
            }
        }
        Ok(())
    }

    pub(super) fn derive(rule: &LoweredRule, typing: &RuleTyping) -> Self {
        let var_type = |var: &VarId| {
            *typing
                .var_types
                .get(var)
                .expect("typed var")
                .stored()
                .expect("validated scalar find")
        };
        let columns = rule
            .finds
            .iter()
            .map(|term| match term {
                FindTerm::Var(var) => match typing.var_types[var] {
                    QueryType::Stored(ty) => SignatureColumn::Project { ty },
                    QueryType::Observation(kind) => SignatureColumn::ProjectObservation(kind),
                },
                FindTerm::Segments { left, .. } => SignatureColumn::Project {
                    ty: ValueType::Interval {
                        element: var_type(left)
                            .interval_element()
                            .expect("validated interval"),
                    },
                },
                FindTerm::Compute(expr) => SignatureColumn::Project {
                    ty: expr
                        .result_type(|var| {
                            typing
                                .var_types
                                .get(&var)
                                .and_then(QueryType::stored)
                                .copied()
                        })
                        .expect("validated output expression"),
                },
                FindTerm::Event(_) => SignatureColumn::Project {
                    ty: ValueType::Event,
                },
                FindTerm::Probability { .. } => SignatureColumn::Probability,
                FindTerm::Expectation { .. } => SignatureColumn::Expectation,
                FindTerm::Test(_) => SignatureColumn::Project {
                    ty: ValueType::Bool,
                },
                FindTerm::Count => SignatureColumn::Fold {
                    ty: ValueType::U64,
                    op: AggKind::Count,
                },
                FindTerm::Aggregate { op, over } => SignatureColumn::Fold {
                    ty: var_type(over),
                    op: AggKind::of(*op),
                },
                FindTerm::Pack { over } => SignatureColumn::Fold {
                    ty: match var_type(over).interval_element() {
                        Some(element) => ValueType::Interval { element },
                        None => ValueType::Event,
                    },
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
            FoldOp::Mean => Self::Mean,
            FoldOp::Min => Self::Min,
            FoldOp::Max => Self::Max,
        }
    }
}

impl Context {
    #[expect(clippy::too_many_lines, reason = "one exhaustive find grammar check")]
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
            let required: Vec<VarId> = match term {
                FindTerm::Var(_) | FindTerm::Count => Vec::new(),
                FindTerm::Compute(expr) => expr.variables().collect(),
                FindTerm::Segments { left, right, .. } => vec![*left, *right],
                FindTerm::Aggregate { over, .. } | FindTerm::Pack { over } => vec![*over],
                FindTerm::Expectation { value, when, given } => {
                    value.variables().chain([*when, *given]).collect()
                }
                _ => term
                    .event_variables()
                    .expect("Event find")
                    .into_iter()
                    .collect(),
            };
            for var in required {
                if matches!(self.var_types.get(&var), Some(QueryType::Observation(_))) {
                    return Err(ValidationError::ObservationOperand { find, var });
                }
            }
            match term {
                FindTerm::Event(_) | FindTerm::Test(_) | FindTerm::Probability { .. } => {
                    for var in term.event_variables().expect("Event expression") {
                        if !self.atom_vars.contains(&var) {
                            return Err(ValidationError::UnboundFindVariable { var });
                        }
                        if *self.resolved_var_type(var) != ValueType::Event {
                            return Err(ValidationError::EventExpression {
                                find,
                                source: crate::EventExprError::NotEvent(var),
                            });
                        }
                    }
                }
                FindTerm::Expectation { value, when, given } => {
                    for var in value.variables().chain([*when, *given]) {
                        if !self.atom_vars.contains(&var) {
                            return Err(ValidationError::UnboundFindVariable { var });
                        }
                    }
                    for var in value.variables() {
                        if self.closed_vars.contains_key(&var) {
                            return Err(ValidationError::AggregateOverClosedReference { find });
                        }
                        if !matches!(self.resolved_var_type(var), ValueType::I64 | ValueType::U64) {
                            return Err(ValidationError::AggregateInputType { find });
                        }
                    }
                    for var in [when, given] {
                        if *self.resolved_var_type(*var) != ValueType::Event {
                            return Err(ValidationError::EventExpression {
                                find,
                                source: crate::EventExprError::NotEvent(*var),
                            });
                        }
                    }
                }
                FindTerm::Segments { left, right, .. } => {
                    for var in [left, right] {
                        if !self.atom_vars.contains(var) {
                            return Err(ValidationError::UnboundFindVariable { var: *var });
                        }
                    }
                    let left = self.resolved_var_type(*left).interval_element();
                    let right = self.resolved_var_type(*right).interval_element();
                    if left.is_none() || left != right {
                        return Err(ValidationError::ScalarExpression {
                            find,
                            source: crate::ScalarError::TypeMismatch,
                        });
                    }
                    if rule.finds.iter().any(|f| {
                        matches!(
                            f,
                            FindTerm::Count
                                | FindTerm::Aggregate { .. }
                                | FindTerm::Pack { .. }
                                | FindTerm::Expectation { .. }
                        )
                    }) {
                        return Err(ValidationError::ScalarExpression {
                            find,
                            source: crate::ScalarError::TypeMismatch,
                        });
                    }
                }
                FindTerm::Compute(expr) => {
                    let ty = expr
                        .result_type(|var| {
                            self.var_types
                                .get(&var)
                                .and_then(QueryType::stored)
                                .copied()
                        })
                        .map_err(|source| ValidationError::ScalarExpression { find, source })?;
                    if !matches!(
                        ty,
                        ValueType::I64 | ValueType::U64 | ValueType::F64 | ValueType::Bool
                    ) {
                        return Err(ValidationError::ScalarExpression {
                            find,
                            source: crate::ScalarError::NotNumeric,
                        });
                    }
                    for var in expr.variables() {
                        if !self.atom_vars.contains(&var) {
                            return Err(ValidationError::UnboundFindVariable { var });
                        }
                    }
                }
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
                    let admitted = match self.resolved_var_type(*over) {
                        ValueType::F64 => true,
                        ValueType::U64 | ValueType::I64 => !matches!(op, FoldOp::Mean),
                        _ => false,
                    };
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
                    if !self.resolved_var_type(*over).is_interval()
                        && *self.resolved_var_type(*over) != ValueType::Event
                    {
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
