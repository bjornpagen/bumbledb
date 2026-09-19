//! BEBA v1: named compilation of an exact memory recipe. No cached graph,
//! code assignment or strategy is trusted from the wire.
#![allow(clippy::large_types_passed_by_value)]
use super::wire::{Reader, Writer};
use super::{BeliefDescriptor, BeliefDescriptorLimits, Budget, DescriptorLimits};
use crate::{
    BeliefArena, BeliefSpaceIds, Capacity, Control, Error, ExactArithmetic, Result, SpaceId,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeliefArenaDescriptor {
    pub memory: BeliefDescriptor,
    pub identities: BeliefSpaceIds,
}

impl BeliefSpaceIds {
    pub(super) fn names(self) -> [SpaceId; 5] {
        [
            self.states,
            self.actions,
            self.environment,
            self.state_actions,
            self.transitions,
        ]
    }
}

impl BeliefArenaDescriptor {
    /// Capture the recipe and its five authored presentation names.
    /// # Errors
    /// Encoding, aggregate descriptor bounds, cancellation or allocation refusal.
    pub fn capture(
        arena: &BeliefArena,
        limits: DescriptorLimits,
        control: &dyn Control,
    ) -> Result<Self> {
        let identities = arena.identities();
        let mut budget = Budget::new(limits);
        names(&identities, &mut budget, control)?;
        let memory = BeliefDescriptor::capture_in(arena.memory(), &mut budget, control)?;
        Ok(Self { memory, identities })
    }

    fn preflight(&self, limits: DescriptorLimits, control: &dyn Control) -> Result<()> {
        let mut budget = Budget::new(limits);
        names(&self.identities, &mut budget, control)?;
        self.memory.preflight_in(&mut budget, control)
    }

    /// Reconstruct memory and compile the exact named code spaces and arena.
    /// # Errors
    /// Has memory admission's contract; empty memory/action rosters cannot compile.
    /// All new code/product owners use the supplied Event kernel limits.
    pub fn admit(
        &self,
        limits: BeliefDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<BeliefArena> {
        self.preflight(limits.descriptors, work.control())?;
        self.memory.admit(limits, work)?.arena_with_limits(
            self.identities,
            limits.descriptors.events,
            work.control(),
        )
    }

    /// Encode untrusted compilation data; no semantic certificate is implied.
    /// # Errors
    /// Extent, allocation or cancellation refusal.
    pub fn to_bytes(&self, limits: DescriptorLimits, control: &dyn Control) -> Result<Vec<u8>> {
        self.preflight(limits, control)?;
        let mut out = Writer {
            bytes: Vec::new(),
            limits,
            control,
        };
        out.put(b"BEBA\x01")?;
        for name in self.identities.names() {
            out.put(&name.0)?;
        }
        self.memory.write(&mut out)?;
        control.checkpoint()?;
        Ok(out.bytes)
    }

    /// Parse only. The body embeds the BEBM recipe grammar without its header;
    /// the entire object shares one nested byte/item allowance.
    /// # Errors
    /// Truncated/trailing input, unknown versions, capacities or cancellation.
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
        if input.take(4)? != b"BEBA" {
            return Err(Error::InvalidEncoding);
        }
        let version = input.take(1)?[0];
        if version != 1 {
            return Err(Error::UnsupportedVersion(version));
        }
        input.budget.item(0)?;
        let mut name = || {
            input.budget.item(32)?;
            input.identity()
        };
        let identities = BeliefSpaceIds {
            states: name()?,
            actions: name()?,
            environment: name()?,
            state_actions: name()?,
            transitions: name()?,
        };
        let memory = BeliefDescriptor::read(&mut input)?;
        if !input.bytes.is_empty() {
            return Err(Error::InvalidEncoding);
        }
        control.checkpoint()?;
        Ok(Self { memory, identities })
    }

    /// Parse and reconstruct the compiled controller's complete context.
    /// # Errors
    /// Has `from_bytes` and `admit`'s refusal contracts.
    pub fn import(
        bytes: &[u8],
        limits: BeliefDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<BeliefArena> {
        Self::from_bytes(bytes, limits.descriptors, work.control())?.admit(limits, work)
    }
}

fn names(ids: &BeliefSpaceIds, budget: &mut Budget, control: &dyn Control) -> Result<()> {
    control.checkpoint()?;
    budget.item(0)?;
    for _ in ids.names() {
        budget.item(32)?;
    }
    Ok(())
}
