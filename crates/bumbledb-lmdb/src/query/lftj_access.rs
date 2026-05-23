use super::*;

pub(super) fn build_lftj_atom_plans<'image>(
    image: &'image crate::QueryImage,
    inputs: &EncodedInputs,
    atoms: &[NormAtom],
    variable_order_ids: &[usize],
    counters: &mut PlanCounters,
) -> Result<Vec<LftjAtomPlan<'image>>> {
    atoms
        .iter()
        .map(|atom| build_lftj_atom_plan(image, inputs, atom, variable_order_ids, counters))
        .collect()
}

fn build_lftj_atom_plan<'image>(
    image: &'image crate::QueryImage,
    inputs: &EncodedInputs,
    atom: &NormAtom,
    variable_order_ids: &[usize],
    counters: &mut PlanCounters,
) -> Result<LftjAtomPlan<'image>> {
    let source = image
        .relation_by_id(atom.relation)
        .ok_or_else(|| Error::unknown_relation(&atom.relation_name))?;
    let variables = atom_variables_in_plan_order(atom, variable_order_ids);
    if variables.is_empty() {
        let slice = static_atom_lazy_access_slice(source, inputs, atom)?;
        counters.lftj_lazy_access_slices += 1;
        return Ok(LftjAtomPlan {
            variables,
            fact_count: slice.fact_count,
            source: LftjAtomSource::LazyAccess(slice),
        });
    }
    if let Some(slice) = lazy_lftj_access_slice(source, inputs, atom, &variables)? {
        counters.lftj_lazy_access_slices += 1;
        return Ok(LftjAtomPlan {
            variables,
            fact_count: slice.fact_count,
            source: LftjAtomSource::LazyAccess(slice),
        });
    }
    Err(Error::internal(format!(
        "LFTJ atom {} has no lazy durable access path for variables {:?}",
        atom.relation_name, variables
    )))
}

fn lazy_lftj_access_slice<'a>(
    source: &'a RelationImage,
    inputs: &EncodedInputs,
    atom: &NormAtom,
    variables: &[usize],
) -> Result<Option<LazyAccessSlice<'a>>> {
    if variables.is_empty() || atom_repeats_variable(atom) {
        return Ok(None);
    }

    let mut best: Option<(usize, LazyAccessSlice<'a>)> = None;
    for index in source.indexes() {
        let Some((prefix, prefix_field_count, fields)) =
            lazy_access_shape(index, inputs, atom, variables)?
        else {
            continue;
        };
        let Some(filters) =
            lazy_access_filters(index, inputs, atom, variables, prefix_field_count, &fields)?
        else {
            continue;
        };
        let range = index.prefix_range(&prefix);
        let fact_count = range.end.saturating_sub(range.start);
        let slice = LazyAccessSlice {
            index,
            fields,
            filters,
            range,
            fact_count,
        };
        if best
            .as_ref()
            .is_none_or(|(existing, _)| fact_count < *existing)
        {
            best = Some((fact_count, slice));
        }
    }
    Ok(best.map(|(_, slice)| slice))
}

fn lazy_access_shape(
    index: &crate::query_image::RelationIndexImage,
    inputs: &EncodedInputs,
    atom: &NormAtom,
    variables: &[usize],
) -> Result<Option<LazyAccessShape>> {
    let mut prefix = Vec::new();
    let mut prefix_field_count = 0usize;
    let mut fields = Vec::with_capacity(variables.len());
    let mut variable_cursor = 0usize;
    let mut saw_variable = false;

    for field in &index.fields {
        let Some(atom_field) = atom
            .fields
            .iter()
            .find(|atom_field| atom_field.field == *field)
        else {
            if saw_variable {
                continue;
            }
            break;
        };
        match &atom_field.term {
            NormTerm::Input(input) if !saw_variable => {
                let input = inputs
                    .get(*input)
                    .ok_or_else(|| Error::internal("missing normalized input"))?;
                prefix.extend_from_slice(input.as_bytes());
                prefix_field_count += 1;
            }
            NormTerm::Literal(literal) if !saw_variable => {
                prefix.extend_from_slice(literal.as_bytes());
                prefix_field_count += 1;
            }
            NormTerm::Var(variable)
                if variable_cursor < variables.len()
                    && variable.0 as usize == variables[variable_cursor] =>
            {
                saw_variable = true;
                fields.push(atom_field.field);
                variable_cursor += 1;
                if variable_cursor == variables.len() {
                    break;
                }
            }
            NormTerm::Input(_) | NormTerm::Literal(_) | NormTerm::Wildcard if saw_variable => {}
            NormTerm::Input(_) | NormTerm::Literal(_) | NormTerm::Var(_) | NormTerm::Wildcard => {
                return Ok(None);
            }
        }
    }

    if variable_cursor == variables.len() {
        Ok(Some((prefix, prefix_field_count, fields)))
    } else {
        Ok(None)
    }
}

