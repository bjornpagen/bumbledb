//! Plain, portable descriptions of finite structural maps and relation roles.
//! Admission reconstructs every certificate; no serialized claim is trusted.
use crate::{
    Capacity, CompleteFibreSquare, Control, CoordinateMap, Error, Event, FaceProduct, FibreProduct,
    Limits, RelationalProduct, Result, Space, SpaceId, WorldRelation,
};

mod wire;

/// Source and target are canonical full-space BEVT values. Readouts are
/// canonical Events on the source, in target semantic-coordinate order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapDescriptor {
    pub source: Vec<u8>,
    pub target: Vec<u8>,
    pub readouts: Vec<Vec<u8>>,
}

/// The two environment maps are in original semantic concatenation order.
/// Reversal exchanges endpoint roles without exchanging stored coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FibreDescriptor {
    pub identity: SpaceId,
    pub left: Box<MapDescriptor>,
    pub right: Box<MapDescriptor>,
    pub reversed: bool,
}

/// Untrusted, inspectable data. `admit` is the sole conversion to executable
/// maps/products. Names remain authored; import never invents a new source.
///
/// ```
/// use bumbledb_event::{AdmittedDescriptor, CoordinateMap, Descriptor,
///     DescriptorLimits, Error, Space, SpaceId};
/// let space = Space::new(SpaceId([1; 32]), 1, &())?;
/// let identity = CoordinateMap::coordinates(&space, &space, &[0], &())?;
/// let limits = DescriptorLimits::default();
/// let data = Descriptor::capture(&AdmittedDescriptor::Map(identity), limits, &())?;
/// let bytes = data.to_bytes(limits, &())?;
/// let AdmittedDescriptor::Map(restored) = Descriptor::import(&bytes, limits, &())?
///     else { unreachable!() };
/// assert_eq!(restored.map_world(1)?, 1);
/// # Ok::<(), Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Descriptor {
    Map(MapDescriptor),
    Surjective(MapDescriptor),
    Faces {
        identity: SpaceId,
        environments: Vec<MapDescriptor>,
    },
    Fibre(FibreDescriptor),
    Relation {
        product: FibreDescriptor,
        region: Vec<u8>,
    },
    Composition {
        identity: SpaceId,
        st: FibreDescriptor,
        tu: FibreDescriptor,
        su: FibreDescriptor,
    },
    Square {
        product: FibreDescriptor,
        left: MapDescriptor,
        right: MapDescriptor,
    },
}

/// Constructor-checked values. Certificates are recreated from the full
/// descriptor, including surjectivity, joint fibres and environment agreement.
#[derive(Debug, Clone)]
pub enum AdmittedDescriptor {
    Map(CoordinateMap),
    Surjective(crate::SurjectiveMap),
    Faces(FaceProduct),
    Fibre(FibreProduct),
    Relation(WorldRelation),
    Composition(RelationalProduct),
    Square(CompleteFibreSquare),
}

/// Input payload/roster limits plus per-owner Event kernel limits. These bound
/// admission workspaces; they are not a total allocator or retained-memory quota.
#[derive(Debug, Clone, Copy)]
pub struct DescriptorLimits {
    pub bytes: usize,
    pub items: usize,
    pub events: Limits,
}

impl Default for DescriptorLimits {
    fn default() -> Self {
        Self {
            bytes: 16 * 1024 * 1024,
            items: 4096,
            events: Limits::default(),
        }
    }
}

struct Budget {
    limits: DescriptorLimits,
    bytes: usize,
    items: usize,
}

impl Budget {
    fn new(limits: DescriptorLimits) -> Self {
        Self {
            limits,
            bytes: 0,
            items: 0,
        }
    }

    fn item(&mut self, bytes: usize) -> Result<()> {
        self.items = self
            .items
            .checked_add(1)
            .ok_or(Error::Capacity(Capacity::DescriptorItems))?;
        self.bytes = self
            .bytes
            .checked_add(bytes)
            .ok_or(Error::Capacity(Capacity::DescriptorBytes))?;
        if self.items > self.limits.items {
            return Err(Error::Capacity(Capacity::DescriptorItems));
        }
        if self.bytes > self.limits.bytes {
            return Err(Error::Capacity(Capacity::DescriptorBytes));
        }
        Ok(())
    }

    fn event(&mut self, value: &Event, control: &dyn Control) -> Result<Vec<u8>> {
        let bytes = value.to_bytes(control)?;
        self.item(bytes.len())?;
        Ok(bytes)
    }
}

