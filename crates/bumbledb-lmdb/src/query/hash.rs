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
