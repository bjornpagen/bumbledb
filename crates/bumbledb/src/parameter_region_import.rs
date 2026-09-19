//! Owned exact parameter-region query import. Capturing a region neither names
//! an ambient domain nor chooses a prior; both are separate explicit inputs.
use crate::Result;
use crate::event::{ExactArithmetic, ParameterCodecLimits, ParameterRegion};
use std::sync::Arc;

#[derive(Debug)]
struct Region {
    value: ParameterRegion,
    bytes: Box<[u8]>,
}
#[derive(Debug, Clone)]
pub struct ParameterRegionImport(Arc<Region>);
impl PartialEq for ParameterRegionImport {
    fn eq(&self, other: &Self) -> bool {
        self.0.bytes == other.0.bytes
    }
}
impl Eq for ParameterRegionImport {}
impl ParameterRegionImport {
    /// Retain the complete canonical region, including empty/disconnected sets.
    /// # Errors
    /// Solver/encoding bounds or cancellation.
    pub fn capture(
        value: ParameterRegion,
        limits: ParameterCodecLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let bytes = value.to_bytes(limits, work)?.into_boxed_slice();
        Ok(Self(Arc::new(Region { value, bytes })))
    }
    /// # Errors
    /// Malformed/noncanonical regions, solver/encoding bounds or cancellation.
    pub fn from_bytes(
        bytes: &[u8],
        limits: ParameterCodecLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        Self::capture(
            ParameterRegion::from_bytes(bytes, limits, work)?,
            limits,
            work,
        )
    }
    #[must_use]
    pub fn value(&self) -> &ParameterRegion {
        &self.0.value
    }
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.0.bytes
    }
}
