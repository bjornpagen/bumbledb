//! Finite indexed Event rosters. Ordinary relation columns supply their labels;
//! no parameterized observable field or probability weights are introduced.
use std::sync::Arc;

use crate::{BoolOp4, Capacity, Control, CoordinateMap, Error, Event, Result, Space};

/// Maximum number of retained cells, including empty cells. A resource limit
/// never silently drops a bucket or changes a partition's parent.
#[derive(Debug, Clone, Copy)]
pub struct PartitionLimits {
    pub cells: usize,
}

impl Default for PartitionLimits {
    fn default() -> Self {
        Self { cells: 100_000 }
    }
}

impl PartitionLimits {
    fn check(self, cells: usize) -> Result<()> {
        if cells > self.cells {
            return Err(Error::Capacity(Capacity::PartitionCells));
        }
        Ok(())
    }
}

#[derive(Debug)]
struct Partition {
    parent: Event,
    cells: Vec<Event>,
}

/// An owned indexed roster, disjoint and covering its parent. Empty cells retain
/// their positions; parent may be empty without making the world space empty.
/// Scalar values and row identities stay in ordinary application/schema columns.
///
/// ```
/// use bumbledb_event::{BoolOp4, EventPartition, PartitionLimits, Space, SpaceId};
/// # fn main() -> bumbledb_event::Result<()> {
/// let space = Space::new(SpaceId([1; 32]), 1, &())?;
/// let holding = space.coordinate(0, &())?;
/// // Two distinct cards share the same holding event. They still count twice.
/// let counts = space.count_partition(&[holding.clone(), holding.clone()],
///     PartitionLimits::default(), &())?;
/// assert_eq!(counts.cells().len(), 3);
/// assert!(counts.cells()[1].is_empty());
/// assert_eq!(counts.cells()[2], holding);
/// // Ordinary value mapping: whether the count is positive.
/// let positive = counts.coarsen(&[0, 1, 1], 2, PartitionLimits::default(), &())?;
/// assert_eq!(positive.cells()[1], holding);
/// assert!(positive.select(&[0, 1], &())?.is_full());
/// let evidence = holding.clone();
/// let on_evidence = EventPartition::on(&evidence,
///     &[space.full(), holding.complement()], PartitionLimits::default(), &())?;
/// assert_eq!(on_evidence.parent(), &evidence);
/// assert!(on_evidence.cells()[1].is_empty());
/// assert!(on_evidence.cells()[0].apply(BoolOp4::DIFFERENCE, &evidence, &())?.is_empty());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct EventPartition(Arc<Partition>);

impl EventPartition {
    /// Align every input, intersect it with `parent`, then check distinct-cell
    /// disjointness and complete coverage there. Overlap outside parent is harmless.
    /// An empty roster is valid exactly when parent is empty. This is indexed
    /// partition admission, not equal-label grouping: union equal scalar values
    /// before calling it when validating a finite observable.
    /// # Errors
    /// Refuses foreign contexts, overlap/gaps on parent, resources or cancellation.
    pub fn on(
        parent: &Event,
        cells: &[Event],
        limits: PartitionLimits,
        control: &dyn Control,
    ) -> Result<Self> {
        control.checkpoint()?;
        limits.check(cells.len())?;
        let space = parent.space();
        let mut aligned = Vec::new();
        aligned.try_reserve_exact(cells.len())?;
        for cell in cells {
            aligned.push(cell.align_to(&space, control)?);
        }
        let mut covered = space.empty();
        for cell in &mut aligned {
            *cell = cell.apply(BoolOp4::AND, parent, control)?;
            if !cell.apply(BoolOp4::AND, &covered, control)?.is_empty() {
                return Err(Error::PartitionOverlap);
            }
            covered = covered.apply(BoolOp4::OR, cell, control)?;
        }
        if covered != *parent {
            return Err(Error::PartitionGap);
        }
        control.checkpoint()?;
        Ok(Self::admitted(parent.clone(), aligned))
    }

