//! Explicit source-bound interpretation of numerical truth. A captured plan
//! names its original source and, optionally, a new deterministic presentation.
//! Query construction never invents a source identity, prior or conditioning.
use crate::event::{Error, Event, ExactArithmetic, ParameterRefinement, Space, SpaceId};
use crate::number_expr::ObservationInputKind;
use crate::{
    NumberExprError, ObservationNumberCodecLimits, ObservationPredicate, PredicateEvents,
    PredicateExpr, Result, VarId,
};
use std::sync::Arc;

#[derive(Debug)]
struct Plan {
    source: Space,
    bytes: Box<[u8]>,
    refinement: Option<SpaceId>,
}

/// A checked full source and an explicitly named optional guard extension.
/// Identity is the full canonical source marker plus the requested identity.
#[derive(Debug, Clone)]
pub struct PredicateGuardPlan(Arc<Plan>);
impl PartialEq for PredicateGuardPlan {
    fn eq(&self, other: &Self) -> bool {
        self.0.refinement == other.0.refinement && self.0.bytes == other.0.bytes
    }
}
impl Eq for PredicateGuardPlan {}
impl PredicateGuardPlan {
    /// Capture and independently admit the original source, including its law.
    /// A refinement requires a parameter source, even for a constant predicate.
    /// # Errors
    /// Source/codec/arithmetic bounds, missing parameters or cancellation.
    pub fn capture(
        source: &Space,
        refinement: Option<SpaceId>,
        limits: ObservationNumberCodecLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let bytes = source.full().to_bytes(work.control())?;
        Self::from_parts(&bytes, refinement, limits, work)
    }
    /// The source bytes must be a full canonical BEVT marker. Plain envelopes
    /// and valid proper subregions do not certify a guard interpretation plan.
    /// # Errors
    /// Noncanonical/nonfull source, missing parameters, resource bounds or cancellation.
    pub fn from_parts(
        bytes: &[u8],
        refinement: Option<SpaceId>,
        limits: ObservationNumberCodecLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        work.control().checkpoint()?;
        if bytes.len() > limits.sources.descriptors.bytes {
            return Err(Error::Capacity(crate::event::Capacity::DescriptorBytes).into());
        }
        let full = Event::from_bytes_with_parameter_limits(
            bytes,
            None,
            limits.sources.descriptors.events,
            limits.sources.parameters,
            work,
        )?;
        if !full.is_full() || full.to_bytes(work.control())? != bytes {
            return Err(Error::InvalidEncoding.into());
        }
        let source = full.space();
        if refinement.is_some() && source.parameter_domain().is_none() {
            return Err(Error::MissingParameter.into());
        }
        let mut owned = Vec::new();
        owned.try_reserve_exact(bytes.len()).map_err(Error::from)?;
        owned.extend_from_slice(bytes);
        Ok(Self(Arc::new(Plan {
            source,
            bytes: owned.into_boxed_slice(),
            refinement,
        })))
    }
    #[must_use]
    pub fn source(&self) -> &Space {
        &self.0.source
    }
    #[must_use]
    pub fn source_bytes(&self) -> &[u8] {
        &self.0.bytes
    }
    #[must_use]
    pub fn refinement_identity(&self) -> Option<SpaceId> {
        self.0.refinement
    }

