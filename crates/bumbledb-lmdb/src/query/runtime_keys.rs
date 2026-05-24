use crate::colt::KeyScratch;
use crate::query::model::NormalizedQuery;
use crate::query::sink::Binding;
use crate::tuple::EncodedTupleRef;
use crate::{Error, Result};

pub(super) fn key_from_binding_with_scratch<'scratch>(
    query: &NormalizedQuery,
    binding: &Binding,
    vars: &[usize],
    scratch: &'scratch mut KeyScratch,
) -> Result<EncodedTupleRef<'scratch>> {
    scratch.clear();
    for variable in vars {
        let value = binding
            .value(*variable)
            .ok_or_else(|| Error::corrupt(format!("missing variable {variable}")))?;
        let expected = query.variables[*variable].value_type.encoded_width();
        if value.len() != expected {
            return Err(Error::corrupt(format!(
                "binding width mismatch for variable {variable}"
            )));
        }
        scratch.extend_from_slice(value);
    }
    Ok(EncodedTupleRef::new(scratch.bytes()))
}

pub(super) fn key_from_binding_by_bound_widths_with_scratch<'scratch>(
    binding: &Binding,
    vars: &[usize],
    scratch: &'scratch mut KeyScratch,
) -> Result<EncodedTupleRef<'scratch>> {
    scratch.clear();
    for variable in vars {
        let value = binding
            .value(*variable)
            .ok_or_else(|| Error::corrupt(format!("missing variable {variable}")))?;
        scratch.extend_from_slice(value);
    }
    Ok(EncodedTupleRef::new(scratch.bytes()))
}
