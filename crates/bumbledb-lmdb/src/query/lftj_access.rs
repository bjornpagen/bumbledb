fn build_lftj_atom_plans<'image>(
    image: &'image crate::QueryImage,
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
    atoms: &[NormAtom],
    variable_order_ids: &[usize],
    counters: &mut PlanCounters,
) -> Result<Vec<LftjAtomPlan<'image>>> {
    atoms
        .iter()
        .map(|atom| build_lftj_atom_plan(image, query, inputs, atom, variable_order_ids, counters))
        .collect()
}

fn build_lftj_atom_plan<'image>(
    image: &'image crate::QueryImage,
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
    atom: &NormAtom,
    variable_order_ids: &[usize],
    counters: &mut PlanCounters,
) -> Result<LftjAtomPlan<'image>> {
    let source = image
        .relation_by_id(atom.relation)
        .ok_or_else(|| Error::unknown_relation(&atom.relation_name))?;
    let variables = atom_variables_in_plan_order(atom, variable_order_ids);
    let local_comparisons = atom_local_comparison_predicates(query, atom);
    if let Some(slice) =
        lazy_lftj_access_slice(source, inputs, atom, &variables, &local_comparisons)?
    {
        counters.lftj_lazy_access_slices += 1;
        counters.lftj_eager_builds_avoided += 1;
        return Ok(LftjAtomPlan {
            variables,
            fact_count: slice.fact_count,
            source: LftjAtomSource::LazyAccess(slice),
        });
    }
    let cache_key = lftj_atom_cache_key(atom, &variables, inputs, &local_comparisons);
    let cached = image.cached_sorted_trie(cache_key, || {
        if let Some(build) =
            build_durable_lftj_sorted_trie(source, inputs, atom, &variables, &local_comparisons)?
        {
            Ok(build)
        } else {
            build_lftj_sorted_trie(source, query, inputs, atom, &variables, &local_comparisons)
        }
    })?;
    if cached.hit {
        counters.sorted_trie_cache_hits += 1;
    } else {
        counters.sorted_trie_cache_misses += 1;
        counters.sorted_trie_builds += 1;
        counters.sorted_trie_build_micros = counters
            .sorted_trie_build_micros
            .saturating_add(cached.build_micros as u64);
        counters.atom_temp_relation_builds += 1;
        counters.atom_temp_relation_source_facts = counters
            .atom_temp_relation_source_facts
            .saturating_add(cached.source_facts_scanned);
        counters.atom_temp_relation_facts = counters
            .atom_temp_relation_facts
            .saturating_add(cached.index.stats.fact_count as u64);
        counters.lftj_atom_source_facts_scanned = counters
            .lftj_atom_source_facts_scanned
            .saturating_add(cached.source_facts_scanned);
        counters.lftj_atom_facts_retained = counters
            .lftj_atom_facts_retained
            .saturating_add(cached.facts_retained);
        counters.lftj_atom_bytes_copied = counters
            .lftj_atom_bytes_copied
            .saturating_add(cached.bytes_copied);
        counters.lftj_atom_scan_micros = counters
            .lftj_atom_scan_micros
            .saturating_add(cached.scan_micros);
        counters.lftj_atom_column_micros = counters
            .lftj_atom_column_micros
            .saturating_add(cached.column_micros);
        counters.lftj_atom_sort_micros = counters
            .lftj_atom_sort_micros
            .saturating_add(cached.sort_micros);
    }
    Ok(LftjAtomPlan {
        variables,
        fact_count: cached.index.stats.fact_count,
        source: LftjAtomSource::SortedTrie(cached.index.clone()),
    })
}

