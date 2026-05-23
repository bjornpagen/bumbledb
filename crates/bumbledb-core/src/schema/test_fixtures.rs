use crate::schema::{
    ConstraintDescriptor, EnumDescriptor, FieldDescriptor, RelationDescriptor, SchemaDescriptor,
    ValueType,
};

pub(super) fn ledger_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "LedgerDb",
        vec![
            RelationDescriptor::new(
                "Account",
                vec![
                    FieldDescriptor::generated_serial("id", "AccountId", "Account"),
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
                    FieldDescriptor::generated_serial("id", "PostingId", "Posting"),
                    FieldDescriptor::new("entry", serial_type("JournalEntryId", "JournalEntry")),
                    FieldDescriptor::new("account", serial_type("AccountId", "Account")),
                    FieldDescriptor::new("instrument", serial_type("InstrumentId", "Instrument")),
                    FieldDescriptor::new("amount", ValueType::I64),
                    FieldDescriptor::new("at_micros", ValueType::I64),
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
                "Holder",
                vec![
                    FieldDescriptor::generated_serial("id", "HolderId", "Holder"),
                    FieldDescriptor::new("name", ValueType::String),
                ],
            )
            .with_unique("id", ["id"])
            .with_constraint(ConstraintDescriptor::unique("name", ["name"])),
            RelationDescriptor::new(
                "SourceDocument",
                vec![
                    FieldDescriptor::generated_serial("id", "SourceDocumentId", "SourceDocument"),
                    FieldDescriptor::new("payload", ValueType::Bytes),
                ],
            )
            .with_unique("id", ["id"]),
            RelationDescriptor::new(
                "OrgParent",
                vec![
                    FieldDescriptor::new("child", serial_type("OrgId", "Org")),
                    FieldDescriptor::new("parent", serial_type("OrgId", "Org")),
                ],
            )
            .with_unique("child_parent", ["child", "parent"]),
        ],
    )
    .with_enum(EnumDescriptor::codes("Currency", [1, 2]))
}

pub(super) fn valid_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "ValidationDb",
        vec![
            RelationDescriptor::new(
                "Parent",
                vec![
                    FieldDescriptor::generated_serial("id", "ParentId", "Parent"),
                    FieldDescriptor::new("code", ValueType::U64),
                ],
            )
            .with_unique("id", ["id"])
            .with_constraint(ConstraintDescriptor::unique("code", ["code"])),
            RelationDescriptor::new(
                "Child",
                vec![
                    FieldDescriptor::generated_serial("id", "ChildId", "Child"),
                    FieldDescriptor::new("parent", serial_type("ParentId", "Parent")),
                ],
            )
            .with_unique("id", ["id"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "parent",
                ["parent"],
                "Parent",
                "id",
            )),
        ],
    )
}

pub(super) fn compound_fk_schema() -> SchemaDescriptor {
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
    )
}

pub(super) fn enum_fk_schema() -> SchemaDescriptor {
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
            .with_unique("by_code", ["code"]),
            RelationDescriptor::new(
                "Account",
                vec![
                    FieldDescriptor::generated_serial("id", "AccountId", "Account"),
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
                "by_code",
            )),
        ],
    )
    .with_enum(EnumDescriptor::codes("Currency", [1, 2]))
    .with_enum(EnumDescriptor::codes("Country", [1, 2]))
}

pub(super) fn compound_enum_fk_schema() -> SchemaDescriptor {
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
                    FieldDescriptor::generated_serial("id", "AccountId", "Account"),
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
    .with_enum(EnumDescriptor::codes("Country", [1, 2]))
    .with_enum(EnumDescriptor::codes("Currency", [1, 2]))
}

pub(super) fn compound_serial_enum_fk_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "CompoundSerialEnumFkDb",
        vec![
            RelationDescriptor::new(
                "AccountCurrency",
                vec![
                    FieldDescriptor::new("account", serial_type("AccountId", "Account")),
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
                    FieldDescriptor::generated_serial("id", "PostingId", "Posting"),
                    FieldDescriptor::new("account", serial_type("AccountId", "Account")),
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
    .with_enum(EnumDescriptor::codes("Currency", [1, 2]))
}

pub(super) fn serial_type(type_name: &str, owning_relation: &str) -> ValueType {
    ValueType::Serial {
        type_name: type_name.to_owned(),
        owning_relation: owning_relation.to_owned(),
    }
}
