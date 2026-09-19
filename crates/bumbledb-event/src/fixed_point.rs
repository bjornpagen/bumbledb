//! Finite, sealed iteration. The structural world bound is separate from the
//! operational budget. No partial approximant is returned as a fixed point.
use crate::product::require_same_map;
use crate::{
    BoolOp4, Capacity, Control, Error, Event, EventPartition, EventProgram, EventProgramBuilder,
    MapOp, ModalOp, PartitionLimits, RelationalProduct, Result, Space, Variance, WorldRelation,
};

/// A sealed finite Event presentation with its exact number of legal cells.
/// A parameter cell may contain infinitely many worlds. Every Event in this
/// presentation is constant on each cell; no new guards appear during iteration.
/// Decision-diagram nodes and completed-code aliases are not cells.
#[derive(Debug, Clone)]
pub struct FiniteCarrier {
    space: Space,
    atoms: u64,
}

impl FiniteCarrier {
    /// Seal the existing presentation and count original legal cells.
    /// # Errors
    /// Refuses cancellation or count-kernel resource exhaustion.
    pub fn new(space: &Space, control: &dyn Control) -> Result<Self> {
        let atoms = space.full().atom_count(control)?;
        if atoms == 0 || atoms > (1u64 << space.dimensions()) {
            return Err(Error::FixedPointInvariant);
        }
        Ok(Self {
            space: space.clone(),
            atoms,
        })
    }
    #[must_use]
    pub fn space(&self) -> &Space {
        &self.space
    }
    #[must_use]
    pub fn atoms(&self) -> u64 {
        self.atoms
    }
}