impl MapDescriptor {
    fn capture(value: &CoordinateMap, budget: &mut Budget, control: &dyn Control) -> Result<Self> {
        control.checkpoint()?;
        budget.item(0)?;
        let source = budget.event(&value.source().full(), control)?;
        let target = budget.event(&value.target().full(), control)?;
        let mut readouts = Vec::new();
        readouts.try_reserve_exact(value.readouts().len())?;
        for value in value.readouts() {
            readouts.push(budget.event(value, control)?);
        }
        Ok(Self {
            source,
            target,
            readouts,
        })
    }

    fn preflight(&self, budget: &mut Budget, control: &dyn Control) -> Result<()> {
        budget.item(0)?;
        for bytes in std::iter::once(&self.source)
            .chain([&self.target])
            .chain(&self.readouts)
        {
            control.checkpoint()?;
            budget.item(bytes.len())?;
        }
        Ok(())
    }

    fn admit(&self, limits: DescriptorLimits, control: &dyn Control) -> Result<CoordinateMap> {
        let source = full_space(&self.source, limits, control)?;
        let target = full_space(&self.target, limits, control)?;
        if self.readouts.len() != usize::from(target.dimensions()) {
            return Err(Error::MapArity);
        }
        let mut readouts = Vec::new();
        readouts.try_reserve_exact(self.readouts.len())?;
        for bytes in &self.readouts {
            readouts.push(event(bytes, limits, control)?);
        }
        CoordinateMap::new(&source, &target, &readouts, control)
    }
}

impl FibreDescriptor {
    fn capture(value: &FibreProduct, budget: &mut Budget, control: &dyn Control) -> Result<Self> {
        budget.item(0)?;
        let base = if value.is_reversed() {
            value.converse()
        } else {
            value.clone()
        };
        Ok(Self {
            identity: value.space().identity(),
            left: Box::new(MapDescriptor::capture(
                base.left_environment().map(),
                budget,
                control,
            )?),
            right: Box::new(MapDescriptor::capture(
                base.right_environment().map(),
                budget,
                control,
            )?),
            reversed: value.is_reversed(),
        })
    }

    fn preflight(&self, budget: &mut Budget, control: &dyn Control) -> Result<()> {
        budget.item(0)?;
        self.left.preflight(budget, control)?;
        self.right.preflight(budget, control)
    }

    fn admit(&self, limits: DescriptorLimits, control: &dyn Control) -> Result<FibreProduct> {
        let left = self
            .left
            .admit(limits, control)?
            .certify_surjective(control)?;
        let right = self
            .right
            .admit(limits, control)?
            .certify_surjective(control)?;
        let dimensions = left.map().source().dimensions() + right.map().source().dimensions();
        let order: Vec<_> = (0..dimensions).collect();
        let value =
            FibreProduct::with_order(self.identity, &left, &right, &order, limits.events, control)?;
        Ok(if self.reversed {
            value.converse()
        } else {
            value
        })
    }
}

impl Descriptor {
    /// Capture canonical data, independent of arenas and physical graph order.
    /// # Errors
    /// Refuses encoding failure, cancellation or explicit descriptor limits.
    pub fn capture(
        value: &AdmittedDescriptor,
        limits: DescriptorLimits,
        control: &dyn Control,
    ) -> Result<Self> {
        control.checkpoint()?;
        let mut budget = Budget::new(limits);
        budget.item(0)?;
        Ok(match value {
            AdmittedDescriptor::Map(value) => {
                Self::Map(MapDescriptor::capture(value, &mut budget, control)?)
            }
            AdmittedDescriptor::Surjective(value) => {
                Self::Surjective(MapDescriptor::capture(value.map(), &mut budget, control)?)
            }
            AdmittedDescriptor::Faces(value) => {
                let mut environments = Vec::new();
                if value.environments().len() > limits.items {
                    return Err(Error::Capacity(Capacity::DescriptorItems));
                }
                environments.try_reserve_exact(value.environments().len())?;
                for map in value.environments() {
                    environments.push(MapDescriptor::capture(map.map(), &mut budget, control)?);
                }
                Self::Faces {
                    identity: value.space().identity(),
                    environments,
                }
            }
            AdmittedDescriptor::Fibre(value) => {
                Self::Fibre(FibreDescriptor::capture(value, &mut budget, control)?)
            }
            AdmittedDescriptor::Relation(value) => Self::Relation {
                product: FibreDescriptor::capture(value.product(), &mut budget, control)?,
                region: budget.event(value.region(), control)?,
            },
            AdmittedDescriptor::Composition(value) => {
                let [st, tu, su] = value.products();
                Self::Composition {
                    identity: value.workspace().space().identity(),
                    st: FibreDescriptor::capture(st, &mut budget, control)?,
                    tu: FibreDescriptor::capture(tu, &mut budget, control)?,
                    su: FibreDescriptor::capture(su, &mut budget, control)?,
                }
            }
            AdmittedDescriptor::Square(value) => Self::Square {
                product: FibreDescriptor::capture(value.product(), &mut budget, control)?,
                left: MapDescriptor::capture(value.left(), &mut budget, control)?,
                right: MapDescriptor::capture(value.right(), &mut budget, control)?,
            },
        })
    }

