//! BEAC v1 transports arenas and objective/policy recipes. Admission replays
//! the solver and checks selected choices; it never trusts a stored winner/rank.
#![allow(clippy::large_types_passed_by_value)]
use super::wire::{Reader, Writer};
use super::{Budget, DescriptorLimits, FibreDescriptor, RelationDescriptor, structural};
use crate::{
    ActionArena, Capacity, Control, Error, ExactArithmetic, FixedPointLimits,
    ParameterSourceLimits, PartitionLimits, ReachStrategy, Result, SafetyStrategy, WorldRelation,
};

/// A general fully observed arena. The outcome may differ from the state space
/// for one-step operations; iterative strategies require exact endoroles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionArenaDescriptor {
    pub actions: FibreDescriptor,
    pub transition: RelationDescriptor,
}

/// An untrusted executable recipe. `None` requests all permitted solver choices;
/// `Some` requests a restriction of those choices with complete winning-domain
/// coverage. Capturing a strategy always retains its actual selected policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionDescriptor {
    Arena(ActionArenaDescriptor),
    Reach {
        arena: ActionArenaDescriptor,
        goal: Vec<u8>,
        policy: Option<Vec<u8>>,
    },
    Safe {
        arena: ActionArenaDescriptor,
        invariant: Vec<u8>,
        policy: Option<Vec<u8>>,
    },
}

#[derive(Debug, Clone)]
pub enum AdmittedActionDescriptor {
    Arena(ActionArena),
    Reach(ReachStrategy),
    Safe(SafetyStrategy),
}

/// Transport, per-owner kernel, parameter, iteration and rank capacities. This
/// does not impose a total retained-allocator quota or certify hidden visibility.
#[derive(Debug, Clone, Copy, Default)]
pub struct ActionDescriptorLimits {
    pub descriptors: DescriptorLimits,
    pub parameters: ParameterSourceLimits,
    pub fixed_points: FixedPointLimits,
    pub partitions: PartitionLimits,
}

impl ActionArenaDescriptor {
    fn capture(value: &ActionArena, budget: &mut Budget, control: &dyn Control) -> Result<Self> {
        budget.item(0)?;
        Ok(Self {
            actions: FibreDescriptor::capture(value.actions(), budget, control)?,
            transition: RelationDescriptor::capture(value.transition(), budget, control)?,
        })
    }
    fn preflight(&self, budget: &mut Budget, control: &dyn Control) -> Result<()> {
        control.checkpoint()?;
        budget.item(0)?;
        self.actions.preflight(budget, control)?;
        self.transition.preflight(budget, control)
    }
    fn admit(
        &self,
        limits: ActionDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ActionArena> {
        let actions =
            structural::fibre(&self.actions, limits.descriptors, limits.parameters, work)?;
        let transition = structural::relation(
            &self.transition,
            limits.descriptors,
            limits.parameters,
            work,
        )?;
        ActionArena::new_with_parameters(&actions, &transition, limits.parameters, work)
    }
    fn write(&self, out: &mut Writer<'_>) -> Result<()> {
        out.fibre(&self.actions)?;
        out.fibre(&self.transition.product)?;
        out.blob(&self.transition.region)
    }
    fn read(input: &mut Reader<'_>) -> Result<Self> {
        input.budget.item(0)?;
        let actions = input.fibre()?;
        input.budget.item(0)?;
        let product = input.fibre()?;
        let region = input.blob()?;
        Ok(Self {
            actions,
            transition: RelationDescriptor { product, region },
        })
    }
}

type ObjectivePolicy<'a> = (&'a [u8], Option<&'a [u8]>);

impl ActionDescriptor {
    /// Capture owned canonical source/role, objective and selected-policy data.
    /// This preserves retained execution semantics, not discarded author syntax.
    /// # Errors
    /// Encoding, cumulative descriptor extent, allocation or cancellation refusal.
    pub fn capture(
        value: &AdmittedActionDescriptor,
        limits: DescriptorLimits,
        control: &dyn Control,
    ) -> Result<Self> {
        control.checkpoint()?;
        let mut budget = Budget::new(limits);
        budget.item(0)?;
        Ok(match value {
            AdmittedActionDescriptor::Arena(arena) => {
                Self::Arena(ActionArenaDescriptor::capture(arena, &mut budget, control)?)
            }
            AdmittedActionDescriptor::Reach(strategy) => Self::Reach {
                arena: ActionArenaDescriptor::capture(strategy.arena(), &mut budget, control)?,
                goal: budget.event(strategy.goal(), control)?,
                policy: Some(budget.event(strategy.policy().region(), control)?),
            },
            AdmittedActionDescriptor::Safe(strategy) => Self::Safe {
                arena: ActionArenaDescriptor::capture(strategy.arena(), &mut budget, control)?,
                invariant: budget.event(strategy.invariant(), control)?,
                policy: Some(budget.event(strategy.policy().region(), control)?),
            },
        })
    }

    fn parts(&self) -> (&ActionArenaDescriptor, Option<ObjectivePolicy<'_>>) {
        match self {
            Self::Arena(arena) => (arena, None),
            Self::Reach {
                arena,
                goal,
                policy,
            } => (arena, Some((goal, policy.as_deref()))),
            Self::Safe {
                arena,
                invariant,
                policy,
            } => (arena, Some((invariant, policy.as_deref()))),
        }
    }
    fn preflight(&self, limits: DescriptorLimits, control: &dyn Control) -> Result<()> {
        control.checkpoint()?;
        let mut budget = Budget::new(limits);
        budget.item(0)?;
        let (arena, strategy) = self.parts();
        arena.preflight(&mut budget, control)?;
        if let Some((objective, policy)) = strategy {
            budget.item(objective.len())?;
            if let Some(policy) = policy {
                budget.item(policy.len())?;
            }
        }
        Ok(())
    }

