use super::*;
use crate::{ConstraintError, Environment};
use bumbledb_core::schema::{ConstraintDescriptor, FieldDescriptor, IndexDescriptor};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn inserts_facts_accesses_stats_and_reopens() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = storage_schema(&env)?;

    env.write(|txn| {
        txn.insert(&schema, holder_fact(1, "Alice"))?;
        txn.insert(&schema, account_fact(1, 1, 1))?;
        Ok::<(), Error>(())
    })?;

    env.read(|txn| {
        assert_eq!(txn.last_committed_tx_id()?, 1);
        assert_eq!(txn.relation_fact_count(&schema, "Holder")?, 1);
        assert_eq!(txn.canonical_fact_count(&schema, "Holder")?, 1);
        assert_eq!(txn.relation_fact_count(&schema, "Account")?, 1);
        assert_eq!(txn.canonical_fact_count(&schema, "Account")?, 1);
        assert_eq!(
            txn.access_entry_count(&schema, "Holder", FACT_SET_ACCESS_NAME)?,
            1
        );
        assert_eq!(txn.access_entry_count(&schema, "Holder", "unique_name")?, 1);
        assert_eq!(
            txn.access_entry_count(&schema, "Account", FACT_SET_ACCESS_NAME)?,
            1
        );
        assert_eq!(txn.access_entry_count(&schema, "Account", "by_holder")?, 1);
        assert_eq!(
            txn.access_entry_count(&schema, "Account", "unique_holder_currency")?,
            1
        );
        assert!(txn.access_entry_exists(
            &schema,
            &holder_fact(1, "Alice"),
            FACT_SET_ACCESS_NAME
        )?);
        assert!(txn.access_entry_exists(&schema, &account_fact(1, 1, 1), "by_holder")?);
        assert!(txn.dictionary_string_id("Alice")?.is_some());
        Ok::<(), Error>(())
    })?;

    drop(env);
    let env = Environment::open(dir.path())?;
    let schema = storage_schema(&env)?;
    env.read(|txn| {
        assert_eq!(txn.last_committed_tx_id()?, 1);
        assert_eq!(txn.relation_fact_count(&schema, "Holder")?, 1);
        assert_eq!(txn.canonical_fact_count(&schema, "Holder")?, 1);
        assert!(txn.exact_fact_exists(&schema, &holder_fact(1, "Alice"))?);
        assert!(txn.dictionary_string_id("Alice")?.is_some());
        Ok::<(), Error>(())
    })?;

    Ok(())
}

#[test]
fn duplicate_unique_and_foreign_key_failures_abort_cleanly() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = storage_schema(&env)?;

    env.write(|txn| {
        txn.insert(&schema, holder_fact(1, "Alice"))?;
        Ok::<(), Error>(())
    })?;

    let duplicate = env.write(|txn| txn.insert(&schema, holder_fact(1, "Alice")));
    assert_eq!(duplicate?, InsertOutcome::AlreadyPresent);

    env.read(|txn| {
        assert_eq!(txn.last_committed_tx_id()?, 1);
        assert_eq!(txn.relation_fact_count(&schema, "Holder")?, 1);
        assert_eq!(
            txn.access_entry_count(&schema, "Holder", FACT_SET_ACCESS_NAME)?,
            1
        );
        Ok::<(), Error>(())
    })?;

    let duplicate_unique = env.write(|txn| txn.insert(&schema, holder_fact(1, "Bob")));
    assert!(matches!(
        duplicate_unique,
        Err(Error::Constraint(ConstraintError::UniqueViolation { .. }))
    ));

    let unique = env.write(|txn| txn.insert(&schema, holder_fact(2, "Alice")));
    assert!(matches!(
        unique,
        Err(Error::Constraint(ConstraintError::UniqueViolation { .. }))
    ));

    let fk = env.write(|txn| txn.insert(&schema, account_fact(1, 999, 1)));
    assert!(matches!(
        fk,
        Err(Error::Constraint(
            ConstraintError::ForeignKeyViolation { .. }
        ))
    ));

    env.read(|txn| {
        assert_eq!(txn.last_committed_tx_id()?, 1);
        assert_eq!(txn.relation_fact_count(&schema, "Holder")?, 1);
        assert_eq!(txn.relation_fact_count(&schema, "Account")?, 0);
        assert_eq!(txn.dictionary_string_id("Bob")?, None);
        Ok::<(), Error>(())
    })?;
    Ok(())
}

