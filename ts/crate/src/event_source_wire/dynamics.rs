//! Explicit conditional channels and fixed-law revisions. No operation invents
//! observation identity, provider provenance, independence or missing mass.
use super::{SourceOutput, bytes, encoded, event, function, map, scalar, space};
use crate::marshal::{ValueOut, output_vec};
use crate::runtime::Output;
use crate::runtime_wire::thrown;
use bumbledb::event::{
    AdmittedDescriptor, AdmittedSourceDescriptor, Capacity, Descriptor, Error, EventPartition,
    ExactArithmetic, ExactRational, FiniteKernel, RevisionImpossible, RevisionOutcome,
    RevisionReceipt, SourceDescriptor, SourceDescriptorLimits, SourceRevision, SurjectiveMap,
};
use bumbledb::work::WorkContext;
use napi::bindgen_prelude::{Env, Object, Uint8Array};

#[derive(Clone, Copy)]
pub(super) enum Op {
    KernelNew,
    KernelValidate,
    KernelDescribe,
    KernelClose,
    KernelFactors,
    Condition,
    Likelihood,
    Jeffrey,
    RevisionValidate,
    RevisionInspect,
}
impl Op {
    pub(super) fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "kernel.new" => Self::KernelNew,
            "kernel.validate" => Self::KernelValidate,
            "kernel.describe" => Self::KernelDescribe,
            "kernel.close" => Self::KernelClose,
            "kernel.factorsThrough" => Self::KernelFactors,
            "revision.condition" => Self::Condition,
            "revision.likelihood" => Self::Likelihood,
            "revision.jeffrey" => Self::Jeffrey,
            "revision.validate" => Self::RevisionValidate,
            "revision.inspect" => Self::RevisionInspect,
            _ => return None,
        })
    }
    pub(super) const fn arity(self, n: usize) -> bool {
        match self {
            Self::Jeffrey => n > 0 && n % 2 == 1,
            Self::KernelValidate
            | Self::KernelDescribe
            | Self::RevisionValidate
            | Self::RevisionInspect => n == 1,
            _ => n == 2,
        }
    }
}

pub enum Details {
    Kernel {
        parent: Vec<u8>,
        density: Vec<u8>,
    },
    Extension {
        space: Vec<u8>,
        parent: Vec<u8>,
    },
    Revision {
        prior: Vec<u8>,
        receipt: Receipt,
        outcome: Outcome,
    },
}
pub enum Receipt {
    Condition {
        evidence: Vec<u8>,
        mass: Vec<u8>,
    },
    Likelihood {
        likelihood: Vec<u8>,
        normalizer: Vec<u8>,
    },
    Jeffrey {
        cells: Vec<Vec<u8>>,
        old_masses: Vec<Vec<u8>>,
        targets: Vec<Vec<u8>>,
    },
}
pub enum Outcome {
    Revised {
        posterior: Vec<u8>,
        translation: Vec<u8>,
    },
    Impossible {
        cause: &'static str,
        cells: Vec<u32>,
    },
}