/// Operational limits. Exceeding either refuses; it does not shrink the carrier
/// or turn an approximation into an exact result. Space kernel limits still apply.
#[derive(Debug, Clone, Copy)]
pub struct FixedPointLimits {
    pub iterations: u64,
    pub program_steps: u64,
}
impl Default for FixedPointLimits {
    fn default() -> Self {
        Self {
            iterations: 1_000_000,
            program_steps: 20_000_000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FixedPointResult {
    event: Event,
    iterations: u64,
    program_steps: u64,
}

/// Exact first-entry layers of bottom iteration. Cell i first appears in
/// application i+1. Cells are nonempty and disjoint, covering the final result;
/// an immediately empty fixed point has no layers. These are structural ranks,
/// not probabilities or a promise that an arbitrary action follows progress.
#[derive(Debug, Clone)]
pub struct LayeredFixedPointResult {
    result: FixedPointResult,
    layers: EventPartition,
}

impl LayeredFixedPointResult {
    #[must_use]
    pub fn result(&self) -> &FixedPointResult {
        &self.result
    }
    #[must_use]
    pub fn layers(&self) -> &EventPartition {
        &self.layers
    }
}
impl FixedPointResult {
    #[must_use]
    pub fn event(&self) -> &Event {
        &self.event
    }
    #[must_use]
    pub fn into_event(self) -> Event {
        self.event
    }
    /// Includes the application that detected equality.
    #[must_use]
    pub fn iterations(&self) -> u64 {
        self.iterations
    }
    #[must_use]
    pub fn program_steps(&self) -> u64 {
        self.program_steps
    }
}

/// A monotone endoprogram on one finite, stable native carrier. Operators are
/// sealed algebra instructions, never arbitrary closures or model callbacks.
#[derive(Debug, Clone)]
pub struct FixedPointProgram {
    program: EventProgram,
    carrier: FiniteCarrier,
}

impl EventProgram {
    /// Validate endocontext, positive variance and a finite logical-atom bound.
    /// # Errors
    /// Refuses differing input/output contexts, unproved monotonicity or resources.
    pub fn fixed_points(&self, control: &dyn Control) -> Result<FixedPointProgram> {
        self.output().full().align_to(self.input(), control)?;
        if !matches!(
            self.variance(),
            Variance::Independent | Variance::Increasing
        ) {
            return Err(Error::NonMonotoneProgram);
        }
        Ok(FixedPointProgram {
            program: self.clone(),
            carrier: FiniteCarrier::new(self.input(), control)?,
        })
    }
}

impl FixedPointProgram {
    #[must_use]
    pub fn program(&self) -> &EventProgram {
        &self.program
    }
    #[must_use]
    pub fn carrier(&self) -> &FiniteCarrier {
        &self.carrier
    }

    /// Least fixed point, starting at empty and stopping at canonical equality.
    /// # Errors
    /// Refuses resource limits/cancellation; never returns a truncated result.
    pub fn least(
        &self,
        limits: FixedPointLimits,
        control: &dyn Control,
    ) -> Result<FixedPointResult> {
        self.run(false, limits, None, control)
            .map(|(result, _)| result)
    }

    /// Least fixed point with a retained partition of first-entry layers.
    /// The extra cell budget bounds retained ranks; hitting it refuses the
    /// entire result, even if an unranked run could finish within its own budget.
    /// # Errors
    /// Has `least`'s contract, plus explicit layer/allocation capacity.
    pub fn least_with_layers(
        &self,
        limits: FixedPointLimits,
        layer_limits: PartitionLimits,
        control: &dyn Control,
    ) -> Result<LayeredFixedPointResult> {
        let (result, layers) = self.run(false, limits, Some(layer_limits), control)?;
        let layers = EventPartition::on(result.event(), &layers, layer_limits, control)?;
        Ok(LayeredFixedPointResult { result, layers })
    }

    /// Greatest fixed point, starting at full with the same finite bound.
    /// # Errors
    /// Has `least`'s explicit refusal contract.
    pub fn greatest(
        &self,
        limits: FixedPointLimits,
        control: &dyn Control,
    ) -> Result<FixedPointResult> {
        self.run(true, limits, None, control)
            .map(|(result, _)| result)
    }

    fn run(
        &self,
        greatest: bool,
        limits: FixedPointLimits,
        layer_limits: Option<PartitionLimits>,
        control: &dyn Control,
    ) -> Result<(FixedPointResult, Vec<Event>)> {
        control.checkpoint()?;
        let space = self.carrier.space();
        let mut value = if greatest {
            space.full()
        } else {
            space.empty()
        };
        let mut steps = 0;
        let mut layers = Vec::new();
        // Space has at most 2^62 legal codes, so the detection application fits.
        for iteration in 1..=self.carrier.atoms() + 1 {
            control.checkpoint()?;
            if iteration > limits.iterations {
                return Err(Error::Capacity(Capacity::FixedPointIterations));
            }
            let next = self
                .program
                .evaluate_counted(&value, &mut steps, limits.program_steps, control)?
                .align_to(space, control)?;
            if next == value {
                return Ok((
                    FixedPointResult {
                        event: next,
                        iterations: iteration,
                        program_steps: steps,
                    },
                    layers,
                ));
            }
            let ordered = if greatest {
                next.signature(&value, control)?.included()
            } else {
                value.signature(&next, control)?.included()
            };
            if !ordered {
                return Err(Error::FixedPointInvariant);
            }
            if let Some(layer_limits) = layer_limits {
                if layers.len() >= layer_limits.cells {
                    return Err(Error::Capacity(Capacity::PartitionCells));
                }
                layers.try_reserve(1)?;
                layers.push(next.apply(BoolOp4::DIFFERENCE, &value, control)?);
            }
            value = next;
        }
        Err(Error::FixedPointInvariant)
    }
}

impl WorldRelation {
    fn predicate_program(
        &self,
        event: &Event,
        mode: ModalOp,
        connective: BoolOp4,
        control: &dyn Control,
    ) -> Result<FixedPointProgram> {
        require_same_map(
            self.product().left_environment().map(),
            self.product().right_environment().map(),
            control,
        )?;
        let mut builder = EventProgramBuilder::new(self.input(), control)?;
        let input = builder.input();
        let goal = builder.constant(event, control)?;
        let step = builder.modal(mode, self, &input, control)?;
        let root = builder.apply(connective, &goal, &step, control)?;
        builder.finish(&root, control)?.fixed_points(control)
    }

    /// Some finite allowed path reaches the event, including the empty path.
    /// # Errors
    /// Refuses a non-endorelation, wrong goal context or unavailable resources.
    pub fn can_reach(
        &self,
        event: &Event,
        limits: FixedPointLimits,
        control: &dyn Control,
    ) -> Result<FixedPointResult> {
        self.predicate_program(event, ModalOp::May, BoolOp4::OR, control)?
            .least(limits, control)
    }

    /// Every maximal path reaches the event. Dead ends outside it and cycles
    /// that avoid it are excluded; there is no implicit fairness assumption.
    /// # Errors
    /// Has `can_reach`'s endocontext and resource contract.
    pub fn inevitably_reach(
        &self,
        event: &Event,
        limits: FixedPointLimits,
        control: &dyn Control,
    ) -> Result<FixedPointResult> {
        self.predicate_program(event, ModalOp::Must, BoolOp4::OR, control)?
            .least(limits, control)
    }

    /// The event holds now and at every reachable state. A safe terminal state
    /// is allowed; requiring continued enabled behavior is a separate Must program.
    /// # Errors
    /// Has `can_reach`'s endocontext and resource contract.
    pub fn safe_throughout(
        &self,
        event: &Event,
        limits: FixedPointLimits,
        control: &dyn Control,
    ) -> Result<FixedPointResult> {
        self.predicate_program(event, ModalOp::All, BoolOp4::AND, control)?
            .greatest(limits, control)
    }
}

impl RelationalProduct {
    /// Prepare `X ↦ Id | R;X` on this plan's result pair context. Checked
    /// reindexing establishes one endorelation and preserves the middle witness.
    /// # Errors
    /// Refuses non-endorelation roles/contexts, cancellation or capacity.
    pub fn star_program(
        &self,
        relation: &WorldRelation,
        control: &dyn Control,
    ) -> Result<FixedPointProgram> {
        let [st, tu, su] = self.products();
        let [left, right, result] = self.views();
        let identity = WorldRelation::identity(su, control)?;
        let to_right = tu.map_to(su, control)?;
        let relation = relation.in_product(st, control)?;
        let mut builder = EventProgramBuilder::new(su.space(), control)?;
        let input = builder.input();
        let input = builder.map(MapOp::Pullback, &to_right, &input, control)?;
        let input = builder.map(MapOp::Pullback, right.map(), &input, control)?;
        let captured = builder.constant(relation.region(), control)?;
        let captured = builder.map(MapOp::Pullback, left.map(), &captured, control)?;
        let joined = builder.apply(BoolOp4::AND, &captured, &input, control)?;
        let image = builder.map(MapOp::Image, result.map(), &joined, control)?;
        let identity = builder.constant(identity.region(), control)?;
        let root = builder.apply(BoolOp4::OR, &identity, &image, control)?;
        builder.finish(&root, control)?.fixed_points(control)
    }

    /// Exact reflexive transitive closure in the prepared result product.
    /// The finite carrier counts legal pairs, not nodes or only endpoint states.
    /// # Errors
    /// Refuses invalid roles/contexts or resource exhaustion; returns no approximation.
    pub fn star(
        &self,
        relation: &WorldRelation,
        limits: FixedPointLimits,
        control: &dyn Control,
    ) -> Result<WorldRelation> {
        let result = self
            .star_program(relation, control)?
            .least(limits, control)?;
        WorldRelation::new(self.products()[2], result.event(), control)
    }
}