    /// Rebuild all source/map/product certificates, solve the objective and
    /// validate the retained policy with the native strategy checker. One exact
    /// arithmetic counter covers source replay, products, solving and restriction.
    /// # Errors
    /// Malformed contexts/roles, unsafe or incomplete policies, arithmetic,
    /// iteration/rank/descriptor/kernel capacities or cancellation.
    pub fn admit(
        &self,
        limits: ActionDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<AdmittedActionDescriptor> {
        self.preflight(limits.descriptors, work.control())?;
        let (data, strategy) = self.parts();
        let arena = data.admit(limits, work)?;
        let Some((objective, policy)) = strategy else {
            return Ok(AdmittedActionDescriptor::Arena(arena));
        };
        let objective = structural::event(objective, limits.descriptors, limits.parameters, work)?;
        // Admit authored policy data even when the winning region is empty.
        let policy = policy
            .map(|bytes| {
                WorldRelation::new(
                    arena.actions(),
                    &structural::event(bytes, limits.descriptors, limits.parameters, work)?,
                    work.control(),
                )
            })
            .transpose()?;
        match self {
            Self::Reach { .. } => {
                let strategy = arena.winning_reach_with_parameters(
                    &objective,
                    limits.fixed_points,
                    limits.partitions,
                    limits.parameters,
                    work,
                )?;
                Ok(AdmittedActionDescriptor::Reach(match policy {
                    Some(policy) => {
                        strategy.with_policy_and_parameters(&policy, limits.parameters, work)?
                    }
                    None => strategy,
                }))
            }
            Self::Safe { .. } => {
                let strategy = arena.winning_safe_with_parameters(
                    &objective,
                    limits.fixed_points,
                    limits.parameters,
                    work,
                )?;
                Ok(AdmittedActionDescriptor::Safe(match policy {
                    Some(policy) => {
                        strategy.with_policy_and_parameters(&policy, limits.parameters, work)?
                    }
                    None => strategy,
                }))
            }
            Self::Arena(_) => unreachable!(),
        }
    }

    /// Encode plain data; encoding is not semantic admission.
    /// # Errors
    /// Transport extent, allocation or cancellation refusal.
    pub fn to_bytes(&self, limits: DescriptorLimits, control: &dyn Control) -> Result<Vec<u8>> {
        self.preflight(limits, control)?;
        let mut out = Writer {
            bytes: Vec::new(),
            limits,
            control,
        };
        out.put(b"BEAC\x01")?;
        out.put(&[match self {
            Self::Arena(_) => 0,
            Self::Reach { .. } => 1,
            Self::Safe { .. } => 2,
        }])?;
        let (arena, strategy) = self.parts();
        arena.write(&mut out)?;
        if let Some((objective, policy)) = strategy {
            out.blob(objective)?;
            out.put(&[u8::from(policy.is_some())])?;
            if let Some(policy) = policy {
                out.blob(policy)?;
            }
        }
        control.checkpoint()?;
        Ok(out.bytes)
    }

    /// Parse the fixed-depth BEAC grammar into untrusted data.
    /// # Errors
    /// Truncation, unknown tags/versions, trailing bytes, extent or cancellation.
    pub fn from_bytes(
        bytes: &[u8],
        limits: DescriptorLimits,
        control: &dyn Control,
    ) -> Result<Self> {
        control.checkpoint()?;
        if bytes.len() > limits.bytes {
            return Err(Error::Capacity(Capacity::DescriptorBytes));
        }
        let mut input = Reader {
            bytes,
            budget: Budget::new(limits),
            control,
        };
        input.budget.item(0)?;
        if input.take(4)? != b"BEAC" {
            return Err(Error::InvalidEncoding);
        }
        let version = input.take(1)?[0];
        if version != 1 {
            return Err(Error::UnsupportedVersion(version));
        }
        let tag = input.take(1)?[0];
        if tag > 2 {
            return Err(Error::InvalidEncoding);
        }
        let arena = ActionArenaDescriptor::read(&mut input)?;
        let result = if tag == 0 {
            Self::Arena(arena)
        } else {
            let objective = input.blob()?;
            let policy = match input.take(1)?[0] {
                0 => None,
                1 => Some(input.blob()?),
                _ => return Err(Error::InvalidEncoding),
            };
            if tag == 1 {
                Self::Reach {
                    arena,
                    goal: objective,
                    policy,
                }
            } else {
                Self::Safe {
                    arena,
                    invariant: objective,
                    policy,
                }
            }
        };
        if !input.bytes.is_empty() {
            return Err(Error::InvalidEncoding);
        }
        control.checkpoint()?;
        Ok(result)
    }

    /// Parse and reconstruct the complete checked arena or strategy.
    /// # Errors
    /// Has `from_bytes` and `admit`'s refusal contracts; no partial result escapes.
    pub fn import(
        bytes: &[u8],
        limits: ActionDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<AdmittedActionDescriptor> {
        Self::from_bytes(bytes, limits.descriptors, work.control())?.admit(limits, work)
    }
}
