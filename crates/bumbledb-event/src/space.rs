use std::hash::{Hash, Hasher};
use std::sync::{
    Arc, Mutex, MutexGuard,
    atomic::{AtomicU64, Ordering},
};

use crate::arena::{Arena, MAX_COORDINATES, Operation, Ref};
use crate::{BoolOp4, Capacity, Control, Error, Limits, Result, Signature};

/// Application-supplied semantic identity of a named source environment.
/// Allocating a different identity declares a different outcome environment;
/// it does not assert probabilistic independence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SpaceId(pub [u8; 32]);

/// Two-word resident binding key. Process-local, never a persistence format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct EventKey {
    space: u64,
    region: u64,
}
const _: () = assert!(size_of::<EventKey>() == 16);

impl EventKey {
    #[must_use]
    pub const fn words(self) -> [u64; 2] {
        [self.space, self.region]
    }
}

#[derive(Debug)]
pub(crate) struct Owner {
    token: u64,
    identity: SpaceId,
    dimensions: u8,
    pub(crate) support: Ref,
    anchor: u64,
    arena: Arc<Mutex<Arena>>,
}

impl Owner {
    pub(crate) fn lock(&self) -> Result<MutexGuard<'_, Arena>> {
        self.arena.lock().map_err(|_| Error::Poisoned)
    }

    pub(crate) fn complete(&self, op: &mut Operation<'_>, raw: Ref) -> Result<Ref> {
        // rho fixes legal worlds and sends all other codes to one legal anchor.
        // ITE, rather than masking, makes complement a single polarity bit.
        let anchor_value = Ref::from(op.arena.evaluate(raw, self.anchor));
        op.ite(self.support, raw, anchor_value)
    }
}

fn token() -> Result<u64> {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map_err(|_| Error::Capacity(Capacity::SpaceTokens))
}

/// An inhabited world space. Clones retain the same owner and source identity.
/// Coordinates are semantic bit indices; physical split order is separate.
#[derive(Debug, Clone)]
pub struct Space(pub(crate) Arc<Owner>);

/// An owned canonical condition. Empty/full values retain this owner too.
#[derive(Debug, Clone)]
pub struct Event {
    owner: Arc<Owner>,
    pub(crate) root: Ref,
}

/// Retained arena capacity includes intermediates and all sharing spaces.
/// The byte count estimates vector/hash storage, not allocator overhead or RSS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Statistics {
    pub records: usize,
    pub table_words: usize,
    pub estimated_retained_bytes: usize,
}

impl Space {
    /// Make an unmeasured full Boolean space with a named outcome environment.
    /// # Errors
    /// Refuses more than 62 raw coordinates, cancellation or allocation failure.
    pub fn new(identity: SpaceId, dimensions: u8, control: &dyn Control) -> Result<Self> {
        if dimensions > MAX_COORDINATES {
            return Err(Error::Capacity(Capacity::Coordinates));
        }
        let mut order = [0; MAX_COORDINATES as usize];
        for (v, i) in order.iter_mut().zip(0..dimensions) {
            *v = i;
        }
        Self::with_order(
            identity,
            &order[..usize::from(dimensions)],
            Limits::default(),
            control,
        )
    }

    /// Select an immutable working order and explicit resource limits.
    /// # Errors
    /// Refuses an invalid coordinate permutation or unavailable resources.
    pub fn with_order(
        identity: SpaceId,
        order: &[u8],
        limits: Limits,
        control: &dyn Control,
    ) -> Result<Self> {
        control.checkpoint()?;
        let arena = Arena::new(order, limits)?;
        let dimensions =
            u8::try_from(order.len()).map_err(|_| Error::Capacity(Capacity::Coordinates))?;
        Ok(Self(Arc::new(Owner {
            token: token()?,
            identity,
            dimensions,
            support: 1,
            anchor: 0,
            arena: Arc::new(Mutex::new(arena)),
        })))
    }

    #[must_use]
    pub fn identity(&self) -> SpaceId {
        self.0.identity
    }
    #[must_use]
    pub fn dimensions(&self) -> u8 {
        self.0.dimensions
    }
    #[must_use]
    pub fn empty(&self) -> Event {
        self.event(0)
    }
    #[must_use]
    pub fn full(&self) -> Event {
        self.event(1)
    }

