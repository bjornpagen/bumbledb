//! One param name is scalar or set, never both.
//@ error: is used as both
//@ line: 19

bumbledb::schema! {
    pub Grades;

    relation Attempt {
        id: uuid as AttemptId,
        units: u64,
        score: f64,
    }

    Attempt(id) -> Attempt;
}

pub fn q() -> bumbledb::Query {
    bumbledb::query!(Grades {
        (score) | Attempt(id, units in ?units, score), score > ?units;
    })
    .into_query()
}
