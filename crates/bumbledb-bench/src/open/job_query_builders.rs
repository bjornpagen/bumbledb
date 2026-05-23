fn build_job_broad_cast_keyword_company(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("Title")?
        .var("id", "movie")?
        .var("kind", "kind")?
        .done()
        .rel("CastInfo")?
        .var("movie", "movie")?
        .var("person", "person")?
        .var("role", "role")?
        .done()
        .rel("RoleType")?
        .var("id", "role")?
        .done()
        .rel("MovieKeyword")?
        .var("movie", "movie")?
        .var("keyword", "keyword")?
        .done()
        .rel("Keyword")?
        .var("id", "keyword")?
        .done()
        .rel("MovieCompanies")?
        .var("movie", "movie")?
        .var("company", "company")?
        .var("company_type", "company_type")?
        .done()
        .rel("CompanyName")?
        .var("id", "company")?
        .done()
        .rel("CompanyType")?
        .var("id", "company_type")?
        .done()
        .find_var("movie")?
        .finish()
}

fn build_job_broad_movie_info_star(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("Title")?
        .var("id", "movie")?
        .done()
        .rel("CastInfo")?
        .var("movie", "movie")?
        .var("role", "role")?
        .done()
        .rel("RoleType")?
        .var("id", "role")?
        .done()
        .rel("MovieCompanies")?
        .var("movie", "movie")?
        .var("company_type", "company_type")?
        .done()
        .rel("CompanyType")?
        .var("id", "company_type")?
        .done()
        .rel("MovieKeyword")?
        .var("movie", "movie")?
        .var("keyword", "keyword")?
        .done()
        .rel("Keyword")?
        .var("id", "keyword")?
        .done()
        .rel("MovieInfo")?
        .var("movie", "movie")?
        .var("info_type", "info_type")?
        .done()
        .rel("InfoType")?
        .var("id", "info_type")?
        .done()
        .rel("MovieInfoIdx")?
        .var("movie", "movie")?
        .var("info_type", "idx_info_type")?
        .done()
        .rel("InfoType")?
        .var("id", "idx_info_type")?
        .done()
        .find_var("movie")?
        .finish()
}

fn build_job_q01_top_production(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("CompanyType")?
        .var("id", "company_type")?
        .string("kind", "production companies")?
        .done()
        .rel("InfoType")?
        .var("id", "info_type")?
        .string("info", "top 250 rank")?
        .done()
        .rel("MovieCompanies")?
        .var("movie", "movie")?
        .var("company_type", "company_type")?
        .done()
        .rel("MovieInfoIdx")?
        .var("movie", "movie")?
        .var("info_type", "info_type")?
        .done()
        .rel("Title")?
        .var("id", "movie")?
        .done()
        .find_var("movie")?
        .finish()
}

fn build_job_q09_voice_us_actor(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("AkaName")?
        .var("person", "person")?
        .done()
        .rel("CastInfo")?
        .var("person", "person")?
        .var("movie", "movie")?
        .var("person_role", "character")?
        .var("role", "role")?
        .done()
        .rel("CharName")?
        .var("id", "character")?
        .done()
        .rel("CompanyName")?
        .var("id", "company")?
        .string("country_code", "[us]")?
        .done()
        .rel("MovieCompanies")?
        .var("movie", "movie")?
        .var("company", "company")?
        .done()
        .rel("Name")?
        .var("id", "person")?
        .string("gender", "m")?
        .done()
        .rel("RoleType")?
        .var("id", "role")?
        .string("role", "actor")?
        .done()
        .rel("Title")?
        .var("id", "movie")?
        .var("production_year", "year")?
        .done()
        .cmp(
            OperandRef::var("year"),
            ComparisonOperator::Gte,
            OperandRef::literal(Literal::Integer(2005)),
        )?
        .cmp(
            OperandRef::var("year"),
            ComparisonOperator::Lte,
            OperandRef::literal(Literal::Integer(2015)),
        )?
        .find_var("movie")?
        .finish()
}

fn build_job_q16_character_title_us(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("AkaName")?
        .var("person", "person")?
        .done()
        .rel("CastInfo")?
        .var("person", "person")?
        .var("movie", "movie")?
        .done()
        .rel("CompanyName")?
        .var("id", "company")?
        .string("country_code", "[us]")?
        .done()
        .rel("Keyword")?
        .var("id", "keyword")?
        .string("keyword", "character-name-in-title")?
        .done()
        .rel("MovieCompanies")?
        .var("movie", "movie")?
        .var("company", "company")?
        .done()
        .rel("MovieKeyword")?
        .var("movie", "movie")?
        .var("keyword", "keyword")?
        .done()
        .rel("Name")?
        .var("id", "person")?
        .done()
        .rel("Title")?
        .var("id", "movie")?
        .var("episode_nr", "episode")?
        .done()
        .cmp(
            OperandRef::var("episode"),
            ComparisonOperator::Gte,
            OperandRef::literal(Literal::Integer(50)),
        )?
        .cmp(
            OperandRef::var("episode"),
            ComparisonOperator::Lt,
            OperandRef::literal(Literal::Integer(100)),
        )?
        .find_var("movie")?
        .finish()
}

