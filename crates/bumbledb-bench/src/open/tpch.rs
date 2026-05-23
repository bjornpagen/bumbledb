fn tpch_open_dataset(
    dir: &Path,
    limit: Option<usize>,
) -> Result<Dataset, Box<dyn std::error::Error>> {
    let mut facts = Vec::new();
    let mut customers = BTreeSet::new();
    let mut suppliers = BTreeSet::new();
    let mut parts = BTreeSet::new();
    let mut orders = BTreeSet::new();
    read_pipe(dir, "customer.tbl", limit, |record| {
        let id = parse_u64(get(&record, 0));
        customers.insert(id);
        facts.push(Fact::new(
            "Customer",
            [
                ("id", Value::Serial(id)),
                ("nation", Value::U64(parse_u64(get(&record, 3)))),
            ],
        ));
        Ok(())
    })?;
    read_pipe(dir, "supplier.tbl", limit, |record| {
        let id = parse_u64(get(&record, 0));
        suppliers.insert(id);
        facts.push(Fact::new(
            "Supplier",
            [
                ("id", Value::Serial(id)),
                ("nation", Value::U64(parse_u64(get(&record, 3)))),
            ],
        ));
        Ok(())
    })?;
    read_pipe(dir, "part.tbl", limit, |record| {
        let id = parse_u64(get(&record, 0));
        parts.insert(id);
        facts.push(Fact::new(
            "Part",
            [
                ("id", Value::Serial(id)),
                ("brand", Value::String(get(&record, 3).to_owned())),
            ],
        ));
        Ok(())
    })?;
    read_pipe(dir, "orders.tbl", limit, |record| {
        let id = parse_u64(get(&record, 0));
        let customer = parse_u64(get(&record, 1));
        if !customers.contains(&customer) {
            return Ok(());
        }
        orders.insert(id);
        facts.push(Fact::new(
            "Orders",
            [
                ("id", Value::Serial(id)),
                ("customer", Value::Serial(customer)),
                (
                    "order_date",
                    Value::Timestamp(TimestampMicros(parse_date(get(&record, 4)))),
                ),
            ],
        ));
        Ok(())
    })?;
    read_pipe(dir, "lineitem.tbl", scaled_limit(limit, 4), |record| {
        let order = parse_u64(get(&record, 0));
        let part = parse_u64(get(&record, 1));
        let supplier = parse_u64(get(&record, 2));
        if !(orders.contains(&order) && parts.contains(&part) && suppliers.contains(&supplier)) {
            return Ok(());
        }
        facts.push(Fact::new(
            "LineItem",
            [
                ("id", Value::Serial(facts.len() as u64 + 1)),
                ("order", Value::Serial(order)),
                ("part", Value::Serial(part)),
                ("supplier", Value::Serial(supplier)),
                ("quantity", Value::I64(parse_decimal_i64(get(&record, 4)))),
                (
                    "extended_price",
                    Value::Decimal(DecimalRaw(parse_decimal_i128(get(&record, 5)))),
                ),
                (
                    "ship_date",
                    Value::Timestamp(TimestampMicros(parse_date(get(&record, 10)))),
                ),
            ],
        ));
        Ok(())
    })?;

    let mut dataset = super_tpch_dataset();
    dataset.name = "tpch-open";
    dataset.facts = facts;
    dataset.fact_source = None;
    Ok(dataset)
}