#[test]
fn invalid_enum_value_fails_before_insert() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = storage_schema(&env)?;

    env.write(|txn| {
        txn.insert(&schema, holder_fact(1, "Alice"))?;
        Ok::<(), Error>(())
    })?;

    let invalid = env.write(|txn| txn.insert(&schema, account_fact(1, 1, 123)));
    assert!(matches!(
        invalid,
        Err(Error::Constraint(ConstraintError::TypeMismatch { .. }))
    ));

    env.read(|txn| {
        assert_eq!(txn.relation_fact_count(&schema, "Account")?, 0);
        Ok::<(), Error>(())
    })?;
    Ok(())
}

#[test]
fn compound_foreign_key_insert_and_restrict_delete() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = compound_fk_schema(&env)?;

    let missing_parent = env.write(|txn| {
        txn.insert(
            &schema,
            Fact::new(
                "Child",
                [
                    ("id", Value::U64(1)),
                    ("parent_a", Value::U64(10)),
                    ("parent_b", Value::U64(20)),
                ],
            ),
        )
    });
    assert!(matches!(
        missing_parent,
        Err(Error::Constraint(
            ConstraintError::ForeignKeyViolation { .. }
        ))
    ));

    env.write(|txn| {
        txn.insert(
            &schema,
            Fact::new("Parent", [("a", Value::U64(10)), ("b", Value::U64(20))]),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "Child",
                [
                    ("id", Value::U64(1)),
                    ("parent_a", Value::U64(10)),
                    ("parent_b", Value::U64(20)),
                ],
            ),
        )?;
        Ok::<(), Error>(())
    })?;

    let restricted = env.write(|txn| {
        txn.delete(
            &schema,
            Fact::new("Parent", [("a", Value::U64(10)), ("b", Value::U64(20))]),
        )
    });
    assert!(matches!(
        restricted,
        Err(Error::Constraint(ConstraintError::RestrictViolation { .. }))
    ));

    env.write(|txn| {
        txn.delete(
            &schema,
            Fact::new(
                "Child",
                [
                    ("id", Value::U64(1)),
                    ("parent_a", Value::U64(10)),
                    ("parent_b", Value::U64(20)),
                ],
            ),
        )?;
        txn.delete(
            &schema,
            Fact::new("Parent", [("a", Value::U64(10)), ("b", Value::U64(20))]),
        )?;
        Ok::<(), Error>(())
    })?;

    env.read(|txn| {
        assert_eq!(txn.relation_fact_count(&schema, "Parent")?, 0);
        assert_eq!(txn.relation_fact_count(&schema, "Child")?, 0);
        assert!(
            schema
                .access_paths("Child")?
                .iter()
                .any(|path| path.index_name == "by_fk_parent")
        );
        Ok::<(), Error>(())
    })?;
    Ok(())
}

