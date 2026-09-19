//! Structured exact query results, owning all nested Event/function transports.
use bumbledb::event::{
    AdmittedFamilyDescriptor, AdmittedSourceDescriptor, ArithmeticLimits, Capacity, Control, Error,
    ExactArithmetic, ParameterFunction, SourceDescriptor, SourceDescriptorLimits,
};
use bumbledb::{ProbabilityAnswer, ProbabilityValue};
use napi::bindgen_prelude::{Env, Object, Uint8Array};

#[derive(Debug)]
pub struct ProbabilityOutput {
    law: &'static str,
    event: Vec<u8>,
    given: Vec<u8>,
    numerator: Vec<u8>,
    mass: Vec<u8>,
    value: Option<Vec<u8>>,
    defined: Option<Vec<u8>>,
}
fn function(
    value: &ParameterFunction,
    control: &dyn Control,
    work: &mut ExactArithmetic<'_>,
) -> bumbledb::event::Result<Vec<u8>> {
    let limits = SourceDescriptorLimits::default();
    SourceDescriptor::capture(
        &AdmittedSourceDescriptor::Family(Box::new(AdmittedFamilyDescriptor::Parameter(
            value.clone(),
        ))),
        limits,
        work,
    )?
    .to_bytes(limits, control)
}
/// One cooperative arithmetic and byte budget for an entire delivery batch.
pub(crate) struct ObservationOutputWork<'a> {
    arithmetic: ExactArithmetic<'a>,
    remaining: usize,
}
impl<'a> ObservationOutputWork<'a> {
    pub(crate) fn new(control: &'a dyn Control) -> Self {
        Self {
            arithmetic: ExactArithmetic::new(ArithmeticLimits::default(), control),
            remaining: 16 * 1024 * 1024,
        }
    }
}
impl ProbabilityOutput {
    pub(crate) fn new(
        value: &ProbabilityAnswer,
        control: &dyn Control,
        budget: &mut ObservationOutputWork<'_>,
    ) -> bumbledb::event::Result<Self> {
        let work = &mut budget.arithmetic;
        let event = value.event().to_bytes(control)?;
        let given = value.given().to_bytes(control)?;
        let result = match value.value() {
            ProbabilityValue::Fixed { observation, value } => Self {
                law: "fixed",
                event,
                given,
                numerator: observation.numerator().to_bytes(work)?,
                mass: observation.evidence_mass().to_bytes(work)?,
                value: value.as_ref().map(|v| v.to_bytes(work)).transpose()?,
                defined: None,
            },
            ProbabilityValue::Parameter(v) => Self {
                law: "parameter",
                event,
                given,
                numerator: function(v.numerator(), control, work)?,
                mass: function(v.evidence_mass(), control, work)?,
                value: Some(function(v.conditional(), control, work)?),
                defined: Some(v.defined_on().to_bytes(
                    SourceDescriptorLimits::default().parameters.parameters,
                    work,
                )?),
            },
        };
        let bytes = [
            &result.event,
            &result.given,
            &result.numerator,
            &result.mass,
        ]
        .into_iter()
        .chain(result.value.iter())
        .chain(result.defined.iter())
        .try_fold(0usize, |n, v| n.checked_add(v.len()))
        .ok_or(Error::Capacity(Capacity::DescriptorBytes))?;
        if bytes > budget.remaining {
            return Err(Error::Capacity(Capacity::DescriptorBytes));
        }
        budget.remaining -= bytes;
        control.checkpoint()?;
        Ok(result)
    }
    pub(crate) fn object(self, env: &Env) -> napi::Result<Object<'_>> {
        let mut result = Object::new(env)?;
        result.set("kind", "probability")?;
        result.set("law", self.law)?;
        result.set("event", Uint8Array::from(self.event))?;
        result.set("given", Uint8Array::from(self.given))?;
        result.set("numerator", Uint8Array::from(self.numerator))?;
        result.set("evidenceMass", Uint8Array::from(self.mass))?;
        result.set("value", self.value.map(Uint8Array::from))?;
        result.set("defined", self.defined.map(Uint8Array::from))?;
        Ok(result)
    }
}
