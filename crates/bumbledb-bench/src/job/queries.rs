use bumbledb_core::query_builder::{OperandRef, QueryBuildResult, QueryBuilder};
use bumbledb_core::query_ir::{ComparisonOperator, Literal, TypedQuery};
use bumbledb_core::schema::SchemaDescriptor;

pub(super) struct JobQuery {
    pub(super) name: &'static str,
    pub(super) query: TypedQuery,
    pub(super) sqlite: &'static str,
}

pub(super) fn job_queries(schema: &SchemaDescriptor) -> QueryBuildResult<Vec<JobQuery>> {
    Ok(vec![
        JobQuery {
            name: "job_q01_top_production",
            query: q01(schema)?,
            sqlite: "SELECT DISTINCT t.id FROM company_type ct JOIN movie_companies mc ON mc.company_type = ct.id JOIN movie_info_idx mi_idx ON mi_idx.movie = mc.movie JOIN info_type it ON it.id = mi_idx.info_type JOIN title t ON t.id = mc.movie WHERE ct.kind = 'production companies' AND it.info = 'top 250 rank' ORDER BY 1",
        },
        JobQuery {
            name: "job_q09_voice_us_actor",
            query: q09(schema)?,
            sqlite: "SELECT DISTINCT t.id FROM aka_name an JOIN name n ON n.id = an.person JOIN cast_info ci ON ci.person = n.id JOIN char_name chn ON chn.id = ci.person_role JOIN role_type rt ON rt.id = ci.role JOIN title t ON t.id = ci.movie JOIN movie_companies mc ON mc.movie = t.id JOIN company_name cn ON cn.id = mc.company WHERE cn.country_code = '[us]' AND n.gender = 'm' AND rt.role = 'actor' AND t.production_year BETWEEN 2005 AND 2015 ORDER BY 1",
        },
        JobQuery {
            name: "job_q16_character_title_us",
            query: q16(schema)?,
            sqlite: "SELECT DISTINCT t.id FROM aka_name an JOIN name n ON n.id = an.person JOIN cast_info ci ON ci.person = n.id JOIN title t ON t.id = ci.movie JOIN movie_keyword mk ON mk.movie = t.id JOIN keyword k ON k.id = mk.keyword JOIN movie_companies mc ON mc.movie = t.id JOIN company_name cn ON cn.id = mc.company WHERE cn.country_code = '[us]' AND k.keyword = 'character-name-in-title' AND t.episode_nr >= 50 AND t.episode_nr < 100 ORDER BY 1",
        },
        JobQuery {
            name: "job_q24_voice_keyword_actor",
            query: q24(schema)?,
            sqlite: "SELECT DISTINCT t.id FROM aka_name an JOIN name n ON n.id = an.person JOIN cast_info ci ON ci.person = n.id JOIN char_name chn ON chn.id = ci.person_role JOIN role_type rt ON rt.id = ci.role JOIN title t ON t.id = ci.movie JOIN movie_companies mc ON mc.movie = t.id JOIN company_name cn ON cn.id = mc.company JOIN movie_keyword mk ON mk.movie = t.id JOIN keyword k ON k.id = mk.keyword WHERE cn.country_code = '[us]' AND k.keyword = 'hero' AND n.gender = 'm' AND rt.role = 'actor' AND t.production_year > 2010 ORDER BY 1",
        },
        JobQuery {
            name: "job_movie_link_bridge",
            query: bridge(schema)?,
            sqlite: "SELECT DISTINCT t1.id FROM movie_link ml JOIN link_type lt ON lt.id = ml.link_type JOIN title t1 ON t1.id = ml.movie JOIN title t2 ON t2.id = ml.linked_movie JOIN movie_companies mc1 ON mc1.movie = t1.id JOIN company_name cn1 ON cn1.id = mc1.company JOIN movie_companies mc2 ON mc2.movie = t2.id JOIN company_name cn2 ON cn2.id = mc2.company JOIN movie_info_idx mi_idx1 ON mi_idx1.movie = t1.id JOIN info_type it1 ON it1.id = mi_idx1.info_type JOIN movie_info_idx mi_idx2 ON mi_idx2.movie = t2.id JOIN info_type it2 ON it2.id = mi_idx2.info_type ORDER BY 1",
        },
    ])
}

fn q01(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut q = QueryBuilder::new(schema);
    q.rel("CompanyType")?
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

fn q09(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut q = QueryBuilder::new(schema);
    q.rel("AkaName")?
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
        .done();
    range(&mut q, "year", ComparisonOperator::Gte, 2005)?;
    range(&mut q, "year", ComparisonOperator::Lte, 2015)?;
    q.find_var("movie")?.finish()
}

fn q16(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut q = QueryBuilder::new(schema);
    q.rel("AkaName")?
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
        .done();
    range(&mut q, "episode", ComparisonOperator::Gte, 50)?;
    range(&mut q, "episode", ComparisonOperator::Lt, 100)?;
    q.find_var("movie")?.finish()
}

fn q24(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut q = QueryBuilder::new(schema);
    q.rel("AkaName")?
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
        .done();
    range(&mut q, "year", ComparisonOperator::Gt, 2010)?;
    q.find_var("movie")?.finish()
}

fn bridge(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    let mut q = QueryBuilder::new(schema);
    q.rel("MovieLink")?
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

fn range(
    q: &mut QueryBuilder<'_>,
    var: &str,
    op: ComparisonOperator,
    value: i128,
) -> QueryBuildResult<()> {
    q.cmp(
        OperandRef::var(var),
        op,
        OperandRef::literal(Literal::Integer(value)),
    )?;
    Ok(())
}