#[test]
fn enum_foreign_key_insert_and_restrict_delete() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = enum_fk_schema(&env)?;

    let missing_currency = env.write(|txn| {
        txn.insert(
            &schema,
            Fact::new(
                "Account",
                [("id", Value::U64(1)), ("currency", Value::Enum(1))],
            ),
        )
    });
    assert!(matches!(
        missing_currency,
        Err(Error::Constraint(
            ConstraintError::ForeignKeyViolation { .. }
        ))
    ));

    env.write(|txn| {
        txn.insert(&schema, Fact::new("Currency", [("code", Value::Enum(1))]))?;
        txn.insert(
            &schema,
            Fact::new(
                "Account",
                [("id", Value::U64(1)), ("currency", Value::Enum(1))],
            ),
        )?;
        Ok::<(), Error>(())
    })?;

    let restricted =
        env.write(|txn| txn.delete(&schema, Fact::new("Currency", [("code", Value::Enum(1))])));
    assert!(matches!(
        restricted,
        Err(Error::Constraint(ConstraintError::RestrictViolation { .. }))
    ));

    env.read(|txn| {
        let account = collect_facts(txn.scan_relation(&schema, "Account")?)?;
        assert_eq!(
            account,
            [Fact::new(
                "Account",
                [("id", Value::U64(1)), ("currency", Value::Enum(1))]
            )]
        );
        Ok::<(), Error>(())
    })?;
    Ok(())
}

#[test]
fn compound_enum_foreign_key_insert_and_restrict_delete() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = compound_enum_fk_schema(&env)?;

    let missing_policy = env.write(|txn| {
        txn.insert(
            &schema,
            Fact::new(
                "Account",
                [
                    ("id", Value::U64(1)),
                    ("country", Value::Enum(1)),
                    ("currency", Value::Enum(2)),
                ],
            ),
        )
    });
    assert!(matches!(
        missing_policy,
        Err(Error::Constraint(
            ConstraintError::ForeignKeyViolation { .. }
        ))
    ));

    env.write(|txn| {
        txn.insert(
            &schema,
            Fact::new(
                "Policy",
                [("country", Value::Enum(1)), ("currency", Value::Enum(2))],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "Account",
                [
                    ("id", Value::U64(1)),
                    ("country", Value::Enum(1)),
                    ("currency", Value::Enum(2)),
                ],
            ),
        )?;
        Ok::<(), Error>(())
    })?;

    let restricted = env.write(|txn| {
        txn.delete(
            &schema,
            Fact::new(
                "Policy",
                [("country", Value::Enum(1)), ("currency", Value::Enum(2))],
            ),
        )
    });
    assert!(matches!(
        restricted,
        Err(Error::Constraint(ConstraintError::RestrictViolation { .. }))
    ));

    env.read(|txn| {
        assert_eq!(txn.relation_fact_count(&schema, "Policy")?, 1);
        assert_eq!(txn.relation_fact_count(&schema, "Account")?, 1);
        Ok::<(), Error>(())
    })?;
    Ok(())
}

#[test]
fn compound_serial_enum_foreign_key_insert_and_restrict_delete() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = compound_serial_enum_fk_schema(&env)?;

    let missing_account = env.write(|txn| {
        txn.insert(
            &schema,
            Fact::new(
                "Posting",
                [
                    ("id", Value::U64(1)),
                    ("account", Value::Serial(7)),
                    ("currency", Value::Enum(1)),
                ],
            ),
        )
    });
    assert!(matches!(
        missing_account,
        Err(Error::Constraint(
            ConstraintError::ForeignKeyViolation { .. }
        ))
    ));

    env.write(|txn| {
        txn.insert(
            &schema,
            Fact::new(
                "AccountCurrency",
                [("account", Value::Serial(7)), ("currency", Value::Enum(1))],
            ),
        )?;
        txn.insert(
            &schema,
            Fact::new(
                "Posting",
                [
                    ("id", Value::U64(1)),
                    ("account", Value::Serial(7)),
                    ("currency", Value::Enum(1)),
                ],
            ),
        )?;
        Ok::<(), Error>(())
    })?;

    for (id, account, currency) in [(2, 8, 1), (3, 7, 2)] {
        let missing_component = env.write(|txn| {
            txn.insert(
                &schema,
                Fact::new(
                    "Posting",
                    [
                        ("id", Value::U64(id)),
                        ("account", Value::Serial(account)),
                        ("currency", Value::Enum(currency)),
                    ],
                ),
            )
        });
        assert!(matches!(
            missing_component,
            Err(Error::Constraint(
                ConstraintError::ForeignKeyViolation { .. }
            ))
        ));
    }

    let restricted = env.write(|txn| {
        txn.delete(
            &schema,
            Fact::new(
                "AccountCurrency",
                [("account", Value::Serial(7)), ("currency", Value::Enum(1))],
            ),
        )
    });
    assert!(matches!(
        restricted,
        Err(Error::Constraint(ConstraintError::RestrictViolation { .. }))
    ));

    env.write(|txn| {
        assert_eq!(
            txn.delete(
                &schema,
                Fact::new(
                    "Posting",
                    [
                        ("id", Value::U64(1)),
                        ("account", Value::Serial(7)),
                        ("currency", Value::Enum(1)),
                    ],
                ),
            )?,
            DeleteOutcome::Deleted
        );
        assert_eq!(
            txn.delete(
                &schema,
                Fact::new(
                    "AccountCurrency",
                    [("account", Value::Serial(7)), ("currency", Value::Enum(1))],
                ),
            )?,
            DeleteOutcome::Deleted
        );
        Ok::<(), Error>(())
    })?;
    Ok(())
}

