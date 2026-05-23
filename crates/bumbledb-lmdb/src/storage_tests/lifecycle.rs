use super::*;

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
