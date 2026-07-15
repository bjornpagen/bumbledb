//! The program form's output law: bare rules ARE the output predicate,
//! so a program of only named rules has nothing to answer — refused at
//! the macro, before the roster ever sees it.
//@ error: a program's output rules are written bare

bumbledb::schema! {
    pub Org;

    relation Parent {
        child: u64,
        parent: u64,
    }
}

pub fn q() -> bumbledb::Program {
    bumbledb_query::query!(Org {
        reach(c, a) | Parent(child: c, parent: a);
        reach(c, a) | Parent(child: c, parent: m), reach(0: m, 1: a);
    })
}
