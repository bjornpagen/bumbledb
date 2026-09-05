//! An unknown param name in `params!` is a missing builder method — the
//! typestate builder carries exactly the template's param names.
//@ error: no method named `student_typo`

bumbledb::schema! {
    pub Grades;

    relation Attempt {
        id: id128 as AttemptId,
        student: id128 as StudentId,
        score: f64,
    }

    Attempt(id) -> Attempt;
}

pub fn bound() -> Vec<bumbledb::ParamArg<'static>> {
    let attempts_for = bumbledb_query::query!(Grades {
        (score) | Attempt(id, student == ?student, score);
    });
    attempts_for.bind(bumbledb_query::params! {
        student_typo: bumbledb::Id128::from_bytes([1; 16])
    })
}