fn build_job_q24_voice_keyword_actor(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("AkaName")?
        .var("person", "person")?
        .done()
        .rel("CastInfo")?
        .var("person", "person")?
        .var("movie", "movie")?
        .var("person_role", "character")?
        .var("role", "role")?
        .done()
        .rel("CharName")?
        .var("id", "character")?
        .done()
        .rel("CompanyName")?
        .var("id", "company")?
        .string("country_code", "[us]")?
        .done()
        .rel("Keyword")?
        .var("id", "keyword")?
        .string("keyword", "hero")?
        .done()
        .rel("MovieCompanies")?
        .var("movie", "movie")?
        .var("company", "company")?
        .done()
        .rel("MovieKeyword")?
        .var("movie", "movie")?
        .var("keyword", "keyword")?
        .done()
        .rel("Name")?
        .var("id", "person")?
        .string("gender", "m")?
        .done()
        .rel("RoleType")?
        .var("id", "role")?
        .string("role", "actor")?
        .done()
        .rel("Title")?
        .var("id", "movie")?
        .var("production_year", "year")?
        .done()
        .cmp(
            OperandRef::var("year"),
            ComparisonOperator::Gt,
            OperandRef::literal(Literal::Integer(2010)),
        )?
        .find_var("movie")?
        .finish()
}

fn build_job_movie_link_bridge(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("MovieLink")?
        .var("movie", "movie1")?
        .var("linked_movie", "movie2")?
        .var("link_type", "link_type")?
        .done()
        .rel("LinkType")?
        .var("id", "link_type")?
        .done()
        .rel("Title")?
        .var("id", "movie1")?
        .done()
        .rel("Title")?
        .var("id", "movie2")?
        .done()
        .rel("MovieCompanies")?
        .var("movie", "movie1")?
        .var("company", "company1")?
        .done()
        .rel("CompanyName")?
        .var("id", "company1")?
        .done()
        .rel("MovieCompanies")?
        .var("movie", "movie2")?
        .var("company", "company2")?
        .done()
        .rel("CompanyName")?
        .var("id", "company2")?
        .done()
        .rel("MovieInfoIdx")?
        .var("movie", "movie1")?
        .var("info_type", "info_type1")?
        .done()
        .rel("InfoType")?
        .var("id", "info_type1")?
        .done()
        .rel("MovieInfoIdx")?
        .var("movie", "movie2")?
        .var("info_type", "info_type2")?
        .done()
        .rel("InfoType")?
        .var("id", "info_type2")?
        .done()
        .find_var("movie1")?
        .finish()
}

fn build_job_q33_linked_series_companies(
    schema: &SchemaDescriptor,
) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("CompanyName")?
        .var("id", "company1")?
        .string("country_code", "[us]")?
        .done()
        .rel("CompanyName")?
        .var("id", "company2")?
        .done()
        .rel("KindType")?
        .var("id", "kind1")?
        .string("kind", "tv series")?
        .done()
        .rel("KindType")?
        .var("id", "kind2")?
        .string("kind", "tv series")?
        .done()
        .rel("LinkType")?
        .var("id", "link_type")?
        .string("link", "sequel")?
        .done()
        .rel("MovieCompanies")?
        .var("movie", "movie1")?
        .var("company", "company1")?
        .done()
        .rel("MovieCompanies")?
        .var("movie", "movie2")?
        .var("company", "company2")?
        .done()
        .rel("MovieLink")?
        .var("movie", "movie1")?
        .var("linked_movie", "movie2")?
        .var("link_type", "link_type")?
        .done()
        .rel("Title")?
        .var("id", "movie1")?
        .var("kind", "kind1")?
        .done()
        .rel("Title")?
        .var("id", "movie2")?
        .var("kind", "kind2")?
        .var("production_year", "year2")?
        .done()
        .cmp(
            OperandRef::var("year2"),
            ComparisonOperator::Gte,
            OperandRef::literal(Literal::Integer(2005)),
        )?
        .cmp(
            OperandRef::var("year2"),
            ComparisonOperator::Lte,
            OperandRef::literal(Literal::Integer(2008)),
        )?
        .find_var("movie1")?
        .finish()
}

