//! Owned numerical answers retain BENO derivation identity beside the exact
//! partial value. Delivery never collapses an undefined observation to zero.
use crate::query_probability::{ObservationOutputWork, function};
use bumbledb::ObservationNumberImport;
use bumbledb::event::{Capacity, Control, Error, ParameterCodecLimits, PartialNumber};
use napi::bindgen_prelude::{Env, Object, Uint8Array};

#[derive(Debug)]
pub struct NumberOutput {
    bytes: Vec<u8>,
    law: &'static str,
    value: Option<Vec<u8>>,
    defined: Option<Vec<u8>>,
    domain: Option<Vec<u8>>,
}
impl NumberOutput {
    pub(crate) fn new(
        number: &ObservationNumberImport,
        control: &dyn Control,
        budget: &mut ObservationOutputWork<'_>,
    ) -> bumbledb::event::Result<Self> {
        control.checkpoint()?;
        budget.remaining = budget
            .remaining
            .checked_sub(number.bytes().len())
            .ok_or(Error::Capacity(Capacity::DescriptorBytes))?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(number.bytes().len())?;
        bytes.extend_from_slice(number.bytes());
        let work = &mut budget.arithmetic;
        let result = match number.value().value() {
            PartialNumber::Fixed(value) => Self {
                bytes,
                law: "fixed",
                value: value.as_ref().map(|v| v.to_bytes(work)).transpose()?,
                defined: None,
                domain: None,
            },
            PartialNumber::Parameter(value) => Self {
                bytes,
                law: "parameter",
                value: Some(function(value, control, work)?),
                defined: Some(
                    value
                        .defined_on()
                        .to_bytes(ParameterCodecLimits::default(), work)?,
                ),
                domain: Some(
                    value
                        .ambient()
                        .to_bytes(ParameterCodecLimits::default(), work)?,
                ),
            },
        };
        for bytes in result
            .value
            .iter()
            .chain(result.defined.iter())
            .chain(result.domain.iter())
        {
            budget.remaining = budget
                .remaining
                .checked_sub(bytes.len())
                .ok_or(Error::Capacity(Capacity::DescriptorBytes))?;
        }
        control.checkpoint()?;
        Ok(result)
    }
    pub(crate) fn object(self, env: &Env) -> napi::Result<Object<'_>> {
        let mut object = Object::new(env)?;
        object.set("kind", "number")?;
        object.set("bytes", Uint8Array::from(self.bytes))?;
        object.set("law", self.law)?;
        object.set("value", self.value.map(Uint8Array::from))?;
        object.set("defined", self.defined.map(Uint8Array::from))?;
        object.set("domain", self.domain.map(Uint8Array::from))?;
        Ok(object)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumbledb::event::{
        ArithmeticLimits, ExactArithmetic, ExactRational, NumberOp, ParameterDomain, ParameterId,
        ParameterRegion,
    };
    use bumbledb::{ObservationNumber, ObservationNumberCodecLimits, ObservationNumberLimits};

    #[test]
    fn number_delivery_keeps_undefinedness_domains_identity_and_batch_limits() {
        let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &());
        let limits = ObservationNumberLimits::default();
        let one = ObservationNumber::literal(ExactRational::one(), limits, &mut work).unwrap();
        let zero = ObservationNumber::literal(ExactRational::zero(), limits, &mut work).unwrap();
        let undefined = one
            .apply(NumberOp::Divide, &zero, limits, &mut work)
            .unwrap();
        let domain = ParameterDomain::new(ParameterRegion::full(ParameterId([233; 32]))).unwrap();
        let partial = undefined.on_domain(&domain, limits, &mut work).unwrap();
        for value in [&one, &undefined, &partial] {
            let imported = ObservationNumberImport::capture(
                value,
                ObservationNumberCodecLimits::default(),
                &mut work,
            )
            .unwrap();
            let mut budget = ObservationOutputWork::new(&());
            let output = NumberOutput::new(&imported, &(), &mut budget).unwrap();
            assert_eq!(output.bytes, imported.bytes());
            match value.value() {
                PartialNumber::Fixed(value) => {
                    assert_eq!(output.law, "fixed");
                    assert_eq!(
                        output
                            .value
                            .as_ref()
                            .map(|b| ExactRational::from_bytes(b, &mut work).unwrap())
                            .as_ref(),
                        value.as_ref()
                    );
                    assert!(output.domain.is_none());
                    assert!(output.defined.is_none());
                }
                PartialNumber::Parameter(value) => {
                    assert_eq!(output.law, "parameter");
                    assert!(
                        ParameterRegion::from_bytes(
                            output.defined.as_ref().unwrap(),
                            ParameterCodecLimits::default(),
                            &mut work
                        )
                        .unwrap()
                        .is_empty()
                    );
                    assert_eq!(
                        output.domain.as_deref(),
                        Some(
                            value
                                .ambient()
                                .to_bytes(ParameterCodecLimits::default(), &mut work)
                                .unwrap()
                                .as_slice()
                        )
                    );
                    assert!(output.value.is_some());
                }
            }
            let spent = 16 * 1024 * 1024 - budget.remaining;
            let mut budget = ObservationOutputWork::new(&());
            budget.remaining = spent;
            NumberOutput::new(&imported, &(), &mut budget).unwrap();
            assert_eq!(budget.remaining, 0);
            assert!(matches!(
                NumberOutput::new(&imported, &(), &mut budget),
                Err(Error::Capacity(Capacity::DescriptorBytes))
            ));
            let cancelled = bumbledb::WorkContext::new();
            cancelled.cancel();
            assert!(matches!(
                NumberOutput::new(
                    &imported,
                    &cancelled,
                    &mut ObservationOutputWork::new(&cancelled)
                ),
                Err(Error::Cancelled)
            ));
        }
    }
}