    pub(crate) fn event(&self, root: Ref) -> Event {
        Event {
            owner: self.0.clone(),
            root,
        }
    }

    /// Lock a pair once, in the same total order used by checked alignment.
    /// `None` means both contexts use the first arena (possibly with different
    /// support). No raw reference crosses an arena without an explicit rebuild.
    pub(crate) fn with_arena_pair<T>(
        &self,
        other: &Self,
        run: impl FnOnce(&mut Arena, Option<&mut Arena>) -> Result<T>,
    ) -> Result<T> {
        if Arc::ptr_eq(&self.0.arena, &other.0.arena) {
            let mut arena = self.0.lock()?;
            run(&mut arena, None)
        } else if Arc::as_ptr(&self.0.arena).addr() < Arc::as_ptr(&other.0.arena).addr() {
            let mut first = self.0.lock()?;
            let mut second = other.0.lock()?;
            run(&mut first, Some(&mut second))
        } else {
            let mut second = other.0.lock()?;
            let mut first = self.0.lock()?;
            run(&mut first, Some(&mut second))
        }
    }

    /// The condition that one named coordinate is true, under legal support.
    /// # Errors
    /// Refuses invalid coordinates, cancellation and resource exhaustion.
    pub fn coordinate(&self, coordinate: u8, control: &dyn Control) -> Result<Event> {
        control.checkpoint()?;
        let mut arena = self.0.lock()?;
        let mut op = Operation::new(&mut arena, control)?;
        let raw = op.variable(coordinate)?;
        let root = self.0.complete(&mut op, raw)?;
        control.checkpoint()?;
        Ok(self.event(root))
    }

    /// Import truth bits over the ascending set bits of `coordinates`.
    /// Outside-support differences disappear through legal completion. Explicit
    /// import is limited to 20 coordinates; symbolic construction supports 62.
    /// # Errors
    /// Refuses invalid extents/padding, capacities, cancellation or allocation.
    pub fn table(&self, coordinates: u64, words: &[u64], control: &dyn Control) -> Result<Event> {
        control.checkpoint()?;
        let mut arena = self.0.lock()?;
        let mut op = Operation::new(&mut arena, control)?;
        let raw = op.import_table(coordinates, words)?;
        let root = self.0.complete(&mut op, raw)?;
        control.checkpoint()?;
        Ok(self.event(root))
    }

    /// The region where between `minimum` and `maximum` supplied conditions
    /// hold, inclusively. Repeated inputs retain their cardinality meaning.
    /// An empty list has count zero. Every owner is checked even when the range
    /// makes the answer constant.
    /// # Errors
    /// Refuses mismatched owners, cancellation or unavailable resources.
    pub fn cardinality(
        &self,
        events: &[Event],
        minimum: usize,
        maximum: usize,
        control: &dyn Control,
    ) -> Result<Event> {
        for event in events {
            if !Arc::ptr_eq(&self.0, &event.owner) {
                return Err(Error::SpaceMismatch);
            }
        }
        control.checkpoint()?;
        if minimum > maximum || minimum > events.len() {
            return Ok(self.empty());
        }
        let maximum = maximum.min(events.len());
        let length = maximum.checked_add(1).ok_or(Error::Allocation)?;
        let mut exact = Vec::new();
        exact.try_reserve_exact(length)?;
        exact.resize(length, 0);
        exact[0] = 1;
        let mut arena = self.0.lock()?;
        let mut op = Operation::new(&mut arena, control)?;
        for (seen, event) in events.iter().enumerate() {
            for count in (0..=maximum.min(seen + 1)).rev() {
                let previous = if count == 0 { 0 } else { exact[count - 1] };
                exact[count] = op.ite(event.root, previous, exact[count])?;
            }
        }
        let mut root = 0;
        for &part in &exact[minimum..] {
            root = op.apply(BoolOp4::OR, root, part)?;
        }
        control.checkpoint()?;
        Ok(self.event(root))
    }

