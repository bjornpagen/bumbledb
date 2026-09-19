//! Exact truth cases are delivered beside the owned BENP derivation. Only
//! explicit `PredicateTest` quantifiers reduce a predicate to a scalar Boolean.
use crate::query_probability::ObservationOutputWork;
use bumbledb::{
    ObservationPredicateImport,
    event::{Capacity, Control, Error, NumberPredicateView, ParameterCodecLimits},
};
use napi::bindgen_prelude::{Env, Object, Uint8Array};

#[derive(Debug)]
pub struct PredicateOutput {
    bytes: Vec<u8>,
    law: &'static str,
    value: Option<bool>,
    holds: Option<Vec<u8>>,
    fails: Option<Vec<u8>>,
    undefined: Option<Vec<u8>>,
    domain: Option<Vec<u8>>,
}
impl PredicateOutput {
    pub(crate) fn new(
        predicate: &ObservationPredicateImport,
        control: &dyn Control,
        budget: &mut ObservationOutputWork<'_>,
    ) -> bumbledb::event::Result<Self> {
        control.checkpoint()?;
        budget.remaining = budget
            .remaining
            .checked_sub(predicate.bytes().len())
            .ok_or(Error::Capacity(Capacity::DescriptorBytes))?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(predicate.bytes().len())?;
        bytes.extend_from_slice(predicate.bytes());
        let work = &mut budget.arithmetic;
        let output = match predicate.value().predicate().view() {
            NumberPredicateView::Fixed(value) => Self {
                bytes,
                law: "fixed",
                value,
                holds: None,
                fails: None,
                undefined: None,
                domain: None,
            },
            NumberPredicateView::Parameter {
                ambient,
                holds,
                fails,
                undefined,
            } => Self {
                bytes,
                law: "parameter",
                value: None,
                holds: Some(holds.to_bytes(ParameterCodecLimits::default(), work)?),
                fails: Some(fails.to_bytes(ParameterCodecLimits::default(), work)?),
                undefined: Some(undefined.to_bytes(ParameterCodecLimits::default(), work)?),
                domain: Some(ambient.to_bytes(ParameterCodecLimits::default(), work)?),
            },
        };
        for bytes in output
            .holds
            .iter()
            .chain(output.fails.iter())
            .chain(output.undefined.iter())
            .chain(output.domain.iter())
        {
            budget.remaining = budget
                .remaining
                .checked_sub(bytes.len())
                .ok_or(Error::Capacity(Capacity::DescriptorBytes))?;
        }
        control.checkpoint()?;
        Ok(output)
    }
    pub(crate) fn object(self, env: &Env) -> napi::Result<Object<'_>> {
        let mut object = Object::new(env)?;
        object.set("kind", "predicate")?;
        object.set("bytes", Uint8Array::from(self.bytes))?;
        object.set("law", self.law)?;
        object.set("value", self.value)?;
        object.set("holds", self.holds.map(Uint8Array::from))?;
        object.set("fails", self.fails.map(Uint8Array::from))?;
        object.set("undefined", self.undefined.map(Uint8Array::from))?;
        object.set("domain", self.domain.map(Uint8Array::from))?;
        Ok(object)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumbledb::event::{
        ArithmeticLimits, ExactArithmetic, ExactRational, NumberOp, ParameterDomain, ParameterId,
        ParameterRegion, PolynomialSigns,
    };
    use bumbledb::{ObservationNumber, ObservationNumberCodecLimits, ObservationNumberLimits};

    #[test]
    fn delivery_preserves_truth_holes_domains_and_batch_byte_limits() {
        let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &());
        let limits = ObservationNumberLimits::default();
        let one = ObservationNumber::literal(ExactRational::one(), limits, &mut work).unwrap();
        let zero = ObservationNumber::literal(ExactRational::zero(), limits, &mut work).unwrap();
        let undefined = one
            .apply(NumberOp::Divide, &zero, limits, &mut work)
            .unwrap();
        let domain = ParameterDomain::new(ParameterRegion::full(ParameterId([234; 32]))).unwrap();
        for number in [&one, &zero, &undefined] {
            let fixed = number
                .where_sign(PolynomialSigns::POSITIVE, limits, &mut work)
                .unwrap();
            let family = fixed.on_domain(&domain, limits, &mut work).unwrap();
            for value in [&fixed, &family] {
                let imported = ObservationPredicateImport::capture(
                    value,
                    ObservationNumberCodecLimits::default(),
                    &mut work,
                )
                .unwrap();
                let mut budget = ObservationOutputWork::new(&());
                let output = PredicateOutput::new(&imported, &(), &mut budget).unwrap();
                assert_eq!(output.bytes, imported.bytes());
                match value.predicate().view() {
                    NumberPredicateView::Fixed(expected) => {
                        assert_eq!(output.law, "fixed");
                        assert_eq!(output.value, expected);
                        assert!(
                            output.holds.is_none()
                                && output.fails.is_none()
                                && output.undefined.is_none()
                                && output.domain.is_none()
                        );
                    }
                    NumberPredicateView::Parameter {
                        ambient,
                        holds,
                        fails,
                        undefined,
                    } => {
                        assert_eq!(output.law, "parameter");
                        assert_eq!(output.value, None);
                        for (bytes, expected) in [
                            (output.holds.as_ref().unwrap(), holds),
                            (output.fails.as_ref().unwrap(), fails),
                            (output.undefined.as_ref().unwrap(), undefined),
                        ] {
                            assert_eq!(
                                *bytes,
                                expected
                                    .to_bytes(ParameterCodecLimits::default(), &mut work)
                                    .unwrap()
                            );
                        }
                        assert_eq!(
                            output.domain.as_ref().unwrap(),
                            &ambient
                                .to_bytes(ParameterCodecLimits::default(), &mut work)
                                .unwrap()
                        );
                    }
                }
                let spent = 16 * 1024 * 1024 - budget.remaining;
                let mut budget = ObservationOutputWork::new(&());
                budget.remaining = spent;
                PredicateOutput::new(&imported, &(), &mut budget).unwrap();
                assert_eq!(budget.remaining, 0);
                assert!(matches!(
                    PredicateOutput::new(&imported, &(), &mut budget),
                    Err(Error::Capacity(Capacity::DescriptorBytes))
                ));
                let cancelled = bumbledb::WorkContext::new();
                cancelled.cancel();
                assert!(matches!(
                    PredicateOutput::new(
                        &imported,
                        &cancelled,
                        &mut ObservationOutputWork::new(&cancelled)
                    ),
                    Err(Error::Cancelled)
                ));
            }
        }
    }
}
