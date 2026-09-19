//! Full products over a common, sealed environment presentation. A product has
//! every compatible legal pair; behavioral restrictions are separate Events.
use std::collections::HashMap;
use std::sync::Arc;

use crate::arena::{MAX_COORDINATES, Operation, Ref};
use crate::{
    BoolOp4, Capacity, Control, CoordinateMap, Error, Event, Limits, Result, Space, SpaceId,
    SurjectiveMap,
};

#[derive(Debug)]
struct Faces {
    space: Space,
    environments: Vec<SurjectiveMap>,
    projections: Vec<SurjectiveMap>,
}

/// A full product of any nonempty roster of faces over one sealed environment.
/// Face indices select owned descriptors; they are not unchecked bit offsets.
/// Every face has nonempty fibres throughout the shared admitted environment.
#[derive(Debug, Clone)]
pub struct FaceProduct(Arc<Faces>);

impl FaceProduct {
    /// Construct all compatible legal face tuples, with interleaved bit order.
    /// Repeated faces are state aliases, not independent probabilistic draws.
    /// # Errors
    /// Refuses no faces, incompatible environments, more than 62 total bits,
    /// cancellation or unavailable resources.
    pub fn new(identity: SpaceId, faces: &[SurjectiveMap], control: &dyn Control) -> Result<Self> {
        Self::with_limits(identity, faces, Limits::default(), control)
    }

    /// Construct interleaved products under explicit per-owner resource limits.
    /// # Errors
    /// Has `new`'s support and context checks, plus the supplied capacity bounds.
    pub fn with_limits(
        identity: SpaceId,
        faces: &[SurjectiveMap],
        limits: Limits,
        control: &dyn Control,
    ) -> Result<Self> {
        let order = interleaved(faces, control)?;
        Self::with_order(identity, faces, &order, limits, control)
    }

    /// Set physical order and resource policy without changing tuple meaning.
    /// # Errors
    /// Has `new`'s contract, and refuses a wrong coordinate extent or permutation.
    pub fn with_order(
        identity: SpaceId,
        faces: &[SurjectiveMap],
        order: &[u8],
        limits: Limits,
        control: &dyn Control,
    ) -> Result<Self> {
        Self::with_order_and_parameters(
            identity,
            faces,
            order,
            limits,
            crate::ParameterSourceLimits::default(),
            &mut crate::ExactArithmetic::new(crate::ArithmeticLimits::default(), control),
        )
    }

    /// Construct a full product with one shared parameter arithmetic allowance.
    /// Guard attachment and every projection consume the supplied allowance.
    /// # Errors
    /// Has `with_order`'s contract, plus parameter-source and arithmetic capacities.
    pub fn with_order_and_parameters(
        identity: SpaceId,
        faces: &[SurjectiveMap],
        order: &[u8],
        limits: Limits,
        parameters: crate::ParameterSourceLimits,
        work: &mut crate::ExactArithmetic<'_>,
    ) -> Result<Self> {
        let control = work.control();
        control.checkpoint()?;
        let first = faces.first().ok_or(Error::NoFaces)?;
        let mut dimensions = 0usize;
        for face in faces {
            face.map()
                .target()
                .full()
                .align_to(first.map().target(), control)?;
            dimensions = dimensions
                .checked_add(usize::from(face.map().source().dimensions()))
                .ok_or(Error::Capacity(Capacity::Coordinates))?;
        }
        if dimensions > usize::from(MAX_COORDINATES) {
            return Err(Error::Capacity(Capacity::Coordinates));
        }
        if order.len() != dimensions {
            return Err(Error::InvalidOrder);
        }
        let raw = Space::with_order(identity, order, limits, control)?;
        let mut legal = raw.full();
        let mut environment = Vec::new();
        environment.try_reserve_exact(first.map().readouts().len())?;
        let mut offset = 0;
        for (index, face) in faces.iter().enumerate() {
            let source = face.map().source();
            let coordinates = coordinates(offset, source.dimensions());
            let coordinates = &coordinates[..usize::from(source.dimensions())];
            let support = lift_raw(&raw, source, source.0.support, coordinates, control)?;
            legal = legal.apply(BoolOp4::AND, &support, control)?;
            for (bit, readout) in face.map().readouts().iter().enumerate() {
                let value = lift_raw(&raw, source, readout.root, coordinates, control)?;
                if index == 0 {
                    environment.push(value);
                } else {
                    let equal = value.apply(BoolOp4::EQUIVALENCE, &environment[bit], control)?;
                    legal = legal.apply(BoolOp4::AND, &equal, control)?;
                }
            }
            offset += source.dimensions();
        }
        let mut space = raw.restrict(&legal, control)?;
        if let Some(domain) = first.map().source().parameter_domain() {
            let mut guards = Vec::new();
            let mut offset = 0u8;
            for face in faces {
                let source = face.map().source();
                let declared = source
                    .parameter_guards()
                    .ok_or(Error::ParameterScopeMismatch)?;
                guards.try_reserve(declared.len())?;
                for guard in declared {
                    guards.push(crate::ParameterGuard {
                        coordinate: offset + guard.coordinate,
                        region: guard.region.clone(),
                    });
                }
                offset += source.dimensions();
            }
            space = space.with_parameters(domain.clone(), &guards, parameters, work)?;
        }
        let mut projections = Vec::new();
        projections.try_reserve_exact(faces.len())?;
        let mut environments = Vec::new();
        environments.try_reserve_exact(faces.len())?;
        let mut offset = 0;
        for face in faces {
            let source = face.map().source();
            let coordinates = coordinates(offset, source.dimensions());
            let mut readouts = Vec::new();
            readouts.try_reserve_exact(usize::from(source.dimensions()))?;
            for &coordinate in &coordinates[..usize::from(source.dimensions())] {
                readouts.push(space.coordinate(coordinate, control)?);
            }
            let projection =
                CoordinateMap::new_with_parameters(&space, source, &readouts, parameters, work)?
                    .certify_surjective(control)?;
            projections.push(projection);
            environments.push(face.clone());
            offset += source.dimensions();
        }
        control.checkpoint()?;
        Ok(Self(Arc::new(Faces {
            space,
            environments,
            projections,
        })))
    }

