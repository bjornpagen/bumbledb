fn sqlite_count(
    conn: &mut Connection,
    sql: &str,
    params: &[SqlParam],
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(sql)?;
    let facts = stmt
        .query_map(params_from_iter(params.iter()), |_| Ok(()))?
        .count();
    Ok(facts)
}

fn sqlite_result_facts(
    conn: &mut Connection,
    sql: &str,
    params: &[SqlParam],
) -> Result<Vec<Vec<SqlValue>>, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(sql)?;
    let column_count = stmt.column_count();
    let mut facts = stmt.query(params_from_iter(params.iter()))?;
    let mut out = Vec::new();
    while let Some(fact) = facts.next()? {
        let mut values = Vec::with_capacity(column_count);
        for index in 0..column_count {
            values.push(match fact.get_ref(index)? {
                rusqlite::types::ValueRef::Null => {
                    return Err("SQLite NULL is not a valid Bumbledb benchmark value".into());
                }
                rusqlite::types::ValueRef::Integer(value) => SqlValue::Integer(value),
                rusqlite::types::ValueRef::Real(_) => {
                    return Err("SQLite REAL is not a valid Bumbledb benchmark value".into());
                }
                rusqlite::types::ValueRef::Text(value) => {
                    SqlValue::Text(std::str::from_utf8(value)?.to_owned())
                }
                rusqlite::types::ValueRef::Blob(value) => SqlValue::Blob(value.to_vec()),
            });
        }
        out.push(values);
    }
    Ok(out)
}

fn bumbledb_sql_facts(
    output: &QueryOutput,
) -> Result<Vec<Vec<SqlValue>>, Box<dyn std::error::Error>> {
    output
        .result
        .facts
        .iter()
        .map(|fact| fact.iter().map(sql_value).collect())
        .collect()
}

fn sql_value(value: &Value) -> Result<SqlValue, Box<dyn std::error::Error>> {
    Ok(match value {
        Value::Bool(value) => SqlValue::Integer(i64::from(*value)),
        Value::U64(value) | Value::Serial(value) => SqlValue::Integer((*value).try_into()?),
        Value::I64(value) => SqlValue::Integer(*value),
        Value::Timestamp(TimestampMicros(value)) => SqlValue::Integer(*value),
        Value::Decimal(DecimalRaw(value)) => SqlValue::Integer((*value).try_into()?),
        Value::Enum(value) => SqlValue::Integer(i64::from(*value)),
        Value::String(value) => SqlValue::Text(value.clone()),
        Value::Bytes(value) => SqlValue::Blob(value.clone()),
    })
}

fn sorted_sql_facts(mut facts: Vec<Vec<SqlValue>>) -> Vec<Vec<SqlValue>> {
    facts.sort();
    facts
}

fn correctness_mode(query: &TypedQuery) -> CorrectnessMode {
    if query
        .find
        .iter()
        .any(|term| matches!(term, TypedFindTerm::Aggregate { .. }))
    {
        CorrectnessMode::AggregateValues
    } else {
        CorrectnessMode::ResultSet
    }
}
