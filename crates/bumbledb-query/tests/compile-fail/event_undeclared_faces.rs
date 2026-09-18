//@ error: relation requires a declared `use faces` import
use bumbledb_query::query;
bumbledb::schema! {
    pub Events;
    relation Claim { when: event }
}
fn main() {
    let _ = query!(Events {
        (out: Event(Domain(Relation(a, missing)))) | Claim(when: a);
    });
}
