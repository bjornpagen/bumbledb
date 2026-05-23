use super::*;

pub(super) fn job_queries() -> Vec<BenchQuery> {
    vec![
        BenchQuery {
            name: "job_broad_cast_keyword_company",
            build: build_job_broad_cast_keyword_company,
            inputs: Vec::new(),
            sqlite: r#"
                SELECT DISTINCT t.id
                FROM title t
                JOIN cast_info ci ON ci.movie_id = t.id
                JOIN role_type rt ON rt.id = ci.role_id
                JOIN movie_keyword mk ON mk.movie_id = t.id
                JOIN keyword k ON k.id = mk.keyword_id
                JOIN movie_companies mc ON mc.movie_id = t.id
                JOIN company_name cn ON cn.id = mc.company_id
                JOIN company_type ct ON ct.id = mc.company_type_id
            "#,
            sqlite_params: Vec::new(),
        },
        BenchQuery {
            name: "job_broad_movie_info_star",
            build: build_job_broad_movie_info_star,
            inputs: Vec::new(),
            sqlite: r#"
                SELECT DISTINCT t.id
                FROM title t
                JOIN cast_info ci ON ci.movie_id = t.id
                JOIN role_type rt ON rt.id = ci.role_id
                JOIN movie_companies mc ON mc.movie_id = t.id
                JOIN company_type ct ON ct.id = mc.company_type_id
                JOIN movie_keyword mk ON mk.movie_id = t.id
                JOIN keyword k ON k.id = mk.keyword_id
                JOIN movie_info mi ON mi.movie_id = t.id
                JOIN info_type it ON it.id = mi.info_type_id
                JOIN movie_info_idx mi_idx ON mi_idx.movie_id = t.id
                JOIN info_type it_idx ON it_idx.id = mi_idx.info_type_id
            "#,
            sqlite_params: Vec::new(),
        },
        BenchQuery {
            name: "job_q01_top_production",
            build: build_job_q01_top_production,
            inputs: Vec::new(),
            sqlite: r#"
                SELECT DISTINCT t.id
                FROM company_type ct
                JOIN movie_companies mc ON mc.company_type_id = ct.id
                JOIN movie_info_idx mi_idx ON mi_idx.movie_id = mc.movie_id
                JOIN info_type it ON it.id = mi_idx.info_type_id
                JOIN title t ON t.id = mc.movie_id
                WHERE ct.kind = 'production companies'
                  AND it.info = 'top 250 rank'
            "#,
            sqlite_params: Vec::new(),
        },
        BenchQuery {
            name: "job_q09_voice_us_actor",
            build: build_job_q09_voice_us_actor,
            inputs: Vec::new(),
            sqlite: r#"
                SELECT DISTINCT t.id
                FROM aka_name an
                JOIN name n ON n.id = an.person_id
                JOIN cast_info ci ON ci.person_id = n.id
                JOIN char_name chn ON chn.id = ci.person_role_id
                JOIN role_type rt ON rt.id = ci.role_id
                JOIN title t ON t.id = ci.movie_id
                JOIN movie_companies mc ON mc.movie_id = t.id
                JOIN company_name cn ON cn.id = mc.company_id
                WHERE cn.country_code = '[us]'
                  AND n.gender = 'm'
                  AND rt.role = 'actor'
                  AND t.production_year BETWEEN 2005 AND 2015
            "#,
            sqlite_params: Vec::new(),
        },
        BenchQuery {
            name: "job_q16_character_title_us",
            build: build_job_q16_character_title_us,
            inputs: Vec::new(),
            sqlite: r#"
                SELECT DISTINCT t.id
                FROM aka_name an
                JOIN name n ON n.id = an.person_id
                JOIN cast_info ci ON ci.person_id = n.id
                JOIN title t ON t.id = ci.movie_id
                JOIN movie_keyword mk ON mk.movie_id = t.id
                JOIN keyword k ON k.id = mk.keyword_id
                JOIN movie_companies mc ON mc.movie_id = t.id
                JOIN company_name cn ON cn.id = mc.company_id
                WHERE cn.country_code = '[us]'
                  AND k.keyword = 'character-name-in-title'
                  AND t.episode_nr >= 50
                  AND t.episode_nr < 100
            "#,
            sqlite_params: Vec::new(),
        },
        BenchQuery {
            name: "job_q24_voice_keyword_actor",
            build: build_job_q24_voice_keyword_actor,
            inputs: Vec::new(),
            sqlite: r#"
                SELECT DISTINCT t.id
                FROM aka_name an
                JOIN name n ON n.id = an.person_id
                JOIN cast_info ci ON ci.person_id = n.id
                JOIN char_name chn ON chn.id = ci.person_role_id
                JOIN role_type rt ON rt.id = ci.role_id
                JOIN title t ON t.id = ci.movie_id
                JOIN movie_companies mc ON mc.movie_id = t.id
                JOIN company_name cn ON cn.id = mc.company_id
                JOIN movie_keyword mk ON mk.movie_id = t.id
                JOIN keyword k ON k.id = mk.keyword_id
                WHERE cn.country_code = '[us]'
                  AND k.keyword = 'hero'
                  AND n.gender = 'm'
                  AND rt.role = 'actor'
                  AND t.production_year > 2010
            "#,
            sqlite_params: Vec::new(),
        },
        BenchQuery {
            name: "job_movie_link_bridge",
            build: build_job_movie_link_bridge,
            inputs: Vec::new(),
            sqlite: r#"
                SELECT DISTINCT t1.id
                FROM movie_link ml
                JOIN link_type lt ON lt.id = ml.link_type_id
                JOIN title t1 ON t1.id = ml.movie_id
                JOIN title t2 ON t2.id = ml.linked_movie_id
                JOIN movie_companies mc1 ON mc1.movie_id = t1.id
                JOIN company_name cn1 ON cn1.id = mc1.company_id
                JOIN movie_companies mc2 ON mc2.movie_id = t2.id
                JOIN company_name cn2 ON cn2.id = mc2.company_id
                JOIN movie_info_idx mi_idx1 ON mi_idx1.movie_id = t1.id
                JOIN info_type it1 ON it1.id = mi_idx1.info_type_id
                JOIN movie_info_idx mi_idx2 ON mi_idx2.movie_id = t2.id
                JOIN info_type it2 ON it2.id = mi_idx2.info_type_id
            "#,
            sqlite_params: Vec::new(),
        },
        BenchQuery {
            name: "job_q33_linked_series_companies",
            build: build_job_q33_linked_series_companies,
            inputs: Vec::new(),
            sqlite: r#"
                SELECT DISTINCT t1.id
                FROM company_name cn1
                JOIN movie_companies mc1 ON mc1.company_id = cn1.id
                JOIN title t1 ON t1.id = mc1.movie_id
                JOIN kind_type kt1 ON kt1.id = t1.kind_id
                JOIN movie_link ml ON ml.movie_id = t1.id
                JOIN link_type lt ON lt.id = ml.link_type_id
                JOIN title t2 ON t2.id = ml.linked_movie_id
                JOIN kind_type kt2 ON kt2.id = t2.kind_id
                JOIN movie_companies mc2 ON mc2.movie_id = t2.id
                JOIN company_name cn2 ON cn2.id = mc2.company_id
                WHERE cn1.country_code = '[us]'
                  AND kt1.kind = 'tv series'
                  AND kt2.kind = 'tv series'
                  AND lt.link = 'sequel'
                  AND t2.production_year BETWEEN 2005 AND 2008
            "#,
            sqlite_params: Vec::new(),
        },
    ]
}