#[test]
fn exact_delete_then_insert_updates_current_entries_and_counts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = storage_schema(&env)?;

    env.write(|txn| {
        txn.insert(&schema, holder_fact(1, "Alice"))?;
        txn.insert(&schema, account_fact(1, 1, 1))?;
        Ok::<(), Error>(())
    })?;

    env.write(|txn| {
        assert_eq!(
            txn.delete(&schema, account_fact(1, 1, 1))?,
            DeleteOutcome::Deleted
        );
        assert_eq!(
            txn.insert(&schema, account_fact(1, 1, 2))?,
            InsertOutcome::Inserted
        );
        Ok::<(), Error>(())
    })?;

    env.read(|txn| {
        assert_eq!(txn.last_committed_tx_id()?, 2);
        assert_eq!(txn.relation_fact_count(&schema, "Account")?, 1);
        assert_eq!(
            txn.access_entry_count(&schema, "Account", FACT_SET_ACCESS_NAME)?,
            1
        );
        assert!(!txn.access_entry_exists(&schema, &account_fact(1, 1, 1), FACT_SET_ACCESS_NAME)?);
        assert!(txn.access_entry_exists(&schema, &account_fact(1, 1, 2), FACT_SET_ACCESS_NAME)?);
        Ok::<(), Error>(())
    })?;

    env.write(|txn| {
        txn.insert(&schema, account_fact(2, 1, 1))?;
        Ok::<(), Error>(())
    })?;
    Ok(())
}

#[test]
fn deletes_restrict_then_remove_accesses_and_facts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = storage_schema(&env)?;

    env.write(|txn| {
        txn.insert(&schema, holder_fact(1, "Alice"))?;
        txn.insert(&schema, account_fact(1, 1, 1))?;
        Ok::<(), Error>(())
    })?;

    let restricted = env.write(|txn| txn.delete(&schema, holder_fact(1, "Alice")));
    assert!(matches!(
        restricted,
        Err(Error::Constraint(ConstraintError::RestrictViolation { .. }))
    ));

    env.write(|txn| {
        txn.delete(&schema, account_fact(1, 1, 1))?;
        txn.delete(&schema, holder_fact(1, "Alice"))?;
        Ok::<(), Error>(())
    })?;

    env.read(|txn| {
        assert_eq!(txn.last_committed_tx_id()?, 2);
        assert_eq!(txn.relation_fact_count(&schema, "Holder")?, 0);
        assert_eq!(txn.relation_fact_count(&schema, "Account")?, 0);
        assert!(!txn.exact_fact_exists(&schema, &holder_fact(1, "Alice"))?);
        assert_eq!(txn.access_entry_count(&schema, "Account", "by_holder")?, 0);
        Ok::<(), Error>(())
    })?;
    Ok(())
}

