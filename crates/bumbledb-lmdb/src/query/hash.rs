fn query_shape_key(schema: &StorageSchema, query: &NormalizedQuery) -> QueryShapeKey {
    let mut hasher = blake3::Hasher::new();
    hash_bytes_len_prefixed(&mut hasher, b"bumbledb.query_shape.v1");
    hasher.update(&schema.descriptor().fingerprint().0);
    hash_u64(&mut hasher, query.vars.len() as u64);
    for var in &query.vars {
        hash_u16(&mut hasher, var.id.0);
        hash_bytes_len_prefixed(&mut hasher, var.name.as_bytes());
        hash_value_type(&mut hasher, &var.value_type);
    }
    hash_u64(&mut hasher, query.inputs.len() as u64);
    for input in &query.inputs {
        hash_u16(&mut hasher, input.id.0);
        hash_bytes_len_prefixed(&mut hasher, input.name.as_bytes());
        hash_value_type(&mut hasher, &input.value_type);
    }
    hash_u64(&mut hasher, query.atoms.len() as u64);
    for atom in &query.atoms {
        hash_u16(&mut hasher, atom.id.0);
        hash_u16(&mut hasher, atom.relation.0);
        hash_bytes_len_prefixed(&mut hasher, atom.relation_name.as_bytes());
        hash_u64(&mut hasher, atom.fields.len() as u64);
        for field in &atom.fields {
            hash_u16(&mut hasher, field.field.0);
            hash_bytes_len_prefixed(&mut hasher, field.field_name.as_bytes());
            hash_value_type(&mut hasher, &field.value_type);
            hash_norm_term(&mut hasher, &field.term);
        }
    }
    hash_u64(&mut hasher, query.predicates.len() as u64);
    for predicate in &query.predicates {
        hash_u16(&mut hasher, predicate.id.0);
        hash_comparison_operator(&mut hasher, predicate.op);
        hash_value_type(&mut hasher, &predicate.value_type);
        for operand in &predicate.operands {
            hash_norm_operand(&mut hasher, operand);
        }
    }
    hash_u64(&mut hasher, query.find.len() as u64);
    for term in &query.find {
        hash_find_term(&mut hasher, term);
    }
    hash_output_plan(&mut hasher, &query.output);
    QueryShapeKey(*hasher.finalize().as_bytes())
}

fn query_image_scope_for_query(schema: &StorageSchema, query: &NormalizedQuery) -> QueryImageScope {
    QueryImageScope::relations_all(schema, query.atoms.iter().map(|atom| atom.relation))
}

fn hash_u8(hasher: &mut blake3::Hasher, value: u8) {
    hasher.update(&[value]);
}

fn hash_u16(hasher: &mut blake3::Hasher, value: u16) {
    hasher.update(&value.to_be_bytes());
}

fn hash_u32(hasher: &mut blake3::Hasher, value: u32) {
    hasher.update(&value.to_be_bytes());
}

fn hash_u64(hasher: &mut blake3::Hasher, value: u64) {
    hasher.update(&value.to_be_bytes());
}

fn hash_bytes_len_prefixed(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hash_u64(hasher, bytes.len() as u64);
    hasher.update(bytes);
}

fn hash_value_type(hasher: &mut blake3::Hasher, value_type: &ValueType) {
    match value_type {
        ValueType::Bool => hash_u8(hasher, 1),
        ValueType::U64 => hash_u8(hasher, 2),
        ValueType::I64 => hash_u8(hasher, 3),
        ValueType::TimestampMicros => hash_u8(hasher, 4),
        ValueType::Decimal { scale } => {
            hash_u8(hasher, 5);
            hash_u32(hasher, *scale);
        }
        ValueType::Enum { name } => {
            hash_u8(hasher, 7);
            hash_bytes_len_prefixed(hasher, name.as_bytes());
        }
        ValueType::String => hash_u8(hasher, 8),
        ValueType::Bytes => hash_u8(hasher, 9),
        ValueType::Serial {
            type_name,
            owning_relation,
        } => {
            hash_u8(hasher, 10);
            hash_bytes_len_prefixed(hasher, type_name.as_bytes());
            hash_bytes_len_prefixed(hasher, owning_relation.as_bytes());
        }
    }
}

