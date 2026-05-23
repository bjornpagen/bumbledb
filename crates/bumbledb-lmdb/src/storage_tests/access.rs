use super::*;

#[test]
fn read_access_paths_decode_facts_and_preserve_snapshots() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = storage_schema(&env)?;

    env.write(|txn| {
        txn.insert(&schema, holder_fact(1, "Alice"))?;
        txn.insert(&schema, holder_fact(2, "Bob"))?;
        txn.insert(&schema, account_fact(1, 1, 1))?;
        txn.insert(&schema, account_fact(2, 1, 2))?;
        Ok::<(), Error>(())
    })?;

    env.read(|txn| {
        assert!(txn.exact_fact_exists(&schema, &holder_fact(1, "Alice"))?);
        assert!(txn.exact_fact_exists(&schema, &account_fact(1, 1, 1))?);

        let access_paths = schema.access_paths("Account")?;
        assert!(
            access_paths
                .iter()
                .any(|path| path.index_name == FACT_SET_ACCESS_NAME)
        );
        assert!(
            access_paths
                .iter()
                .any(|path| path.index_name == "by_holder")
        );
        assert!(
            access_paths
                .iter()
                .any(|path| path.index_name == "by_opened")
        );
        assert!(
            access_paths
                .iter()
                .any(|path| path.index_name == "unique_holder_currency")
        );

        let full = collect_facts(txn.scan_relation(&schema, "Account")?)?;
        assert_same_facts(&full, &[account_fact(1, 1, 1), account_fact(2, 1, 2)])?;

        let by_holder_items = collect_items(txn.scan_prefix(
            &schema,
            "Account",
            "by_holder",
            &FieldValues::new("Account", [("holder", Value::Serial(1))]),
        )?)?;
        assert_same_facts(
            &by_holder_items
                .iter()
                .map(|item| item.fact.clone())
                .collect::<Vec<_>>(),
            &[account_fact(1, 1, 1), account_fact(2, 1, 2)],
        )?;
        assert!(
            by_holder_items
                .iter()
                .all(|item| item.encoded_component("holder").is_some())
        );

        let unique_holder = collect_facts(txn.scan_prefix(
            &schema,
            "Holder",
            "unique_name",
            &FieldValues::new("Holder", [("name", Value::String("Alice".to_owned()))]),
        )?)?;
        assert_eq!(unique_holder, [holder_fact(1, "Alice")]);

        let ranged = collect_facts(txn.scan_range(
            &schema,
            "Account",
            "by_opened",
            Some(Value::Timestamp(TimestampMicros(15))),
            Some(Value::Timestamp(TimestampMicros(31))),
        )?)?;
        assert_same_facts(&ranged, &[account_fact(2, 1, 2)])?;

        for path in access_paths {
            let facts = collect_facts(txn.scan_prefix(
                &schema,
                "Account",
                &path.index_name,
                &FieldValues::new("Account", std::iter::empty::<(&str, Value)>()),
            )?)?;
            assert_same_facts(&facts, &[account_fact(1, 1, 1), account_fact(2, 1, 2)])?;
        }

        env.write(|write| {
            write.insert(&schema, account_fact(3, 2, 1))?;
            Ok::<(), Error>(())
        })?;

        let still_two = collect_facts(txn.scan_relation(&schema, "Account")?)?;
        assert_same_facts(&still_two, &[account_fact(1, 1, 1), account_fact(2, 1, 2)])?;
        Ok::<(), Error>(())
    })?;

    env.read(|txn| {
        let now_three = collect_facts(txn.scan_relation(&schema, "Account")?)?;
        assert_same_facts(
            &now_three,
            &[
                account_fact(1, 1, 1),
                account_fact(2, 1, 2),
                account_fact(3, 2, 1),
            ],
        )?;
        Ok::<(), Error>(())
    })?;
    Ok(())
}

