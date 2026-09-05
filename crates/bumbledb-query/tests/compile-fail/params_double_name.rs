//! One value per param: `params!` refuses a doubled name at the macro
//! (the typestate would refuse it one step later regardless).
//@ error: params!: floor is supplied twice

bumbledb::schema! {
    pub Grades;

    relation Attempt {
        id: id128 as AttemptId,
        score: f64,
    }

    Attempt(id) -> Attempt;
}

pub fn bound() -> Vec<bumbledb::ParamArg<'static>> {
    let attempts_for = bumbledb_query::query!(Grades {
        (score) | Attempt(id, score), score > ?floor;
    });
    attempts_for.bind(bumbledb_query::params! {
        floor: 0.5f64,
        floor: 0.9f64
    })
}
