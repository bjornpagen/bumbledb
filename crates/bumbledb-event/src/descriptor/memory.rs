//! BEBM v1 transports a possibility-memory recipe, never a claimed reachable
//! graph. Admission checks every source/role and rebuilds the complete memory.
#![allow(clippy::large_types_passed_by_value)]

use super::wire::{Reader, Writer};
use super::{Budget, DescriptorLimits, RelationDescriptor, structural};
use crate::{
    BeliefLimits, BeliefMemory, Capacity, Control, Error, EventPartition, ExactArithmetic,
    ParameterSourceLimits, PartitionLimits, Result,
};

/// Compatibility name for a relation in an indexed memory action roster.
pub type BeliefActionDescriptor = RelationDescriptor;

/// Untrusted recipe with full source, initial evidence and authored indexed
/// observation/action rosters. Empty observations retain their positions.
/// There are no serialized state indices, edges, winning sets or policy claims.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeliefDescriptor {
    pub source: Vec<u8>,
    pub given: Vec<u8>,
    pub observations: Vec<Vec<u8>>,
    pub actions: Vec<BeliefActionDescriptor>,
}

/// Cumulative transport bounds, parameter admission, partition and graph
/// capacities. These are not an aggregate retained-allocator quota.
#[derive(Debug, Clone, Copy, Default)]
pub struct BeliefDescriptorLimits {
    pub descriptors: DescriptorLimits,
    pub parameters: ParameterSourceLimits,
    pub partitions: PartitionLimits,
    pub beliefs: BeliefLimits,
}

impl BeliefDescriptor {
    /// Capture the retained normalized action presentation and canonical Events.
    /// This does not recover discarded input syntax or provider provenance.
    /// # Errors
    /// Encoding, descriptor capacities, allocation or cancellation refusal.
    pub fn capture(
        memory: &BeliefMemory,
        limits: DescriptorLimits,
        control: &dyn Control,
    ) -> Result<Self> {
        Self::capture_in(memory, &mut Budget::new(limits), control)
    }

    pub(super) fn capture_in(
        memory: &BeliefMemory,
        budget: &mut Budget,
        control: &dyn Control,
    ) -> Result<Self> {
        control.checkpoint()?;
        budget.item(0)?;
        let source = budget.event(memory.observations().parent(), control)?;
        let given = budget.event(memory.given(), control)?;
        let mut observations = Vec::new();
        for cell in memory.observations().cells() {
            let cell = budget.event(cell, control)?;
            observations.try_reserve(1)?;
            observations.push(cell);
        }
        let mut actions = Vec::new();
        for action in memory.actions() {
            let data = RelationDescriptor::capture(action, budget, control)?;
            actions.try_reserve(1)?;
            actions.push(data);
        }
        control.checkpoint()?;
        Ok(Self {
            source,
            given,
            observations,
            actions,
        })
    }

    fn preflight(&self, limits: DescriptorLimits, control: &dyn Control) -> Result<()> {
        self.preflight_in(&mut Budget::new(limits), control)
    }

    pub(super) fn preflight_in(&self, budget: &mut Budget, control: &dyn Control) -> Result<()> {
        control.checkpoint()?;
        budget.item(0)?;
        for bytes in std::iter::once(&self.source)
            .chain([&self.given])
            .chain(&self.observations)
        {
            control.checkpoint()?;
            budget.item(bytes.len())?;
        }
        for action in &self.actions {
            action.preflight(budget, control)?;
        }
        Ok(())
    }

    /// Admit all source markers, readouts, complete products, role agreement and
    /// full observation coverage before building reachable possibility memory.
    /// One arithmetic counter covers every nested source and action normalization.
    /// # Errors
    /// Malformed Events, context/partition/role violations, capacities or cancellation.
    pub fn admit(
        &self,
        limits: BeliefDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<BeliefMemory> {
        let control = work.control();
        self.preflight(limits.descriptors, control)?;
        if self.observations.len() > limits.partitions.cells {
            return Err(Error::Capacity(Capacity::PartitionCells));
        }
        let source = structural::full(&self.source, limits.descriptors, limits.parameters, work)?;
        let given = structural::event(&self.given, limits.descriptors, limits.parameters, work)?;
        let mut cells = Vec::new();
        cells.try_reserve_exact(self.observations.len())?;
        for cell in &self.observations {
            cells.push(structural::event(
                cell,
                limits.descriptors,
                limits.parameters,
                work,
            )?);
        }
        let observations = EventPartition::on(&source.full(), &cells, limits.partitions, control)?;
        let mut actions = Vec::new();
        actions.try_reserve_exact(self.actions.len())?;
        for action in &self.actions {
            actions.push(structural::relation(
                action,
                limits.descriptors,
                limits.parameters,
                work,
            )?);
        }
        BeliefMemory::new_with_parameters(
            &actions,
            &observations,
            &given,
            limits.beliefs,
            limits.parameters,
            work,
        )
    }

    /// Encode a recipe. Encoding does not establish its mathematical validity.
    /// # Errors
    /// Descriptor extent, allocation or cancellation refusal.
    pub fn to_bytes(&self, limits: DescriptorLimits, control: &dyn Control) -> Result<Vec<u8>> {
        self.preflight(limits, control)?;
        let mut out = Writer {
            bytes: Vec::new(),
            limits,
            control,
        };
        out.put(b"BEBM\x01")?;
        self.write(&mut out)?;
        control.checkpoint()?;
        Ok(out.bytes)
    }

    pub(super) fn write(&self, out: &mut Writer<'_>) -> Result<()> {
        out.blob(&self.source)?;
        out.blob(&self.given)?;
        out.number(self.observations.len())?;
        for cell in &self.observations {
            out.blob(cell)?;
        }
        out.number(self.actions.len())?;
        for action in &self.actions {
            out.fibre(&action.product)?;
            out.blob(&action.region)?;
        }
        Ok(())
    }

    /// Parse syntax into untrusted data; use `import` to reconstruct memory.
    /// # Errors
    /// Unknown version, malformed/trailing bytes, capacities or cancellation.
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
        if input.take(4)? != b"BEBM" {
            return Err(Error::InvalidEncoding);
        }
        let version = input.take(1)?[0];
        if version != 1 {
            return Err(Error::UnsupportedVersion(version));
        }
        let result = Self::read(&mut input)?;
        if !input.bytes.is_empty() {
            return Err(Error::InvalidEncoding);
        }
        control.checkpoint()?;
        Ok(result)
    }

    pub(super) fn read(input: &mut Reader<'_>) -> Result<Self> {
        input.budget.item(0)?;
        let source = input.blob()?;
        let given = input.blob()?;
        let count = input.count()?;
        let mut observations = Vec::new();
        observations.try_reserve_exact(count)?;
        for _ in 0..count {
            observations.push(input.blob()?);
        }
        let count = input.count()?;
        let mut actions = Vec::new();
        actions.try_reserve_exact(count)?;
        for _ in 0..count {
            input.budget.item(0)?;
            let product = input.fibre()?;
            let region = input.blob()?;
            actions.push(BeliefActionDescriptor { product, region });
        }
        Ok(Self {
            source,
            given,
            observations,
            actions,
        })
    }

    /// Parse and reconstruct the complete reachable graph through checked constructors.
    /// # Errors
    /// Has `from_bytes` and `admit`'s refusal contracts. No partial graph escapes.
    pub fn import(
        bytes: &[u8],
        limits: BeliefDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<BeliefMemory> {
        Self::from_bytes(bytes, limits.descriptors, work.control())?.admit(limits, work)
    }
}