struct Budget {
    bytes: usize,
    items: usize,
}
impl Budget {
    fn new() -> Self {
        let limits = SourceDescriptorLimits::default().descriptors;
        Self {
            bytes: limits.bytes,
            items: limits.items,
        }
    }
    fn charge(&mut self, bytes: usize, items: usize) -> bumbledb::event::Result<()> {
        self.bytes = self
            .bytes
            .checked_sub(bytes)
            .ok_or(Error::Capacity(Capacity::DescriptorBytes))?;
        self.items = self
            .items
            .checked_sub(items)
            .ok_or(Error::Capacity(Capacity::DescriptorItems))?;
        Ok(())
    }
    fn own(&mut self, bytes: Vec<u8>) -> bumbledb::event::Result<Vec<u8>> {
        self.charge(bytes.len(), 1)?;
        Ok(bytes)
    }
    fn rational(
        &mut self,
        value: &ExactRational,
        work: &mut ExactArithmetic<'_>,
    ) -> bumbledb::event::Result<Vec<u8>> {
        self.own(value.to_bytes(work)?)
    }
    fn map(
        &mut self,
        value: &SurjectiveMap,
        control: &WorkContext,
    ) -> bumbledb::event::Result<Vec<u8>> {
        let limits = SourceDescriptorLimits::default().descriptors;
        self.own(
            Descriptor::capture(
                &AdmittedDescriptor::Surjective(value.clone()),
                limits,
                control,
            )?
            .to_bytes(limits, control)?,
        )
    }
}
fn source(
    value: &AdmittedSourceDescriptor,
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> bumbledb::event::Result<Output> {
    let limits = SourceDescriptorLimits::default();
    bytes(SourceDescriptor::capture(value, limits, work)?.to_bytes(limits, control)?)
}
fn kernel(input: &[u8], work: &mut ExactArithmetic<'_>) -> bumbledb::event::Result<FiniteKernel> {
    match SourceDescriptor::import(input, SourceDescriptorLimits::default(), work)? {
        AdmittedSourceDescriptor::Kernel(v) => Ok(v),
        _ => Err(Error::RoleMismatch),
    }
}
fn revision(
    input: &[u8],
    work: &mut ExactArithmetic<'_>,
) -> bumbledb::event::Result<SourceRevision> {
    match SourceDescriptor::import(input, SourceDescriptorLimits::default(), work)? {
        AdmittedSourceDescriptor::Revision(v) => Ok(v),
        _ => Err(Error::RoleMismatch),
    }
}
fn details(value: Details) -> Output {
    Output::EventSource(SourceOutput::Dynamics(value))
}

pub(super) fn execute(
    op: Op,
    inputs: &[Vec<u8>],
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> bumbledb::event::Result<Output> {
    let limits = SourceDescriptorLimits::default();
    let mut budget = Budget::new();
    budget.charge(0, 1)?;
    match op {
        Op::KernelNew => {
            let parent = map(&inputs[0], limits, control, work)?;
            let density = function(&inputs[1], limits, work)?;
            source(
                &AdmittedSourceDescriptor::Kernel(FiniteKernel::new(
                    &parent,
                    &density,
                    limits.functions,
                    work,
                )?),
                control,
                work,
            )
        }
        Op::KernelValidate | Op::KernelDescribe | Op::KernelClose | Op::KernelFactors => {
            let kernel = kernel(&inputs[0], work)?;
            match op {
                Op::KernelValidate => {
                    source(&AdmittedSourceDescriptor::Kernel(kernel), control, work)
                }
                Op::KernelDescribe => Ok(details(Details::Kernel {
                    parent: budget.map(kernel.parent(), control)?,
                    density: budget.own(encoded(
                        kernel.density().clone(),
                        limits,
                        control,
                        work,
                    )?)?,
                })),
                Op::KernelClose => {
                    let prior = space(&inputs[1], limits, work)?;
                    let closed = kernel.close(&prior, limits.functions, limits.laws, work)?;
                    Ok(details(Details::Extension {
                        space: budget.own(closed.space().full().to_bytes(control)?)?,
                        parent: budget.map(closed.parent_surjective(), control)?,
                    }))
                }
                Op::KernelFactors => {
                    let observation =
                        map(&inputs[1], limits, control, work)?.certify_surjective(control)?;
                    Ok(scalar(ValueOut::Bool(
                        kernel.factors_through(&observation, control)?,
                    )))
                }
                _ => unreachable!(),
            }
        }
        Op::RevisionValidate => source(
            &AdmittedSourceDescriptor::Revision(revision(&inputs[0], work)?),
            control,
            work,
        ),
        Op::RevisionInspect => inspect(&revision(&inputs[0], work)?, control, work),
        Op::Condition | Op::Likelihood | Op::Jeffrey => {
            let prior = space(&inputs[0], limits, work)?;
            let result = match op {
                Op::Condition => prior.condition(
                    &event(&inputs[1], limits, work)?,
                    limits.functions,
                    limits.laws,
                    work,
                )?,
                Op::Likelihood => prior.likelihood(
                    &function(&inputs[1], limits, work)?,
                    limits.functions,
                    limits.laws,
                    work,
                )?,
                Op::Jeffrey => {
                    let mut cells = Vec::new();
                    let mut targets = Vec::new();
                    cells.try_reserve_exact(inputs.len() / 2)?;
                    targets.try_reserve_exact(inputs.len() / 2)?;
                    for pair in inputs[1..].as_chunks::<2>().0 {
                        cells.push(event(&pair[0], limits, work)?);
                        targets.push(ExactRational::from_bytes(&pair[1], work)?);
                    }
                    let partition =
                        EventPartition::on(&prior.full(), &cells, limits.partitions, control)?;
                    prior.jeffrey(&partition, &targets, limits.functions, limits.laws, work)?
                }
                _ => unreachable!(),
            };
            source(&AdmittedSourceDescriptor::Revision(result), control, work)
        }
    }
}

fn inspect(
    value: &SourceRevision,
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> bumbledb::event::Result<Output> {
    let limits = SourceDescriptorLimits::default();
    let mut budget = Budget::new();
    budget.charge(0, 3)?; // result, receipt and outcome records
    let prior = budget.own(value.prior().full().to_bytes(control)?)?;
    let receipt = match value.receipt() {
        RevisionReceipt::Condition { evidence, mass } => Receipt::Condition {
            evidence: budget.own(evidence.to_bytes(control)?)?,
            mass: budget.rational(mass, work)?,
        },
        RevisionReceipt::Likelihood {
            likelihood,
            normalizer,
        } => Receipt::Likelihood {
            likelihood: budget.own(encoded(likelihood.clone(), limits, control, work)?)?,
            normalizer: budget.rational(normalizer, work)?,
        },
        RevisionReceipt::Jeffrey {
            partition,
            old_masses,
            targets,
        } => {
            let n = partition.cells().len();
            budget.charge(0, 3)?;
            if n > budget.items / 3 {
                return Err(Error::Capacity(Capacity::DescriptorItems));
            }
            let mut cells = Vec::new();
            let mut masses = Vec::new();
            let mut values = Vec::new();
            cells.try_reserve_exact(n)?;
            masses.try_reserve_exact(n)?;
            values.try_reserve_exact(n)?;
            for ((cell, mass), target) in partition
                .cells()
                .iter()
                .zip(old_masses.iter())
                .zip(targets.iter())
            {
                cells.push(budget.own(cell.to_bytes(control)?)?);
                masses.push(budget.rational(mass, work)?);
                values.push(budget.rational(target, work)?);
            }
            Receipt::Jeffrey {
                cells,
                old_masses: masses,
                targets: values,
            }
        }
    };
    let outcome = match value.outcome() {
        RevisionOutcome::Revised(result) => Outcome::Revised {
            posterior: budget.own(result.space().full().to_bytes(control)?)?,
            translation: budget.map(result.translation_surjective(), control)?,
        },
        RevisionOutcome::Impossible(reason) => {
            let (cause, positions) = match reason {
                RevisionImpossible::ZeroEvidence => ("zeroEvidence", &[][..]),
                RevisionImpossible::ZeroLikelihood => ("zeroLikelihood", &[][..]),
                RevisionImpossible::UnsupportedTargets { cells } => {
                    ("unsupportedTargets", cells.as_ref())
                }
            };
            budget.charge(0, positions.len() + 1)?;
            let mut cells = Vec::new();
            cells.try_reserve_exact(positions.len())?;
            for &position in positions {
                cells.push(
                    u32::try_from(position)
                        .map_err(|_| Error::Capacity(Capacity::DescriptorItems))?,
                );
            }
            Outcome::Impossible { cause, cells }
        }
    };
    Ok(details(Details::Revision {
        prior,
        receipt,
        outcome,
    }))
}

fn list(values: Vec<Vec<u8>>, env: Env) -> napi::Result<Vec<Uint8Array>> {
    let mut out = output_vec(values.len()).map_err(|e| thrown(env, e))?;
    out.extend(values.into_iter().map(Uint8Array::from));
    Ok(out)
}
impl Details {
    pub(super) fn object(self, env: &Env) -> napi::Result<Object<'_>> {
        let mut object = Object::new(env)?;
        match self {
            Self::Kernel { parent, density } => {
                object.set("parent", Uint8Array::from(parent))?;
                object.set("density", Uint8Array::from(density))?;
            }
            Self::Extension { space, parent } => {
                object.set("space", Uint8Array::from(space))?;
                object.set("parent", Uint8Array::from(parent))?;
            }
            Self::Revision {
                prior,
                receipt,
                outcome,
            } => {
                object.set("prior", Uint8Array::from(prior))?;
                object.set("receipt", receipt.object(env)?)?;
                object.set("outcome", outcome.object(env)?)?;
            }
        }
        Ok(object)
    }
}
impl Receipt {
    fn object(self, env: &Env) -> napi::Result<Object<'_>> {
        let mut object = Object::new(env)?;
        match self {
            Self::Condition { evidence, mass } => {
                object.set("kind", "condition")?;
                object.set("evidence", Uint8Array::from(evidence))?;
                object.set("mass", Uint8Array::from(mass))?;
            }
            Self::Likelihood {
                likelihood,
                normalizer,
            } => {
                object.set("kind", "likelihood")?;
                object.set("likelihood", Uint8Array::from(likelihood))?;
                object.set("normalizer", Uint8Array::from(normalizer))?;
            }
            Self::Jeffrey {
                cells,
                old_masses,
                targets,
            } => {
                object.set("kind", "jeffrey")?;
                object.set("cells", list(cells, *env)?)?;
                object.set("oldMasses", list(old_masses, *env)?)?;
                object.set("targets", list(targets, *env)?)?;
            }
        }
        Ok(object)
    }
}
impl Outcome {
    fn object(self, env: &Env) -> napi::Result<Object<'_>> {
        let mut object = Object::new(env)?;
        match self {
            Self::Revised {
                posterior,
                translation,
            } => {
                object.set("kind", "revised")?;
                object.set("posterior", Uint8Array::from(posterior))?;
                object.set("translation", Uint8Array::from(translation))?;
            }
            Self::Impossible { cause, cells } => {
                object.set("kind", "impossible")?;
                object.set("cause", cause)?;
                object.set("cells", cells)?;
            }
        }
        Ok(object)
    }
}

#[cfg(test)]
mod tests;
