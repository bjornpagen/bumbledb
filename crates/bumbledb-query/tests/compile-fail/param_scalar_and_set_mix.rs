//! One param name binds one engine slot, and a slot is scalar XOR set —
//! a name used in both positions refuses at expansion with the name.
//@ error: is used as both

bumbledb::schema! {
    pub Grades;

    relation Attempt {
        id: id128 as AttemptId,
        units: u64,
        score: f64,
    }

    Attempt(id) -> Attempt;
}

pub fn q() -> bumbledb::Query {
    bumbledb_query::query!(Grades {
        (score) | Attempt(id, units in ?units, score), score > ?units;
    })
    .into_query()
}
