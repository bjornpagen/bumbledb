use super::corpus::{boxed, distinct_rows};
use super::costars::costars;
use super::country_rollup::country_rollup;
use super::filmography::{filmography, filmography_params};
use super::five_way::{five_way, five_way_params};
use super::keyword_kind::{keyword_kind, keyword_kind_params};
use super::keyword_neighborhood::keyword_neighborhood;
use super::person_params::person_params;
use super::{
    ids, schema, Scenario, ScenarioQuery, CASTS, COMPANIES, KEYWORDS, KINDS, MOVIES,
    MOVIE_COMPANIES, MOVIE_KEYWORDS, PEOPLE,
};

/// The scenario registration.
#[must_use]
pub fn scenario() -> Scenario {
    Scenario {
        name: "joins",
        about: "JOB-style join-order stress: skewed fan-ins, correlated predicates",
        schema,
        descriptor: || bumbledb::Theory::descriptor(super::Joins),
        rows: |seed| {
            vec![
                (ids::KIND, boxed(seed, ids::KIND, KINDS)),
                (ids::COMPANY, boxed(seed, ids::COMPANY, COMPANIES)),
                (ids::PERSON, boxed(seed, ids::PERSON, PEOPLE)),
                (ids::KEYWORD, boxed(seed, ids::KEYWORD, KEYWORDS)),
                (ids::MOVIE, boxed(seed, ids::MOVIE, MOVIES)),
                (
                    ids::CAST_INFO,
                    Box::new(distinct_rows(seed, ids::CAST_INFO, CASTS).into_iter()),
                ),
                (
                    ids::MOVIE_COMPANY,
                    Box::new(distinct_rows(seed, ids::MOVIE_COMPANY, MOVIE_COMPANIES).into_iter()),
                ),
                (
                    ids::MOVIE_KEYWORD,
                    Box::new(distinct_rows(seed, ids::MOVIE_KEYWORD, MOVIE_KEYWORDS).into_iter()),
                ),
            ]
        },
        extra_indexes: &[
            "CREATE INDEX ix_movie_year ON \"Movie\"(\"year\")",
            "CREATE INDEX ix_movie_kind ON \"Movie\"(\"kind\")",
            "CREATE INDEX ix_cast_person ON \"CastInfo\"(\"person\")",
            "CREATE INDEX ix_mk_keyword ON \"MovieKeyword\"(\"keyword\")",
            "CREATE INDEX ix_mc_company ON \"MovieCompany\"(\"company\")",
            "CREATE INDEX ix_person_gender ON \"Person\"(\"gender\")",
            "CREATE INDEX ix_company_country ON \"Company\"(\"country\")",
        ],
        queries: || {
            vec![
                ScenarioQuery {
                    name: "j1_filmography",
                    query: filmography,
                    params: filmography_params,
                    about: "2-atom containment walk under 25%-hot fan-in skew",
                },
                ScenarioQuery {
                    name: "j2_costars",
                    query: costars,
                    params: |seed| person_params(seed, 2),
                    about: "self-join through the fact table, hot vs cold",
                },
                ScenarioQuery {
                    name: "j3_keyword_kind",
                    query: keyword_kind,
                    params: keyword_kind_params,
                    about: "3-way pinched by string point + year range",
                },
                ScenarioQuery {
                    name: "j4_five_way",
                    query: five_way,
                    params: five_way_params,
                    about: "JOB-shaped 5-way, dims filter both sides",
                },
                ScenarioQuery {
                    name: "j5_country_rollup",
                    query: country_rollup,
                    params: |_| vec![vec![]],
                    about: "full-join rollup: Min(year)+Count by country",
                },
                ScenarioQuery {
                    name: "j6_keyword_neighborhood",
                    query: keyword_neighborhood,
                    params: |seed| person_params(seed, 6),
                    about: "fan-out explosion through shared keywords",
                },
            ]
        },
    }
}
