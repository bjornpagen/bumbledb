use super::{Error, Result, limits};
use crate::marshal::output_vec;
use crate::runtime_wire::thrown;
use bumbledb::event::Capacity;
use napi::bindgen_prelude::{Env, Object, Uint8Array};

pub(super) struct Budget {
    bytes: super::super::output::Budget,
    remaining: usize,
}
impl Budget {
    pub(super) fn new() -> Self {
        Self {
            bytes: super::super::output::Budget::default(),
            remaining: limits().descriptors.items,
        }
    }
    pub(super) fn items(&mut self, count: usize) -> Result<()> {
        self.remaining = self
            .remaining
            .checked_sub(count)
            .ok_or(Error::Capacity(Capacity::DescriptorItems))?;
        Ok(())
    }
    pub(super) fn blob(&mut self, bytes: Vec<u8>) -> Result<Vec<u8>> {
        self.items(1)?;
        self.bytes.blob(bytes)
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
    Restriction {
        prior: Vec<u8>,
        refined: Vec<u8>,
        space: Vec<u8>,
        refinement: Vec<u8>,
        inclusion: Vec<u8>,
    },
    Refinements(Vec<Vec<u8>>),
    Revision {
        identity: Vec<u8>,
        prior: Vec<u8>,
        defined: Vec<u8>,
        receipt: Receipt,
        outcome: Option<Revised>,
    },
}
pub struct Revised {
    pub refined: Vec<u8>,
    pub posterior: Vec<u8>,
    pub restriction: Vec<u8>,
    pub translation: Vec<u8>,
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
        targets: Vec<Vec<u8>>,
        old_masses: Vec<Vec<u8>>,
        unsupported: Vec<Vec<u8>>,
    },
}
fn list(values: Vec<Vec<u8>>, env: Env) -> napi::Result<Vec<Uint8Array>> {
    let mut out = output_vec(values.len()).map_err(|e| thrown(env, e))?;
    out.extend(values.into_iter().map(Uint8Array::from));
    Ok(out)
}
impl Details {
    pub(crate) fn object(self, env: &Env) -> napi::Result<Object<'_>> {
        let mut result = Object::new(env)?;
        match self {
            Self::Kernel { parent, density } => {
                result.set("parent", Uint8Array::from(parent))?;
                result.set("density", Uint8Array::from(density))?;
            }
            Self::Extension { space, parent } => {
                result.set("space", Uint8Array::from(space))?;
                result.set("parent", Uint8Array::from(parent))?;
            }
            Self::Restriction {
                prior,
                refined,
                space,
                refinement,
                inclusion,
            } => {
                result.set("prior", Uint8Array::from(prior))?;
                result.set("refinedPrior", Uint8Array::from(refined))?;
                result.set("space", Uint8Array::from(space))?;
                result.set("refinement", Uint8Array::from(refinement))?;
                result.set("inclusion", Uint8Array::from(inclusion))?;
            }
            Self::Refinements(values) => {
                result.set("refinements", list(values, *env)?)?;
            }
            Self::Revision {
                identity,
                prior,
                defined,
                receipt,
                outcome,
            } => {
                result.set("identity", Uint8Array::from(identity))?;
                result.set("prior", Uint8Array::from(prior))?;
                result.set("defined", Uint8Array::from(defined))?;
                result.set("receipt", receipt.object(env)?)?;
                let mut revised = Object::new(env)?;
                if let Some(value) = outcome {
                    revised.set("kind", "revised")?;
                    revised.set("refinedPrior", Uint8Array::from(value.refined))?;
                    revised.set("posterior", Uint8Array::from(value.posterior))?;
                    revised.set("restriction", Uint8Array::from(value.restriction))?;
                    revised.set("translation", Uint8Array::from(value.translation))?;
                } else {
                    revised.set("kind", "impossible")?;
                }
                result.set("outcome", revised)?;
            }
        }
        Ok(result)
    }
}
impl Receipt {
    fn object(self, env: &Env) -> napi::Result<Object<'_>> {
        let mut result = Object::new(env)?;
        match self {
            Self::Condition { evidence, mass } => {
                result.set("kind", "condition")?;
                result.set("evidence", Uint8Array::from(evidence))?;
                result.set("mass", Uint8Array::from(mass))?;
            }
            Self::Likelihood {
                likelihood,
                normalizer,
            } => {
                result.set("kind", "likelihood")?;
                result.set("likelihood", Uint8Array::from(likelihood))?;
                result.set("normalizer", Uint8Array::from(normalizer))?;
            }
            Self::Jeffrey {
                cells,
                targets,
                old_masses,
                unsupported,
            } => {
                result.set("kind", "jeffrey")?;
                result.set("cells", list(cells, *env)?)?;
                result.set("targets", list(targets, *env)?)?;
                result.set("oldMasses", list(old_masses, *env)?)?;
                result.set("unsupportedRegions", list(unsupported, *env)?)?;
            }
        }
        Ok(result)
    }
}