#[test]
fn access_keys_store_declared_fields_plus_fact_id() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = storage_schema(&env)?;

    env.write(|txn| {
        txn.insert(&schema, holder_fact(1, "Alice"))?;
        txn.insert(&schema, account_fact(1, 1, 1))?;
        Ok::<(), Error>(())
    })?;

    env.read(|txn| {
        let (relation_id, relation) = schema.relation("Account")?;
        let layout = schema
            .layout("Account", "by_holder")
            .ok_or_else(|| Error::internal("missing Account.by_holder access"))?;
        assert_eq!(
            layout
                .components
                .iter()
                .map(|component| component.field_name.as_str())
                .collect::<Vec<_>>(),
            vec!["holder"]
        );

        let encoded = txn.encode_fact_existing(relation_id, relation, &account_fact(1, 1, 1))?;
        let holder_bytes = encoded.field(relation, "holder")?;
        let prefix = access_prefix(relation_id, layout.index_id);
        let keys = txn.raw_index_keys_with_prefix(&prefix)?;
        assert_eq!(keys.len(), 1);

        let key = &keys[0];
        let expected_len = prefix.len() + holder_bytes.len() + FACT_ID_BYTES;
        assert_eq!(key.len(), expected_len);
        assert_eq!(key.get(..prefix.len()), Some(prefix.as_slice()));
        assert_eq!(
            key.get(prefix.len()..prefix.len() + holder_bytes.len()),
            Some(holder_bytes)
        );
        let encoded_id = fact_id(&encoded);
        assert_eq!(
            key.get(expected_len - FACT_ID_BYTES..),
            Some(encoded_id.as_slice())
        );

        let stored_fact = txn
            .raw_index_value(&fact_id_key(relation_id, &encoded))?
            .ok_or_else(|| Error::corrupt("missing fact id lookup"))?;
        assert_eq!(stored_fact.as_slice(), encoded.bytes());

        let by_holder_items = collect_items(txn.scan_prefix(
            &schema,
            "Account",
            "by_holder",
            &FieldValues::new("Account", [("holder", Value::Serial(1))]),
        )?)?;
        assert_eq!(by_holder_items.len(), 1);
        assert!(by_holder_items[0].encoded_component("holder").is_some());
        assert!(by_holder_items[0].encoded_component("id").is_none());
        assert!(by_holder_items[0].encoded_component("currency").is_none());
        assert!(by_holder_items[0].encoded_component("opened").is_none());
        Ok::<(), Error>(())
    })?;

    Ok(())
}

#[test]
fn constraint_guards_use_dedicated_namespaces() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = storage_schema(&env)?;

    env.write(|txn| {
        txn.insert(&schema, holder_fact(1, "Alice"))?;
        txn.insert(&schema, account_fact(1, 1, 1))?;
        Ok::<(), Error>(())
    })?;

    env.read(|txn| {
        let (holder_id, holder_relation) = schema.relation("Holder")?;
        let holder_encoded =
            txn.encode_fact_existing(holder_id, holder_relation, &holder_fact(1, "Alice"))?;
        let (_, holder_id_fields) = target_unique_constraint(holder_relation, "id")?;
        let holder_unique_key = unique_entry_key_from_fact(
            holder_id,
            "id",
            holder_relation,
            &holder_encoded,
            holder_id_fields,
        )?;
        assert_eq!(holder_unique_key[0], NS_UNIQUE_ENTRY);
        let holder_unique_value = txn
            .raw_index_value(&holder_unique_key)?
            .ok_or_else(|| Error::corrupt("missing unique guard"))?;
        let holder_fact_id = fact_id(&holder_encoded);
        assert_eq!(holder_unique_value.as_slice(), holder_fact_id.as_slice());

        let (account_id, account_relation) = schema.relation("Account")?;
        let account_encoded =
            txn.encode_fact_existing(account_id, account_relation, &account_fact(1, 1, 1))?;
        let reverse_prefix = reverse_fk_prefix(
            holder_id,
            "id",
            account_encoded.field(account_relation, "holder")?,
        );
        let reverse_keys = txn.raw_index_keys_with_prefix(&reverse_prefix)?;
        assert_eq!(reverse_keys.len(), 1);
        assert_eq!(reverse_keys[0][0], NS_REVERSE_FK_ENTRY);

        assert_eq!(txn.raw_index_keys_with_prefix(&[NS_UNIQUE_ENTRY])?.len(), 4);
        assert_eq!(
            txn.raw_index_keys_with_prefix(&[NS_REVERSE_FK_ENTRY])?
                .len(),
            1
        );
        Ok::<(), Error>(())
    })?;

    env.write(|txn| {
        txn.delete(&schema, account_fact(1, 1, 1))?;
        Ok::<(), Error>(())
    })?;

    env.read(|txn| {
        assert_eq!(txn.raw_index_keys_with_prefix(&[NS_UNIQUE_ENTRY])?.len(), 2);
        assert_eq!(
            txn.raw_index_keys_with_prefix(&[NS_REVERSE_FK_ENTRY])?
                .len(),
            0
        );
        Ok::<(), Error>(())
    })?;

    Ok(())
}