    fn admitted(parent: Event, cells: Vec<Event>) -> Self {
        Self(Arc::new(Partition { parent, cells }))
    }

    #[must_use]
    pub fn parent(&self) -> &Event {
        &self.0.parent
    }

    /// Cell order is the original indexed roster, including empty cells.
    #[must_use]
    pub fn cells(&self) -> &[Event] {
        &self.0.cells
    }

    /// Find the unique selected cell at one legal world. `None` means the world
    /// is outside parent, not a missing value inside it. This is a witness lookup,
    /// not a probability sample or a required enumeration of the world space.
    /// # Errors
    /// Refuses an illegal world or cancellation.
    pub fn locate(&self, world: u64, control: &dyn Control) -> Result<Option<usize>> {
        control.checkpoint()?;
        if !self.parent().contains(world)? {
            return Ok(None);
        }
        for (index, cell) in self.cells().iter().enumerate() {
            control.checkpoint()?;
            if cell.contains(world)? {
                return Ok(Some(index));
            }
        }
        // Private construction preserves coverage; this is never a sparse value.
        Err(Error::PartitionGap)
    }

    /// Union selected cell positions. Repeated indices are harmless, and an
    /// empty selection is the owned empty Event in this partition's space.
    /// # Errors
    /// Refuses an invalid index, cancellation or kernel capacity.
    pub fn select(&self, indices: &[usize], control: &dyn Control) -> Result<Event> {
        for &index in indices {
            control.checkpoint()?;
            if index >= self.cells().len() {
                return Err(Error::PartitionIndex);
            }
        }
        let mut result = self.parent().space().empty();
        for &index in indices {
            result = result.apply(BoolOp4::OR, &self.cells()[index], control)?;
        }
        control.checkpoint()?;
        Ok(result)
    }

    /// Group cells by an ordinary supplied value mapping. Each input position
    /// needs one valid output index, even for empty cells. All declared output
    /// buckets are retained, including unreachable ones. Parent is unchanged.
    /// # Errors
    /// Refuses wrong arity/indices, allocation, capacity or cancellation.
    pub fn coarsen(
        &self,
        destination: &[usize],
        output_cells: usize,
        limits: PartitionLimits,
        control: &dyn Control,
    ) -> Result<Self> {
        control.checkpoint()?;
        if destination.len() != self.cells().len() {
            return Err(Error::PartitionArity);
        }
        limits.check(output_cells)?;
        for &index in destination {
            control.checkpoint()?;
            if index >= output_cells {
                return Err(Error::PartitionIndex);
            }
        }
        let mut cells = Vec::new();
        cells.try_reserve_exact(output_cells)?;
        cells.resize(output_cells, self.parent().space().empty());
        for (cell, &index) in self.cells().iter().zip(destination) {
            cells[index] = cells[index].apply(BoolOp4::OR, cell, control)?;
        }
        control.checkpoint()?;
        Ok(Self::admitted(self.parent().clone(), cells))
    }

    /// Common refinement on the intersection of parents. Output index is
    /// `left_index * right.cells().len() + right_index`; all pairs remain present.
    /// This is same-world conjunction, never an independence assumption.
    /// # Errors
    /// Checks contexts even for empty parents/rosters; refuses resources/cancellation.
    pub fn refine(
        &self,
        right: &Self,
        limits: PartitionLimits,
        control: &dyn Control,
    ) -> Result<Self> {
        let space = self.parent().space();
        let right_parent = right.parent().align_to(&space, control)?;
        let extent = self
            .cells()
            .len()
            .checked_mul(right.cells().len())
            .ok_or(Error::Capacity(Capacity::PartitionCells))?;
        limits.check(extent)?;
        let parent = self.parent().apply(BoolOp4::AND, &right_parent, control)?;
        let mut right_cells = Vec::new();
        right_cells.try_reserve_exact(right.cells().len())?;
        for cell in right.cells() {
            right_cells.push(cell.align_to(&space, control)?);
        }
        let mut cells = Vec::new();
        cells.try_reserve_exact(extent)?;
        for left in self.cells() {
            for right in &right_cells {
                cells.push(left.apply(BoolOp4::AND, right, control)?);
            }
        }
        control.checkpoint()?;
        Ok(Self::admitted(parent, cells))
    }

