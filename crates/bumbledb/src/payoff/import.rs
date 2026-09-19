//! Closed, owned numerical imports. The bytes retain a deterministic presentation;
//! source functions keep their full context even when used on an empty patch.
use crate::event::{
    AdmittedFamilyDescriptor, AdmittedSourceDescriptor, Capacity, Error, ExactArithmetic,
    ExactRational, FamilyFunction, FiniteFunction, Result, SourceDescriptor,
    SourceDescriptorLimits, Space,
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum ImportedPayoff {
    Rational(ExactRational),
    Finite(FiniteFunction),
    Family(FamilyFunction),
}
impl ImportedPayoff {
    #[must_use]
    pub fn space(&self) -> Option<&Space> {
        match self {
            Self::Rational(_) => None,
            Self::Finite(value) => Some(value.space()),
            Self::Family(value) => Some(value.space()),
        }
    }
}

#[derive(Debug)]
struct Import {
    value: ImportedPayoff,
    bytes: Box<[u8]>,
    marker: Option<Box<[u8]>>,
}

/// An admitted exact constant or total source-owned function. Equality compares
/// encoded presentations, not solver-checked family equivalence. Query syntax
/// retains data and owners; it never executes a host callback to obtain payoffs.
#[derive(Debug, Clone)]
pub struct PayoffImport(Arc<Import>);

impl PartialEq for PayoffImport {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0.bytes == other.0.bytes
    }
}
impl Eq for PayoffImport {}
impl std::hash::Hash for PayoffImport {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.bytes.hash(state);
    }
}

impl PayoffImport {
    /// Capture an already admitted value as bounded BERA/BESC query data.
    /// # Errors
    /// Encoding, resource limits or cancellation.
    pub fn capture(
        value: ImportedPayoff,
        limits: SourceDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let bytes = match &value {
            ImportedPayoff::Rational(value) => value.to_bytes(work)?,
            ImportedPayoff::Finite(value) => SourceDescriptor::capture(
                &AdmittedSourceDescriptor::Function(value.clone()),
                limits,
                work,
            )?
            .to_bytes(limits, work.control())?,
            ImportedPayoff::Family(value) => SourceDescriptor::capture(
                &AdmittedSourceDescriptor::Family(Box::new(AdmittedFamilyDescriptor::Function(
                    value.clone(),
                ))),
                limits,
                work,
            )?
            .to_bytes(limits, work.control())?,
        };
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(Error::Capacity(Capacity::DescriptorBytes));
        }
        let marker = value
            .space()
            .map(|space| {
                space
                    .full()
                    .to_bytes(work.control())
                    .map(Vec::into_boxed_slice)
            })
            .transpose()?;
        Ok(Self(Arc::new(Import {
            value,
            bytes: bytes.into_boxed_slice(),
            marker,
        })))
    }

    /// Reconstruct and retain a rational or total function. Other BESC roles,
    /// including partial parameter-only functions, cannot be payoff imports.
    /// # Errors
    /// Malformed/wrong-role data, source admission, limits or cancellation.
    pub fn from_bytes(
        bytes: &[u8],
        limits: SourceDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(Error::Capacity(Capacity::DescriptorBytes));
        }
        let value = if bytes.starts_with(b"BERA") {
            ImportedPayoff::Rational(ExactRational::from_bytes(bytes, work)?)
        } else {
            match SourceDescriptor::import(bytes, limits, work)? {
                AdmittedSourceDescriptor::Function(value) => ImportedPayoff::Finite(value),
                AdmittedSourceDescriptor::Family(value) => match *value {
                    AdmittedFamilyDescriptor::Function(value) => ImportedPayoff::Family(value),
                    _ => return Err(Error::InvalidEncoding),
                },
                _ => return Err(Error::InvalidEncoding),
            }
        };
        Self::capture(value, limits, work)
    }

    #[must_use]
    pub fn value(&self) -> &ImportedPayoff {
        &self.0.value
    }
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.0.bytes
    }
    #[must_use]
    pub fn space(&self) -> Option<&Space> {
        self.value().space()
    }
    pub(crate) fn marker(&self) -> Option<&[u8]> {
        self.0.marker.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{
        ArithmeticLimits, FunctionLimits, ParameterDomain, ParameterFunction, ParameterId,
        ParameterRegion, ParameterSourceLimits,
    };

    #[test]
    fn imports_reject_other_roles_malformed_encodings_and_share_work() {
        let limits = SourceDescriptorLimits::default();
        let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &());
        let bytes =
            ExactRational::fraction("123456789012345678901234567890123456789", "7", &mut work)
                .unwrap()
                .to_bytes(&mut work)
                .unwrap();
        let mut first = ExactArithmetic::new(ArithmeticLimits::default(), &());
        let imported = PayoffImport::from_bytes(&bytes, limits, &mut first).unwrap();
        assert!(imported.space().is_none());
        assert_eq!(imported.bytes(), bytes);
        let mut shared = ExactArithmetic::new(
            ArithmeticLimits {
                operations: first.operations(),
                ..ArithmeticLimits::default()
            },
            &(),
        );
        PayoffImport::from_bytes(&bytes, limits, &mut shared).unwrap();
        assert!(matches!(
            PayoffImport::from_bytes(&bytes, limits, &mut shared),
            Err(Error::Capacity(Capacity::ArithmeticSteps))
        ));
        for bad in [
            &b"BERA\x01"[..],
            &b"BESC\xff"[..],
            &bytes[..bytes.len() - 1],
        ] {
            assert!(PayoffImport::from_bytes(bad, limits, &mut work).is_err());
        }
        let domain = ParameterDomain::new(ParameterRegion::full(ParameterId([243; 32]))).unwrap();
        let parameter = ParameterFunction::new(
            domain,
            &[],
            ParameterSourceLimits::default().parameters.region,
            FunctionLimits::default(),
            &mut work,
        )
        .unwrap();
        let bytes = SourceDescriptor::capture(
            &AdmittedSourceDescriptor::Family(Box::new(AdmittedFamilyDescriptor::Parameter(
                parameter,
            ))),
            limits,
            &mut work,
        )
        .unwrap()
        .to_bytes(limits, &())
        .unwrap();
        assert!(matches!(
            PayoffImport::from_bytes(&bytes, limits, &mut work),
            Err(Error::InvalidEncoding)
        ));
        let control = crate::WorkContext::new();
        control.cancel();
        assert!(matches!(
            PayoffImport::from_bytes(
                imported.bytes(),
                limits,
                &mut ExactArithmetic::new(ArithmeticLimits::default(), &control)
            ),
            Err(Error::Cancelled)
        ));
    }
}
