use super::{Capacity, Error, MAX_EVENT_BYTES, Result};
use crate::marshal::output_vec;
use crate::runtime::Output;
use crate::runtime::RuntimeError;
use crate::runtime_wire::{OperationHandle, take_output, thrown};
use napi::bindgen_prelude::{Env, External, Object, Uint8Array};
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
                let witness = value
                    .map(|value| {
                        let (kind, bytes) = match value {
                            Witness::Rational(bytes) => ("rational", bytes),
                            Witness::Algebraic(bytes) => ("algebraic", bytes),
                        };
                        let mut result = Object::new(&handle)?;
                        result.set("kind", kind)?;
                        result.set("value", Uint8Array::from(bytes))?;
                        napi::Result::Ok(result)
                    })
                    .transpose()?;
                object.set("witness", witness)?;
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
                    let mut result = Object::new(&handle)?;
                    result.set("numerator", Uint8Array::from(piece.numerator))?;
                    result.set("denominator", Uint8Array::from(piece.denominator))?;
                    result.set("defined", Uint8Array::from(piece.defined))?;
                    values.push(result);
                }
                object.set("pieces", values)?;
            }
        }
        // SAFETY: the owned object belongs to the active N-API environment.
        unsafe { Object::to_napi_value(env, object) }
    }
}
