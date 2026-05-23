use super::*;
use crate::{ConstraintError, Environment};
use bumbledb_core::schema::{ConstraintDescriptor, FieldDescriptor, IndexDescriptor};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

#[path = "storage_tests/access.rs"]
mod access;
#[path = "storage_tests/constraints.rs"]
mod constraints;
#[path = "storage_tests/lifecycle.rs"]
mod lifecycle;

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