    /// Capture a nonempty restriction as a new context. Existing Events remain
    /// immutable; use `Event::in_space` to transfer them explicitly.
    /// # Errors
    /// Refuses mismatched owners, empty support or unavailable resources.
    pub fn restrict(&self, legal: &Event, control: &dyn Control) -> Result<Self> {
        if !Arc::ptr_eq(&self.0, &legal.owner) {
            return Err(Error::SpaceMismatch);
        }
        control.checkpoint()?;
        let mut arena = self.0.lock()?;
        let mut op = Operation::new(&mut arena, control)?;
        let support = op.apply(BoolOp4::AND, self.0.support, legal.root)?;
        let anchor = op.arena.witness(support).ok_or(Error::EmptySpace)?;
        control.checkpoint()?;
        Ok(Self(Arc::new(Owner {
            token: token()?,
            identity: self.identity(),
            dimensions: self.dimensions(),
            support,
            anchor,
            arena: self.0.arena.clone(),
        })))
    }

    /// # Errors
    /// Fails only if a panic poisoned the shared manager.
    pub fn statistics(&self) -> Result<Statistics> {
        let arena = self.0.lock()?;
        Ok(Statistics {
            records: arena.records(),
            table_words: arena.words(),
            estimated_retained_bytes: arena.retained_bytes(),
        })
    }
}

impl Event {
    /// Capture the reachable completed-function and original-support diagrams.
    /// The snapshot owns its records. Borrowed views hold no manager lock, so a
    /// caller can inspect a graph while constructing Events in the same space.
    /// This is a working representation, not a canonical persistence encoding.
    /// # Errors
    /// Refuses cancellation, allocation or inspection-capacity exhaustion.
    pub fn diagram(&self, control: &dyn Control) -> Result<crate::Diagram> {
        control.checkpoint()?;
        let mut arena = self.owner.lock()?;
        crate::diagram::capture(
            self.owner.identity,
            self.owner.support,
            self.root,
            &mut arena,
            control,
        )
    }

    /// Canonical versioned bytes of this unmeasured space and region. Allocation,
    /// decoder aliases, working order and local-table cutoff do not determine
    /// these bytes. Conversion can exceed an explicit resource limit.
    /// # Errors
    /// Refuses cancellation, allocation or conversion-capacity exhaustion.
    pub fn to_bytes(&self, control: &dyn Control) -> Result<Vec<u8>> {
        control.checkpoint()?;
        let mut arena = self.owner.lock()?;
        crate::codec::encode(
            self.owner.identity,
            self.owner.dimensions,
            self.owner.support,
            self.root,
            &mut arena,
            control,
        )
    }

    /// Validate canonical bytes and reconstruct an independently owned value.
    /// # Errors
    /// Refuses unsupported versions, malformed graphs, empty support, and
    /// cancellation or exhausted resource limits. No partial value is published.
    pub fn from_bytes(bytes: &[u8], control: &dyn Control) -> Result<Self> {
        Self::from_bytes_with_order(bytes, None, Limits::default(), control)
    }

    /// Reconstruct canonical bytes into a chosen physical order and limits.
    /// # Errors
    /// Has the same checked decoding contract as `from_bytes`; the working order
    /// must additionally be a permutation of the encoded coordinates.
    pub fn from_bytes_with_order(
        bytes: &[u8],
        order: Option<&[u8]>,
        limits: Limits,
        control: &dyn Control,
    ) -> Result<Self> {
        let decoded = crate::codec::decode(bytes, order, limits, control)?;
        let anchor = decoded
            .arena
            .witness(decoded.support)
            .ok_or(Error::EmptySpace)?;
        let owner = Arc::new(Owner {
            token: token()?,
            identity: decoded.identity,
            dimensions: decoded.dimensions,
            support: decoded.support,
            anchor,
            arena: Arc::new(Mutex::new(decoded.arena)),
        });
        Ok(Self {
            owner,
            root: decoded.event,
        })
    }

