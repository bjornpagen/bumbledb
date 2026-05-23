fn job_schema() -> SchemaDescriptor {
    let mut relations = vec![
        job_relation(
            "AkaName",
            vec![
                serial_key_field("AkaNameId", "AkaName"),
                serial_field("NameId", "person", "Name"),
                job_string_field("name"),
                job_string_field("imdb_index"),
                job_string_field("name_pcode_cf"),
                job_string_field("name_pcode_nf"),
                job_string_field("surname_pcode"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_person_id",
            ["person", "id"],
        )),
        job_relation(
            "AkaTitle",
            vec![
                serial_key_field("AkaTitleId", "AkaTitle"),
                serial_field("TitleId", "movie", "Title"),
                job_string_field("title"),
                job_string_field("imdb_index"),
                serial_field("KindTypeId", "kind", "KindType"),
                job_range_i64_field("production_year"),
                job_string_field("phonetic_code"),
                job_u64_field("episode_of"),
                job_i64_field("season_nr"),
                job_range_i64_field("episode_nr"),
                job_string_field("note"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_movie_kind",
            ["movie", "kind", "id"],
        )),
        job_relation(
            "CastInfo",
            vec![
                serial_key_field("CastInfoId", "CastInfo"),
                serial_field("NameId", "person", "Name"),
                serial_field("TitleId", "movie", "Title"),
                serial_field("CharNameId", "person_role", "CharName"),
                job_string_field("note"),
                job_i64_field("nr_order"),
                serial_field("RoleTypeId", "role", "RoleType"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_movie_role_person",
            ["movie", "role", "person", "person_role", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_person_movie",
            ["person", "movie", "role", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_person_role_movie",
            ["person_role", "movie", "person", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_role_movie",
            ["role", "movie", "person", "id"],
        )),
        job_relation(
            "CharName",
            vec![
                serial_key_field("CharNameId", "CharName"),
                job_string_field("name"),
                job_string_field("imdb_index"),
                job_i64_field("imdb_id"),
                job_string_field("name_pcode_nf"),
                job_string_field("surname_pcode"),
            ],
        )
        .with_index(IndexDescriptor::permutation("by_name", ["name", "id"])),
        job_relation(
            "CompCastType",
            vec![
                serial_key_field("CompCastTypeId", "CompCastType"),
                job_string_field("kind"),
            ],
        )
        .with_index(IndexDescriptor::permutation("by_kind", ["kind", "id"])),
        job_relation(
            "CompanyName",
            vec![
                serial_key_field("CompanyNameId", "CompanyName"),
                job_string_field("name"),
                job_string_field("country_code"),
                job_i64_field("imdb_id"),
                job_string_field("name_pcode_nf"),
                job_string_field("name_pcode_sf"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_country",
            ["country_code", "id"],
        ))
        .with_index(IndexDescriptor::permutation("by_name", ["name", "id"])),
        job_relation(
            "CompanyType",
            vec![
                serial_key_field("CompanyTypeId", "CompanyType"),
                job_string_field("kind"),
            ],
        )
        .with_index(IndexDescriptor::permutation("by_kind", ["kind", "id"])),
        job_relation(
            "CompleteCast",
            vec![
                serial_key_field("CompleteCastId", "CompleteCast"),
                serial_field("TitleId", "movie", "Title"),
                serial_field("CompCastTypeId", "subject", "CompCastType"),
                serial_field("CompCastTypeId", "status", "CompCastType"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_movie_subject_status",
            ["movie", "subject", "status", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_subject_status",
            ["subject", "status", "movie", "id"],
        )),
        job_relation(
            "InfoType",
            vec![
                serial_key_field("InfoTypeId", "InfoType"),
                job_string_field("info"),
            ],
        )
        .with_index(IndexDescriptor::permutation("by_info", ["info", "id"])),
        job_relation(
            "Keyword",
            vec![
                serial_key_field("KeywordId", "Keyword"),
                job_string_field("keyword"),
                job_string_field("phonetic_code"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_keyword",
            ["keyword", "id"],
        )),
        job_relation(
            "KindType",
            vec![
                serial_key_field("KindTypeId", "KindType"),
                job_string_field("kind"),
            ],
        )
        .with_index(IndexDescriptor::permutation("by_kind", ["kind", "id"])),
        job_relation(
            "LinkType",
            vec![
                serial_key_field("LinkTypeId", "LinkType"),
                job_string_field("link"),
            ],
        )
        .with_index(IndexDescriptor::permutation("by_link", ["link", "id"])),
        job_relation(
            "MovieCompanies",
            vec![
                serial_key_field("MovieCompaniesId", "MovieCompanies"),
                serial_field("TitleId", "movie", "Title"),
                serial_field("CompanyNameId", "company", "CompanyName"),
                serial_field("CompanyTypeId", "company_type", "CompanyType"),
                job_string_field("note"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_movie_company_type",
            ["movie", "company_type", "company", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_company_movie",
            ["company", "movie", "company_type", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_company_type_movie",
            ["company_type", "movie", "company", "id"],
        )),
        job_relation(
            "MovieInfo",
            vec![
                serial_key_field("MovieInfoId", "MovieInfo"),
                serial_field("TitleId", "movie", "Title"),
                serial_field("InfoTypeId", "info_type", "InfoType"),
                job_string_field("info"),
                job_string_field("note"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_movie_type",
            ["movie", "info_type", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_type_movie",
            ["info_type", "movie", "id"],
        )),
        job_relation(
            "MovieInfoIdx",
            vec![
                serial_key_field("MovieInfoIdxId", "MovieInfoIdx"),
                serial_field("TitleId", "movie", "Title"),
                serial_field("InfoTypeId", "info_type", "InfoType"),
                job_string_field("info"),
                job_string_field("note"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_movie_type",
            ["movie", "info_type", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_type_movie",
            ["info_type", "movie", "id"],
        )),
        job_relation(
            "MovieKeyword",
            vec![
                serial_key_field("MovieKeywordId", "MovieKeyword"),
                serial_field("TitleId", "movie", "Title"),
                serial_field("KeywordId", "keyword", "Keyword"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_movie_keyword",
            ["movie", "keyword", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_keyword_movie",
            ["keyword", "movie", "id"],
        )),
        job_relation(
            "MovieLink",
            vec![
                serial_key_field("MovieLinkId", "MovieLink"),
                serial_field("TitleId", "movie", "Title"),
                serial_field("TitleId", "linked_movie", "Title"),
                serial_field("LinkTypeId", "link_type", "LinkType"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_movie_linked",
            ["movie", "linked_movie", "link_type", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_linked",
            ["linked_movie", "movie", "link_type", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_link_type_movie",
            ["link_type", "movie", "linked_movie", "id"],
        )),
        job_relation(
            "Name",
            vec![
                serial_key_field("NameId", "Name"),
                job_string_field("name"),
                job_string_field("imdb_index"),
                job_i64_field("imdb_id"),
                job_string_field("gender"),
                job_string_field("name_pcode_cf"),
                job_string_field("name_pcode_nf"),
                job_string_field("surname_pcode"),
            ],
        )
        .with_index(IndexDescriptor::permutation("by_gender", ["gender", "id"]))
        .with_index(IndexDescriptor::permutation("by_name", ["name", "id"])),
        job_relation(
            "PersonInfo",
            vec![
                serial_key_field("PersonInfoId", "PersonInfo"),
                serial_field("NameId", "person", "Name"),
                serial_field("InfoTypeId", "info_type", "InfoType"),
                job_string_field("info"),
                job_string_field("note"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_person_info_type",
            ["person", "info_type", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_info_type_person",
            ["info_type", "person", "id"],
        )),
        job_relation(
            "RoleType",
            vec![
                serial_key_field("RoleTypeId", "RoleType"),
                job_string_field("role"),
            ],
        )
        .with_index(IndexDescriptor::permutation("by_role", ["role", "id"])),
        job_relation(
            "Title",
            vec![
                serial_key_field("TitleId", "Title"),
                job_string_field("title"),
                job_string_field("imdb_index"),
                serial_field("KindTypeId", "kind", "KindType"),
                job_range_i64_field("production_year"),
                job_i64_field("imdb_id"),
                job_string_field("phonetic_code"),
                job_u64_field("episode_of"),
                job_i64_field("season_nr"),
                job_range_i64_field("episode_nr"),
                job_string_field("series_years"),
            ],
        )
        .with_index(IndexDescriptor::permutation(
            "by_kind_year",
            ["kind", "production_year", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_year",
            ["production_year", "id"],
        ))
        .with_index(IndexDescriptor::permutation(
            "by_episode",
            ["episode_nr", "id"],
        )),
    ];
    relations.sort_by_key(|relation| job_relation_order(&relation.name));
    add_serial_foreign_keys(SchemaDescriptor::new("JoinOrderBenchmarkDb", relations))
}

fn job_relation(name: impl Into<String>, fields: Vec<FieldDescriptor>) -> RelationDescriptor {
    RelationDescriptor::new(name, fields).with_unique("id", ["id"])
}

fn add_serial_foreign_keys(mut schema: SchemaDescriptor) -> SchemaDescriptor {
    for relation in &mut schema.relations {
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
    schema
}

fn job_relation_order(name: &str) -> usize {
    match name {
        "CompCastType" => 0,
        "CompanyName" => 1,
        "CompanyType" => 2,
        "InfoType" => 3,
        "Keyword" => 4,
        "KindType" => 5,
        "LinkType" => 6,
        "RoleType" => 7,
        "CharName" => 8,
        "Name" => 9,
        "Title" => 10,
        "AkaName" => 11,
        "AkaTitle" => 12,
        "CastInfo" => 13,
        "CompleteCast" => 14,
        "MovieCompanies" => 15,
        "MovieInfo" => 16,
        "MovieInfoIdx" => 17,
        "MovieKeyword" => 18,
        "MovieLink" => 19,
        "PersonInfo" => 20,
        _ => usize::MAX,
    }
}

