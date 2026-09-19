//! Pure-data numerical query programs. Observation component selection is
//! explicit, and scalar columns never masquerade as completed observations.
use crate::event::{
    Capacity, Error, ExactArithmetic, ExactRational, NumberOp, ParameterCodecLimits,
    ParameterDomain,
};
use crate::{
    ObservationComponent, ObservationNumber, ObservationNumberCodecLimits, ObservationNumberImport,
    Result, VarId,
};
use std::sync::Arc;

#[derive(Debug)]
struct Domain {
    value: ParameterDomain,
    bytes: Box<[u8]>,
}

/// An explicitly captured numerical domain. This neither changes an Event's
/// source nor binds a prior to its parameter.
#[derive(Debug, Clone)]
pub struct NumberDomain(Arc<Domain>);
impl PartialEq for NumberDomain {
    fn eq(&self, other: &Self) -> bool {
        self.0.bytes == other.0.bytes
    }
}
impl Eq for NumberDomain {}
impl NumberDomain {
    /// # Errors
    /// Solver/encoding bounds or cancellation.
    pub fn capture(
        value: ParameterDomain,
        limits: ParameterCodecLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let bytes = value.to_bytes(limits, work)?.into_boxed_slice();
        Ok(Self(Arc::new(Domain { value, bytes })))
    }
    /// # Errors
    /// Malformed/empty domains, solver/encoding bounds or cancellation.
    pub fn from_bytes(
        bytes: &[u8],
        limits: ParameterCodecLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        Self::capture(
            ParameterDomain::from_bytes(bytes, limits, work)?,
            limits,
            work,
        )
    }
    #[must_use]
    pub fn value(&self) -> &ParameterDomain {
        &self.0.value
    }
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.0.bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumberExpr {
    /// Project an already completed numerical value into this derivation.
    Var(VarId),
    /// Exact I64/U64 column read, retained as a literal value in the result.
    Integer(VarId),
    Component {
        observation: VarId,
        component: ObservationComponent,
    },
    Literal(ExactRational),
    Imported(ObservationNumberImport),
    Binary {
        op: NumberOp,
        left: Box<Self>,
        right: Box<Self>,
    },
    Negate(Box<Self>),
    Abs(Box<Self>),
    Pow {
        value: Box<Self>,
        exponent: u32,
    },
    OnDomain {
        value: Box<Self>,
        domain: NumberDomain,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberExprError {
    TooDeep,
    TooLarge,
    UnboundVariable(VarId),
    TypeMismatch(VarId),
}
impl std::fmt::Display for NumberExprError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooDeep => out.write_str("numerical expression exceeds its depth limit"),
            Self::TooLarge => out.write_str("numerical expression exceeds its node limit"),
            Self::UnboundVariable(var) => write!(out, "unbound numerical operand {var:?}"),
            Self::TypeMismatch(var) => write!(out, "wrong numerical operand type for {var:?}"),
        }
    }
}
impl std::error::Error for NumberExprError {}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ObservationInputKind {
    Predicate,
    Number,
    Integer,
    Observation,
}

