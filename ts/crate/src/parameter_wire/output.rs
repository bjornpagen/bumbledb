use super::{Capacity, Error, MAX_EVENT_BYTES, Result};
use crate::marshal::output_vec;
use crate::runtime::Output;
use crate::runtime::RuntimeError;
use crate::runtime_wire::{OperationHandle, take_output, thrown};
use napi::bindgen_prelude::{BigInt, Env, External, Object, Uint8Array};
use napi_derive::napi;

#[derive(Default)]
pub(super) struct Budget {
    used: usize,
}
impl Budget {
    pub(super) fn blob(&mut self, bytes: Vec<u8>) -> Result<Vec<u8>> {
        if bytes.len() > MAX_EVENT_BYTES - self.used {
            return Err(Error::Capacity(Capacity::DescriptorBytes));
        }
        self.used += bytes.len();
        Ok(bytes)
    }
}
pub enum Witness {
    Rational(Vec<u8>),
    Algebraic(Vec<u8>),
}
pub struct FunctionPiece {
    pub numerator: Vec<u8>,
    pub denominator: Vec<u8>,
    pub defined: Vec<u8>,
}
pub enum ParameterOutput {
    Region {
        parameter: [u8; 32],
        boundaries: Vec<Vec<u8>>,
        membership: Vec<bool>,
    },
    Roots(Vec<Vec<u8>>),
    Root {
        parameter: [u8; 32],
        polynomial: Vec<u8>,
        lower: Vec<u8>,
        upper: Vec<u8>,
    },
    Witness(Option<Witness>),
    Value(Option<Vec<u8>>),
    Function {
        ambient: Vec<u8>,
        defined: Vec<u8>,
        pieces: Vec<FunctionPiece>,
    },
    Source {
        domain: Vec<u8>,
        guards: Vec<(u8, Vec<u8>)>,
        outcomes: u64,
    },
    Family {
        space: Vec<u8>,
        pieces: Vec<(Vec<u8>, FunctionPiece)>,
    },
    Refinement {
        source: Vec<u8>,
        refined: Vec<u8>,
    },
    World(Option<(Witness, u64)>),
    Cardinality(Option<u64>),
    Observation {
        kind: &'static str,
        input: Vec<u8>,
        given: Vec<u8>,
        numerator: Vec<u8>,
        mass: Vec<u8>,
        value: Vec<u8>,
        defined: Vec<u8>,
    },
}

impl Witness {
    fn object(self, env: &Env) -> napi::Result<Object<'_>> {
        let (kind, bytes) = match self {
            Self::Rational(bytes) => ("rational", bytes),
            Self::Algebraic(bytes) => ("algebraic", bytes),
        };
        let mut result = Object::new(env)?;
        result.set("kind", kind)?;
        result.set("value", Uint8Array::from(bytes))?;
        Ok(result)
    }
}
impl FunctionPiece {
    fn object(self, env: &Env) -> napi::Result<Object<'_>> {
        let mut result = Object::new(env)?;
        result.set("numerator", Uint8Array::from(self.numerator))?;
        result.set("denominator", Uint8Array::from(self.denominator))?;
        result.set("defined", Uint8Array::from(self.defined))?;
        Ok(result)
    }
}

