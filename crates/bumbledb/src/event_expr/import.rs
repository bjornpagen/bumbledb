//! Retained structural imports: inspectable data and its reconstructed owner.
//! Equality is canonical descriptor equality, never arena allocation identity.
use std::sync::Arc;

use crate::event::{
    AdmittedDescriptor, Control, CoordinateMap, Descriptor, DescriptorLimits, MapDescriptor, MapOp,
    Result, Space,
};

#[derive(Debug)]
struct Import {
    data: Descriptor,
    bytes: Box<[u8]>,
    admitted: AdmittedDescriptor,
    fingerprint: blake3::Hash,
}

/// An immutable, checked descriptor retained by query IR. It owns its maps and
/// exposes the exact portable data. Import runs before query execution; there
/// are no host callbacks, implicit names or source allocations in a query head.
#[derive(Clone)]
pub struct EventImport(Arc<Import>);

impl std::fmt::Debug for EventImport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventImport")
            .field("bedc_bytes", &self.0.bytes.len())
            .field("fingerprint", &self.0.fingerprint)
            .finish()
    }
}

impl PartialEq for EventImport {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0.bytes == other.0.bytes
    }
}
impl Eq for EventImport {}

impl EventImport {
    /// Capture an already checked native descriptor as canonical query data.
    /// # Errors
    /// Encoding, descriptor budget, allocation or cancellation refusal.
    pub fn capture(
        value: &AdmittedDescriptor,
        limits: DescriptorLimits,
        control: &dyn Control,
    ) -> Result<Self> {
        let data = Descriptor::capture(value, limits, control)?;
        let bytes = data.to_bytes(limits, control)?.into_boxed_slice();
        let fingerprint = blake3::hash(&bytes);
        Ok(Self(Arc::new(Import {
            data,
            bytes,
            admitted: value.clone(),
            fingerprint,
        })))
    }

    /// Reconstruct every certificate before retaining untrusted descriptor data.
    /// # Errors
    /// The checked BEDC admission and capture refusal contracts.
    pub fn admit(
        data: &Descriptor,
        limits: DescriptorLimits,
        control: &dyn Control,
    ) -> Result<Self> {
        Self::capture(&data.admit(limits, control)?, limits, control)
    }

    /// Import canonical query data from BEDC bytes and recheck all certificates.
    /// # Errors
    /// The BEDC parser, semantic admission and capture refusal contracts.
    pub fn from_bytes(
        bytes: &[u8],
        limits: DescriptorLimits,
        control: &dyn Control,
    ) -> Result<Self> {
        Self::admit(
            &Descriptor::from_bytes(bytes, limits, control)?,
            limits,
            control,
        )
    }

    #[must_use]
    pub fn descriptor(&self) -> &Descriptor {
        &self.0.data
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.0.bytes
    }

    pub(super) fn map(&self) -> Option<(&MapDescriptor, &CoordinateMap)> {
        match (&self.0.data, &self.0.admitted) {
            (Descriptor::Map(data), AdmittedDescriptor::Map(map)) => Some((data, map)),
            (Descriptor::Surjective(data), AdmittedDescriptor::Surjective(map)) => {
                Some((data, map.map()))
            }
            _ => None,
        }
    }

    pub(crate) fn map_spaces(&self, operation: MapOp) -> Option<(&Space, &Space)> {
        let (_, map) = self.map()?;
        Some(match operation {
            MapOp::Pullback => (map.target(), map.source()),
            MapOp::Image | MapOp::UniversalImage | MapOp::NonvacuousImage => {
                (map.source(), map.target())
            }
            MapOp::Possible | MapOp::Guaranteed => (map.source(), map.source()),
        })
    }

    pub(super) fn map_markers(&self, operation: MapOp) -> Option<(&[u8], &[u8])> {
        let (data, _) = self.map()?;
        Some(match operation {
            MapOp::Pullback => (&data.target, &data.source),
            MapOp::Image | MapOp::UniversalImage | MapOp::NonvacuousImage => {
                (&data.source, &data.target)
            }
            MapOp::Possible | MapOp::Guaranteed => (&data.source, &data.source),
        })
    }

    pub(crate) fn evaluate_map(
        &self,
        operation: MapOp,
        value: &crate::Event,
        control: &dyn Control,
    ) -> Result<crate::Event> {
        let (_, map) = self.map().expect("query shape admits the map kind");
        match operation {
            MapOp::Pullback => map.pullback(value, control),
            MapOp::Image => map.image(value, control),
            MapOp::UniversalImage => map.universal_image(value, control),
            MapOp::NonvacuousImage => map.nonvacuous_image(value, control),
            MapOp::Possible => map.possible(value, control),
            MapOp::Guaranteed => map.guaranteed(value, control),
        }
    }
}
