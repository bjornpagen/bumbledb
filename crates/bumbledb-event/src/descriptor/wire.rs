//! BEDC v1: a fixed-depth descriptor grammar. Parsing yields untrusted data;
//! `Descriptor::import` additionally reconstructs all semantic certificates.
use super::{
    AdmittedDescriptor, Budget, Capacity, Control, Descriptor, DescriptorLimits, Error,
    FibreDescriptor, MapDescriptor, Result, SpaceId,
};

impl Descriptor {
    /// Encode this plain descriptor. Semantic admission is separate; encoded
    /// bytes do not certify that its map, relation or square is constructible.
    /// # Errors
    /// Descriptor extent, allocation or cancellation refusal.
    pub fn to_bytes(&self, limits: DescriptorLimits, control: &dyn Control) -> Result<Vec<u8>> {
        self.preflight(limits, control)?;
        let mut out = Writer {
            bytes: Vec::new(),
            limits,
            control,
        };
        out.put(b"BEDC\x01")?;
        match self {
            Self::Map(value) => {
                out.put(&[0])?;
                out.map(value)?;
            }
            Self::Surjective(value) => {
                out.put(&[1])?;
                out.map(value)?;
            }
            Self::Faces {
                identity,
                environments,
            } => {
                out.put(&[2])?;
                out.put(&identity.0)?;
                out.number(environments.len())?;
                for map in environments {
                    out.map(map)?;
                }
            }
            Self::Fibre(value) => {
                out.put(&[3])?;
                out.fibre(value)?;
            }
            Self::Relation { product, region } => {
                out.put(&[4])?;
                out.fibre(product)?;
                out.blob(region)?;
            }
            Self::Composition {
                identity,
                st,
                tu,
                su,
            } => {
                out.put(&[5])?;
                out.put(&identity.0)?;
                for product in [st, tu, su] {
                    out.fibre(product)?;
                }
            }
            Self::Square {
                product,
                left,
                right,
            } => {
                out.put(&[6])?;
                out.fibre(product)?;
                out.map(left)?;
                out.map(right)?;
            }
        }
        control.checkpoint()?;
        Ok(out.bytes)
    }

    /// Parse BEDC syntax into plain data. No certificate is trusted or returned.
    /// For executable imports use `import`, which also calls `admit`.
    /// # Errors
    /// Truncation, trailing bytes, unknown tags/versions, extent or resources.
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
        if input.take(4)? != b"BEDC" {
            return Err(Error::InvalidEncoding);
        }
        let version = input.take(1)?[0];
        if version != 1 {
            return Err(Error::UnsupportedVersion(version));
        }
        let value = match input.take(1)?[0] {
            0 => Self::Map(input.map()?),
            1 => Self::Surjective(input.map()?),
            2 => {
                let identity = input.identity()?;
                let count = input.count()?;
                let mut environments = Vec::new();
                environments.try_reserve_exact(count)?;
                for _ in 0..count {
                    environments.push(input.map()?);
                }
                Self::Faces {
                    identity,
                    environments,
                }
            }
            3 => Self::Fibre(input.fibre()?),
            4 => Self::Relation {
                product: input.fibre()?,
                region: input.blob()?,
            },
            5 => Self::Composition {
                identity: input.identity()?,
                st: input.fibre()?,
                tu: input.fibre()?,
                su: input.fibre()?,
            },
            6 => Self::Square {
                product: input.fibre()?,
                left: input.map()?,
                right: input.map()?,
            },
            _ => return Err(Error::InvalidEncoding),
        };
        if !input.bytes.is_empty() {
            return Err(Error::InvalidEncoding);
        }
        control.checkpoint()?;
        Ok(value)
    }

    /// Parse and reconstruct an executable descriptor through checked native
    /// constructors. Format tags for surjectivity/squares request those checks.
    /// # Errors
    /// Has the refusal contracts of `from_bytes` and `admit`.
    pub fn import(
        bytes: &[u8],
        limits: DescriptorLimits,
        control: &dyn Control,
    ) -> Result<AdmittedDescriptor> {
        Self::from_bytes(bytes, limits, control)?.admit(limits, control)
    }
}

