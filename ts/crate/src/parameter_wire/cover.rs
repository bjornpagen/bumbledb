//! Checked local-function covers. Submitted patch records are replayed, never
//! trusted as certificates. All retained outputs share one byte budget.
use super::output::Budget;
use super::source::{event, finite};
use super::{ExactArithmetic, Output, ParameterOutput, Result, WorkContext, bytes, family, limits};
use bumbledb::event::{
    AdmittedSourceDescriptor, FamilyFunction, FiniteFunction, FunctionPatch, SourceDescriptor,
};
use napi::bindgen_prelude::{Env, Object, Uint8Array};

#[derive(Clone, Copy)]
pub(super) enum Op {
    Finite,
    Family,
    FiniteMask,
    FamilyMask,
}
impl Op {
    pub(super) fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "finite" => Self::Finite,
            "family" => Self::Family,
            "finiteMask" => Self::FiniteMask,
            "familyMask" => Self::FamilyMask,
            _ => return None,
        })
    }
    pub(super) fn valid(self, count: usize, argument: u8) -> bool {
        argument == 0
            && match self {
                Self::Finite | Self::Family => count > 0 && count % 2 == 1,
                Self::FiniteMask | Self::FamilyMask => count == 2,
            }
    }
}
fn finite_bytes(
    value: &FiniteFunction,
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Vec<u8>> {
    SourceDescriptor::capture(
        &AdmittedSourceDescriptor::Function(value.clone()),
        limits(),
        work,
    )?
    .to_bytes(limits(), control)
}
pub(super) fn execute(
    op: Op,
    inputs: &[Vec<u8>],
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Output> {
    match op {
        Op::FiniteMask => {
            let function = finite(&inputs[0], work)?;
            let region = event(&inputs[1], work)?;
            bytes(finite_bytes(
                &function.mask(&region, limits().functions, work)?,
                control,
                work,
            )?)
        }
        Op::FamilyMask => {
            let function = family::family(&inputs[0], work)?;
            let region = event(&inputs[1], work)?;
            bytes(family::encoded(
                function.mask(&region, limits().parameters, work)?,
                control,
                work,
            )?)
        }
        Op::Finite => finite_cover(inputs, control, work),
        Op::Family => family_cover(inputs, control, work),
    }
}
fn finite_cover(
    inputs: &[Vec<u8>],
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Output> {
    let parent = event(&inputs[0], work)?;
    let mut patches = Vec::new();
    patches.try_reserve_exact(inputs.len() / 2)?;
    for pair in inputs[1..].as_chunks::<2>().0 {
        patches.push(FunctionPatch {
            region: event(&pair[0], work)?,
            function: finite(&pair[1], work)?,
        });
    }
    let cover = FiniteFunction::glue(&parent, &patches, limits().functions, work)?;
    let mut budget = Budget::default();
    let mut out = Details::new(
        cover.parent(),
        finite_bytes(cover.function(), control, work)?,
        &mut budget,
        control,
    )?;
    out.patches.try_reserve_exact(cover.patches().len())?;
    for patch in cover.patches() {
        out.patches.push((
            budget.blob(patch.region.to_bytes(control)?)?,
            budget.blob(finite_bytes(&patch.function, control, work)?)?,
        ));
    }
    Ok(Output::Parameter(ParameterOutput::Cover(out)))
}
fn family_cover(
    inputs: &[Vec<u8>],
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Output> {
    let parent = event(&inputs[0], work)?;
    let mut patches = Vec::new();
    patches.try_reserve_exact(inputs.len() / 2)?;
    for pair in inputs[1..].as_chunks::<2>().0 {
        patches.push(FunctionPatch {
            region: event(&pair[0], work)?,
            function: family::family(&pair[1], work)?,
        });
    }
    let cover = FamilyFunction::glue(&parent, &patches, limits().parameters, work)?;
    let mut budget = Budget::default();
    let mut out = Details::new(
        cover.parent(),
        family::encoded(cover.function().clone(), control, work)?,
        &mut budget,
        control,
    )?;
    out.patches.try_reserve_exact(cover.patches().len())?;
    for patch in cover.patches() {
        out.patches.push((
            budget.blob(patch.region.to_bytes(control)?)?,
            budget.blob(family::encoded(patch.function.clone(), control, work)?)?,
        ));
    }
    Ok(Output::Parameter(ParameterOutput::Cover(out)))
}
pub struct Details {
    parent: Vec<u8>,
    function: Vec<u8>,
    patches: Vec<(Vec<u8>, Vec<u8>)>,
}
impl Details {
    fn new(
        parent: &bumbledb::Event,
        function: Vec<u8>,
        budget: &mut Budget,
        control: &WorkContext,
    ) -> Result<Self> {
        Ok(Self {
            parent: budget.blob(parent.to_bytes(control)?)?,
            function: budget.blob(function)?,
            patches: Vec::new(),
        })
    }
    pub(super) fn object(self, env: &Env) -> napi::Result<Object<'_>> {
        let mut object = Object::new(env)?;
        object.set("parent", Uint8Array::from(self.parent))?;
        object.set("function", Uint8Array::from(self.function))?;
        let mut patches = crate::marshal::output_vec(self.patches.len())
            .map_err(|e| crate::runtime_wire::thrown(*env, e))?;
        for (region, function) in self.patches {
            let mut patch = Object::new(env)?;
            patch.set("region", Uint8Array::from(region))?;
            patch.set("function", Uint8Array::from(function))?;
            patches.push(patch);
        }
        object.set("patches", patches)?;
        Ok(object)
    }
}