    #[must_use]
    pub fn space(&self) -> &Space {
        &self.0.space
    }

    /// Checked projections in authored face order. Coordinates belonging to
    /// unused faces can only be forgotten through a map or quantified operation.
    #[must_use]
    pub fn projections(&self) -> &[SurjectiveMap] {
        &self.0.projections
    }

    #[must_use]
    pub fn environments(&self) -> &[SurjectiveMap] {
        &self.0.environments
    }
}

#[derive(Debug)]
struct Product {
    space: Space,
    left_environment: SurjectiveMap,
    right_environment: SurjectiveMap,
    left: SurjectiveMap,
    right: SurjectiveMap,
}

/// All legal endpoint pairs with the same environment value. Both
/// endpoints have a nonempty fibre at every environment because their maps
/// are certified onto. There is no probability law or independence assertion.
///
/// Original semantic coordinates concatenate left and right codes. Converse
/// exchanges endpoint roles without moving the underlying Event coordinates.
/// Parameterized endpoints capture the same actual parameter assignment;
/// equality of guard codes alone is not equality of real parameters.
#[derive(Debug, Clone)]
pub struct FibreProduct {
    inner: Arc<Product>,
    reversed: bool,
}

/// A commuting square whose joint map reaches every compatible endpoint pair.
/// This is the complete-fibre condition for existential/universal base change;
/// unlike two separate surjectivity certificates, it retains the shared witness.
/// No uniqueness of lifts or law preservation is asserted.
#[derive(Debug, Clone)]
pub struct CompleteFibreSquare {
    product: FibreProduct,
    joint: SurjectiveMap,
    left: CoordinateMap,
    right: CoordinateMap,
}

impl FibreProduct {
    pub(crate) fn is_reversed(&self) -> bool {
        self.reversed
    }

    /// Construct full legal fibres with an interleaved physical order. The
    /// caller names this product's coordinate context explicitly.
    /// # Errors
    /// Refuses incompatible environments, more than 62 combined coordinates,
    /// cancellation or exhausted resources.
    pub fn new(
        identity: SpaceId,
        left: &SurjectiveMap,
        right: &SurjectiveMap,
        control: &dyn Control,
    ) -> Result<Self> {
        Self::with_limits(identity, left, right, Limits::default(), control)
    }

    /// Construct full legal fibres under explicit per-owner kernel limits.
    /// # Errors
    /// Has `new`'s contract plus the supplied capacity bounds.
    pub fn with_limits(
        identity: SpaceId,
        left: &SurjectiveMap,
        right: &SurjectiveMap,
        limits: Limits,
        control: &dyn Control,
    ) -> Result<Self> {
        let order = interleaved(&[left.clone(), right.clone()], control)?;
        Self::with_order(identity, left, right, &order, limits, control)
    }