fn static_atom_lazy_access_slice<'a>(
    source: &'a RelationImage,
    inputs: &EncodedInputs,
    atom: &NormAtom,
) -> Result<LazyAccessSlice<'a>> {
    let mut best: Option<(usize, Vec<u8>, &'a crate::query_image::RelationIndexImage)> = None;
    for index in source.indexes() {
        if !atom
            .fields
            .iter()
            .all(|field| index.contains_field(field.field))
        {
            continue;
        }
        let mut prefix = Vec::new();
        let mut prefix_fields = 0usize;
        for field in &index.fields {
            let Some(atom_field) = atom
                .fields
                .iter()
                .find(|atom_field| atom_field.field == *field)
            else {
                break;
            };
            let expected = match &atom_field.term {
                NormTerm::Input(input) => inputs.get(*input),
                NormTerm::Literal(literal) => Some(literal),
                NormTerm::Var(_) | NormTerm::Wildcard => None,
            };
            let Some(expected) = expected else {
                break;
            };
            prefix.extend_from_slice(expected.as_bytes());
            prefix_fields += 1;
        }
        if best
            .as_ref()
            .is_none_or(|(existing, _, _)| prefix_fields > *existing)
        {
            best = Some((prefix_fields, prefix, index));
        }
    }
    let Some((_, prefix, index)) = best else {
        return Err(Error::internal(
            "static LFTJ atom has no durable access path",
        ));
    };
    let range = index.prefix_range(&prefix);
    let mut fact_count = 0usize;
    for position in range.clone() {
        let entry = index
            .entry_at(position)
            .ok_or_else(|| Error::internal("missing durable index entry"))?;
        if atom_index_entry_value_slots(index, inputs, atom, entry, 0)?.is_some() {
            fact_count += 1;
        }
    }
    Ok(LazyAccessSlice {
        index,
        fields: Vec::new(),
        filters: Vec::new(),
        range,
        fact_count,
    })
}

fn lazy_access_filters(
    index: &crate::query_image::RelationIndexImage,
    inputs: &EncodedInputs,
    atom: &NormAtom,
    variables: &[usize],
    prefix_field_count: usize,
    fields: &[FieldId],
) -> Result<Option<Vec<LazyFieldFilter>>> {
    let mut filters = Vec::new();
    for field in &atom.fields {
        match &field.term {
            NormTerm::Input(input) => {
                if index.fields[..prefix_field_count].contains(&field.field) {
                    continue;
                }
                if !index.contains_field(field.field) {
                    return Ok(None);
                }
                let input = inputs
                    .get(*input)
                    .ok_or_else(|| Error::internal("missing normalized input"))?;
                filters.push(LazyFieldFilter {
                    field: field.field,
                    expected: input.clone(),
                });
            }
            NormTerm::Literal(literal) => {
                if index.fields[..prefix_field_count].contains(&field.field) {
                    continue;
                }
                if !index.contains_field(field.field) {
                    return Ok(None);
                }
                filters.push(LazyFieldFilter {
                    field: field.field,
                    expected: literal.clone(),
                });
            }
            NormTerm::Var(variable) => {
                if !variables.contains(&(variable.0 as usize))
                    || !fields.iter().any(|candidate| candidate == &field.field)
                {
                    return Ok(None);
                }
            }
            NormTerm::Wildcard => {}
        }
    }
    Ok(Some(filters))
}

fn atom_repeats_variable(atom: &NormAtom) -> bool {
    let mut seen = BTreeSet::new();
    for field in &atom.fields {
        if let NormTerm::Var(variable) = field.term
            && !seen.insert(variable)
        {
            return true;
        }
    }
    false
}

type AtomValueSlots = SmallVec<[Option<EncodedOwned>; 8]>;

fn atom_index_entry_value_slots(
    index: &crate::query_image::RelationIndexImage,
    inputs: &EncodedInputs,
    atom: &NormAtom,
    entry: &[u8],
    variable_count: usize,
) -> Result<Option<AtomValueSlots>> {
    let mut slots = empty_atom_slots(variable_count);
    for field in &atom.fields {
        let bytes = index
            .component_bytes(entry, field.field)
            .ok_or_else(|| Error::internal("missing atom field in relation index image"))?;
        match &field.term {
            NormTerm::Var(variable) => {
                let variable = variable.0 as usize;
                if !bind_atom_slot(&mut slots, variable, &field.value_type, bytes)? {
                    return Ok(None);
                }
            }
            NormTerm::Input(input) => {
                let input = inputs
                    .get(*input)
                    .ok_or_else(|| Error::internal("missing normalized input"))?;
                if input.as_bytes() != bytes {
                    return Ok(None);
                }
            }
            NormTerm::Literal(literal) => {
                if literal.as_bytes() != bytes {
                    return Ok(None);
                }
            }
            NormTerm::Wildcard => {}
        }
    }
    Ok(Some(slots))
}

fn empty_atom_slots(variable_count: usize) -> AtomValueSlots {
    std::iter::repeat_with(|| None)
        .take(variable_count)
        .collect()
}

fn bind_atom_slot(
    slots: &mut AtomValueSlots,
    variable: usize,
    value_type: &ValueType,
    bytes: &[u8],
) -> Result<bool> {
    let slot = slots
        .get_mut(variable)
        .ok_or_else(|| Error::internal("atom variable id out of bounds"))?;
    if let Some(existing) = slot {
        return Ok(existing.as_bytes() == bytes);
    }
    *slot = Some(encoded_owned_for_width(value_type.encoded_width(), bytes)?);
    Ok(true)
}

fn atom_variables_in_plan_order(atom: &NormAtom, variable_order_ids: &[usize]) -> Vec<usize> {
    variable_order_ids
        .iter()
        .copied()
        .filter(|variable| atom_contains_variable(atom, *variable))
        .collect()
}
