use bumbledb_log::schema_file::{self, schema_id};

#[test]
fn old_unbounded_capacity_snapshots_keep_their_identity() {
    let old = r#"{"relations":[{"name":"Counted","fields":[{"name":"id","type":"u64"}]}],
        "statements":[{"functionality":{"relation":0,"projection":[0]}},
        {"capacity":{"target":{"relation":0,"projection":[0]},
        "weight":"unit","lo":"0","source":{"relation":0,"projection":[0]}}}]}"#;
    let descriptor = schema_file::parse(old).unwrap();
    let canonical = schema_file::render(&descriptor);
    assert!(canonical.contains("\"hi\":null"));
    assert_eq!(schema_file::parse(&canonical).unwrap(), descriptor);
    assert_eq!(
        schema_id(&descriptor).unwrap(),
        schema_id(&schema_file::parse(&canonical).unwrap()).unwrap()
    );
    assert!(
        schema_file::parse(&old.replace("\"lo\":\"0\"", "\"lo\":\"0\",\"future\":true")).is_err()
    );
}
