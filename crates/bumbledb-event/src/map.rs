//! Checked deterministic maps of legal worlds. Readouts are Events, so maps
//! admit nonlinear observations as well as coordinate projections/permutations.
//! Structural images carry no probability law and assert no independence.
use std::collections::HashMap;
use std::sync::Arc;

use crate::arena::{Arena, MAX_COORDINATES, Operation, Ref};
use crate::{BoolOp4, Capacity, Control, Error, Event, Result, Space};

#[derive(Debug)]
struct Map {
    source: Space,
    target: Space,
    readouts: Vec<Event>,
}

/// An owned total function from legal source worlds to legal target worlds.
/// Each target bit has one Boolean readout on the source. Construction proves
/// support preservation; it does not prove surjectivity, complete fibre squares,
/// preservation of a designated law, or stochastic independence.
///
/// ```
/// use bumbledb_event::{CoordinateMap, Error, Space, SpaceId};
/// let source = Space::new(SpaceId([1; 32]), 1, &())?;
/// let target = Space::new(SpaceId([2; 32]), 2, &())?;
/// let copied = CoordinateMap::coordinates(&source, &target, &[0, 0], &())?;
/// let reachable = copied.support_image(&())?;
/// assert_eq!(reachable.count(&())?, 2); // precisely 00 and 11
/// assert!(!reachable.contains(1)?);
/// assert!(!reachable.contains(2)?);
/// let first = target.coordinate(0, &())?;
/// let second = target.coordinate(1, &())?;
/// assert_eq!(copied.pullback(&first, &())?, copied.pullback(&second, &())?);
/// # Ok::<(), Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct CoordinateMap(Arc<Map>);

/// Separate evidence that a checked map reaches every legal target world.
/// This licenses reflection of Event equality, inclusion and possibility.
/// It does not license moving quantifiers across an arbitrary map square.
#[derive(Debug, Clone)]
pub struct SurjectiveMap(CoordinateMap);

impl CoordinateMap {
    /// Check one Event readout per target semantic bit. Every readout is aligned
    /// to the source before checking the *original* target support by substitution.
    /// Completed target predicates alone cannot establish support preservation.
    /// # Errors
    /// Refuses incorrect arity, foreign readouts, illegal images, cancellation,
    /// or exhausted resources. A failed construction publishes no map.
    pub fn new(
        source: &Space,
        target: &Space,
        readouts: &[Event],
        control: &dyn Control,
    ) -> Result<Self> {
        control.checkpoint()?;
        if readouts.len() != usize::from(target.dimensions()) {
            return Err(Error::MapArity);
        }
        let mut owned = Vec::new();
        owned.try_reserve_exact(readouts.len())?;
        for event in readouts {
            owned.push(event.align_to(source, control)?);
        }
        source.parameter_map(target, &owned, control)?;
        let replacements = roots(&owned);
        source.with_arena_pair(target, |arena, other| {
            let mut op = Operation::new(arena, control)?;
            let allowed = op.substitute(
                other.as_deref(),
                target.0.support,
                &replacements[..owned.len()],
                &mut HashMap::new(),
            )?;
            let bad = op.apply(BoolOp4::DIFFERENCE, source.0.support, allowed)?;
            if bad != 0 {
                return Err(Error::MapOutsideSupport);
            }
            Ok(())
        })?;
        control.checkpoint()?;
        Ok(Self(Arc::new(Map {
            source: source.clone(),
            target: target.clone(),
            readouts: owned,
        })))
    }

    /// Construct a coordinate map; repeated indices explicitly copy a bit.
    /// # Errors
    /// Has `new`'s contract and refuses unknown source coordinates.
    pub fn coordinates(
        source: &Space,
        target: &Space,
        coordinates: &[u8],
        control: &dyn Control,
    ) -> Result<Self> {
        control.checkpoint()?;
        if coordinates.len() != usize::from(target.dimensions()) {
            return Err(Error::MapArity);
        }
        let mut readouts = Vec::new();
        readouts.try_reserve_exact(coordinates.len())?;
        for &coordinate in coordinates {
            readouts.push(source.coordinate(coordinate, control)?);
        }
        Self::new(source, target, &readouts, control)
    }

    /// Symbolic identity on an admitted space, without enumerating its worlds.
    /// # Errors
    /// Refuses cancellation or unavailable resources.
    pub fn identity(space: &Space, control: &dyn Control) -> Result<Self> {
        let mut coordinates = [0; MAX_COORDINATES as usize];
        for coordinate in 0..space.dimensions() {
            coordinates[usize::from(coordinate)] = coordinate;
        }
        Self::coordinates(
            space,
            space,
            &coordinates[..usize::from(space.dimensions())],
            control,
        )
    }

    #[must_use]
    pub fn source(&self) -> &Space {
        &self.0.source
    }

