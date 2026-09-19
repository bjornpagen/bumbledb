use super::super::{family, function, name, source};
use super::output::{Receipt, Revised};
use super::{
    AdmittedFamilyDescriptor as Family, Budget, Details, Error, ExactArithmetic, Output, Result,
    WorkContext, bytes, details, encoded, encoded_map, import, limits,
};
use bumbledb::event::{
    Capacity, EventPartition, ParameterJeffrey, ParameterRegion, ParameterRevisedSource, Space,
    SpaceId,
};

#[derive(Clone, Copy)]
pub(crate) enum Op {
    Condition,
    Likelihood,
    Jeffrey,
    Validate,
    Inspect,
    Pullback,
}
impl Op {
    pub(super) fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "condition" => Self::Condition,
            "likelihood" => Self::Likelihood,
            "jeffrey" => Self::Jeffrey,
            "validate" => Self::Validate,
            "inspect" => Self::Inspect,
            "pullback" => Self::Pullback,
            _ => return None,
        })
    }
    pub(super) fn valid(self, count: usize) -> bool {
        match self {
            Self::Condition | Self::Likelihood => count == 3,
            Self::Jeffrey => count >= 2 && count.is_multiple_of(2),
            Self::Pullback => count == 2,
            _ => count == 1,
        }
    }
}
pub(super) fn execute(
    op: Op,
    inputs: &[Vec<u8>],
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Output> {
    let value = match op {
        Op::Condition | Op::Likelihood | Op::Jeffrey => create(op, inputs, control, work)?,
        _ => import(&inputs[0], work)?,
    };
    let view = View::new(&value)?;
    match op {
        Op::Inspect => inspect(&value, &view, control, work),
        Op::Pullback => {
            // Decode the supplied Event even for an impossible revision.
            let event = source::event(&inputs[1], work)?;
            let revised = view.revised.ok_or(Error::RoleMismatch)?;
            source::encode(&revised.pullback(&event, control)?, control)
        }
        _ => bytes(encoded(value, control, work)?),
    }
}
fn create(
    op: Op,
    inputs: &[Vec<u8>],
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Family> {
    let identity = SpaceId(name(&inputs[0])?.0);
    let prior = source::space(&inputs[1], work)?;
    Ok(match op {
        Op::Condition => Family::Conditioning(prior.parameter_condition(
            identity,
            &source::event(&inputs[2], work)?,
            limits().parameters,
            work,
        )?),
        Op::Likelihood => Family::Likelihood(prior.parameter_likelihood(
            identity,
            &family::family(&inputs[2], work)?,
            limits().parameters,
            work,
        )?),
        Op::Jeffrey => {
            let count = (inputs.len() - 2) / 2;
            if count > limits().partitions.cells {
                return Err(Error::Capacity(Capacity::DescriptorItems));
            }
            let mut cells = Vec::new();
            let mut targets = Vec::new();
            cells.try_reserve_exact(count)?;
            targets.try_reserve_exact(count)?;
            for pair in inputs[2..].as_chunks::<2>().0 {
                cells.push(source::event(&pair[0], work)?);
                targets.push(function::function(&pair[1], work)?);
            }
            let partition =
                EventPartition::on(&prior.full(), &cells, limits().partitions, control)?;
            Family::Jeffrey(prior.parameter_jeffrey(
                identity,
                &partition,
                &targets,
                limits().parameters,
                work,
            )?)
        }
        _ => return Err(Error::InvalidEncoding),
    })
}
struct View<'a> {
    identity: SpaceId,
    prior: &'a Space,
    defined: &'a ParameterRegion,
    revised: Option<&'a ParameterRevisedSource>,
}
impl<'a> View<'a> {
    fn new(value: &'a Family) -> Result<Self> {
        Ok(match value {
            Family::Conditioning(v) => Self {
                identity: v.identity(),
                prior: v.prior(),
                defined: v.defined_on(),
                revised: v.revised(),
            },
            Family::Likelihood(v) => Self {
                identity: v.identity(),
                prior: v.prior(),
                defined: v.defined_on(),
                revised: v.revised(),
            },
            Family::Jeffrey(v) => Self {
                identity: v.identity(),
                prior: v.prior(),
                defined: v.defined_on(),
                revised: v.revised(),
            },
            _ => return Err(Error::RoleMismatch),
        })
    }
}
fn inspect(
    value: &Family,
    view: &View<'_>,
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Output> {
    let mut budget = Budget::new();
    budget.items(3)?; // result, receipt and outcome
    let prior = budget.blob(view.prior.full().to_bytes(control)?)?;
    let defined = budget.blob(
        view.defined
            .to_bytes(limits().parameters.parameters, work)?,
    )?;
    let receipt = match value {
        Family::Conditioning(v) => Receipt::Condition {
            evidence: budget.blob(v.evidence().to_bytes(control)?)?,
            mass: budget.blob(function::encoded(v.evidence_mass().clone(), control, work)?)?,
        },
        Family::Likelihood(v) => Receipt::Likelihood {
            likelihood: budget.blob(family::encoded(v.likelihood().clone(), control, work)?)?,
            normalizer: budget.blob(function::encoded(v.normalizer().clone(), control, work)?)?,
        },
        Family::Jeffrey(v) => jeffrey(v, &mut budget, control, work)?,
        _ => return Err(Error::RoleMismatch),
    };
    let outcome = view
        .revised
        .map(|v| -> Result<Revised> {
            Ok(Revised {
                refined: budget.blob(v.refined_prior().full().to_bytes(control)?)?,
                posterior: budget.blob(v.space().full().to_bytes(control)?)?,
                restriction: budget.blob(encoded(
                    Family::Restriction(v.restriction().clone()),
                    control,
                    work,
                )?)?,
                translation: budget.blob(encoded_map(v.translation(), false, control)?)?,
            })
        })
        .transpose()?;
    Ok(details(Details::Revision {
        identity: budget.blob(view.identity.0.to_vec())?,
        prior,
        defined,
        receipt,
        outcome,
    }))
}
fn jeffrey(
    v: &ParameterJeffrey,
    budget: &mut Budget,
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Receipt> {
    let count = v.partition().cells().len();
    budget.items(4)?;
    let mut cells = Vec::new();
    let mut targets = Vec::new();
    let mut old_masses = Vec::new();
    let mut unsupported = Vec::new();
    for list in [&mut cells, &mut targets, &mut old_masses, &mut unsupported] {
        list.try_reserve_exact(count)?;
    }
    for position in 0..count {
        cells.push(budget.blob(v.partition().cells()[position].to_bytes(control)?)?);
        targets.push(budget.blob(function::encoded(
            v.targets()[position].clone(),
            control,
            work,
        )?)?);
        old_masses.push(budget.blob(function::encoded(
            v.old_masses()[position].clone(),
            control,
            work,
        )?)?);
        unsupported.push(budget.blob(
            v.unsupported_regions()[position].to_bytes(limits().parameters.parameters, work)?,
        )?);
    }
    Ok(Receipt::Jeffrey {
        cells,
        targets,
        old_masses,
        unsupported,
    })
}