fn hash_encoded_owned(hasher: &mut blake3::Hasher, value: &EncodedOwned) {
    match value {
        EncodedOwned::One(bytes) => {
            hash_u8(hasher, 1);
            hash_bytes_len_prefixed(hasher, bytes);
        }
        EncodedOwned::Eight(bytes) => {
            hash_u8(hasher, 8);
            hash_bytes_len_prefixed(hasher, bytes);
        }
        EncodedOwned::Sixteen(bytes) => {
            hash_u8(hasher, 16);
            hash_bytes_len_prefixed(hasher, bytes);
        }
    }
}

fn hash_norm_term(hasher: &mut blake3::Hasher, term: &NormTerm) {
    match term {
        NormTerm::Var(variable) => {
            hash_u8(hasher, 1);
            hash_u16(hasher, variable.0);
        }
        NormTerm::Input(input) => {
            hash_u8(hasher, 2);
            hash_u16(hasher, input.0);
        }
        NormTerm::Literal(value) => {
            hash_u8(hasher, 3);
            hash_encoded_owned(hasher, value);
        }
        NormTerm::Wildcard => hash_u8(hasher, 4),
    }
}

fn hash_norm_operand(hasher: &mut blake3::Hasher, operand: &NormOperand) {
    match operand {
        NormOperand::Var(variable) => {
            hash_u8(hasher, 1);
            hash_u16(hasher, variable.0);
        }
        NormOperand::Input(input) => {
            hash_u8(hasher, 2);
            hash_u16(hasher, input.0);
        }
        NormOperand::Literal(value) => {
            hash_u8(hasher, 3);
            hash_encoded_owned(hasher, value);
        }
    }
}

fn hash_comparison_operator(hasher: &mut blake3::Hasher, op: ComparisonOperator) {
    hash_u8(
        hasher,
        match op {
            ComparisonOperator::Eq => 1,
            ComparisonOperator::NotEq => 2,
            ComparisonOperator::Lt => 3,
            ComparisonOperator::Lte => 4,
            ComparisonOperator::Gt => 5,
            ComparisonOperator::Gte => 6,
        },
    );
}

fn hash_aggregate_function(hasher: &mut blake3::Hasher, function: AggregateFunction) {
    hash_u8(
        hasher,
        match function {
            AggregateFunction::CountDomain => 1,
            AggregateFunction::CountDistinct => 2,
            AggregateFunction::Sum => 3,
            AggregateFunction::Min => 4,
            AggregateFunction::Max => 5,
        },
    );
}

fn hash_find_term(hasher: &mut blake3::Hasher, term: &NormFindTerm) {
    match term {
        NormFindTerm::Variable { variable } => {
            hash_u8(hasher, 1);
            hash_u16(hasher, variable.0);
        }
        NormFindTerm::Aggregate {
            function,
            variable,
            domain,
            value_type,
        } => {
            hash_u8(hasher, 2);
            hash_aggregate_function(hasher, *function);
            hash_u16(hasher, variable.0);
            hash_u64(hasher, domain.len() as u64);
            for variable in domain {
                hash_u16(hasher, variable.0);
            }
            hash_value_type(hasher, value_type);
        }
    }
}

fn hash_output_plan(hasher: &mut blake3::Hasher, output: &OutputPlan) {
    match output {
        OutputPlan::Project(project) => {
            hash_u8(hasher, 1);
            hash_u64(hasher, project.vars.len() as u64);
            for variable in &project.vars {
                hash_u16(hasher, variable.0);
            }
        }
        OutputPlan::Aggregate(aggregate) => {
            hash_u8(hasher, 2);
            hash_u64(hasher, aggregate.group_vars.len() as u64);
            for variable in &aggregate.group_vars {
                hash_u16(hasher, variable.0);
            }
            hash_u64(hasher, aggregate.aggregates.len() as u64);
            for term in &aggregate.aggregates {
                hash_aggregate_function(hasher, term.function);
                hash_u16(hasher, term.var.0);
                hash_u64(hasher, term.domain_vars.len() as u64);
                for variable in &term.domain_vars {
                    hash_u16(hasher, variable.0);
                }
                hash_value_type(hasher, &term.value_type);
            }
        }
    }
}