fn lazy_lftj_access_slice<'a>(
    source: &'a RelationImage,
    inputs: &EncodedInputs,
    atom: &NormAtom,
    variables: &[usize],
    local_comparisons: &[&NormPredicate],
) -> Result<Option<LazyAccessSlice<'a>>> {
    if variables.is_empty()
        || variables.len() > 2
        || !local_comparisons.is_empty()
        || atom_repeats_variable(atom)
    {
        return Ok(None);
    }

    let mut best: Option<(usize, LazyAccessSlice<'a>)> = None;
    for index in source.indexes() {
        let Some((prefix, prefix_field_count, fields)) =
            lazy_access_shape(index, inputs, atom, variables)?
        else {
            continue;
        };
        if !lazy_access_covers_atom(index, atom, variables, prefix_field_count, &fields) {
            continue;
        }
        let range = index.prefix_range(&prefix);
        let fact_count = range.end.saturating_sub(range.start);
        let slice = LazyAccessSlice {
            index,
            fields,
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

fn lazy_access_covers_atom(
    index: &crate::query_image::RelationIndexImage,
    atom: &NormAtom,
    variables: &[usize],
    prefix_field_count: usize,
    fields: &[FieldId],
) -> bool {
    atom.fields.iter().all(|field| match &field.term {
        NormTerm::Input(_) | NormTerm::Literal(_) => index.fields[..prefix_field_count]
            .iter()
            .any(|candidate| candidate == &field.field),
        NormTerm::Var(variable) => {
            variables.contains(&(variable.0 as usize))
                && fields.iter().any(|candidate| candidate == &field.field)
        }
        NormTerm::Wildcard => true,
    })
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

fn build_durable_lftj_sorted_trie(
    source: &RelationImage,
    inputs: &EncodedInputs,
    atom: &NormAtom,
    variables: &[usize],
    local_comparisons: &[&NormPredicate],
) -> Result<Option<SortedTrieBuild>> {
    if variables.is_empty() || !local_comparisons.is_empty() {
        return Ok(None);
    }
    for index in source.indexes() {
        if !atom
            .fields
            .iter()
            .all(|field| index.contains_field(field.field))
        {
            continue;
        }
        let mut prefix = Vec::new();
        let mut cursor = 0usize;
        while let Some(field) = index.fields.get(cursor) {
            let Some(atom_field) = atom
                .fields
                .iter()
                .find(|atom_field| atom_field.field == *field)
            else {
                break;
            };
            match &atom_field.term {
                NormTerm::Input(input) => {
                    let Some(input) = inputs.get(*input) else {
                        return Err(Error::internal("missing normalized input"));
                    };
                    prefix.extend_from_slice(input.as_bytes());
                    cursor += 1;
                }
                NormTerm::Literal(literal) => {
                    prefix.extend_from_slice(literal.as_bytes());
                    cursor += 1;
                }
                NormTerm::Var(_) | NormTerm::Wildcard => break,
            }
        }
        let prefix_field_count = cursor;
        let mut fields = Vec::new();
        let mut eligible = true;
        for variable in variables {
            let Some(atom_field) = atom.fields.iter().find(
                |field| matches!(field.term, NormTerm::Var(id) if id.0 as usize == *variable),
            ) else {
                eligible = false;
                break;
            };
            if index.fields.get(cursor) != Some(&atom_field.field) {
                eligible = false;
                break;
            }
            fields.push(atom_field.field);
            cursor += 1;
        }
        if !eligible {
            continue;
        }
        if atom.fields.iter().any(|field| match &field.term {
            NormTerm::Input(_) | NormTerm::Literal(_) => {
                !index.fields[..prefix_field_count].contains(&field.field)
            }
            NormTerm::Var(variable) => !variables.contains(&(variable.0 as usize)),
            NormTerm::Wildcard => false,
        }) {
            continue;
        }
        return build_sorted_trie_from_relation_index(source.id, index, &prefix, &fields).map(Some);
    }
    Ok(None)
}

fn build_sorted_trie_from_relation_index(
    relation: crate::RelationId,
    index: &crate::query_image::RelationIndexImage,
    prefix: &[u8],
    fields: &[FieldId],
) -> Result<SortedTrieBuild> {
    let start = Instant::now();
    let range = index.prefix_range(prefix);
    let fact_count = range.end.saturating_sub(range.start);
    let order = (0..fact_count)
        .map(|fact| FactId(fact as u32))
        .collect::<Vec<_>>();
    let levels = durable_sorted_trie_levels(index, range.start, fact_count, fields)?;
    let distinct_by_depth = levels
        .iter()
        .map(|level| level.keys.len())
        .collect::<Vec<_>>();
    let mut avg_fanout_by_depth = Vec::new();
    let mut max_fanout_by_depth = Vec::new();
    for level in &levels {
        let mut group_sizes = BTreeMap::<u32, usize>::new();
        for parent in &level.parent {
            *group_sizes.entry(*parent).or_insert(0) += 1;
        }
        let max = group_sizes.values().copied().max().unwrap_or(0);
        let avg = if group_sizes.is_empty() {
            0.0
        } else {
            group_sizes.values().sum::<usize>() as f64 / group_sizes.len() as f64
        };
        max_fanout_by_depth.push(max);
        avg_fanout_by_depth.push(avg);
    }
    let trie = SortedTrieIndex {
        relation,
        name: format!("durable_{}_lftj", index.access.0),
        fields: fields.to_vec(),
        order,
        levels,
        stats: crate::TrieStats {
            fact_count,
            distinct_by_depth,
            avg_fanout_by_depth,
            max_fanout_by_depth,
            build_micros: start.elapsed().as_micros(),
        },
    };
    Ok(SortedTrieBuild {
        index: trie,
        source_facts_scanned: fact_count as u64,
        facts_retained: fact_count as u64,
        bytes_copied: 0,
        scan_micros: 0,
        column_micros: 0,
        sort_micros: start.elapsed().as_micros().min(u128::from(u64::MAX)) as u64,
    })
}

fn durable_sorted_trie_levels(
    index: &crate::query_image::RelationIndexImage,
    base: usize,
    fact_count: usize,
    fields: &[FieldId],
) -> Result<Vec<crate::TrieLevel>> {
    let mut levels = Vec::new();
    let mut parents = vec![(0usize, fact_count, u32::MAX)];
    for field in fields {
        let mut level = crate::TrieLevel {
            field: *field,
            keys: Vec::new(),
            ranges: Vec::new(),
            parent: Vec::new(),
        };
        let mut next_parents = Vec::new();
        for (parent_start, parent_end, parent_index) in parents {
            let mut start = parent_start;
            while start < parent_end {
                let key = durable_index_component_owned(index, base + start, *field)?;
                let mut end = start + 1;
                while end < parent_end {
                    let next = durable_index_component_owned(index, base + end, *field)?;
                    if next != key {
                        break;
                    }
                    end += 1;
                }
                let entry_index = level.keys.len() as u32;
                level.keys.push(key);
                level.ranges.push(FactRange {
                    start: FactId(start as u32),
                    end: FactId(end as u32),
                });
                level.parent.push(parent_index);
                next_parents.push((start, end, entry_index));
                start = end;
            }
        }
        parents = next_parents;
        levels.push(level);
    }
    Ok(levels)
}

fn durable_index_component_owned(
    index: &crate::query_image::RelationIndexImage,
    position: usize,
    field: FieldId,
) -> Result<EncodedOwned> {
    let entry = index
        .entry_at(position)
        .ok_or_else(|| Error::internal("missing durable index entry"))?;
    let bytes = index
        .component_bytes(entry, field)
        .ok_or_else(|| Error::internal("missing durable index trie field"))?;
    encoded_owned_for_width(bytes.len(), bytes)
}

fn build_lftj_sorted_trie(
    source: &RelationImage,
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
    atom: &NormAtom,
    variables: &[usize],
    local_comparisons: &[&NormPredicate],
) -> Result<SortedTrieBuild> {
    let fields = variables
        .iter()
        .enumerate()
        .map(|(id, variable)| crate::FieldImage {
            id: FieldId(id as u16),
            name: query.vars[*variable].name.clone(),
            value_type: query.vars[*variable].value_type.clone(),
            width: query.vars[*variable].value_type.encoded_width(),
        })
        .collect::<Vec<_>>();
    let mut builders = encoded_column_builders(&fields, 0)?;
    let mut included_facts = 0usize;
    let source_facts_scanned;

    let mut bytes_copied = 0u64;
    let scan_start = Instant::now();
    {
        let _span = tracing::debug_span!("bumbledb.query.lftj.build.scan_filter_copy").entered();
        if let Some(indexed) = append_indexed_lftj_atom_values(
            &mut builders,
            source,
            query,
            inputs,
            atom,
            variables,
            local_comparisons,
        )? {
            source_facts_scanned = indexed.source_facts_scanned;
            included_facts = indexed.facts_retained as usize;
            bytes_copied = bytes_copied.saturating_add(indexed.bytes_appended);
        } else {
            source_facts_scanned = source.fact_count as u64;
            for fact in 0..source.fact_count {
                let fact = FactId(fact as u32);
                let Some(slots) =
                    atom_fact_value_slots(source, inputs, atom, fact, query.vars.len())?
                else {
                    continue;
                };
                if !atom_local_comparisons_pass_slots(local_comparisons, inputs, &slots)? {
                    continue;
                }
                included_facts += 1;
                bytes_copied = bytes_copied.saturating_add(append_atom_slots(
                    &mut builders,
                    &slots,
                    variables,
                )?);
            }
        }
    }
    let scan_micros = elapsed_micros(scan_start).min(u128::from(u64::MAX)) as u64;

    let fact_count = if variables.is_empty() {
        included_facts
    } else {
        builders[0].len()
    };
    let encoded_column_bytes = builders
        .iter()
        .map(EncodedColumnBuilder::byte_len)
        .sum::<usize>();
    let column_start = Instant::now();
    let columns = {
        let _span = tracing::debug_span!("bumbledb.query.lftj.build.column_image").entered();
        finish_column_builders(builders)
    };
    let column_micros = elapsed_micros(column_start).min(u128::from(u64::MAX)) as u64;
    let relation = RelationImage {
        id: source.id,
        name: atom.relation_name.clone(),
        fact_count,
        fields,
        columns,
        indexes: Vec::new(),
        stats: RelationStats {
            fact_count,
            field_count: variables.len(),
            encoded_column_bytes,
        },
    };
    let sort_start = Instant::now();
    let trie = {
        let _span = tracing::debug_span!("bumbledb.query.lftj.build.sorted_trie").entered();
        crate::query_image::build_sorted_trie_index(
            &relation,
            IndexSpec::new(
                format!("{}_lftj", atom.relation_name),
                (0..variables.len()).map(|id| FieldId(id as u16)),
            ),
        )?
    };
    let sort_micros = elapsed_micros(sort_start).min(u128::from(u64::MAX)) as u64;
    Ok(SortedTrieBuild {
        index: trie,
        source_facts_scanned,
        facts_retained: fact_count as u64,
        bytes_copied,
        scan_micros,
        column_micros,
        sort_micros,
    })
}

struct IndexedPrefixAppendStats {
    source_facts_scanned: u64,
    facts_retained: u64,
    bytes_appended: u64,
}

type AtomValueSlots = SmallVec<[Option<EncodedOwned>; 8]>;

fn append_indexed_lftj_atom_values(
    builders: &mut [EncodedColumnBuilder],
    source: &RelationImage,
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
    atom: &NormAtom,
    variables: &[usize],
    local_comparisons: &[&NormPredicate],
) -> Result<Option<IndexedPrefixAppendStats>> {
    let mut best = None;
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
        if prefix_fields == 0 {
            continue;
        }
        if best
            .as_ref()
            .is_none_or(|(fields, _, _): &(usize, Vec<u8>, usize)| prefix_fields > *fields)
        {
            best = Some((prefix_fields, prefix, index.access.0 as usize));
        }
    }
    let Some((_, prefix, access)) = best else {
        return Ok(None);
    };
    let index = source
        .indexes()
        .iter()
        .find(|index| index.access.0 as usize == access)
        .ok_or_else(|| Error::internal("missing selected LFTJ atom index"))?;
    let mut source_facts_scanned = 0u64;
    let mut facts_retained = 0u64;
    let mut bytes_appended = 0u64;
    let _span = tracing::trace_span!(
        "bumbledb.query.lftj_atom.indexed_prefix",
        relation = %source.name,
        prefix_bytes = prefix.len()
    )
    .entered();
    for entry in index.entries_with_prefix(&prefix) {
        source_facts_scanned += 1;
        if let Some(slots) =
            atom_index_entry_value_slots(index, inputs, atom, entry, query.vars.len())?
            && atom_local_comparisons_pass_slots(local_comparisons, inputs, &slots)?
        {
            facts_retained += 1;
            bytes_appended =
                bytes_appended.saturating_add(append_atom_slots(builders, &slots, variables)?);
        }
    }
    Ok(Some(IndexedPrefixAppendStats {
        source_facts_scanned,
        facts_retained,
        bytes_appended,
    }))
}

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

fn append_atom_slots(
    builders: &mut [EncodedColumnBuilder],
    slots: &AtomValueSlots,
    variables: &[usize],
) -> Result<u64> {
    let mut bytes_appended = 0u64;
    for (column, variable) in variables.iter().enumerate() {
        let value = slots
            .get(*variable)
            .and_then(Option::as_ref)
            .ok_or_else(|| Error::internal("missing LFTJ variable value"))?;
        builders
            .get_mut(column)
            .ok_or_else(|| Error::internal("missing LFTJ column builder"))?
            .append_encoded_owned(value)?;
        bytes_appended = bytes_appended.saturating_add(value.as_bytes().len() as u64);
    }
    Ok(bytes_appended)
}

fn atom_local_comparisons_pass_slots(
    local_comparisons: &[&NormPredicate],
    inputs: &EncodedInputs,
    slots: &AtomValueSlots,
) -> Result<bool> {
    for predicate in local_comparisons {
        let mut saw_local_variable = false;
        let mut encoded: [Option<&[u8]>; 2] = [None, None];
        for (index, operand) in predicate.operands.iter().enumerate() {
            let Some(out) = encoded.get_mut(index) else {
                return Err(Error::internal("comparison operand index out of bounds"));
            };
            *out = match operand {
                NormOperand::Var(variable) => {
                    let Some(value) = slots.get(variable.0 as usize).and_then(Option::as_ref)
                    else {
                        break;
                    };
                    saw_local_variable = true;
                    Some(value.as_bytes())
                }
                NormOperand::Input(input) => {
                    let Some(input) = inputs.get(*input) else {
                        break;
                    };
                    Some(input.as_bytes())
                }
                NormOperand::Literal(literal) => Some(literal.as_bytes()),
            };
        }
        let [Some(left), Some(right)] = encoded else {
            continue;
        };
        if !saw_local_variable {
            continue;
        }
        if encoded_comparison_supported(predicate.op, &predicate.value_type)
            && !compare_encoded_values(left, predicate.op, right)
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn lftj_atom_cache_key(
    atom: &NormAtom,
    variables: &[usize],
    inputs: &EncodedInputs,
    local_comparisons: &[&NormPredicate],
) -> LftjAtomKey {
    let mut hasher = blake3::Hasher::new();
    hash_bytes_len_prefixed(&mut hasher, b"bumbledb.lftj_atom.v2");
    hash_u16(&mut hasher, atom.relation.0);
    hash_u64(&mut hasher, variables.len() as u64);
    for variable in variables {
        let field = atom
            .fields
            .iter()
            .find(|field| matches!(field.term, NormTerm::Var(id) if id.0 as usize == *variable))
            .map(|field| field.field.0)
            .unwrap_or(u16::MAX);
        hash_u16(&mut hasher, field);
    }
    hash_u64(&mut hasher, atom.fields.len() as u64);
    for field in &atom.fields {
        hash_u16(&mut hasher, field.field.0);
        hash_value_type(&mut hasher, &field.value_type);
        match &field.term {
            NormTerm::Var(variable) => {
                hash_u8(&mut hasher, 1);
                let ordinal = variables
                    .iter()
                    .position(|candidate| *candidate == variable.0 as usize)
                    .unwrap_or(usize::MAX);
                hash_u64(&mut hasher, ordinal as u64);
            }
            NormTerm::Input(input) => {
                hash_u8(&mut hasher, 2);
                hash_u16(&mut hasher, input.0);
                if let Some(value) = inputs.get(*input) {
                    hash_encoded_owned(&mut hasher, value);
                } else {
                    hash_u8(&mut hasher, 0);
                }
            }
            NormTerm::Literal(value) => {
                hash_u8(&mut hasher, 3);
                hash_encoded_owned(&mut hasher, value);
            }
            NormTerm::Wildcard => hash_u8(&mut hasher, 4),
        }
    }
    hash_u64(&mut hasher, local_comparisons.len() as u64);
    for predicate in local_comparisons {
        hash_comparison_operator(&mut hasher, predicate.op);
        hash_value_type(&mut hasher, &predicate.value_type);
        for operand in &predicate.operands {
            hash_lftj_atom_comparison_operand(&mut hasher, operand, variables, inputs);
        }
    }
    LftjAtomKey(*hasher.finalize().as_bytes())
}

fn atom_local_comparison_predicates<'a>(
    query: &'a NormalizedQuery,
    atom: &NormAtom,
) -> Vec<&'a NormPredicate> {
    let variables = atom
        .fields
        .iter()
        .filter_map(|field| match field.term {
            NormTerm::Var(variable) => Some(variable.0 as usize),
            NormTerm::Input(_) | NormTerm::Literal(_) | NormTerm::Wildcard => None,
        })
        .collect::<BTreeSet<_>>();
    query
        .predicates
        .iter()
        .filter(|predicate| {
            encoded_comparison_supported(predicate.op, &predicate.value_type)
                && predicate_is_atom_local(predicate, &variables)
        })
        .collect()
}

fn predicate_is_atom_local(predicate: &NormPredicate, atom_variables: &BTreeSet<usize>) -> bool {
    let mut saw_variable = false;
    for operand in &predicate.operands {
        let NormOperand::Var(variable) = operand else {
            continue;
        };
        saw_variable = true;
        if !atom_variables.contains(&(variable.0 as usize)) {
            return false;
        }
    }
    saw_variable
}

fn hash_lftj_atom_comparison_operand(
    hasher: &mut blake3::Hasher,
    operand: &NormOperand,
    variables: &[usize],
    inputs: &EncodedInputs,
) {
    match operand {
        NormOperand::Var(variable) => {
            hash_u8(hasher, 1);
            let ordinal = variables
                .iter()
                .position(|candidate| *candidate == variable.0 as usize)
                .unwrap_or(usize::MAX);
            hash_u64(hasher, ordinal as u64);
        }
        NormOperand::Input(input) => {
            hash_u8(hasher, 2);
            hash_u16(hasher, input.0);
            if let Some(value) = inputs.get(*input) {
                hash_u8(hasher, 1);
                hash_encoded_owned(hasher, value);
            } else {
                hash_u8(hasher, 0);
            }
        }
        NormOperand::Literal(value) => {
            hash_u8(hasher, 3);
            hash_encoded_owned(hasher, value);
        }
    }
}

fn atom_variables_in_plan_order(atom: &NormAtom, variable_order_ids: &[usize]) -> Vec<usize> {
    variable_order_ids
        .iter()
        .copied()
        .filter(|variable| atom_contains_variable(atom, *variable))
        .collect()
}

fn atom_fact_value_slots(
    relation: &RelationImage,
    inputs: &EncodedInputs,
    atom: &NormAtom,
    fact: FactId,
    variable_count: usize,
) -> Result<Option<AtomValueSlots>> {
    let mut slots = empty_atom_slots(variable_count);
    for field in &atom.fields {
        let bytes = relation
            .encoded_bytes(fact, field.field)
            .ok_or_else(|| Error::internal("missing atom field in relation image"))?;
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

