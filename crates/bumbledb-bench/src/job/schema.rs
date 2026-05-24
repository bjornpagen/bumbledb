use bumbledb_core::schema::{
    ConstraintDescriptor, FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType,
};

pub(super) fn job_schema() -> SchemaDescriptor {
    let mut relations = vec![
        relation(
            "CompCastType",
            vec![id("CompCastTypeId", "CompCastType"), string("kind")],
        ),
        relation(
            "CompanyName",
            vec![
                id("CompanyNameId", "CompanyName"),
                string("name"),
                string("country_code"),
                i64f("imdb_id"),
                string("name_pcode_nf"),
                string("name_pcode_sf"),
            ],
        ),
        relation(
            "CompanyType",
            vec![id("CompanyTypeId", "CompanyType"), string("kind")],
        ),
        relation(
            "InfoType",
            vec![id("InfoTypeId", "InfoType"), string("info")],
        ),
        relation(
            "Keyword",
            vec![
                id("KeywordId", "Keyword"),
                string("keyword"),
                string("phonetic_code"),
            ],
        ),
        relation(
            "KindType",
            vec![id("KindTypeId", "KindType"), string("kind")],
        ),
        relation(
            "LinkType",
            vec![id("LinkTypeId", "LinkType"), string("link")],
        ),
        relation(
            "RoleType",
            vec![id("RoleTypeId", "RoleType"), string("role")],
        ),
        relation(
            "CharName",
            vec![
                id("CharNameId", "CharName"),
                string("name"),
                string("imdb_index"),
                i64f("imdb_id"),
                string("name_pcode_nf"),
                string("surname_pcode"),
            ],
        ),
        relation(
            "Name",
            vec![
                id("NameId", "Name"),
                string("name"),
                string("imdb_index"),
                i64f("imdb_id"),
                string("gender"),
                string("name_pcode_cf"),
                string("name_pcode_nf"),
                string("surname_pcode"),
            ],
        ),
        relation(
            "Title",
            vec![
                id("TitleId", "Title"),
                string("title"),
                string("imdb_index"),
                serial("KindTypeId", "kind", "KindType"),
                i64f("production_year"),
                i64f("imdb_id"),
                string("phonetic_code"),
                u64f("episode_of"),
                i64f("season_nr"),
                i64f("episode_nr"),
                string("series_years"),
            ],
        ),
        relation(
            "AkaName",
            vec![
                id("AkaNameId", "AkaName"),
                serial("NameId", "person", "Name"),
                string("name"),
                string("imdb_index"),
                string("name_pcode_cf"),
                string("name_pcode_nf"),
                string("surname_pcode"),
            ],
        ),
        relation(
            "AkaTitle",
            vec![
                id("AkaTitleId", "AkaTitle"),
                serial("TitleId", "movie", "Title"),
                string("title"),
                string("imdb_index"),
                serial("KindTypeId", "kind", "KindType"),
                i64f("production_year"),
                string("phonetic_code"),
                u64f("episode_of"),
                i64f("season_nr"),
                i64f("episode_nr"),
                string("note"),
            ],
        ),
        relation(
            "CastInfo",
            vec![
                id("CastInfoId", "CastInfo"),
                serial("NameId", "person", "Name"),
                serial("TitleId", "movie", "Title"),
                serial("CharNameId", "person_role", "CharName"),
                string("note"),
                i64f("nr_order"),
                serial("RoleTypeId", "role", "RoleType"),
            ],
        ),
        relation(
            "CompleteCast",
            vec![
                id("CompleteCastId", "CompleteCast"),
                serial("TitleId", "movie", "Title"),
                serial("CompCastTypeId", "subject", "CompCastType"),
                serial("CompCastTypeId", "status", "CompCastType"),
            ],
        ),
        relation(
            "MovieCompanies",
            vec![
                id("MovieCompaniesId", "MovieCompanies"),
                serial("TitleId", "movie", "Title"),
                serial("CompanyNameId", "company", "CompanyName"),
                serial("CompanyTypeId", "company_type", "CompanyType"),
                string("note"),
            ],
        ),
        relation(
            "MovieInfo",
            vec![
                id("MovieInfoId", "MovieInfo"),
                serial("TitleId", "movie", "Title"),
                serial("InfoTypeId", "info_type", "InfoType"),
                string("info"),
                string("note"),
            ],
        ),
        relation(
            "MovieInfoIdx",
            vec![
                id("MovieInfoIdxId", "MovieInfoIdx"),
                serial("TitleId", "movie", "Title"),
                serial("InfoTypeId", "info_type", "InfoType"),
                string("info"),
                string("note"),
            ],
        ),
        relation(
            "MovieKeyword",
            vec![
                id("MovieKeywordId", "MovieKeyword"),
                serial("TitleId", "movie", "Title"),
                serial("KeywordId", "keyword", "Keyword"),
            ],
        ),
        relation(
            "MovieLink",
            vec![
                id("MovieLinkId", "MovieLink"),
                serial("TitleId", "movie", "Title"),
                serial("TitleId", "linked_movie", "Title"),
                serial("LinkTypeId", "link_type", "LinkType"),
            ],
        ),
        relation(
            "PersonInfo",
            vec![
                id("PersonInfoId", "PersonInfo"),
                serial("NameId", "person", "Name"),
                serial("InfoTypeId", "info_type", "InfoType"),
                string("info"),
                string("note"),
            ],
        ),
    ];
    add_foreign_keys(&mut relations);
    SchemaDescriptor::new("JoinOrderBenchmarkDb", relations)
}

fn relation(name: &str, fields: Vec<FieldDescriptor>) -> RelationDescriptor {
    RelationDescriptor::new(name, fields).with_unique("id", ["id"])
}

fn id(type_name: &str, owning_relation: &str) -> FieldDescriptor {
    FieldDescriptor::generated_serial("id", type_name, owning_relation)
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
fn u64f(name: &str) -> FieldDescriptor {
    FieldDescriptor::new(name, ValueType::U64)
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
