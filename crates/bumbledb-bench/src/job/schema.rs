use bumbledb_core::schema::{
    ConstraintDescriptor, FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType,
};

pub(super) fn job_schema() -> SchemaDescriptor {
    let mut relations = vec![
        keyed_relation("CompanyName", vec![string("country_code")]),
        keyed_relation("CompanyType", vec![string("kind")]),
        keyed_relation("InfoType", vec![string("info")]),
        keyed_relation("Keyword", vec![string("keyword")]),
        keyed_relation("KindType", vec![string("kind")]),
        keyed_relation("LinkType", vec![string("link")]),
        keyed_relation("RoleType", vec![string("role")]),
        keyed_relation("CharName", Vec::new()),
        keyed_relation("Name", vec![string("gender")]),
        keyed_relation(
            "Title",
            vec![
                serial("KindTypeId", "kind", "KindType"),
                i64f("production_year"),
                i64f("episode_nr"),
            ],
        ),
        fact_relation("AkaName", vec![serial("NameId", "person", "Name")]),
        fact_relation(
            "CastInfo",
            vec![
                serial("NameId", "person", "Name"),
                serial("TitleId", "movie", "Title"),
                serial("CharNameId", "person_role", "CharName"),
                serial("RoleTypeId", "role", "RoleType"),
            ],
        ),
        fact_relation(
            "MovieCompanies",
            vec![
                serial("TitleId", "movie", "Title"),
                serial("CompanyNameId", "company", "CompanyName"),
                serial("CompanyTypeId", "company_type", "CompanyType"),
            ],
        ),
        fact_relation(
            "MovieInfo",
            vec![
                serial("TitleId", "movie", "Title"),
                serial("InfoTypeId", "info_type", "InfoType"),
            ],
        ),
        fact_relation(
            "MovieInfoIdx",
            vec![
                serial("TitleId", "movie", "Title"),
                serial("InfoTypeId", "info_type", "InfoType"),
            ],
        ),
        fact_relation(
            "MovieKeyword",
            vec![
                serial("TitleId", "movie", "Title"),
                serial("KeywordId", "keyword", "Keyword"),
            ],
        ),
        fact_relation(
            "MovieLink",
            vec![
                serial("TitleId", "movie", "Title"),
                serial("TitleId", "linked_movie", "Title"),
                serial("LinkTypeId", "link_type", "LinkType"),
            ],
        ),
    ];
    add_foreign_keys(&mut relations);
    SchemaDescriptor::new("JoinOrderBenchmarkDb", relations)
}

fn keyed_relation(name: &str, mut fields: Vec<FieldDescriptor>) -> RelationDescriptor {
    fields.insert(
        0,
        FieldDescriptor::generated_serial("id", format!("{name}Id"), name),
    );
    RelationDescriptor::new(name, fields).with_unique("id", ["id"])
}

fn fact_relation(name: &str, fields: Vec<FieldDescriptor>) -> RelationDescriptor {
    RelationDescriptor::new(name, fields)
}

fn serial(type_name: &str, field: &str, owning_relation: &str) -> FieldDescriptor {
    FieldDescriptor::new(
        field,
        ValueType::Serial {
            type_name: type_name.to_owned(),
            owning_relation: owning_relation.to_owned(),
        },
    )
}

fn string(name: &str) -> FieldDescriptor {
    FieldDescriptor::new(name, ValueType::String)
}

fn i64f(name: &str) -> FieldDescriptor {
    FieldDescriptor::new(name, ValueType::I64)
}

fn add_foreign_keys(relations: &mut [RelationDescriptor]) {
    for relation in relations {
        for field in relation.fields.clone() {
            let ValueType::Serial {
                owning_relation, ..
            } = field.value_type
            else {
                continue;
            };
            if owning_relation == relation.name {
                continue;
            }
            relation.constraints.push(ConstraintDescriptor::foreign_key(
                field.name.clone(),
                [field.name],
                owning_relation,
                "id",
            ));
        }
    }
}