    #[must_use]
    pub fn key(&self) -> EventKey {
        EventKey {
            space: self.owner.token,
            region: u64::from(self.root),
        }
    }
    #[must_use]
    pub fn space(&self) -> Space {
        Space(self.owner.clone())
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.root == 0
    }
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.root == 1
    }
    #[must_use]
    pub fn complement(&self) -> Self {
        Self {
            owner: self.owner.clone(),
            root: self.root ^ 1,
        }
    }

    fn aligned(&self, other: &Self) -> Result<()> {
        if Arc::ptr_eq(&self.owner, &other.owner) {
            Ok(())
        } else {
            Err(Error::SpaceMismatch)
        }
    }

    /// Exact equality after validating the captured contexts.
    /// # Errors
    /// Unaligned contexts fail even for constant Events.
    pub fn equivalent(&self, other: &Self) -> Result<bool> {
        self.aligned(other)?;
        Ok(self.root == other.root)
    }

    /// Apply any binary truth function after validating both operands.
    /// # Errors
    /// Refuses unaligned owners, cancellation and resource exhaustion.
    pub fn apply(&self, operation: BoolOp4, other: &Self, control: &dyn Control) -> Result<Self> {
        self.aligned(other)?;
        control.checkpoint()?;
        let mut arena = self.owner.lock()?;
        let mut op = Operation::new(&mut arena, control)?;
        let root = op.apply(operation, self.root, other.root)?;
        control.checkpoint()?;
        Ok(Self {
            owner: self.owner.clone(),
            root,
        })
    }

    /// Direct conditional construction; all three owners are checked first.
    /// # Errors
    /// Refuses unaligned owners, cancellation and resource exhaustion.
    pub fn ite(&self, high: &Self, low: &Self, control: &dyn Control) -> Result<Self> {
        self.aligned(high)?;
        self.aligned(low)?;
        control.checkpoint()?;
        let mut arena = self.owner.lock()?;
        let mut op = Operation::new(&mut arena, control)?;
        let root = op.ite(self.root, high.root, low.root)?;
        control.checkpoint()?;
        Ok(Self {
            owner: self.owner.clone(),
            root,
        })
    }

    /// Transfer to an explicitly restricted context sharing these coordinates.
    /// This is a support map, not a probability conditioning operation.
    /// # Errors
    /// Refuses unrelated coordinate owners or a target extending source support.
    pub fn in_space(&self, target: &Space, control: &dyn Control) -> Result<Self> {
        if !Arc::ptr_eq(&self.owner.arena, &target.0.arena) {
            return Err(Error::SpaceMismatch);
        }
        control.checkpoint()?;
        let mut arena = self.owner.lock()?;
        let mut op = Operation::new(&mut arena, control)?;
        if op.apply(BoolOp4::DIFFERENCE, target.0.support, self.owner.support)? != 0 {
            return Err(Error::SpaceMismatch);
        }
        let root = target.0.complete(&mut op, self.root)?;
        control.checkpoint()?;
        Ok(target.event(root))
    }

    /// Align independently owned presentations of the same named space.
    /// Checks equal source identity, coordinates and admissible support before
    /// completing the translated region with the target's decoder.
    /// # Errors
    /// Refuses unequal contexts, cancellation and exhausted resources.
    pub fn align_to(&self, target: &Space, control: &dyn Control) -> Result<Self> {
        if self.owner.identity != target.0.identity || self.owner.dimensions != target.0.dimensions
        {
            return Err(Error::SpaceMismatch);
        }
        control.checkpoint()?;
        if Arc::ptr_eq(&self.owner.arena, &target.0.arena) {
            if self.owner.support != target.0.support {
                return Err(Error::SpaceMismatch);
            }
            return self.in_space(target, control);
        }
        let rebuild = |source: &Arena, destination: &mut Arena| -> Result<Ref> {
            let mut op = Operation::new(destination, control)?;
            let mut memo = std::collections::HashMap::new();
            let support = op.transfer(source, self.owner.support, &mut memo)?;
            if support != target.0.support {
                return Err(Error::SpaceMismatch);
            }
            let root = op.transfer(source, self.root, &mut memo)?;
            target.0.complete(&mut op, root)
        };
        // Total order on live lock addresses prevents opposite-direction
        // translations from deadlocking. Addresses never decide Event identity.
        let root = if Arc::as_ptr(&self.owner.arena).addr() < Arc::as_ptr(&target.0.arena).addr() {
            let source = self.owner.lock()?;
            let mut destination = target.0.lock()?;
            rebuild(&source, &mut destination)?
        } else {
            let mut destination = target.0.lock()?;
            let source = self.owner.lock()?;
            rebuild(&source, &mut destination)?
        };
        control.checkpoint()?;
        Ok(target.event(root))
    }

    /// Existentially forget hidden coordinates and lift the result back into
    /// this space. This is saturation on legal fibres, not marginal probability
    /// or a construction of a new target law.
    /// # Errors
    /// Refuses unknown coordinates, cancellation and resource exhaustion.
    pub fn saturate(&self, hidden: u64, control: &dyn Control) -> Result<Self> {
        control.checkpoint()?;
        let mut arena = self.owner.lock()?;
        if hidden & !arena.mask() != 0 {
            return Err(Error::InvalidOrder);
        }
        let mut op = Operation::new(&mut arena, control)?;
        let legal = op.apply(BoolOp4::AND, self.owner.support, self.root)?;
        let projected = op.exists(legal, hidden)?;
        let root = self.owner.complete(&mut op, projected)?;
        control.checkpoint()?;
        Ok(Self {
            owner: self.owner.clone(),
            root,
        })
    }

    /// Exact membership of a legal world; decoder aliases cannot be queried as
    /// if they were additional outcomes.
    /// # Errors
    /// Refuses illegal worlds or a poisoned manager.
    pub fn contains(&self, world: u64) -> Result<bool> {
        let arena = self.owner.lock()?;
        if world & !arena.mask() != 0 || !arena.evaluate(self.owner.support, world) {
            return Err(Error::IllegalWorld(world));
        }
        Ok(arena.evaluate(self.root, world))
    }

    /// A witness from original legal support, never a decoder alias.
    /// # Errors
    /// Refuses cancellation, exhausted capacity or a poisoned manager.
    pub fn witness(&self, control: &dyn Control) -> Result<Option<u64>> {
        control.checkpoint()?;
        let mut arena = self.owner.lock()?;
        let mut op = Operation::new(&mut arena, control)?;
        let legal = op.apply(BoolOp4::AND, self.owner.support, self.root)?;
        let out = op.arena.witness(legal);
        control.checkpoint()?;
        Ok(out)
    }

    /// Exact legal-world count. This is not a probability measurement.
    /// # Errors
    /// Refuses cancellation, exhausted capacity or a poisoned manager.
    pub fn count(&self, control: &dyn Control) -> Result<u64> {
        control.checkpoint()?;
        let mut arena = self.owner.lock()?;
        let mut op = Operation::new(&mut arena, control)?;
        let legal = op.apply(BoolOp4::AND, self.owner.support, self.root)?;
        let absent = op.arena.mask() & !op.arena.variables(legal);
        let count = op.count(legal)? << absent.count_ones();
        control.checkpoint()?;
        Ok(count)
    }

    /// Exact pair occupancy. Every binary Boolean emptiness test follows from
    /// these four bits, but probabilistic independence does not.
    /// # Errors
    /// Refuses mismatched owners, cancellation or exhausted capacity.
    pub fn signature(&self, other: &Self, control: &dyn Control) -> Result<Signature> {
        self.aligned(other)?;
        control.checkpoint()?;
        let mut arena = self.owner.lock()?;
        let mut op = Operation::new(&mut arena, control)?;
        let mut signature = 0;
        for (bit, truth) in BoolOp4::CELLS.into_iter().enumerate() {
            // Completed functions are onto legal worlds, so nonzero suffices.
            if op.apply(truth, self.root, other.root)? != 0 {
                signature |= 1 << bit;
            }
        }
        control.checkpoint()?;
        Ok(Signature(signature))
    }
}

// Rust equality is scoped handle identity. A caller seeking cross-context
// semantic equality must first perform an explicit checked map/alignment.
impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}
impl Eq for Event {}
impl Hash for Event {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key().hash(state);
    }
}

impl std::ops::Not for &Event {
    type Output = Event;
    fn not(self) -> Event {
        self.complement()
    }
}
