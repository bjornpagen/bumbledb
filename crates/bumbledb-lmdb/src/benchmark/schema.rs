use bumbledb_core::schema::{
    ConstraintDescriptor, FieldDescriptor, IndexDescriptor, RelationDescriptor, SchemaDescriptor,
    ValueType,
};

/// Returns the benchmark schema from the Rosetta Stone workload.
pub fn benchmark_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "BenchmarkLedgerDb",
        vec![
            entity(
                "Holder",
                "HolderId",
                vec![FieldDescriptor::new("name", ValueType::String)],
            ),
            entity(
                "Org",
                "OrgId",
                vec![FieldDescriptor::new("name", ValueType::String)],
            ),
            entity(
                "Instrument",
                "InstrumentId",
                vec![FieldDescriptor::new("symbol", ValueType::String)],
            ),
            entity(
                "SourceDocument",
                "SourceDocumentId",
                vec![FieldDescriptor::new("payload", ValueType::Bytes)],
            ),
            RelationDescriptor::new(
                "Account",
                vec![
                    serial_key_field("AccountId", "Account"),
                    serial_field("HolderId", "holder", "Holder"),
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
                "holder",
                ["holder"],
                "Holder",
                "id",
            )),
            RelationDescriptor::new(
                "JournalEntry",
                vec![
                    serial_key_field("JournalEntryId", "JournalEntry"),
                    serial_field("SourceDocumentId", "source", "SourceDocument"),
                    FieldDescriptor::new("created_at", ValueType::TimestampMicros).range_indexed(),
                ],
            )
            .with_unique("id", ["id"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "source",
                ["source"],
                "SourceDocument",
                "id",
            )),
            RelationDescriptor::new(
                "Posting",
                vec![
                    serial_key_field("PostingId", "Posting"),
                    serial_field("JournalEntryId", "entry", "JournalEntry"),
                    serial_field("AccountId", "account", "Account"),
                    serial_field("InstrumentId", "instrument", "Instrument"),
                    FieldDescriptor::new("amount", ValueType::Decimal { scale: 4 }),
                    FieldDescriptor::new("at", ValueType::TimestampMicros).range_indexed(),
                ],
            )
            .with_unique("id", ["id"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "entry",
                ["entry"],
                "JournalEntry",
                "id",
            ))
            .with_constraint(ConstraintDescriptor::foreign_key(
                "account",
                ["account"],
                "Account",
                "id",
            ))
            .with_constraint(ConstraintDescriptor::foreign_key(
                "instrument",
                ["instrument"],
                "Instrument",
                "id",
            )),
            RelationDescriptor::new(
                "PostingTag",
                vec![
                    serial_field("PostingId", "posting", "Posting"),
                    FieldDescriptor::new(
                        "tag",
                        ValueType::Enum {
                            name: "Tag".to_owned(),
                        },
                    ),
                ],
            )
            .with_unique("posting_tag", ["posting", "tag"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "posting",
                ["posting"],
                "Posting",
                "id",
            ))
            .with_index(IndexDescriptor::permutation("by_tag", ["tag", "posting"])),
            RelationDescriptor::new(
                "OrgParent",
                vec![
                    serial_field("OrgId", "child", "Org"),
                    serial_field("OrgId", "parent", "Org"),
                ],
            )
            .with_unique("child_parent", ["child", "parent"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "child",
                ["child"],
                "Org",
                "id",
            ))
            .with_constraint(ConstraintDescriptor::foreign_key(
                "parent",
                ["parent"],
                "Org",
                "id",
            )),
            RelationDescriptor::new(
                "AuthorizationEdge",
                vec![
                    serial_field("OrgId", "subject", "Org"),
                    serial_field("OrgId", "object", "Org"),
                    FieldDescriptor::new(
                        "permission",
                        ValueType::Enum {
                            name: "Permission".to_owned(),
                        },
                    ),
                ],
            )
            .with_unique(
                "subject_object_permission",
                ["subject", "object", "permission"],
            )
            .with_constraint(ConstraintDescriptor::foreign_key(
                "subject",
                ["subject"],
                "Org",
                "id",
            ))
            .with_constraint(ConstraintDescriptor::foreign_key(
                "object",
                ["object"],
                "Org",
                "id",
            )),
            RelationDescriptor::new(
                "ExchangeRate",
                vec![
                    serial_key_field("ExchangeRateId", "ExchangeRate"),
                    serial_field("InstrumentId", "base", "Instrument"),
                    serial_field("InstrumentId", "quote", "Instrument"),
                    FieldDescriptor::new("at", ValueType::TimestampMicros).range_indexed(),
                    FieldDescriptor::new("rate", ValueType::Decimal { scale: 8 }),
                ],
            )
            .with_unique("id", ["id"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "base",
                ["base"],
                "Instrument",
                "id",
            ))
            .with_constraint(ConstraintDescriptor::foreign_key(
                "quote",
                ["quote"],
                "Instrument",
                "id",
            )),
        ],
    )
    .with_enum(bumbledb_core::schema::EnumDescriptor::codes(
        "Currency",
        [1],
    ))
    .with_enum(bumbledb_core::schema::EnumDescriptor::codes(
        "Tag",
        [1, 2, 3],
    ))
    .with_enum(bumbledb_core::schema::EnumDescriptor::codes(
        "Permission",
        [7],
    ))
}

fn entity(name: &str, id_type: &str, fields: Vec<FieldDescriptor>) -> RelationDescriptor {
    let mut all = vec![serial_key_field(id_type, name)];
    all.extend(fields);
    RelationDescriptor::new(name, all).with_unique("id", ["id"])
}

fn serial_key_field(id_type: &str, relation: &str) -> FieldDescriptor {
    FieldDescriptor::new(
        "id",
        ValueType::Serial {
            type_name: id_type.to_owned(),
            owning_relation: relation.to_owned(),
        },
    )
}

fn serial_field(id_type: &str, field: &str, target: &str) -> FieldDescriptor {
    FieldDescriptor::new(
        field,
        ValueType::Serial {
            type_name: id_type.to_owned(),
            owning_relation: target.to_owned(),
        },
    )
}
