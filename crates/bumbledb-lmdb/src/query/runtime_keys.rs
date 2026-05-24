use crate::query::free_join::FjSubatom;
use crate::query::model::NormalizedQuery;
use crate::query::sink::Binding;
use crate::tuple::EncodedTuple;
use crate::{Error, Result};

pub(super) fn key_from_binding(
    query: &NormalizedQuery,
    binding: &Binding,
    subatom: &FjSubatom,
) -> Result<EncodedTuple> {
    let mut bytes = Vec::new();
    for variable in &subatom.vars {
        let value = binding
            .value(*variable)
            .ok_or_else(|| Error::corrupt(format!("missing variable {variable}")))?;
        let expected = query.variables[*variable].value_type.encoded_width();
        if value.len() != expected {
            return Err(Error::corrupt(format!(
                "binding width mismatch for variable {variable}"
            )));
        }
        bytes.extend_from_slice(value);
    }
    Ok(EncodedTuple::from_bytes(bytes))
}

pub(super) fn key_from_binding_by_bound_widths(
    binding: &Binding,
    subatom: &FjSubatom,
) -> Result<EncodedTuple> {
    let mut bytes = Vec::new();
    for variable in &subatom.vars {
        let value = binding
            .value(*variable)
            .ok_or_else(|| Error::corrupt(format!("missing variable {variable}")))?;
        bytes.extend_from_slice(value);
    }
    Ok(EncodedTuple::from_bytes(bytes))
}