    #[must_use]
    pub fn target(&self) -> &Space {
        &self.0.target
    }

    /// Owned source Events in target semantic-coordinate order.
    #[must_use]
    pub fn readouts(&self) -> &[Event] {
        &self.0.readouts
    }

    /// Evaluate a single legal source world. This is a deterministic readout,
    /// not a sample from any law.
    /// # Errors
    /// Refuses an illegal source code or a poisoned owner.
    pub fn map_world(&self, world: u64) -> Result<u64> {
        let arena = self.source().0.lock()?;
        if world & !arena.mask() != 0 || !arena.evaluate(self.source().0.support, world) {
            return Err(Error::IllegalWorld(world));
        }
        Ok(self
            .readouts()
            .iter()
            .enumerate()
            .fold(0, |value, (bit, readout)| {
                value | (u64::from(arena.evaluate(readout.root, world)) << bit)
            }))
    }

    /// Preimage `f⁻¹(E)`: the source worlds whose readout satisfies E. Preserves
    /// all Boolean operations, including complement, without requiring onto-ness.
    /// # Errors
    /// Refuses a target-context mismatch, cancellation or unavailable resources.
    pub fn pullback(&self, event: &Event, control: &dyn Control) -> Result<Event> {
        let event = event.align_to(self.target(), control)?;
        let replacements = roots(self.readouts());
        let root = self
            .source()
            .with_arena_pair(self.target(), |arena, other| {
                let mut op = Operation::new(arena, control)?;
                let raw = op.substitute(
                    other.as_deref(),
                    event.root,
                    &replacements[..self.readouts().len()],
                    &mut HashMap::new(),
                )?;
                self.source().0.complete(&mut op, raw)
            })?;
        control.checkpoint()?;
        Ok(self.source().event(root))
    }

    /// Existential direct image: target worlds reached by a legal source world
    /// in E. Preserves unions; generally does not preserve intersections or
    /// complements. A result owns the original target context.
    ///
    /// The symbolic kernel eliminates source bits no future readout uses. Source
    /// and target need not fit in a combined coordinate workspace.
    /// # Errors
    /// Refuses a source-context mismatch, cancellation or unavailable resources.
    pub fn image(&self, event: &Event, control: &dyn Control) -> Result<Event> {
        let event = event.align_to(self.source(), control)?;
        let root = self
            .source()
            .with_arena_pair(self.target(), |arena, other| {
                let mut kernel = Image::new(arena, other, self.readouts(), control)?;
                // Original support gates the witnesses before any abstraction.
                // Decoder aliases never supply an additional legal source world.
                let legal =
                    kernel
                        .source
                        .apply(BoolOp4::AND, self.source().0.support, event.root)?;
                let hidden = kernel.source.arena.mask() & !kernel.suffix[0];
                let input = kernel.source.exists(legal, hidden)?;
                let raw = kernel.run(0, input)?;
                match &mut kernel.target {
                    Some(op) => self.target().0.complete(op, raw),
                    None => self.target().0.complete(&mut kernel.source, raw),
                }
            })?;
        control.checkpoint()?;
        Ok(self.target().event(root))
    }

    /// Universal direct image: every source world in the target fibre lies in E.
    /// Unreachable target worlds are included vacuously.
    /// # Errors
    /// Has `image`'s checked context and resource contract.
    pub fn universal_image(&self, event: &Event, control: &dyn Control) -> Result<Event> {
        Ok(self.image(&event.complement(), control)?.complement())
    }

    /// Universal direct image with a nonempty-fibre requirement.
    /// # Errors
    /// Has `image`'s checked context and resource contract.
    pub fn nonvacuous_image(&self, event: &Event, control: &dyn Control) -> Result<Event> {
        let all = self.universal_image(event, control)?;
        let reachable = self.support_image(control)?;
        reachable.apply(BoolOp4::AND, &all, control)
    }

    /// The exact structural range. Reaching a world says nothing about its mass.
    /// # Errors
    /// Refuses cancellation or unavailable resources.
    pub fn support_image(&self, control: &dyn Control) -> Result<Event> {
        self.image(&self.source().full(), control)
    }

    /// Certify that the support image is the entire target space.
    /// # Errors
    /// Refuses a proper image as `IncompleteImage`, cancellation or exhaustion.
    pub fn certify_surjective(&self, control: &dyn Control) -> Result<SurjectiveMap> {
        if !self.support_image(control)?.is_full() {
            return Err(Error::IncompleteImage);
        }
        Ok(SurjectiveMap(self.clone()))
    }

    /// Function composition, applying this map first and `next` second.
    /// The shared middle context is checked even for zero-coordinate targets.
    /// # Errors
    /// Refuses incompatible middle spaces, cancellation or exhausted resources.
    pub fn then(&self, next: &Self, control: &dyn Control) -> Result<Self> {
        next.source().full().align_to(self.target(), control)?;
        let mut readouts = Vec::new();
        readouts.try_reserve_exact(next.readouts().len())?;
        for event in next.readouts() {
            readouts.push(self.pullback(event, control)?);
        }
        Self::new(self.source(), next.target(), &readouts, control)
    }

