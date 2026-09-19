//! Shared finite/family admission. Every nested source, map and product uses
//! the same exact-arithmetic counter; no default allowance is restarted here.
use super::{Budget, DescriptorLimits, FibreDescriptor, MapDescriptor};
use crate::{
    Control, CoordinateMap, Error, Event, ExactArithmetic, FibreProduct, ParameterSourceLimits,
    Result, Space, WorldRelation,
};

/// A relation's original product coordinates and declared direction. Empty
/// regions still carry checked endpoint/environment meanings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationDescriptor {
    pub product: FibreDescriptor,
    pub region: Vec<u8>,
}

impl RelationDescriptor {
    pub(super) fn capture(
        relation: &WorldRelation,
        budget: &mut Budget,
        control: &dyn Control,
    ) -> Result<Self> {
        budget.item(0)?;
        Ok(Self {
            product: FibreDescriptor::capture(relation.product(), budget, control)?,
            region: budget.event(relation.region(), control)?,
        })
    }

    pub(super) fn preflight(&self, budget: &mut Budget, control: &dyn Control) -> Result<()> {
        control.checkpoint()?;
        budget.item(0)?;
        self.product.preflight(budget, control)?;
        budget.item(self.region.len())
    }
}

pub(super) fn event(
    bytes: &[u8],
    limits: DescriptorLimits,
    parameters: ParameterSourceLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<Event> {
    Event::from_bytes_with_parameter_limits(bytes, None, limits.events, parameters, work)
}

pub(super) fn full(
    bytes: &[u8],
    limits: DescriptorLimits,
    parameters: ParameterSourceLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<Space> {
    let value = event(bytes, limits, parameters, work)?;
    if !value.is_full() {
        return Err(Error::InvalidEncoding);
    }
    Ok(value.space())
}

pub(super) fn map(
    data: &MapDescriptor,
    limits: DescriptorLimits,
    parameters: ParameterSourceLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<CoordinateMap> {
    let source = full(&data.source, limits, parameters, work)?;
    let target = full(&data.target, limits, parameters, work)?;
    if data.readouts.len() != usize::from(target.dimensions()) {
        return Err(Error::MapArity);
    }
    let mut readouts = Vec::new();
    readouts.try_reserve_exact(data.readouts.len())?;
    for readout in &data.readouts {
        readouts.push(event(readout, limits, parameters, work)?);
    }
    CoordinateMap::new_with_parameters(&source, &target, &readouts, parameters, work)
}

pub(super) fn fibre(
    data: &FibreDescriptor,
    limits: DescriptorLimits,
    parameters: ParameterSourceLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<FibreProduct> {
    let control = work.control();
    let left = map(&data.left, limits, parameters, work)?.certify_surjective(control)?;
    let right = map(&data.right, limits, parameters, work)?.certify_surjective(control)?;
    let width = left.map().source().dimensions() + right.map().source().dimensions();
    let order: Vec<_> = (0..width).collect();
    let product = FibreProduct::with_order_and_parameters(
        data.identity,
        &left,
        &right,
        &order,
        limits.events,
        parameters,
        work,
    )?;
    Ok(if data.reversed {
        product.converse()
    } else {
        product
    })
}

pub(super) fn relation(
    data: &RelationDescriptor,
    limits: DescriptorLimits,
    parameters: ParameterSourceLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<WorldRelation> {
    let product = fibre(&data.product, limits, parameters, work)?;
    WorldRelation::new(
        &product,
        &event(&data.region, limits, parameters, work)?,
        work.control(),
    )
}