#[napi]
pub fn runtime_event_parameter_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<ParameterOutput> {
    match take_output(env, handle)? {
        Output::Parameter(value) => Ok(value),
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}
impl napi::bindgen_prelude::ToNapiValue for ParameterOutput {
    #[expect(
        unsafe_code,
        reason = "N-API conversion transfers worker-owned exact parameter results"
    )]
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        value: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        let handle = Env::from_raw(env);
        let mut object = Object::new(&handle)?;
        match value {
            Self::Region {
                parameter,
                boundaries,
                membership,
            } => {
                object.set("parameter", Uint8Array::from(parameter.to_vec()))?;
                let mut values = output_vec(boundaries.len()).map_err(|e| thrown(handle, e))?;
                values.extend(boundaries.into_iter().map(Uint8Array::from));
                object.set("boundaries", values)?;
                object.set("membership", membership)?;
            }
            Self::Roots(roots) => {
                let mut values = output_vec(roots.len()).map_err(|e| thrown(handle, e))?;
                values.extend(roots.into_iter().map(Uint8Array::from));
                object.set("roots", values)?;
            }
            Self::Root {
                parameter,
                polynomial,
                lower,
                upper,
            } => {
                object.set("parameter", Uint8Array::from(parameter.to_vec()))?;
                object.set("polynomial", Uint8Array::from(polynomial))?;
                object.set("lower", Uint8Array::from(lower))?;
                object.set("upper", Uint8Array::from(upper))?;
            }
            Self::Witness(value) => {
                object.set("witness", value.map(|v| v.object(&handle)).transpose()?)?;
            }
            Self::Value(value) => object.set("value", value.map(Uint8Array::from))?,
            Self::Function {
                ambient,
                defined,
                pieces,
            } => {
                object.set("ambient", Uint8Array::from(ambient))?;
                object.set("defined", Uint8Array::from(defined))?;
                let mut values = output_vec(pieces.len()).map_err(|e| thrown(handle, e))?;
                for piece in pieces {
                    values.push(piece.object(&handle)?);
                }
                object.set("pieces", values)?;
            }
            other => object = other.source_object(&handle)?,
        }
        // SAFETY: the owned object belongs to the active N-API environment.
        unsafe { Object::to_napi_value(env, object) }
    }
}

impl ParameterOutput {
    fn source_object(self, handle: &Env) -> napi::Result<Object<'_>> {
        let mut object = Object::new(handle)?;
        match self {
            Self::Source {
                domain,
                guards,
                outcomes,
            } => {
                object.set("domain", Uint8Array::from(domain))?;
                object.set("outcomeCoordinates", BigInt::from(outcomes))?;
                let mut values = output_vec(guards.len()).map_err(|e| thrown(*handle, e))?;
                for (coordinate, region) in guards {
                    let mut result = Object::new(handle)?;
                    result.set("coordinate", BigInt::from(u64::from(coordinate)))?;
                    result.set("region", Uint8Array::from(region))?;
                    values.push(result);
                }
                object.set("guards", values)?;
            }
            Self::Family { space, pieces } => {
                object.set("space", Uint8Array::from(space))?;
                let mut values = output_vec(pieces.len()).map_err(|e| thrown(*handle, e))?;
                for (region, value) in pieces {
                    let mut result = Object::new(handle)?;
                    result.set("region", Uint8Array::from(region))?;
                    result.set("value", value.object(handle)?)?;
                    values.push(result);
                }
                object.set("pieces", values)?;
            }
            Self::Refinement { source, refined } => {
                object.set("source", Uint8Array::from(source))?;
                object.set("refined", Uint8Array::from(refined))?;
            }
            Self::World(world) => {
                let result = world
                    .map(|(parameter, outcomes)| {
                        let mut result = Object::new(handle)?;
                        result.set("parameter", parameter.object(handle)?)?;
                        result.set("outcomes", BigInt::from(outcomes))?;
                        napi::Result::Ok(result)
                    })
                    .transpose()?;
                object.set("world", result)?;
            }
            Self::Cardinality(count) => {
                object.set(
                    "kind",
                    if count.is_some() {
                        "finite"
                    } else {
                        "continuum"
                    },
                )?;
                object.set("count", count.map(BigInt::from))?;
            }
            Self::Observation {
                kind,
                input,
                given,
                numerator,
                mass,
                value,
                defined,
            } => {
                object.set("kind", kind)?;
                object.set("input", Uint8Array::from(input))?;
                object.set("given", Uint8Array::from(given))?;
                object.set("numerator", Uint8Array::from(numerator))?;
                object.set("evidenceMass", Uint8Array::from(mass))?;
                object.set("value", Uint8Array::from(value))?;
                object.set("defined", Uint8Array::from(defined))?;
            }
            _ => return Err(thrown(*handle, RuntimeError::InvalidArgument)),
        }
        Ok(object)
    }
}