#[test]
fn composite_facts_insert_duplicate_and_delete() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    let schema = storage_schema(&env)?;

    env.write(|txn| {
        txn.insert(&schema, holder_fact(1, "Alice"))?;
        txn.insert(&schema, account_fact(1, 1, 1))?;
        txn.insert(&schema, tag_fact(1, 7))?;
        Ok::<(), Error>(())
    })?;

    let duplicate = env.write(|txn| txn.insert(&schema, tag_fact(1, 7)));
    assert_eq!(duplicate?, InsertOutcome::AlreadyPresent);

    env.read(|txn| {
        assert_eq!(txn.relation_fact_count(&schema, "AccountTag")?, 1);
        assert_eq!(
            txn.access_entry_count(&schema, "AccountTag", FACT_SET_ACCESS_NAME)?,
            1
        );
        assert_eq!(
            txn.access_entry_count(&schema, "AccountTag", "by_account")?,
            1
        );
        Ok::<(), Error>(())
    })?;

    env.write(|txn| {
        assert_eq!(txn.delete(&schema, tag_fact(1, 7))?, DeleteOutcome::Deleted);
        assert_eq!(txn.delete(&schema, tag_fact(1, 7))?, DeleteOutcome::Absent);
        Ok::<(), Error>(())
    })?;

    env.read(|txn| {
        assert_eq!(txn.relation_fact_count(&schema, "AccountTag")?, 0);
        assert_eq!(
            txn.access_entry_count(&schema, "AccountTag", FACT_SET_ACCESS_NAME)?,
            0
        );
        assert_eq!(
            txn.access_entry_count(&schema, "AccountTag", "by_account")?,
            0
        );
        Ok::<(), Error>(())
    })?;
    Ok(())
}

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

fn storage_schema(env: &Environment) -> Result<StorageSchema> {
    StorageSchema::new(ledger_schema(), env.max_key_size())
}

fn compound_fk_schema(env: &Environment) -> Result<StorageSchema> {
    StorageSchema::new(
        SchemaDescriptor::new(
            "CompoundFkDb",
            vec![
                RelationDescriptor::new(
                    "Parent",
                    vec![
                        FieldDescriptor::new("a", ValueType::U64),
                        FieldDescriptor::new("b", ValueType::U64),
                    ],
                )
                .with_unique("by_ab", ["a", "b"]),
                RelationDescriptor::new(
                    "Child",
                    vec![
                        FieldDescriptor::new("id", ValueType::U64),
                        FieldDescriptor::new("parent_a", ValueType::U64),
                        FieldDescriptor::new("parent_b", ValueType::U64),
                    ],
                )
                .with_unique("id", ["id"])
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "parent",
                    ["parent_a", "parent_b"],
                    "Parent",
                    "by_ab",
                )),
            ],
        ),
        env.max_key_size(),
    )
}

fn enum_fk_schema(env: &Environment) -> Result<StorageSchema> {
    StorageSchema::new(
        SchemaDescriptor::new(
            "EnumFkDb",
            vec![
                RelationDescriptor::new(
                    "Currency",
                    vec![FieldDescriptor::new(
                        "code",
                        ValueType::Enum {
                            name: "Currency".to_owned(),
                        },
                    )],
                )
                .with_unique("code", ["code"]),
                RelationDescriptor::new(
                    "Account",
                    vec![
                        FieldDescriptor::new("id", ValueType::U64),
                        FieldDescriptor::new(
                            "currency",
                            ValueType::Enum {
                                name: "Currency".to_owned(),
                            },
                        ),
                    ],
                )
                .with_unique("id", ["id"])
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "currency",
                    ["currency"],
                    "Currency",
                    "code",
                )),
            ],
        )
        .with_enum(bumbledb_core::schema::EnumDescriptor::codes(
            "Currency",
            [1, 2, 3],
        )),
        env.max_key_size(),
    )
}