pub(super) struct Writer<'a> {
    pub(super) bytes: Vec<u8>,
    pub(super) limits: DescriptorLimits,
    pub(super) control: &'a dyn Control,
}

impl Writer<'_> {
    pub(super) fn put(&mut self, bytes: &[u8]) -> Result<()> {
        let size = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or(Error::Capacity(Capacity::DescriptorBytes))?;
        if size > self.limits.bytes {
            return Err(Error::Capacity(Capacity::DescriptorBytes));
        }
        self.control.checkpoint()?;
        self.bytes.try_reserve(bytes.len())?;
        for chunk in bytes.chunks(65_536) {
            self.control.checkpoint()?;
            self.bytes.extend_from_slice(chunk);
        }
        Ok(())
    }
    pub(super) fn number(&mut self, number: usize) -> Result<()> {
        self.put(&(number as u64).to_le_bytes())
    }
    pub(super) fn blob(&mut self, bytes: &[u8]) -> Result<()> {
        self.number(bytes.len())?;
        self.put(bytes)
    }
    pub(super) fn map(&mut self, value: &MapDescriptor) -> Result<()> {
        self.blob(&value.source)?;
        self.blob(&value.target)?;
        self.number(value.readouts.len())?;
        for bytes in &value.readouts {
            self.blob(bytes)?;
        }
        Ok(())
    }
    fn fibre(&mut self, value: &FibreDescriptor) -> Result<()> {
        self.put(&value.identity.0)?;
        self.put(&[u8::from(value.reversed)])?;
        self.map(&value.left)?;
        self.map(&value.right)
    }
}

pub(super) struct Reader<'a> {
    pub(super) bytes: &'a [u8],
    pub(super) budget: Budget,
    pub(super) control: &'a dyn Control,
}

impl<'a> Reader<'a> {
    pub(super) fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        self.control.checkpoint()?;
        let (value, rest) = self
            .bytes
            .split_at_checked(count)
            .ok_or(Error::InvalidEncoding)?;
        self.bytes = rest;
        Ok(value)
    }
    pub(super) fn number(&mut self) -> Result<usize> {
        usize::try_from(u64::from_le_bytes(
            self.take(8)?.try_into().expect("eight bytes"),
        ))
        .map_err(|_| Error::InvalidEncoding)
    }
    pub(super) fn count(&mut self) -> Result<usize> {
        let count = self.number()?;
        if count > self.budget.limits.items.saturating_sub(self.budget.items) {
            return Err(Error::Capacity(Capacity::DescriptorItems));
        }
        // Every roster entry must have at least an eight-byte length field.
        if count > self.bytes.len() / 8 {
            return Err(Error::InvalidEncoding);
        }
        Ok(count)
    }
    fn identity(&mut self) -> Result<SpaceId> {
        Ok(SpaceId(self.take(32)?.try_into().expect("32 bytes")))
    }
    pub(super) fn blob(&mut self) -> Result<Vec<u8>> {
        let count = self.number()?;
        self.budget.item(count)?;
        let bytes = self.take(count)?;
        let mut result = Vec::new();
        result.try_reserve_exact(count)?;
        for chunk in bytes.chunks(65_536) {
            self.control.checkpoint()?;
            result.extend_from_slice(chunk);
        }
        Ok(result)
    }
    pub(super) fn map(&mut self) -> Result<MapDescriptor> {
        self.budget.item(0)?;
        let source = self.blob()?;
        let target = self.blob()?;
        let count = self.count()?;
        let mut readouts = Vec::new();
        readouts.try_reserve_exact(count)?;
        for _ in 0..count {
            readouts.push(self.blob()?);
        }
        Ok(MapDescriptor {
            source,
            target,
            readouts,
        })
    }
    fn fibre(&mut self) -> Result<FibreDescriptor> {
        self.budget.item(0)?;
        let identity = self.identity()?;
        let reversed = match self.take(1)?[0] {
            0 => false,
            1 => true,
            _ => return Err(Error::InvalidEncoding),
        };
        Ok(FibreDescriptor {
            identity,
            left: Box::new(self.map()?),
            right: Box::new(self.map()?),
            reversed,
        })
    }
}