    /// Choose the working order/limits without changing product coordinates.
    /// # Errors
    /// Has `new`'s contract and refuses an order with incorrect extent/permutation.
    pub fn with_order(
        identity: SpaceId,
        left: &SurjectiveMap,
        right: &SurjectiveMap,
        order: &[u8],
        limits: Limits,
        control: &dyn Control,
    ) -> Result<Self> {
        Self::with_order_and_parameters(
            identity,
            left,
            right,
            order,
            limits,
            crate::ParameterSourceLimits::default(),
            &mut crate::ExactArithmetic::new(crate::ArithmeticLimits::default(), control),
        )
    }

    /// Full fibres with explicit source limits and shared arithmetic.
    /// # Errors
    /// Has `with_order`'s contract, plus parameter-source and arithmetic capacities.
    pub fn with_order_and_parameters(
        identity: SpaceId,
        left: &SurjectiveMap,
        right: &SurjectiveMap,
        order: &[u8],
        limits: Limits,
        parameters: crate::ParameterSourceLimits,
        work: &mut crate::ExactArithmetic<'_>,
    ) -> Result<Self> {
        let faces = FaceProduct::with_order_and_parameters(
            identity,
            &[left.clone(), right.clone()],
            order,
            limits,
            parameters,
            work,
        )?;
        Ok(Self {
            inner: Arc::new(Product {
                space: faces.space().clone(),
                left_environment: left.clone(),
                right_environment: right.clone(),
                left: faces.projections()[0].clone(),
                right: faces.projections()[1].clone(),
            }),
            reversed: false,
        })
    }

    #[must_use]
    pub fn space(&self) -> &Space {
        &self.inner.space
    }

    #[must_use]
    pub fn left(&self) -> &SurjectiveMap {
        if self.reversed {
            &self.inner.right
        } else {
            &self.inner.left
        }
    }

    #[must_use]
    pub fn right(&self) -> &SurjectiveMap {
        if self.reversed {
            &self.inner.left
        } else {
            &self.inner.right
        }
    }

    #[must_use]
    pub fn left_environment(&self) -> &SurjectiveMap {
        if self.reversed {
            &self.inner.right_environment
        } else {
            &self.inner.left_environment
        }
    }

    #[must_use]
    pub fn right_environment(&self) -> &SurjectiveMap {
        if self.reversed {
            &self.inner.left_environment
        } else {
            &self.inner.right_environment
        }
    }

    /// Exchange the two owned face roles. No coordinates, Event roots or named
    /// outcomes are changed, and no random sample is introduced.
    #[must_use]
    pub fn converse(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            reversed: !self.reversed,
        }
    }

    /// Pair two readouts on one source into this product. Checking the original
    /// product support also proves that the square commutes over its environment.
    /// This does not require every product pair to be reached.
    /// # Errors
    /// Refuses context mismatches, incompatible readouts, cancellation or capacity.
    pub fn pair(
        &self,
        left: &CoordinateMap,
        right: &CoordinateMap,
        control: &dyn Control,
    ) -> Result<CoordinateMap> {
        self.pair_with_parameters(
            left,
            right,
            crate::ParameterSourceLimits::default(),
            &mut crate::ExactArithmetic::new(crate::ArithmeticLimits::default(), control),
        )
    }

    /// Pair with explicit source limits and shared arithmetic.
    /// # Errors
    /// Has `pair`'s contract, plus parameter-source and arithmetic capacities.
    pub fn pair_with_parameters(
        &self,
        left: &CoordinateMap,
        right: &CoordinateMap,
        parameters: crate::ParameterSourceLimits,
        work: &mut crate::ExactArithmetic<'_>,
    ) -> Result<CoordinateMap> {
        let control = work.control();
        left.target()
            .full()
            .align_to(self.left().map().target(), control)?;
        right
            .target()
            .full()
            .align_to(self.right().map().target(), control)?;
        right.source().full().align_to(left.source(), control)?;
        let mut readouts = Vec::new();
        readouts.try_reserve_exact(usize::from(self.space().dimensions()))?;
        let (first, second) = if self.reversed {
            (right, left)
        } else {
            (left, right)
        };
        readouts.extend(first.readouts().iter().cloned());
        readouts.extend(second.readouts().iter().cloned());
        CoordinateMap::new_with_parameters(left.source(), self.space(), &readouts, parameters, work)
    }

    /// Check complete joint fibres in addition to commutation.
    /// # Errors
    /// Has `pair`'s contract and refuses a joint image missing any legal pair.
    pub fn certify_square(
        &self,
        left: &CoordinateMap,
        right: &CoordinateMap,
        control: &dyn Control,
    ) -> Result<CompleteFibreSquare> {
        let joint = self
            .pair(left, right, control)?
            .certify_surjective(control)?;
        Ok(CompleteFibreSquare {
            product: self.clone(),
            joint,
            left: left.clone(),
            right: right.clone(),
        })
    }

    /// Reindex into another presentation of the same two endpoint/environment
    /// spaces. Both environment readouts must agree, not merely their ranges.
    /// # Errors
    /// Refuses unequal endpoint contexts or environment maps, cancellation or capacity.
    pub fn map_to(&self, target: &Self, control: &dyn Control) -> Result<CoordinateMap> {
        self.map_to_with_parameters(
            target,
            crate::ParameterSourceLimits::default(),
            &mut crate::ExactArithmetic::new(crate::ArithmeticLimits::default(), control),
        )
    }

    /// Reindex with explicit source limits and shared arithmetic.
    /// # Errors
    /// Has `map_to`'s contract, plus parameter-source and arithmetic capacities.
    pub fn map_to_with_parameters(
        &self,
        target: &Self,
        parameters: crate::ParameterSourceLimits,
        work: &mut crate::ExactArithmetic<'_>,
    ) -> Result<CoordinateMap> {
        let control = work.control();
        require_same_map(
            self.left_environment().map(),
            target.left_environment().map(),
            control,
        )?;
        require_same_map(
            self.right_environment().map(),
            target.right_environment().map(),
            control,
        )?;
        target.pair_with_parameters(self.left().map(), self.right().map(), parameters, work)
    }
}