fn compound_enum_fk_schema(env: &Environment) -> Result<StorageSchema> {
    StorageSchema::new(
        SchemaDescriptor::new(
            "CompoundEnumFkDb",
            vec![
                RelationDescriptor::new(
                    "Policy",
                    vec![
                        FieldDescriptor::new(
                            "country",
                            ValueType::Enum {
                                name: "Country".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "currency",
                            ValueType::Enum {
                                name: "Currency".to_owned(),
                            },
                        ),
                    ],
                )
                .with_unique("by_country_currency", ["country", "currency"]),
                RelationDescriptor::new(
                    "Account",
                    vec![
                        FieldDescriptor::new("id", ValueType::U64),
                        FieldDescriptor::new(
                            "country",
                            ValueType::Enum {
                                name: "Country".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "currency",
                            ValueType::Enum {
                                name: "Currency".to_owned(),
                            },
                        ),
                    ],
                )
                .with_unique("id", ["id"])
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "policy",
                    ["country", "currency"],
                    "Policy",
                    "by_country_currency",
                )),
            ],
        )
        .with_enum(bumbledb_core::schema::EnumDescriptor::codes(
            "Country",
            [1, 2, 3],
        ))
        .with_enum(bumbledb_core::schema::EnumDescriptor::codes(
            "Currency",
            [1, 2, 3],
        )),
        env.max_key_size(),
    )
}

fn compound_serial_enum_fk_schema(env: &Environment) -> Result<StorageSchema> {
    StorageSchema::new(
        SchemaDescriptor::new(
            "CompoundSerialEnumFkDb",
            vec![
                RelationDescriptor::new(
                    "AccountCurrency",
                    vec![
                        FieldDescriptor::new(
                            "account",
                            ValueType::Serial {
                                type_name: "AccountId".to_owned(),
                                owning_relation: "Account".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "currency",
                            ValueType::Enum {
                                name: "Currency".to_owned(),
                            },
                        ),
                    ],
                )
                .with_unique("by_account_currency", ["account", "currency"]),
                RelationDescriptor::new(
                    "Posting",
                    vec![
                        FieldDescriptor::new("id", ValueType::U64),
                        FieldDescriptor::new(
                            "account",
                            ValueType::Serial {
                                type_name: "AccountId".to_owned(),
                                owning_relation: "Account".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "currency",
                            ValueType::Enum {
                                name: "Currency".to_owned(),
                            },
                        ),
                    ],
                )
                .with_unique("id", ["id"])
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "account_currency",
                    ["account", "currency"],
                    "AccountCurrency",
                    "by_account_currency",
                )),
            ],
        )
        .with_enum(bumbledb_core::schema::EnumDescriptor::codes(
            "Currency",
            [1, 2],
        )),
        env.max_key_size(),
    )
}

