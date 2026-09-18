//@ error: readout requires a declared `use map` import
use bumbledb_query::query;

bumbledb::schema! {
    pub Events;
    relation Claim { when: event }
}

fn main() {
    let _ = query!(Events {
        (out: Event(Image(a, missing))) | Claim(when: a);
    });
}
