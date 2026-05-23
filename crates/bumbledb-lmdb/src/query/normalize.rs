fn validate_inputs(
    schema: &StorageSchema,
    query: &TypedQuery,
    inputs: &InputBindings,
) -> Result<()> {
    for input in &query.inputs {
        input_value(schema, query, inputs, input.id)?;
    }
    Ok(())
}

fn value_matches_type(schema: &StorageSchema, value: &Value, value_type: &ValueType) -> bool {
    if let (Value::Enum(code), ValueType::Enum { name }) = (value, value_type) {
        return schema.descriptor().enum_contains_code(name, *code);
    }
    matches!(
        (value, value_type),
        (Value::Bool(_), ValueType::Bool)
            | (Value::U64(_), ValueType::U64)
            | (Value::I64(_), ValueType::I64)
            | (Value::Serial(_), ValueType::Serial { .. })
            | (Value::Timestamp(_), ValueType::TimestampMicros)
            | (Value::Decimal(_), ValueType::Decimal { .. })
            | (Value::Enum(_), ValueType::Enum { .. })
            | (Value::String(_), ValueType::String)
            | (Value::Bytes(_), ValueType::Bytes)
    )
}

fn literal_to_value(literal: &TypedLiteral) -> Result<Value> {
    let value = match (&literal.literal, &literal.value_type) {
        (Literal::Bool(value), ValueType::Bool) => Value::Bool(*value),
        (Literal::String(value), ValueType::String) => Value::String(value.clone()),
        (Literal::Integer(value), ValueType::U64) => Value::U64(*value as u64),
        (Literal::Integer(value), ValueType::I64) => Value::I64(*value as i64),
        (Literal::Integer(value), ValueType::Serial { .. }) => Value::Serial(*value as u64),
        (Literal::Integer(value), ValueType::Enum { .. }) => Value::Enum(*value as u8),
        (Literal::Integer(value), ValueType::TimestampMicros) => {
            Value::Timestamp(TimestampMicros(*value as i64))
        }
        (Literal::Integer(value), ValueType::Decimal { .. }) => Value::Decimal(DecimalRaw(*value)),
        _ => {
            return Err(Error::internal(
                "typed literal does not match literal value",
            ));
        }
    };
    Ok(value)
}

fn normalize_query(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &TypedQuery,
) -> Result<NormalizedQuery> {
    let vars = query
        .variables
        .iter()
        .map(|variable| NormVar {
            id: VarId(variable.id as u16),
            name: variable.name.clone(),
            value_type: variable.value_type.clone(),
        })
        .collect::<Vec<_>>();
    let inputs = query
        .inputs
        .iter()
        .map(|input| NormInput {
            id: InputId(input.id as u16),
            name: input.name.clone(),
            value_type: input.value_type.clone(),
        })
        .collect::<Vec<_>>();
    let mut atoms = Vec::new();
    let mut predicates = Vec::new();
    for clause in &query.clauses {
        match clause {
            TypedClause::Relation(atom) => atoms.push(normalize_atom(txn, atom, atoms.len())?),
            TypedClause::Comparison(comparison) => {
                predicates.push(normalize_predicate(txn, comparison, predicates.len())?)
            }
        }
    }
    let find = query
        .find
        .iter()
        .map(|term| match term {
            TypedFindTerm::Variable { variable } => NormFindTerm::Variable {
                variable: VarId(*variable as u16),
            },
            TypedFindTerm::Aggregate {
                function,
                variable,
                domain,
                value_type,
            } => NormFindTerm::Aggregate {
                function: *function,
                variable: VarId(*variable as u16),
                domain: domain
                    .iter()
                    .map(|variable| VarId(*variable as u16))
                    .collect(),
                value_type: value_type.clone(),
            },
        })
        .collect::<Vec<_>>();
    let output = output_plan_from_find(&find);
    let normalized = NormalizedQuery {
        vars,
        inputs,
        atoms,
        predicates,
        output,
        find,
    };
    validate_normalized_query(schema, &normalized)?;
    Ok(normalized)
}

