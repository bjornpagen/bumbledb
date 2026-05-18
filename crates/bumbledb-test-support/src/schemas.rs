//! Reusable schemas for tests.

use bumbledb_core::schema::{
    FieldDescriptor, GeneratedIdDescriptor, PrimaryKeyDescriptor, RelationDescriptor, RelationKind,
    SchemaDescriptor, ValueType,
};

/// Canonical small ledger schema used by most correctness tests.
pub fn ledger_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "TestLedgerDb",
        vec![
            RelationDescriptor::new(
                "Holder",
                RelationKind::Entity,
                vec![
                    FieldDescriptor::new("id", id_type("HolderId", "Holder")),
                    FieldDescriptor::new("name", ValueType::String),
                ],
                PrimaryKeyDescriptor::new(["id"]),
            )
            .with_generated_id(GeneratedIdDescriptor::new("id"))
            .with_constraint(bumbledb_core::schema::ConstraintDescriptor::unique(
                "name",
                ["name"],
            )),
            RelationDescriptor::new(
                "Account",
                RelationKind::Entity,
                vec![
                    FieldDescriptor::new("id", id_type("AccountId", "Account")),
                    FieldDescriptor::new(
                        "holder",
                        ValueType::Ref {
                            name: "HolderId".to_owned(),
                            target_relation: "Holder".to_owned(),
                        },
                    ),
                    FieldDescriptor::new(
                        "currency",
                        ValueType::Symbol {
                            name: "Currency".to_owned(),
                        },
                    ),
                ],
                PrimaryKeyDescriptor::new(["id"]),
            )
            .with_generated_id(GeneratedIdDescriptor::new("id"))
            .with_constraint(bumbledb_core::schema::ConstraintDescriptor::unique(
                "holder_currency",
                ["holder", "currency"],
            )),
            RelationDescriptor::new(
                "Posting",
                RelationKind::Event,
                vec![
                    FieldDescriptor::new("id", id_type("PostingId", "Posting")),
                    FieldDescriptor::new(
                        "account",
                        ValueType::Ref {
                            name: "AccountId".to_owned(),
                            target_relation: "Account".to_owned(),
                        },
                    ),
                    FieldDescriptor::new("amount", ValueType::Decimal { scale: 4 }),
                    FieldDescriptor::new("at", ValueType::TimestampMicros).range_indexed(),
                ],
                PrimaryKeyDescriptor::new(["id"]),
            )
            .with_generated_id(GeneratedIdDescriptor::new("id")),
            RelationDescriptor::new(
                "AccountTag",
                RelationKind::Edge,
                vec![
                    FieldDescriptor::new(
                        "account",
                        ValueType::Ref {
                            name: "AccountId".to_owned(),
                            target_relation: "Account".to_owned(),
                        },
                    ),
                    FieldDescriptor::new(
                        "tag",
                        ValueType::Symbol {
                            name: "Tag".to_owned(),
                        },
                    ),
                ],
                PrimaryKeyDescriptor::new(["account", "tag"]),
            ),
        ],
    )
}

/// Schema for aggregation overflow tests.
pub fn overflow_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "OverflowDb",
        vec![RelationDescriptor::new(
            "Number",
            RelationKind::Entity,
            vec![
                FieldDescriptor::new("id", id_type("NumberId", "Number")),
                FieldDescriptor::new("n", ValueType::I64),
                FieldDescriptor::new("d", ValueType::Decimal { scale: 0 }),
            ],
            PrimaryKeyDescriptor::new(["id"]),
        )],
    )
}

/// Returns a schema changed enough to produce a different fingerprint.
pub fn changed_ledger_schema() -> SchemaDescriptor {
    let mut schema = ledger_schema();
    schema.relations.push(RelationDescriptor::new(
        "Extra",
        RelationKind::Entity,
        vec![FieldDescriptor::new("id", id_type("ExtraId", "Extra"))],
        PrimaryKeyDescriptor::new(["id"]),
    ));
    schema
}

fn id_type(name: &str, relation: &str) -> ValueType {
    ValueType::Id {
        name: name.to_owned(),
        relation: relation.to_owned(),
    }
}
