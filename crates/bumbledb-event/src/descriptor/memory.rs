//! BEBM v1 transports a possibility-memory recipe, never a claimed reachable
//! graph. Admission checks every source/role and rebuilds the complete memory.
#![allow(clippy::large_types_passed_by_value)]

use super::wire::{Reader, Writer};
use super::{Budget, DescriptorLimits, FibreDescriptor, MapDescriptor};
use crate::{
    BeliefLimits, BeliefMemory, Capacity, Control, CoordinateMap, Error, Event, EventPartition,
    ExactArithmetic, FibreProduct, ParameterSourceLimits, PartitionLimits, Result, Space,
    WorldRelation,
};

/// An action's original product coordinates and declared direction. Empty
/// regions still carry checked endpoint/environment meanings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeliefActionDescriptor {
    pub product: FibreDescriptor,
    pub region: Vec<u8>,
}

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
        control.checkpoint()?;
        let mut budget = Budget::new(limits);
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
            budget.item(0)?;
            let product = FibreDescriptor::capture(action.product(), &mut budget, control)?;
            let region = budget.event(action.region(), control)?;
            actions.try_reserve(1)?;
            actions.push(BeliefActionDescriptor { product, region });
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
        control.checkpoint()?;
        let mut budget = Budget::new(limits);
        budget.item(0)?;
        for bytes in std::iter::once(&self.source)
            .chain([&self.given])
            .chain(&self.observations)
        {
            control.checkpoint()?;
            budget.item(bytes.len())?;
        }
        for action in &self.actions {
            control.checkpoint()?;
            budget.item(0)?;
            action.product.preflight(&mut budget, control)?;
            budget.item(action.region.len())?;
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
        let source = full(&self.source, limits, work)?;
        let given = event(&self.given, limits, work)?;
        let mut cells = Vec::new();
        cells.try_reserve_exact(self.observations.len())?;
        for cell in &self.observations {
            cells.push(event(cell, limits, work)?);
        }
        let observations = EventPartition::on(&source.full(), &cells, limits.partitions, control)?;
        let mut actions = Vec::new();
        actions.try_reserve_exact(self.actions.len())?;
        for action in &self.actions {
            let left = map(&action.product.left, limits, work)?.certify_surjective(control)?;
            let right = map(&action.product.right, limits, work)?.certify_surjective(control)?;
            let width = left.map().source().dimensions() + right.map().source().dimensions();
            let order: Vec<_> = (0..width).collect();
            let product = FibreProduct::with_order_and_parameters(
                action.product.identity,
                &left,
                &right,
                &order,
                limits.descriptors.events,
                limits.parameters,
                work,
            )?;
            let product = if action.product.reversed {
                product.converse()
            } else {
                product
            };
            actions.push(WorldRelation::new(
                &product,
                &event(&action.region, limits, work)?,
                control,
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
        control.checkpoint()?;
        Ok(out.bytes)
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
        input.budget.item(0)?;
        if input.take(4)? != b"BEBM" {
            return Err(Error::InvalidEncoding);
        }
        let version = input.take(1)?[0];
        if version != 1 {
            return Err(Error::UnsupportedVersion(version));
        }
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
        if !input.bytes.is_empty() {
            return Err(Error::InvalidEncoding);
        }
        control.checkpoint()?;
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

fn event(
    bytes: &[u8],
    limits: BeliefDescriptorLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<Event> {
    Event::from_bytes_with_parameter_limits(
        bytes,
        None,
        limits.descriptors.events,
        limits.parameters,
        work,
    )
}
fn full(
    bytes: &[u8],
    limits: BeliefDescriptorLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<Space> {
    let value = event(bytes, limits, work)?;
    if !value.is_full() {
        return Err(Error::InvalidEncoding);
    }
    Ok(value.space())
}
fn map(
    data: &MapDescriptor,
    limits: BeliefDescriptorLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<CoordinateMap> {
    let source = full(&data.source, limits, work)?;
    let target = full(&data.target, limits, work)?;
    if data.readouts.len() != usize::from(target.dimensions()) {
        return Err(Error::MapArity);
    }
    let mut readouts = Vec::new();
    readouts.try_reserve_exact(data.readouts.len())?;
    for readout in &data.readouts {
        readouts.push(event(readout, limits, work)?);
    }
    CoordinateMap::new_with_parameters(&source, &target, &readouts, limits.parameters, work)
}
