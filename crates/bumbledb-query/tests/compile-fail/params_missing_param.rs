//! A missing param is a typestate mismatch: `bind` demands every slot
//! moved to bound, so an incomplete `params!` record cannot typecheck.
//@ error: __BumbledbUnset

bumbledb::schema! {
    pub Grades;

    relation Attempt {
        id: uuid as AttemptId,
        student: uuid as StudentId,
        score: f64,
    }

    Attempt(id) -> Attempt;
}

pub fn bound() -> Vec<bumbledb::ParamArg<'static>> {
    let attempts_for = bumbledb_query::query!(Grades {
        (score) | Attempt(id, student == ?student, score), score > ?floor;
    });
    attempts_for.bind(bumbledb_query::params! {
        student: bumbledb::Uuid::from_bytes([1; 16])
    })
}