fn coordinates(offset: u8, dimensions: u8) -> [u8; MAX_COORDINATES as usize] {
    let mut out = [0; MAX_COORDINATES as usize];
    for coordinate in 0..dimensions {
        out[usize::from(coordinate)] = offset + coordinate;
    }
    out
}

fn interleaved(faces: &[SurjectiveMap], control: &dyn Control) -> Result<Vec<u8>> {
    control.checkpoint()?;
    if faces.is_empty() {
        return Err(Error::NoFaces);
    }
    let mut dimensions = 0;
    let mut maximum = 0;
    for face in faces {
        control.checkpoint()?;
        let width = face.map().source().dimensions();
        dimensions += usize::from(width);
        if dimensions > usize::from(MAX_COORDINATES) {
            return Err(Error::Capacity(Capacity::Coordinates));
        }
        maximum = maximum.max(width);
    }
    let mut order = Vec::new();
    order.try_reserve_exact(dimensions)?;
    for coordinate in 0..maximum {
        let mut offset = 0;
        for face in faces {
            control.checkpoint()?;
            let width = face.map().source().dimensions();
            if coordinate < width {
                order.push(offset + coordinate);
            }
            offset += width;
        }
    }
    Ok(order)
}

impl CompleteFibreSquare {
    #[must_use]
    pub fn product(&self) -> &FibreProduct {
        &self.product
    }

    #[must_use]
    pub fn joint(&self) -> &SurjectiveMap {
        &self.joint
    }

    #[must_use]
    pub fn left(&self) -> &CoordinateMap {
        &self.left
    }

    #[must_use]
    pub fn right(&self) -> &CoordinateMap {
        &self.right
    }
}

pub(crate) fn require_same_map(
    left: &CoordinateMap,
    right: &CoordinateMap,
    control: &dyn Control,
) -> Result<()> {
    if !left.equivalent(right, control)? {
        return Err(Error::EnvironmentMismatch);
    }
    Ok(())
}

// Builder-only raw substitution. Target is a fresh full cube. Source support
// is lifted separately before any completed readout is used as a legal witness.
fn lift_raw(
    target: &Space,
    source: &Space,
    root: Ref,
    coordinates: &[u8],
    control: &dyn Control,
) -> Result<Event> {
    let root = target.with_arena_pair(source, |arena, other| {
        let mut op = Operation::new(arena, control)?;
        let mut replacements = [0; MAX_COORDINATES as usize];
        for (replacement, &coordinate) in replacements.iter_mut().zip(coordinates) {
            *replacement = op.variable(coordinate)?;
        }
        op.substitute(
            other.as_deref(),
            root,
            &replacements[..coordinates.len()],
            &mut HashMap::new(),
        )
    })?;
    control.checkpoint()?;
    Ok(target.event(root))
}
