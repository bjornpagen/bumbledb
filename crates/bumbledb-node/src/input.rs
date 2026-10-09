//! Cold tree-shaped inputs. Each crosses as one JSON string, is decoded once
//! by serde into these wire types (unknown fields refused, depth bounded by
//! `serde_json`'s recursion limit of 128, lone surrogates refused), and is then
//! lowered infallibly to the engine's own types. The wire types also carry
//! type-only napi annotations so `binding.d.ts` spells them for TypeScript.
use napi_derive::napi;
use serde::de::DeserializeOwned;

pub mod options;
pub mod query;
pub mod schema;
pub mod value;

/// A JSON input that does not match its wire type: the serde path to the
/// offending node (for example `rules[0].atoms[2].bindings`) and why.
#[napi(object, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Malformed {
    pub path: String,
    pub message: String,
}

pub(crate) fn decode<T: DeserializeOwned>(text: &str) -> Result<T, Malformed> {
    let mut deserializer = serde_json::Deserializer::from_str(text);
    let value: T =
        serde_path_to_error::deserialize(&mut deserializer).map_err(|error| Malformed {
            path: error.path().to_string(),
            message: error.inner().to_string(),
        })?;
    deserializer.end().map_err(|error| Malformed {
        path: String::new(),
        message: error.to_string(),
    })?;
    Ok(value)
}

#[cfg(test)]
mod tests;
