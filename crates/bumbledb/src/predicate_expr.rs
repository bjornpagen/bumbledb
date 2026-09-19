//! Pure-data predicates on exact partial observations. Truth partitions are
//! values; reducing them to a database Boolean requires explicit quantification.
use crate::event::{BoolOp4, Error, ExactArithmetic, PolynomialSigns};
use crate::number_expr::{ObservationInputKind, ObservationOperand};
use crate::{
    NumberDomain, NumberExpr, NumberExprError, ObservationNumberCodecLimits, ObservationPredicate,
    ObservationPredicateImport, Result, VarId,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PredicateExpr {
    Var(VarId),
    Sign {
        number: NumberExpr,
        signs: PolynomialSigns,
    },
    Imported(ObservationPredicateImport),
    Negate(Box<Self>),
    Binary {
        op: BoolOp4,
        left: Box<Self>,
        right: Box<Self>,
    },
    OnDomain {
        value: Box<Self>,
        domain: NumberDomain,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredicateQuantifier {
    Possibly,
    Always,
    IsTotal,
}
impl PredicateQuantifier {
    #[must_use]
    pub fn evaluate(self, value: &ObservationPredicate) -> bool {
        match self {
            Self::Possibly => value.predicate().possibly(),
            Self::Always => value.predicate().always(),
            Self::IsTotal => value.predicate().is_total(),
        }
    }
}

impl PredicateExpr {
    /// Every written operand variable, including masked or redundant branches.
    pub fn variables(&self) -> impl Iterator<Item = VarId> {
        let mut pending = vec![self];
        let mut variables = Vec::new();
        while let Some(node) = pending.pop() {
            match node {
                Self::Var(var) => variables.push(*var),
                Self::Sign { number, .. } => variables.extend(number.variables()),
                Self::Imported(_) => {}
                Self::Negate(value) | Self::OnDomain { value, .. } => pending.push(value),
                Self::Binary { left, right, .. } => {
                    pending.push(right);
                    pending.push(left);
                }
            }
        }
        variables.into_iter()
    }

    /// Bound the combined predicate/numerical tree before recursive IR cloning
    /// or rendering. Numerical subtrees do not receive a fresh shape allowance.
    pub(crate) fn inputs(
        &self,
    ) -> std::result::Result<Vec<(VarId, ObservationInputKind)>, NumberExprError> {
        let mut inputs = Vec::new();
        self.collect_inputs(1, &mut 0, &mut inputs)?;
        Ok(inputs)
    }

    pub(crate) fn collect_inputs(
        &self,
        depth: usize,
        nodes: &mut usize,
        inputs: &mut Vec<(VarId, ObservationInputKind)>,
    ) -> std::result::Result<(), NumberExprError> {
        let mut pending = vec![(self, depth)];
        while let Some((node, depth)) = pending.pop() {
            if depth > 256 {
                return Err(NumberExprError::TooDeep);
            }
            *nodes += 1;
            if *nodes > 65_536 {
                return Err(NumberExprError::TooLarge);
            }
            match node {
                Self::Var(var) => inputs.push((*var, ObservationInputKind::Predicate)),
                Self::Sign { number, .. } => {
                    number.collect_inputs(depth + 1, nodes, inputs)?;
                }
                Self::Imported(_) => {}
                Self::Negate(value) | Self::OnDomain { value, .. } => {
                    pending.push((value, depth + 1));
                }
                Self::Binary { left, right, .. } => {
                    pending.push((right, depth + 1));
                    pending.push((left, depth + 1));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn evaluate(
        &self,
        mut operand: impl FnMut(VarId) -> Result<ObservationOperand>,
        limits: &ObservationNumberCodecLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ObservationPredicate> {
        let mut pending = vec![(self, false)];
        let mut values = Vec::<ObservationPredicate>::new();
        while let Some((node, finish)) = pending.pop() {
            work.control().checkpoint()?;
            pending.try_reserve(3).map_err(Error::from)?;
            values.try_reserve(1).map_err(Error::from)?;
            let value =
                if finish {
                    let value = values.pop().ok_or(Error::InvalidEncoding)?;
                    match node {
                        Self::Negate(_) => value.negate(limits.numbers, work)?,
                        Self::Binary { op, .. } => values
                            .pop()
                            .ok_or(Error::InvalidEncoding)?
                            .apply(*op, &value, limits.numbers, work)?,
                        Self::OnDomain { domain, .. } => {
                            value.on_domain(domain.value(), limits.numbers, work)?
                        }
                        _ => unreachable!("only operators schedule completion"),
                    }
                } else {
                    match node {
                        Self::Var(var) => {
                            let ObservationOperand::Predicate(value) = operand(*var)? else {
                                return Err(Error::InvalidEncoding.into());
                            };
                            value.predicate().validate(limits.numbers.numbers, work)?;
                            value
                        }
                        Self::Sign { number, signs } => number
                            .evaluate(&mut operand, limits, work)?
                            .where_sign(*signs, limits.numbers, work)?,
                        Self::Imported(value) => {
                            ObservationPredicateImport::from_bytes(value.bytes(), *limits, work)?
                                .value()
                                .clone()
                        }
                        Self::Negate(value) | Self::OnDomain { value, .. } => {
                            pending.push((node, true));
                            pending.push((value, false));
                            continue;
                        }
                        Self::Binary { left, right, .. } => {
                            pending.push((node, true));
                            pending.push((right, false));
                            pending.push((left, false));
                            continue;
                        }
                    }
                };
            values.push(value);
        }
        let value = values.pop().ok_or(Error::InvalidEncoding)?;
        if !values.is_empty() {
            return Err(Error::InvalidEncoding.into());
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_admission_counts_number_and_predicate_nodes_together() {
        let mut number = NumberExpr::Integer(VarId(0));
        for _ in 1..256 {
            number = NumberExpr::Negate(Box::new(number));
        }
        assert!(number.inputs().is_ok());
        let predicate = PredicateExpr::Sign {
            number,
            signs: PolynomialSigns::POSITIVE,
        };
        assert_eq!(predicate.inputs().unwrap_err(), NumberExprError::TooDeep);
        let mut number = NumberExpr::Integer(VarId(0));
        for _ in 0..15 {
            number = NumberExpr::Binary {
                op: crate::event::NumberOp::Add,
                left: Box::new(number.clone()),
                right: Box::new(number),
            };
        }
        let predicate = PredicateExpr::Sign {
            number,
            signs: PolynomialSigns::POSITIVE,
        };
        assert_eq!(predicate.inputs().unwrap().len(), 32_768);
        assert_eq!(
            PredicateExpr::Negate(Box::new(predicate))
                .inputs()
                .unwrap_err(),
            NumberExprError::TooLarge
        );
    }
}
