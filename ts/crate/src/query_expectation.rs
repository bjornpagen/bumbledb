//! Query expectation delivery keeps the admitted roster, including zero cells.
use crate::query_probability::{ObservationOutputWork, function};
use bumbledb::event::{
    AdmittedSourceDescriptor, Capacity, Control, Error, SourceDescriptor, SourceDescriptorLimits,
};
use bumbledb::{ExpectationAnswer, ExpectationValue};
use napi::bindgen_prelude::{Env, Object, Uint8Array};

#[derive(Debug)]
pub struct ExpectationOutput {
    law: &'static str,
    function: Vec<u8>,
    given: Vec<u8>,
    payoffs: Vec<(Vec<u8>, Vec<u8>)>,
    numerator: Vec<u8>,
    mass: Vec<u8>,
    value: Option<Vec<u8>>,
    defined: Option<Vec<u8>>,
}

impl ExpectationOutput {
    pub(crate) fn new(
        value: &ExpectationAnswer,
        control: &dyn Control,
        budget: &mut ObservationOutputWork<'_>,
    ) -> bumbledb::event::Result<Self> {
        let limits = SourceDescriptorLimits::default();
        let mut result = Self {
            law: "fixed",
            function: SourceDescriptor::capture(
                &AdmittedSourceDescriptor::Function(value.function().clone()),
                limits,
                &mut budget.arithmetic,
            )?
            .to_bytes(limits, control)?,
            given: value.given().to_bytes(control)?,
            payoffs: Vec::new(),
            numerator: Vec::new(),
            mass: Vec::new(),
            value: None,
            defined: None,
        };
        for (region, payoff) in value.partition().cells().iter().zip(value.values()) {
            let region = region.to_bytes(control)?;
            let payoff = payoff.to_bytes(&mut budget.arithmetic)?;
            charge(budget, [&region, &payoff])?;
            result.payoffs.push((region, payoff));
        }
        match value.value() {
            ExpectationValue::Fixed { observation, value } => {
                result.numerator = observation.numerator().to_bytes(&mut budget.arithmetic)?;
                result.mass = observation
                    .evidence_mass()
                    .to_bytes(&mut budget.arithmetic)?;
                result.value = value
                    .as_ref()
                    .map(|v| v.to_bytes(&mut budget.arithmetic))
                    .transpose()?;
            }
            ExpectationValue::Parameter(v) => {
                result.law = "parameter";
                result.numerator = function(v.numerator(), control, &mut budget.arithmetic)?;
                result.mass = function(v.evidence_mass(), control, &mut budget.arithmetic)?;
                result.value = Some(function(v.conditional(), control, &mut budget.arithmetic)?);
                result.defined = Some(
                    v.defined_on()
                        .to_bytes(limits.parameters.parameters, &mut budget.arithmetic)?,
                );
            }
        }
        charge(
            budget,
            [
                &result.function,
                &result.given,
                &result.numerator,
                &result.mass,
            ]
            .into_iter()
            .chain(result.value.iter())
            .chain(result.defined.iter()),
        )?;
        control.checkpoint()?;
        Ok(result)
    }

    pub(crate) fn object(self, env: &Env) -> napi::Result<Object<'_>> {
        let mut result = Object::new(env)?;
        result.set("kind", "expectation")?;
        result.set("law", self.law)?;
        result.set("function", Uint8Array::from(self.function))?;
        result.set("given", Uint8Array::from(self.given))?;
        let mut payoffs = Vec::new();
        for (region, value) in self.payoffs {
            let mut piece = Object::new(env)?;
            piece.set("region", Uint8Array::from(region))?;
            piece.set("value", Uint8Array::from(value))?;
            payoffs.push(piece);
        }
        result.set("payoffs", payoffs)?;
        result.set("numerator", Uint8Array::from(self.numerator))?;
        result.set("evidenceMass", Uint8Array::from(self.mass))?;
        result.set("value", self.value.map(Uint8Array::from))?;
        result.set("defined", self.defined.map(Uint8Array::from))?;
        Ok(result)
    }
}

fn charge<'a>(
    budget: &mut ObservationOutputWork<'_>,
    buffers: impl IntoIterator<Item = &'a Vec<u8>>,
) -> bumbledb::event::Result<()> {
    for buffer in buffers {
        budget.remaining = budget
            .remaining
            .checked_sub(buffer.len())
            .ok_or(Error::Capacity(Capacity::DescriptorBytes))?;
    }
    Ok(())
}
