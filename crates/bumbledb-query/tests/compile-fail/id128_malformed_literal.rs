//! An Id128 literal is exactly 32 lowercase hex characters — uppercase,
//! UUID punctuation and other widths refuse at expansion.
//@ error: 32 lowercase hex

bumbledb::schema! {
    pub People;

    relation Person {
        id: id128 as PersonId,
        name: str,
    }

    Person(id) -> Person;
}

pub fn q() -> bumbledb::Query {
    bumbledb_query::query!(People {
        (name) | Person(id == id128:"00112233445566778899AABBCCDDEEFF", name);
    })
    .into_query()
}
