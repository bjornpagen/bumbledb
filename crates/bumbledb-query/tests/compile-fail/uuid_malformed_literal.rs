//! Malformed UUID literals refuse at expansion.
//@ error: UUID syntax

bumbledb::schema! {
    pub People;

    relation Person {
        id: uuid as PersonId,
        name: str,
    }

    Person(id) -> Person;
}

pub fn q() -> bumbledb::Query {
    bumbledb_query::query!(People {
        (name) | Person(id == uuid:"00112233-4455-6677-8899-aabbccddeefg", name);
    })
    .into_query()
}