    fn preflight(&self, limits: DescriptorLimits, control: &dyn Control) -> Result<()> {
        control.checkpoint()?;
        let mut budget = Budget::new(limits);
        budget.item(0)?;
        match self {
            Self::Map(map) | Self::Surjective(map) => map.preflight(&mut budget, control),
            Self::Faces { environments, .. } => {
                for map in environments {
                    map.preflight(&mut budget, control)?;
                }
                Ok(())
            }
            Self::Fibre(product) => product.preflight(&mut budget, control),
            Self::Relation { product, region } => {
                product.preflight(&mut budget, control)?;
                budget.item(region.len())
            }
            Self::Composition { st, tu, su, .. } => {
                for product in [st, tu, su] {
                    product.preflight(&mut budget, control)?;
                }
                Ok(())
            }
            Self::Square {
                product,
                left,
                right,
            } => {
                product.preflight(&mut budget, control)?;
                left.preflight(&mut budget, control)?;
                right.preflight(&mut budget, control)
            }
        }
    }

    /// Reconstruct checked owners and proofs from untrusted plain data.
    /// Every space marker must be full. Products reconstruct complete support;
    /// onto maps, joint squares and shared environments are checked anew.
    /// # Errors
    /// Malformed bytes, wrong supports/roles, incomplete images, cancellation
    /// or capacity. A failed import publishes no partially checked descriptor.
    pub fn admit(
        &self,
        limits: DescriptorLimits,
        control: &dyn Control,
    ) -> Result<AdmittedDescriptor> {
        self.preflight(limits, control)?;
        let result = match self {
            Self::Map(value) => AdmittedDescriptor::Map(value.admit(limits, control)?),
            Self::Surjective(value) => AdmittedDescriptor::Surjective(
                value.admit(limits, control)?.certify_surjective(control)?,
            ),
            Self::Faces {
                identity,
                environments,
            } => {
                let mut faces = Vec::new();
                faces.try_reserve_exact(environments.len())?;
                for map in environments {
                    faces.push(map.admit(limits, control)?.certify_surjective(control)?);
                }
                AdmittedDescriptor::Faces(FaceProduct::with_limits(
                    *identity,
                    &faces,
                    limits.events,
                    control,
                )?)
            }
            Self::Fibre(value) => AdmittedDescriptor::Fibre(value.admit(limits, control)?),
            Self::Relation { product, region } => AdmittedDescriptor::Relation(WorldRelation::new(
                &product.admit(limits, control)?,
                &event(region, limits, control)?,
                control,
            )?),
            Self::Composition {
                identity,
                st,
                tu,
                su,
            } => AdmittedDescriptor::Composition(RelationalProduct::with_limits(
                *identity,
                &st.admit(limits, control)?,
                &tu.admit(limits, control)?,
                &su.admit(limits, control)?,
                limits.events,
                control,
            )?),
            Self::Square {
                product,
                left,
                right,
            } => AdmittedDescriptor::Square(product.admit(limits, control)?.certify_square(
                &left.admit(limits, control)?,
                &right.admit(limits, control)?,
                control,
            )?),
        };
        control.checkpoint()?;
        Ok(result)
    }
}

fn event(bytes: &[u8], limits: DescriptorLimits, control: &dyn Control) -> Result<Event> {
    Event::from_bytes_with_order(bytes, None, limits.events, control)
}

fn full_space(bytes: &[u8], limits: DescriptorLimits, control: &dyn Control) -> Result<Space> {
    let value = event(bytes, limits, control)?;
    if !value.is_full() {
        return Err(Error::InvalidEncoding);
    }
    Ok(value.space())
}