fn normalize_atom(txn: &ReadTxn<'_>, atom: &TypedRelationAtom, atom_id: usize) -> Result<NormAtom> {
    let fields = atom
        .fields
        .iter()
        .map(|field| {
            Ok(NormAtomField {
                field: FieldId(field.field_id as u16),
                field_name: field.field.clone(),
                term: normalize_term(txn, &field.term)?,
                value_type: field.value_type.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(NormAtom {
        id: AtomId(atom_id as u16),
        relation: crate::RelationId(atom.relation_id as u16),
        relation_name: atom.relation.clone(),
        fields,
    })
}

fn normalize_term(txn: &ReadTxn<'_>, term: &TypedTerm) -> Result<NormTerm> {
    Ok(match term {
        TypedTerm::Variable(variable) => NormTerm::Var(VarId(*variable as u16)),
        TypedTerm::Input(input) => NormTerm::Input(InputId(*input as u16)),
        TypedTerm::Literal(literal) => NormTerm::Literal(encode_literal(txn, literal)?),
        TypedTerm::Wildcard => NormTerm::Wildcard,
    })
}

fn normalize_predicate(
    txn: &ReadTxn<'_>,
    comparison: &TypedComparison,
    predicate_id: usize,
) -> Result<NormPredicate> {
    Ok(NormPredicate {
        id: PredicateId(predicate_id as u16),
        operands: [
            normalize_operand(txn, &comparison.left, &comparison.value_type)?,
            normalize_operand(txn, &comparison.right, &comparison.value_type)?,
        ],
        op: comparison.operator,
        value_type: comparison.value_type.clone(),
        earliest_depth: None,
    })
}

fn normalize_operand(
    txn: &ReadTxn<'_>,
    operand: &TypedOperand,
    value_type: &ValueType,
) -> Result<NormOperand> {
    Ok(match operand {
        TypedOperand::Variable(variable) => NormOperand::Var(VarId(*variable as u16)),
        TypedOperand::Input(input) => NormOperand::Input(InputId(*input as u16)),
        TypedOperand::Literal(literal) => {
            let value = literal_to_value(literal)?;
            NormOperand::Literal(encode_owned_value(txn, value_type, &value)?)
        }
    })
}

fn encode_literal(txn: &ReadTxn<'_>, literal: &TypedLiteral) -> Result<EncodedOwned> {
    let value = literal_to_value(literal)?;
    encode_owned_value(txn, &literal.value_type, &value)
}

fn encode_owned_value(
    txn: &ReadTxn<'_>,
    value_type: &ValueType,
    value: &Value,
) -> Result<EncodedOwned> {
    let bytes = txn.encode_query_value(value_type, value)?;
    encoded_owned_from_bytes(value_type, bytes)
}

fn encoded_owned_from_bytes(value_type: &ValueType, bytes: Vec<u8>) -> Result<EncodedOwned> {
    encoded_owned_from_slice(value_type, &bytes)
}

fn encoded_owned_from_slice(value_type: &ValueType, bytes: &[u8]) -> Result<EncodedOwned> {
    match value_type.encoded_width() {
        1 => Ok(EncodedOwned::One(exact_encoded_array::<1>(bytes)?)),
        8 => Ok(EncodedOwned::Eight(exact_encoded_array::<8>(bytes)?)),
        16 => Ok(EncodedOwned::Sixteen(exact_encoded_array::<16>(bytes)?)),
        width => Err(Error::internal(format!(
            "unsupported normalized encoded width {width}"
        ))),
    }
}

fn exact_encoded_array<const N: usize>(bytes: &[u8]) -> Result<[u8; N]> {
    bytes
        .try_into()
        .map_err(|_| Error::internal("normalized encoded value width mismatch"))
}

fn encode_inputs(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &NormalizedQuery,
    inputs: &InputBindings,
) -> Result<EncodedInputs> {
    let values = query
        .inputs
        .iter()
        .map(|input| {
            let value = inputs
                .get(&input.name)
                .ok_or_else(|| Error::missing_input(&input.name))?;
            if !value_matches_type(schema, value, &input.value_type) {
                return Err(Error::query_input_type_mismatch(
                    &input.name,
                    value_type_name(&input.value_type),
                    value.kind_name(),
                ));
            }
            encode_owned_value(txn, &input.value_type, value)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(EncodedInputs { values })
}

fn validate_normalized_query(schema: &StorageSchema, query: &NormalizedQuery) -> Result<()> {
    for atom in &query.atoms {
        let (_, relation) = schema.relation(&atom.relation_name)?;
        if atom.relation.0 as usize >= schema.descriptor().relations.len() {
            return Err(Error::unknown_relation(&atom.relation_name));
        }
        for field in &atom.fields {
            let descriptor = relation
                .fields
                .get(field.field.0 as usize)
                .ok_or_else(|| Error::unknown_field(&atom.relation_name, &field.field_name))?;
            if descriptor.name != field.field_name {
                return Err(Error::unknown_field(&atom.relation_name, &field.field_name));
            }
        }
    }
    Ok(())
}

fn attach_predicate_depths(query: &mut NormalizedQuery, variable_order_ids: &[usize]) {
    let mut depth_by_var = BTreeMap::new();
    for (depth, variable) in variable_order_ids.iter().enumerate() {
        depth_by_var.insert(VarId(*variable as u16), depth);
    }
    for predicate in &mut query.predicates {
        predicate.earliest_depth = predicate
            .operands
            .iter()
            .filter_map(|operand| match operand {
                NormOperand::Var(variable) => depth_by_var.get(variable).copied(),
                NormOperand::Input(_) | NormOperand::Literal(_) => Some(0),
            })
            .max();
    }
}

fn has_aggregate(query: &NormalizedQuery) -> bool {
    query
        .find
        .iter()
        .any(|term| matches!(term, NormFindTerm::Aggregate { .. }))
}

fn result_columns(query: &NormalizedQuery) -> Vec<ResultColumn> {
    query
        .find
        .iter()
        .map(|term| match term {
            NormFindTerm::Variable { variable } => {
                ResultColumn::Variable(query.vars[variable.0 as usize].name.clone())
            }
            NormFindTerm::Aggregate {
                function, variable, ..
            } => ResultColumn::Aggregate {
                function: *function,
                variable: query.vars[variable.0 as usize].name.clone(),
            },
        })
        .collect()
}