    /// Extensional equality of maps between checked equal source/target spaces.
    /// # Errors
    /// Refuses incompatible contexts, cancellation or unavailable resources.
    pub fn equivalent(&self, other: &Self, control: &dyn Control) -> Result<bool> {
        other.source().full().align_to(self.source(), control)?;
        other.target().full().align_to(self.target(), control)?;
        let mut equal = true;
        for (left, right) in self.readouts().iter().zip(other.readouts()) {
            equal &= *left == right.align_to(self.source(), control)?;
        }
        control.checkpoint()?;
        Ok(equal)
    }
}

impl SurjectiveMap {
    #[must_use]
    pub fn map(&self) -> &CoordinateMap {
        &self.0
    }

    /// Factor a source Event through this readout. The returned target Event
    /// is unique because the readout is onto. Admission checks the membership
    /// FD: equal readouts must imply equal Event membership on legal worlds.
    /// # Errors
    /// Refuses an Event varying within a readout fibre, incompatible contexts,
    /// cancellation or exhausted resources.
    pub fn descend(&self, event: &Event, control: &dyn Control) -> Result<Event> {
        let original = event.align_to(self.map().source(), control)?;
        let image = self.map().image(&original, control)?;
        if self.map().pullback(&image, control)? != original {
            return Err(Error::RoleMismatch);
        }
        Ok(image)
    }
}

fn roots(readouts: &[Event]) -> [Ref; MAX_COORDINATES as usize] {
    let mut result = [0; MAX_COORDINATES as usize];
    for (slot, event) in result.iter_mut().zip(readouts) {
        *slot = event.root;
    }
    result
}

struct Image<'s, 't> {
    source: Operation<'s>,
    target: Option<Operation<'t>>,
    readouts: [Ref; MAX_COORDINATES as usize],
    order: [u8; MAX_COORDINATES as usize],
    suffix: [u64; MAX_COORDINATES as usize + 1],
    dimensions: usize,
    memo: HashMap<(usize, Ref), Ref>,
}

impl<'s, 't> Image<'s, 't> {
    fn new(
        source: &'s mut Arena,
        target: Option<&'t mut Arena>,
        readouts: &[Event],
        control: &'s dyn Control,
    ) -> Result<Self>
    where
        's: 't,
    {
        let dimensions = readouts.len();
        let mut order = [0; MAX_COORDINATES as usize];
        order[..dimensions].copy_from_slice(target.as_deref().unwrap_or(source).order());
        let readouts = roots(readouts);
        let mut suffix = [0; MAX_COORDINATES as usize + 1];
        for position in (0..dimensions).rev() {
            suffix[position] =
                suffix[position + 1] | source.variables(readouts[usize::from(order[position])]);
        }
        Ok(Self {
            source: Operation::new(source, control)?,
            target: target
                .map(|arena| Operation::new(arena, control))
                .transpose()?,
            readouts,
            order,
            suffix,
            dimensions,
            memo: HashMap::new(),
        })
    }

    fn run(&mut self, position: usize, input: Ref) -> Result<Ref> {
        self.source.step()?;
        if input == 0 {
            return Ok(0);
        }
        if position == self.dimensions {
            debug_assert_eq!(input, 1, "all source coordinates have been abstracted");
            return Ok(1);
        }
        let key = (position, input);
        if let Some(&root) = self.memo.get(&key) {
            return Ok(root);
        }
        let coordinate = self.order[position];
        let readout = self.readouts[usize::from(coordinate)];
        let hidden = self.source.arena.mask() & !self.suffix[position + 1];
        let low = self.source.apply(BoolOp4::DIFFERENCE, input, readout)?;
        let low = self.source.exists(low, hidden)?;
        let high = self.source.apply(BoolOp4::AND, input, readout)?;
        let high = self.source.exists(high, hidden)?;
        let low = self.run(position + 1, low)?;
        let high = self.run(position + 1, high)?;
        let root = if low == high {
            low
        } else if let Some(op) = &mut self.target {
            let bit = op.variable(coordinate)?;
            op.ite(bit, high, low)?
        } else {
            let bit = self.source.variable(coordinate)?;
            self.source.ite(bit, high, low)?
        };
        let limit = self
            .target
            .as_ref()
            .map_or(self.source.limits().memo_entries, |op| {
                op.limits()
                    .memo_entries
                    .min(self.source.limits().memo_entries)
            });
        if self.memo.len() >= limit {
            return Err(Error::Capacity(Capacity::MemoEntries));
        }
        self.memo.try_reserve(1)?;
        self.memo.insert(key, root);
        Ok(root)
    }
}
