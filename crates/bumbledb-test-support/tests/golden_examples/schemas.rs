use bumbledb_core::schema::{
    ConstraintDescriptor, EnumDescriptor, FieldDescriptor, RelationDescriptor, SchemaDescriptor,
    ValueType,
};

pub(super) fn serial_type(name: &str, relation: &str) -> ValueType {
    ValueType::Serial {
        type_name: name.to_owned(),
        owning_relation: relation.to_owned(),
    }
}

pub(super) fn serial_field(type_name: &str, name: &str, owner: &str) -> FieldDescriptor {
    FieldDescriptor::new(name, serial_type(type_name, owner))
}

pub(super) fn serial_id(type_name: &str, relation: &str) -> FieldDescriptor {
    serial_field(type_name, "id", relation)
}

pub(super) fn sailors_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "GoldenSailorsDb",
        vec![
            RelationDescriptor::new(
                "Sailor",
                vec![
                    serial_id("SailorId", "Sailor"),
                    FieldDescriptor::new("rating", ValueType::U64),
                ],
            )
            .with_unique("id", ["id"]),
            RelationDescriptor::new(
                "Boat",
                vec![
                    serial_id("BoatId", "Boat"),
                    FieldDescriptor::new(
                        "color",
                        ValueType::Enum {
                            name: "Color".to_owned(),
                        },
                    ),
                ],
            )
            .with_unique("id", ["id"]),
            RelationDescriptor::new(
                "Reserve",
                vec![
                    serial_field("SailorId", "sailor", "Sailor"),
                    serial_field("BoatId", "boat", "Boat"),
                    FieldDescriptor::new("day", ValueType::TimestampMicros),
                ],
            )
            .with_unique("sailor_boat_day", ["sailor", "boat", "day"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "sailor",
                ["sailor"],
                "Sailor",
                "id",
            ))
            .with_constraint(ConstraintDescriptor::foreign_key(
                "boat",
                ["boat"],
                "Boat",
                "id",
            )),
        ],
    )
    .with_enum(EnumDescriptor::codes("Color", [1, 2]))
}

pub(super) fn triangle_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "GoldenTriangleDb",
        vec![
            RelationDescriptor::new(
                "EdgeAB",
                vec![
                    FieldDescriptor::new("a", ValueType::U64),
                    FieldDescriptor::new("b", ValueType::U64),
                ],
            )
            .with_unique("ab", ["a", "b"]),
            RelationDescriptor::new(
                "EdgeAC",
                vec![
                    FieldDescriptor::new("a", ValueType::U64),
                    FieldDescriptor::new("c", ValueType::U64),
                ],
            )
            .with_unique("ac", ["a", "c"]),
            RelationDescriptor::new(
                "EdgeBC",
                vec![
                    FieldDescriptor::new("b", ValueType::U64),
                    FieldDescriptor::new("c", ValueType::U64),
                ],
            )
            .with_unique("bc", ["b", "c"]),
        ],
    )
}

pub(super) fn tpch_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "GoldenTpchDb",
        vec![
            RelationDescriptor::new(
                "Customer",
                vec![
                    serial_id("CustomerId", "Customer"),
                    FieldDescriptor::new("nation", ValueType::U64),
                ],
            )
            .with_unique("id", ["id"]),
            RelationDescriptor::new(
                "Supplier",
                vec![
                    serial_id("SupplierId", "Supplier"),
                    FieldDescriptor::new("nation", ValueType::U64),
                ],
            )
            .with_unique("id", ["id"]),
            RelationDescriptor::new(
                "Orders",
                vec![
                    serial_id("OrderId", "Orders"),
                    serial_field("CustomerId", "customer", "Customer"),
                ],
            )
            .with_unique("id", ["id"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "customer",
                ["customer"],
                "Customer",
                "id",
            )),
            RelationDescriptor::new(
                "LineItem",
                vec![
                    serial_id("LineItemId", "LineItem"),
                    serial_field("OrderId", "order", "Orders"),
                    FieldDescriptor::new("extended_price", ValueType::Decimal { scale: 2 }),
                ],
            )
            .with_unique("id", ["id"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "order",
                ["order"],
                "Orders",
                "id",
            )),
        ],
    )
}

pub(super) fn imdb_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "GoldenImdbDb",
        vec![
            RelationDescriptor::new(
                "Title",
                vec![
                    serial_id("TitleId", "Title"),
                    FieldDescriptor::new("year", ValueType::I64),
                ],
            )
            .with_unique("id", ["id"]),
            RelationDescriptor::new("Name", vec![serial_id("NameId", "Name")])
                .with_unique("id", ["id"]),
            RelationDescriptor::new(
                "Principal",
                vec![
                    serial_field("TitleId", "title", "Title"),
                    serial_field("NameId", "name", "Name"),
                    FieldDescriptor::new(
                        "category",
                        ValueType::Enum {
                            name: "Category".to_owned(),
                        },
                    ),
                    FieldDescriptor::new("ordering", ValueType::U64),
                ],
            )
            .with_unique(
                "title_name_category_order",
                ["title", "name", "category", "ordering"],
            )
            .with_constraint(ConstraintDescriptor::foreign_key(
                "title",
                ["title"],
                "Title",
                "id",
            ))
            .with_constraint(ConstraintDescriptor::foreign_key(
                "name",
                ["name"],
                "Name",
                "id",
            )),
        ],
    )
    .with_enum(EnumDescriptor::codes("Category", [1, 2]))
}

pub(super) fn lahman_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "GoldenLahmanDb",
        vec![
            RelationDescriptor::new("Player", vec![serial_id("PlayerId", "Player")])
                .with_unique("id", ["id"]),
            RelationDescriptor::new(
                "Team",
                vec![
                    serial_id("TeamId", "Team"),
                    FieldDescriptor::new("year", ValueType::I64),
                ],
            )
            .with_unique("id", ["id"]),
            RelationDescriptor::new(
                "Batting",
                vec![
                    serial_field("PlayerId", "player", "Player"),
                    serial_field("TeamId", "team", "Team"),
                    FieldDescriptor::new("year", ValueType::I64),
                    FieldDescriptor::new("hits", ValueType::I64),
                ],
            )
            .with_unique("player_team_year", ["player", "team", "year"]),
            RelationDescriptor::new(
                "Salary",
                vec![
                    serial_field("PlayerId", "player", "Player"),
                    serial_field("TeamId", "team", "Team"),
                    FieldDescriptor::new("year", ValueType::I64),
                    FieldDescriptor::new("salary", ValueType::I64),
                ],
            )
            .with_unique("player_team_year", ["player", "team", "year"]),
        ],
    )
}

pub(super) fn ldbc_schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "GoldenLdbcDb",
        vec![
            RelationDescriptor::new("Person", vec![serial_id("PersonId", "Person")])
                .with_unique("id", ["id"]),
            RelationDescriptor::new(
                "Knows",
                vec![
                    serial_field("PersonId", "person1", "Person"),
                    serial_field("PersonId", "person2", "Person"),
                ],
            )
            .with_unique("person1_person2", ["person1", "person2"]),
        ],
    )
}
