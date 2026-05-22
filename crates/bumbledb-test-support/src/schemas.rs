//! Reusable schemas for tests.

use bumbledb_core::schema::{
    ConstraintDescriptor, EnumDescriptor, FieldDescriptor, RelationDescriptor, SchemaDescriptor,
    ValueType,
};

/// Canonical small ledger schema used by most correctness tests.
pub fn ledger_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "TestLedgerDb",
        vec![
            RelationDescriptor::new(
                "Holder",
                vec![
                    FieldDescriptor::new("id", serial_type("HolderId", "Holder")),
                    FieldDescriptor::new("name", ValueType::String),
                ],
            )
            .with_unique("id", ["id"])
            .with_constraint(ConstraintDescriptor::unique("name", ["name"])),
            RelationDescriptor::new(
                "Account",
                vec![
                    FieldDescriptor::new("id", serial_type("AccountId", "Account")),
                    FieldDescriptor::new("holder", serial_type("HolderId", "Holder")),
                    FieldDescriptor::new(
                        "currency",
                        ValueType::Enum {
                            name: "Currency".to_owned(),
                        },
                    ),
                ],
            )
            .with_unique("id", ["id"])
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
                "Posting",
                vec![
                    FieldDescriptor::new("id", serial_type("PostingId", "Posting")),
                    FieldDescriptor::new("account", serial_type("AccountId", "Account")),
                    FieldDescriptor::new("amount", ValueType::Decimal { scale: 4 }),
                    FieldDescriptor::new("at", ValueType::TimestampMicros).range_indexed(),
                ],
            )
            .with_unique("id", ["id"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "account",
                ["account"],
                "Account",
                "id",
            )),
            RelationDescriptor::new(
                "AccountTag",
                vec![
                    FieldDescriptor::new("account", serial_type("AccountId", "Account")),
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
            )),
        ],
    )
    .with_enum(EnumDescriptor::codes("Currency", [1, 2, 3]))
    .with_enum(EnumDescriptor::codes("Tag", [1, 2, 3, 7, 8]))
}

/// Schema for aggregation overflow tests.
pub fn overflow_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "OverflowDb",
        vec![
            RelationDescriptor::new(
                "Number",
                vec![
                    FieldDescriptor::new("id", serial_type("NumberId", "Number")),
                    FieldDescriptor::new("n", ValueType::I64),
                    FieldDescriptor::new("d", ValueType::Decimal { scale: 0 }),
                ],
            )
            .with_unique("id", ["id"]),
        ],
    )
}

/// Returns a schema changed enough to produce a different fingerprint.
pub fn changed_ledger_schema() -> SchemaDescriptor {
    let mut schema = ledger_schema();
    schema.relations.push(
        RelationDescriptor::new(
            "Extra",
            vec![FieldDescriptor::new("id", serial_type("ExtraId", "Extra"))],
        )
        .with_unique("id", ["id"]),
    );
    schema
}

fn serial_type(name: &str, relation: &str) -> ValueType {
    ValueType::Serial {
        type_name: name.to_owned(),
        owning_relation: relation.to_owned(),
    }
}
