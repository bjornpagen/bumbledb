//! A named param cannot be a Rust keyword.
//@ error: is a Rust keyword and cannot name a bind method
//@ line: 18

bumbledb::schema! {
    pub Grades;

    relation Attempt {
        id: uuid as AttemptId,
        score: f64,
    }

    Attempt(id) -> Attempt;
}

pub fn q() -> bumbledb::Query {
    bumbledb::query!(Grades {
        (score) | Attempt(id, score), score > ?loop;
    })
    .into_query()
}
