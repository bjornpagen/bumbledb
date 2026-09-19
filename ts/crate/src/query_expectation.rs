//! Query expectation delivery keeps the admitted roster, including zero cells.
use crate::query_probability::{ObservationOutputWork, function};
use bumbledb::event::{
    AdmittedFamilyDescriptor, AdmittedSourceDescriptor, Capacity, Control, Error, SourceDescriptor,
    SourceDescriptorLimits,
};
use bumbledb::{ExpectationAnswer, ExpectationFunction, ExpectationPayoff, ExpectationValue};
use napi::bindgen_prelude::{Env, Object, Uint8Array};

#[derive(Debug)]
pub struct ExpectationOutput {
    law: &'static str,
    payoff_kind: &'static str,
    function: Vec<u8>,
    given: Vec<u8>,
    pieces: Vec<(Vec<u8>, Vec<u8>)>,
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
            payoff_kind: "scalar",
            function: SourceDescriptor::capture(
                &match value.function() {
                    ExpectationFunction::Finite(f) => AdmittedSourceDescriptor::Function(f.clone()),
                    ExpectationFunction::Family(f) => AdmittedSourceDescriptor::Family(Box::new(
                        AdmittedFamilyDescriptor::Function(f.clone()),
                    )),
                },
                limits,
                &mut budget.arithmetic,
            )?
            .to_bytes(limits, control)?,
            given: value.given().to_bytes(control)?,
            pieces: Vec::new(),
            numerator: Vec::new(),
            mass: Vec::new(),
            value: None,
            defined: None,
        };
        match value.payoff() {
            ExpectationPayoff::Scalar { partition, values } => {
                for (region, payoff) in partition.cells().iter().zip(values) {
                    result.piece(
                        region.to_bytes(control)?,
                        payoff.to_bytes(&mut budget.arithmetic)?,
                        budget,
                    )?;
                }
            }
            ExpectationPayoff::Finite(cover) => {
                result.payoff_kind = "finite";
                for patch in cover.patches() {
                    let function = encode(
                        &AdmittedSourceDescriptor::Function(patch.function.clone()),
                        control,
                        budget,
                    )?;
                    result.piece(patch.region.to_bytes(control)?, function, budget)?;
                }
            }
            ExpectationPayoff::Family(cover) => {
                result.payoff_kind = "family";
                for patch in cover.patches() {
                    let function = encode(
                        &AdmittedSourceDescriptor::Family(Box::new(
                            AdmittedFamilyDescriptor::Function(patch.function.clone()),
                        )),
                        control,
                        budget,
                    )?;
                    result.piece(patch.region.to_bytes(control)?, function, budget)?;
                }
            }
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
            ExpectationValue::Parameter(v) => result.parameter(v, control, budget)?,
            ExpectationValue::Family(v) => result.parameter(v, control, budget)?,
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

    fn piece(
        &mut self,
        region: Vec<u8>,
        value: Vec<u8>,
        budget: &mut ObservationOutputWork<'_>,
    ) -> bumbledb::event::Result<()> {
        charge(budget, [&region, &value])?;
        self.pieces.try_reserve(1).map_err(Error::from)?;
        self.pieces.push((region, value));
        Ok(())
    }
    fn parameter<F>(
        &mut self,
        v: &bumbledb::event::ParameterExpectationObservation<F>,
        control: &dyn Control,
        budget: &mut ObservationOutputWork<'_>,
    ) -> bumbledb::event::Result<()> {
        self.law = "parameter";
        self.numerator = function(v.numerator(), control, &mut budget.arithmetic)?;
        self.mass = function(v.evidence_mass(), control, &mut budget.arithmetic)?;
        self.value = Some(function(v.conditional(), control, &mut budget.arithmetic)?);
        self.defined = Some(v.defined_on().to_bytes(
            SourceDescriptorLimits::default().parameters.parameters,
            &mut budget.arithmetic,
        )?);
        Ok(())
    }

    pub(crate) fn object(self, env: &Env) -> napi::Result<Object<'_>> {
        let mut result = Object::new(env)?;
        result.set("kind", "expectation")?;
        result.set("law", self.law)?;
        result.set("payoffKind", self.payoff_kind)?;
        result.set("function", Uint8Array::from(self.function))?;
        result.set("given", Uint8Array::from(self.given))?;
        let mut payoffs = Vec::new();
        for (region, value) in self.pieces {
            let mut piece = Object::new(env)?;
            piece.set("region", Uint8Array::from(region))?;
            piece.set(
                if self.payoff_kind == "scalar" {
                    "value"
                } else {
                    "function"
                },
                Uint8Array::from(value),
            )?;
            payoffs.push(piece);
        }
        result.set(
            if self.payoff_kind == "scalar" {
                "payoffs"
            } else {
                "patches"
            },
            payoffs,
        )?;
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

fn encode(
    value: &AdmittedSourceDescriptor,
    control: &dyn Control,
    budget: &mut ObservationOutputWork<'_>,
) -> bumbledb::event::Result<Vec<u8>> {
    let limits = SourceDescriptorLimits::default();
    SourceDescriptor::capture(value, limits, &mut budget.arithmetic)?.to_bytes(limits, control)
}
