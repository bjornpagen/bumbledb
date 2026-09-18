//! Information operations are saturation through ordinary checked readouts.
//! Their answers are Events on the original world space. No source law, belief
//! update, new draw or private observer information is inferred by these maps.
use crate::product::require_same_map;
use crate::{
    BoolOp4, Control, CoordinateMap, Error, Event, FibreProduct, Result, SurjectiveMap,
    WorldRelation,
};

/// Whole observation cells classified under explicit evidence. Guaranteed,
/// ruled-out and ambiguous are pairwise disjoint and cover `reachable`.
/// Unreachable cells belong to none of those three cases. Results remain Events
/// on the original source, including worlds outside the supplied evidence.
#[derive(Debug, Clone)]
pub struct InformationCases {
    reachable: Event,
    possible: Event,
    guaranteed: Event,
    ruled_out: Event,
    ambiguous: Event,
}

impl InformationCases {
    /// Cells with at least one legal evidence world.
    #[must_use]
    pub fn reachable(&self) -> &Event {
        &self.reachable
    }

    /// Cells with at least one evidence world satisfying the event.
    #[must_use]
    pub fn possible(&self) -> &Event {
        &self.possible
    }

    /// Reachable cells whose every evidence world satisfies the event.
    #[must_use]
    pub fn guaranteed(&self) -> &Event {
        &self.guaranteed
    }

    /// Reachable cells whose every evidence world refutes the event.
    #[must_use]
    pub fn ruled_out(&self) -> &Event {
        &self.ruled_out
    }

    /// Cells with both a satisfying and refuting evidence world.
    #[must_use]
    pub fn ambiguous(&self) -> &Event {
        &self.ambiguous
    }
}

impl CoordinateMap {
    /// Least observable Event containing `event`: pull back its exact image.
    /// A world remains possible when some legal world with the same readout
    /// satisfies it. Unreachable target codes are never possible witnesses.
    /// # Errors
    /// Refuses an incompatible source Event, cancellation or capacity.
    pub fn possible(&self, event: &Event, control: &dyn Control) -> Result<Event> {
        self.pullback(&self.image(event, control)?, control)
    }

    /// Greatest observable Event contained in `event`. Every source world's
    /// readout has itself as a witness, so full-support certainty is nonvacuous.
    /// # Errors
    /// Has `possible`'s context and resource contract.
    pub fn guaranteed(&self, event: &Event, control: &dyn Control) -> Result<Event> {
        Ok(self.possible(&event.complement(), control)?.complement())
    }

    /// Observable cells with both a satisfying and refuting legal world.
    /// # Errors
    /// Has `possible`'s context and resource contract.
    pub fn ambiguous(&self, event: &Event, control: &dyn Control) -> Result<Event> {
        let positive = self.possible(event, control)?;
        let negative = self.possible(&event.complement(), control)?;
        positive.apply(BoolOp4::AND, &negative, control)
    }

    /// Classify whole original observation cells under explicit evidence.
    /// Impossible evidence makes every case empty, never vacuously certain.
    /// Intersect a result with evidence separately to select actual remaining
    /// worlds. No probability conditioning or change of legal support occurs.
    ///
    /// ```
    /// use bumbledb_event::{CoordinateMap, Error, Space, SpaceId};
    /// let worlds = Space::new(SpaceId([1; 32]), 2, &())?;
    /// let visible = Space::new(SpaceId([2; 32]), 1, &())?;
    /// let observation = CoordinateMap::coordinates(&worlds, &visible, &[0], &())?;
    /// let hidden = worlds.coordinate(1, &())?;
    /// assert!(observation.ambiguous(&hidden, &())?.is_full());
    /// let cases = observation.information(&hidden, &hidden, &())?;
    /// assert!(cases.guaranteed().is_full()); // both observation cells meet evidence
    /// assert!(cases.ambiguous().is_empty());
    /// assert!(!hidden.is_full()); // answers describe whole cells, not just evidence
    /// let impossible = observation.information(&hidden, &worlds.empty(), &())?;
    /// assert!(impossible.reachable().is_empty());
    /// assert!(impossible.guaranteed().is_empty());
    /// # Ok::<(), Error>(())
    /// ```
    /// # Errors
    /// Checks both Events against the source before construction; refuses context
    /// mismatch, cancellation or exhausted resources. No partial cases escape.
    pub fn information(
        &self,
        event: &Event,
        given: &Event,
        control: &dyn Control,
    ) -> Result<InformationCases> {
        let event = event.align_to(self.source(), control)?;
        let given = given.align_to(self.source(), control)?;
        let positive = self.possible(&given.apply(BoolOp4::AND, &event, control)?, control)?;
        let negative =
            self.possible(&given.apply(BoolOp4::DIFFERENCE, &event, control)?, control)?;
        Ok(InformationCases {
            reachable: positive.apply(BoolOp4::OR, &negative, control)?,
            guaranteed: positive.apply(BoolOp4::DIFFERENCE, &negative, control)?,
            ruled_out: negative.apply(BoolOp4::DIFFERENCE, &positive, control)?,
            ambiguous: positive.apply(BoolOp4::AND, &negative, control)?,
            possible: positive,
        })
    }
}

impl SurjectiveMap {
    /// Factor another readout through this one by checking the FD `self → other`.
    /// Returns the unique map h from this target to the other's target such that
    /// `self.then(h) = other`. The other's target need not be fully reachable.
    /// Each Boolean readout must descend through the same admitted fibres.
    /// # Errors
    /// Refuses differing sources, a readout not determined by this observation,
    /// cancellation or unavailable resources. Zero-bit targets still check sources.
    pub fn factor(&self, other: &CoordinateMap, control: &dyn Control) -> Result<CoordinateMap> {
        other
            .source()
            .full()
            .align_to(self.map().source(), control)?;
        let mut readouts = Vec::new();
        readouts.try_reserve_exact(other.readouts().len())?;
        for readout in other.readouts() {
            readouts.push(self.descend(readout, control)?);
        }
        CoordinateMap::new(self.map().target(), other.target(), &readouts, control)
    }
}

impl WorldRelation {
    /// Equality of an observation's readout, as a relation on source worlds.
    /// The supplied pair product must use that source for both endpoints and
    /// the same environment map. The observation must determine that environment;
    /// otherwise clipping would silently make its cells smaller than the readout.
    /// # Errors
    /// Refuses endpoint/environment mismatches, cancellation or resource exhaustion.
    pub fn indistinguishable(
        product: &FibreProduct,
        observation: &CoordinateMap,
        control: &dyn Control,
    ) -> Result<Self> {
        require_same_map(
            product.left_environment().map(),
            product.right_environment().map(),
            control,
        )?;
        observation
            .source()
            .full()
            .align_to(product.left().map().target(), control)?;
        for readout in product.left_environment().map().readouts() {
            let readout = readout.align_to(observation.source(), control)?;
            if observation.possible(&readout, control)? != readout {
                return Err(Error::EnvironmentMismatch);
            }
        }
        let mut region = product.space().full();
        for readout in observation.readouts() {
            let left = product.left().map().pullback(readout, control)?;
            let right = product.right().map().pullback(readout, control)?;
            region = region.apply(
                BoolOp4::AND,
                &left.apply(BoolOp4::EQUIVALENCE, &right, control)?,
                control,
            )?;
        }
        Self::new(product, &region, control)
    }
}