fn ledger_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "LedgerDb",
        vec![
            RelationDescriptor::new(
                "Holder",
                vec![
                    FieldDescriptor::new(
                        "id",
                        ValueType::Serial {
                            type_name: "HolderId".to_owned(),
                            owning_relation: "Holder".to_owned(),
                        },
                    ),
                    FieldDescriptor::new("name", ValueType::String),
                ],
            )
            .with_unique("id", ["id"])
            .with_constraint(ConstraintDescriptor::unique("name", ["name"])),
            RelationDescriptor::new(
                "Account",
                vec![
                    FieldDescriptor::new(
                        "id",
                        ValueType::Serial {
                            type_name: "AccountId".to_owned(),
                            owning_relation: "Account".to_owned(),
                        },
                    ),
                    FieldDescriptor::new(
                        "holder",
                        ValueType::Serial {
                            type_name: "HolderId".to_owned(),
                            owning_relation: "Holder".to_owned(),
                        },
                    ),
                    FieldDescriptor::new(
                        "currency",
                        ValueType::Enum {
                            name: "Currency".to_owned(),
                        },
                    ),
                    FieldDescriptor::new("opened", ValueType::TimestampMicros).range_indexed(),
                ],
            )
            .with_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_holder", ["holder"]))
            .with_constraint(ConstraintDescriptor::unique(
                "holder_currency",
                ["holder", "currency"],
            ))
            .with_constraint(ConstraintDescriptor::foreign_key(
                "holder",
                ["holder"],
                "Holder",
                "id",
            )),
            RelationDescriptor::new(
                "AccountTag",
                vec![
                    FieldDescriptor::new(
                        "account",
                        ValueType::Serial {
                            type_name: "AccountId".to_owned(),
                            owning_relation: "Account".to_owned(),
                        },
                    ),
                    FieldDescriptor::new(
                        "tag",
                        ValueType::Enum {
                            name: "Tag".to_owned(),
                        },
                    ),
                ],
            )
            .with_unique("account_tag", ["account", "tag"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "account",
                ["account"],
                "Account",
                "id",
            ))
            .with_index(IndexDescriptor::equality("by_account", ["account"])),
        ],
    )
    .with_enum(bumbledb_core::schema::EnumDescriptor::codes(
        "Currency",
        [1, 2, 3],
    ))
    .with_enum(bumbledb_core::schema::EnumDescriptor::codes(
        "Tag",
        [1, 2, 3, 7],
    ))
}

fn holder_fact(id: u64, name: &str) -> Fact {
    Fact::new(
        "Holder",
        [
            ("id", Value::Serial(id)),
            ("name", Value::String(name.to_owned())),
        ],
    )
}

fn account_fact(id: u64, holder: u64, currency: u8) -> Fact {
    Fact::new(
        "Account",
        [
            ("id", Value::Serial(id)),
            ("holder", Value::Serial(holder)),
            ("currency", Value::Enum(currency)),
            (
                "opened",
                Value::Timestamp(TimestampMicros((id as i64) * 10)),
            ),
        ],
    )
}

fn tag_fact(account: u64, tag: u8) -> Fact {
    Fact::new(
        "AccountTag",
        [
            ("account", Value::Serial(account)),
            ("tag", Value::Enum(tag)),
        ],
    )
}

fn collect_items(scan: FactCursor<'_, '_, '_>) -> Result<Vec<FactCursorRecord>> {
    scan.collect()
}

fn collect_facts(scan: FactCursor<'_, '_, '_>) -> Result<Vec<Fact>> {
    scan.map(|item| item.map(|item| item.fact)).collect()
}

fn assert_same_facts(actual: &[Fact], expected: &[Fact]) -> Result<()> {
    let mut actual = fact_keys(actual)?;
    let mut expected = fact_keys(expected)?;
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected);
    Ok(())
}

fn fact_keys(facts: &[Fact]) -> Result<Vec<(u64, u64, u8, i64)>> {
    facts
        .iter()
        .map(|fact| {
            let id = match required_value(fact, "id")? {
                Value::Serial(value) => *value,
                other => {
                    return Err(Error::internal(format!("unexpected id value: {other:?}")));
                }
            };
            let holder = match required_value(fact, "holder")? {
                Value::Serial(value) => *value,
                other => {
                    return Err(Error::internal(format!(
                        "unexpected holder value: {other:?}"
                    )));
                }
            };
            let currency = match required_value(fact, "currency")? {
                Value::Enum(value) => *value,
                other => {
                    return Err(Error::internal(format!(
                        "unexpected currency value: {other:?}"
                    )));
                }
            };
            let opened = match required_value(fact, "opened")? {
                Value::Timestamp(value) => value.0,
                other => {
                    return Err(Error::internal(format!(
                        "unexpected opened value: {other:?}"
                    )));
                }
            };
            Ok((id, holder, currency, opened))
        })
        .collect()
}

fn required_value<'a>(fact: &'a Fact, field: &str) -> Result<&'a Value> {
    fact.value(field)
        .ok_or_else(|| Error::internal(format!("missing field {field}")))
}