    /// Pull back an entire partition through a checked map whose target is this
    /// space. Pullback preserves both coverage and disjointness; arbitrary image
    /// does not, so there is no unchecked image-partition constructor.
    /// # Errors
    /// Refuses incorrect target context, cancellation or map-kernel capacity.
    pub fn pullback(&self, map: &CoordinateMap, control: &dyn Control) -> Result<Self> {
        let parent = map.pullback(self.parent(), control)?;
        let mut cells = Vec::new();
        cells.try_reserve_exact(self.cells().len())?;
        for cell in self.cells() {
            cells.push(map.pullback(cell, control)?);
        }
        control.checkpoint()?;
        Ok(Self::admitted(parent, cells))
    }

    /// Convert a full partition and ordinary per-cell target codes to a checked
    /// deterministic readout. Equal codes may merge cells; no onto claim is made.
    /// Every supplied code must be legal, including codes of empty cells. A
    /// partial parent cannot silently invent values for the remaining worlds.
    /// # Errors
    /// Refuses a partial parent, wrong code roster, illegal targets or resources.
    pub fn readout(
        &self,
        target: &Space,
        codes: &[u64],
        control: &dyn Control,
    ) -> Result<CoordinateMap> {
        control.checkpoint()?;
        if codes.len() != self.cells().len() {
            return Err(Error::PartitionArity);
        }
        for &code in codes {
            control.checkpoint()?;
            // full().contains still checks original target support and extent.
            target.full().contains(code)?;
        }
        if !self.parent().is_full() {
            return Err(Error::PartitionGap);
        }
        let space = self.parent().space();
        let mut readouts = Vec::new();
        readouts.try_reserve_exact(usize::from(target.dimensions()))?;
        for bit in 0..target.dimensions() {
            let mut value = space.empty();
            for (cell, &code) in self.cells().iter().zip(codes) {
                control.checkpoint()?;
                if code & (1 << bit) != 0 {
                    value = value.apply(BoolOp4::OR, cell, control)?;
                }
            }
            readouts.push(value);
        }
        CoordinateMap::new(&space, target, &readouts, control)
    }
}

impl Space {
    /// Materialize the complete indexed cardinality partition in one pass.
    /// Bucket k contains worlds satisfying exactly k roster positions. Equal
    /// Events at distinct positions count separately; empty buckets are retained.
    /// # Errors
    /// Refuses mismatched contexts, output-cell capacity, cancellation or resources.
    pub fn count_partition(
        &self,
        events: &[Event],
        limits: PartitionLimits,
        control: &dyn Control,
    ) -> Result<EventPartition> {
        control.checkpoint()?;
        let extent = events
            .len()
            .checked_add(1)
            .ok_or(Error::Capacity(Capacity::PartitionCells))?;
        limits.check(extent)?;
        let mut aligned = Vec::new();
        aligned.try_reserve_exact(events.len())?;
        for event in events {
            aligned.push(event.align_to(self, control)?);
        }
        let mut cells = Vec::new();
        cells.try_reserve_exact(extent)?;
        cells.resize(extent, self.empty());
        cells[0] = self.full();
        for (seen, event) in aligned.iter().enumerate() {
            for count in (0..=seen + 1).rev() {
                let previous = if count == 0 {
                    self.empty()
                } else {
                    cells[count - 1].clone()
                };
                cells[count] = event.ite(&previous, &cells[count], control)?;
            }
        }
        control.checkpoint()?;
        Ok(EventPartition::admitted(self.full(), cells))
    }
}