impl NumberExpr {
    /// Every written variable occurrence, without evaluating or simplifying.
    pub fn variables(&self) -> impl Iterator<Item = VarId> + '_ {
        let mut pending = vec![self];
        std::iter::from_fn(move || {
            while let Some(node) = pending.pop() {
                match node {
                    Self::Var(var) | Self::Integer(var) => return Some(*var),
                    Self::Component { observation, .. } => return Some(*observation),
                    Self::Literal(_) | Self::Imported(_) => {}
                    Self::Binary { left, right, .. } => {
                        pending.push(right);
                        pending.push(left);
                    }
                    Self::Negate(value)
                    | Self::Abs(value)
                    | Self::Pow { value, .. }
                    | Self::OnDomain { value, .. } => pending.push(value),
                }
            }
            None
        })
    }
    /// Validate shape before any recursive IR formatting/cloning. This walk
    /// also keeps every variable occurrence, including zero/undefined branches.
    pub(crate) fn inputs(
        &self,
    ) -> std::result::Result<Vec<(VarId, ObservationInputKind)>, NumberExprError> {
        let mut nodes = 0;
        let mut inputs = Vec::new();
        self.collect_inputs(1, &mut nodes, &mut inputs)?;
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
                Self::Var(var) => inputs.push((*var, ObservationInputKind::Number)),
                Self::Integer(var) => inputs.push((*var, ObservationInputKind::Integer)),
                Self::Component { observation, .. } => {
                    inputs.push((*observation, ObservationInputKind::Observation));
                }
                Self::Literal(_) | Self::Imported(_) => {}
                Self::Binary { left, right, .. } => {
                    pending.push((right, depth + 1));
                    pending.push((left, depth + 1));
                }
                Self::Negate(value)
                | Self::Abs(value)
                | Self::Pow { value, .. }
                | Self::OnDomain { value, .. } => pending.push((value, depth + 1)),
            }
        }
        Ok(())
    }

    pub(crate) fn evaluate(
        &self,
        mut operand: impl FnMut(VarId) -> Result<ObservationOperand>,
        limits: &ObservationNumberCodecLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ObservationNumber> {
        let mut pending = vec![(self, false)];
        let mut values = Vec::<ObservationNumber>::new();
        while let Some((node, finish)) = pending.pop() {
            work.control().checkpoint()?;
            pending.try_reserve(3).map_err(Error::from)?;
            values.try_reserve(1).map_err(Error::from)?;
            let value = if finish {
                let value = values.pop().ok_or(Error::InvalidEncoding)?;
                match node {
                    Self::Binary { op, .. } => values.pop().ok_or(Error::InvalidEncoding)?.apply(
                        *op,
                        &value,
                        limits.numbers,
                        work,
                    )?,
                    Self::Negate(_) => value.negate(limits.numbers, work)?,
                    Self::Abs(_) => value.abs(limits.numbers, work)?,
                    Self::Pow { exponent, .. } => value.pow(*exponent, limits.numbers, work)?,
                    Self::OnDomain { domain, .. } => {
                        value.on_domain(domain.value(), limits.numbers, work)?
                    }
                    _ => unreachable!("only operators schedule completion"),
                }
            } else {
                match node {
                    Self::Var(var) => {
                        let ObservationOperand::Number(value) = operand(*var)? else {
                            return Err(Error::InvalidEncoding.into());
                        };
                        value.value().validate(limits.numbers.numbers, work)?;
                        value
                    }
                    Self::Integer(var) => {
                        let ObservationOperand::Integer(value) = operand(*var)? else {
                            return Err(Error::InvalidEncoding.into());
                        };
                        ObservationNumber::literal(value, limits.numbers, work)?
                    }
                    Self::Component {
                        observation,
                        component,
                    } => match operand(*observation)? {
                        ObservationOperand::Probability(value) => {
                            ObservationNumber::probability(value, *component, limits.numbers, work)?
                        }
                        ObservationOperand::Expectation(value) => {
                            ObservationNumber::expectation(value, *component, limits.numbers, work)?
                        }
                        _ => return Err(Error::InvalidEncoding.into()),
                    },
                    Self::Literal(value) => {
                        ObservationNumber::literal(value.clone(), limits.numbers, work)?
                    }
                    Self::Imported(value) => {
                        ObservationNumberImport::from_bytes(value.bytes(), *limits, work)?
                            .value()
                            .clone()
                    }
                    Self::Binary { left, right, .. } => {
                        pending.push((node, true));
                        pending.push((right, false));
                        pending.push((left, false));
                        continue;
                    }
                    Self::Negate(value)
                    | Self::Abs(value)
                    | Self::Pow { value, .. }
                    | Self::OnDomain { value, .. } => {
                        pending.push((node, true));
                        pending.push((value, false));
                        continue;
                    }
                }
            };
            values.push(value);
        }
        let value = values.pop().ok_or(Error::InvalidEncoding)?;
        if !values.is_empty() {
            return Err(Error::Capacity(Capacity::ProgramNodes).into());
        }
        Ok(value)
    }
}

pub(crate) enum ObservationOperand {
    Predicate(crate::ObservationPredicate),
    Number(ObservationNumber),
    Integer(ExactRational),
    Probability(crate::ProbabilityAnswer),
    Expectation(crate::ExpectationAnswer),
}
