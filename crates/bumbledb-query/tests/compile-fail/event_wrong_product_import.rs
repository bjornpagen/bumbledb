//@ error: composition/residual/closure requires a declared `use product` import
use bumbledb_query::query;
bumbledb::schema! {
    pub Events;
    relation Claim { when: event }
}
fn main() {
    let descriptor = ();
    let _ = query!(Events {
        use faces pair = &descriptor;
        (out: Event(Region(Compose(Relation(a, pair), Relation(a, pair), pair)))) |
            Claim(when: a);
    });
}
