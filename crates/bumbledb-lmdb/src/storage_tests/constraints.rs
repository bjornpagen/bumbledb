use super::*;

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