    pub(crate) fn interpret(
        &self,
        predicate: &ObservationPredicate,
        companions: &[ObservationPredicate],
        limits: &ObservationNumberCodecLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Interpretation> {
        let (cases, refinement) = if let Some(identity) = self.0.refinement {
            let value = if companions.is_empty() {
                predicate.refine(
                    identity,
                    &self.0.source,
                    limits.sources.descriptors.events,
                    limits.sources.parameters,
                    work,
                )?
            } else {
                let mut roster = Vec::new();
                roster
                    .try_reserve_exact(companions.len() + 1)
                    .map_err(Error::from)?;
                roster.push(predicate.clone());
                roster.extend_from_slice(companions);
                crate::PredicateRefinement::common(
                    identity,
                    &self.0.source,
                    &roster,
                    limits.sources.descriptors.events,
                    limits.sources.parameters,
                    work,
                )?
                .into_iter()
                .next()
                .ok_or(Error::NoParameterSources)?
            };
            (value.events().clone(), Some(value.refinement().clone()))
        } else {
            for companion in companions {
                companion.events(&self.0.source, limits.sources.parameters, work)?;
            }
            (
                predicate.events(&self.0.source, limits.sources.parameters, work)?,
                None,
            )
        };
        Ok(Interpretation { cases, refinement })
    }
}

/// Guard cases are ordinary Events. Transport is explicit; exact descent
/// refuses any Event that depends essentially on an added guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardOp {
    Holds,
    Fails,
    Undefined,
    Lift(VarId),
    Descend(VarId),
}
impl GuardOp {
    pub(crate) fn input(self) -> Option<VarId> {
        match self {
            Self::Lift(v) | Self::Descend(v) => Some(v),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardExpr {
    pub plan: PredicateGuardPlan,
    pub predicate: PredicateExpr,
    /// Additional predicates resolved in the same named presentation. A
    /// nonempty roster requests canonical common refinement, even if repeated.
    pub companions: Vec<PredicateExpr>,
    pub operation: GuardOp,
}
impl GuardExpr {
    pub fn variables(&self) -> impl Iterator<Item = VarId> {
        self.predicate
            .variables()
            .chain(self.companions.iter().flat_map(PredicateExpr::variables))
            .chain(self.operation.input())
    }
    pub(crate) fn inputs(
        &self,
    ) -> std::result::Result<Vec<(VarId, ObservationInputKind)>, NumberExprError> {
        let mut inputs = Vec::new();
        let mut nodes = 1;
        self.predicate.collect_inputs(2, &mut nodes, &mut inputs)?;
        for companion in &self.companions {
            companion.collect_inputs(2, &mut nodes, &mut inputs)?;
        }
        if let Some(var) = self.operation.input() {
            inputs.push((var, ObservationInputKind::Event));
        }
        Ok(inputs)
    }
}

pub(crate) struct Interpretation {
    cases: PredicateEvents,
    refinement: Option<ParameterRefinement>,
}
impl Interpretation {
    pub(crate) fn expected_input(&self, op: GuardOp) -> Option<&Space> {
        match op {
            GuardOp::Lift(_) => Some(
                self.refinement
                    .as_ref()
                    .map_or(self.cases.source(), ParameterRefinement::source),
            ),
            GuardOp::Descend(_) => Some(self.cases.source()),
            _ => None,
        }
    }
    pub(crate) fn event(
        &self,
        op: GuardOp,
        input: Option<&Event>,
        control: &dyn crate::event::Control,
    ) -> Result<Event> {
        Ok(match op {
            GuardOp::Holds => self.cases.holds().clone(),
            GuardOp::Fails => self.cases.fails().clone(),
            GuardOp::Undefined => self.cases.undefined().clone(),
            GuardOp::Lift(_) => match &self.refinement {
                Some(refinement) => refinement.lift(input.ok_or(Error::UnknownKey)?, control)?,
                None => input
                    .ok_or(Error::UnknownKey)?
                    .align_to(self.cases.source(), control)?,
            },
            GuardOp::Descend(_) => match &self.refinement {
                Some(refinement) => refinement.descend(input.ok_or(Error::UnknownKey)?, control)?,
                None => input
                    .ok_or(Error::UnknownKey)?
                    .align_to(self.cases.source(), control)?,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{ArithmeticLimits, Capacity, PolynomialSigns};

    #[test]
    fn plan_admission_is_owned_full_bounded_and_cancellable() {
        let work = crate::WorkContext::new();
        let mut arithmetic = ExactArithmetic::new(ArithmeticLimits::default(), &work);
        let source = Space::new(SpaceId([225; 32]), 1, &()).unwrap();
        let limits = ObservationNumberCodecLimits::default();
        let mut bytes = source.full().to_bytes(&()).unwrap();
        let plan = PredicateGuardPlan::from_parts(&bytes, None, limits, &mut arithmetic).unwrap();
        let captured = PredicateGuardPlan::capture(&source, None, limits, &mut arithmetic).unwrap();
        assert_eq!(plan, captured);
        assert_ne!(plan.source().full(), source.full(), "separate owners");
        bytes.fill(0);
        assert!(plan.source().full().is_full());
        let mut small = limits;
        small.sources.descriptors.bytes = plan.source_bytes().len() - 1;
        assert!(matches!(
            PredicateGuardPlan::from_parts(plan.source_bytes(), None, small, &mut arithmetic),
            Err(crate::Error::Event(Error::Capacity(
                Capacity::DescriptorBytes
            )))
        ));
        work.cancel();
        assert!(
            PredicateGuardPlan::from_parts(plan.source_bytes(), None, limits, &mut arithmetic)
                .is_err()
        );
    }

    #[test]
    fn guard_node_shares_the_predicate_and_number_shape_bound() {
        let plan = PredicateGuardPlan::capture(
            &Space::new(SpaceId([226; 32]), 0, &()).unwrap(),
            None,
            ObservationNumberCodecLimits::default(),
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &()),
        )
        .unwrap();
        let guard = |predicate| GuardExpr {
            plan: plan.clone(),
            predicate,
            companions: Vec::new(),
            operation: GuardOp::Holds,
        };
        let mut number = crate::NumberExpr::Integer(VarId(0));
        for _ in 1..255 {
            number = crate::NumberExpr::Negate(Box::new(number));
        }
        let predicate = PredicateExpr::Sign {
            number,
            signs: PolynomialSigns::POSITIVE,
        };
        assert!(predicate.inputs().is_ok());
        assert_eq!(
            guard(predicate).inputs().unwrap_err(),
            NumberExprError::TooDeep
        );
        let mut number = crate::NumberExpr::Integer(VarId(0));
        for _ in 0..15 {
            number = crate::NumberExpr::Binary {
                op: crate::event::NumberOp::Add,
                left: Box::new(number.clone()),
                right: Box::new(number),
            };
        }
        let predicate = PredicateExpr::Sign {
            number,
            signs: PolynomialSigns::POSITIVE,
        };
        assert!(predicate.inputs().is_ok());
        assert_eq!(
            guard(predicate).inputs().unwrap_err(),
            NumberExprError::TooLarge
        );
        let mut common = guard(PredicateExpr::Var(VarId(0)));
        common.companions = vec![PredicateExpr::Var(VarId(1)); 65_534];
        assert_eq!(common.inputs().unwrap().len(), 65_535);
        common.companions.push(PredicateExpr::Var(VarId(2)));
        assert_eq!(common.inputs().unwrap_err(), NumberExprError::TooLarge);
    }
}
