//! A param whose name is a Rust keyword cannot become a typed bind
//! method — the refusal names the param and asks for a rename.
//@ error: cannot become a typed bind method

bumbledb::schema! {
    pub Grades;

    relation Attempt {
        id: id128 as AttemptId,
        score: f64,
    }

    Attempt(id) -> Attempt;
}

pub fn q() -> bumbledb::Query {
    bumbledb_query::query!(Grades {
        (score) | Attempt(id, score), score > ?loop;
    })
    .into_query()
}
