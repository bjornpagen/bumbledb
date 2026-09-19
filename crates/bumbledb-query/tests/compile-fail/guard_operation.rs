//@ error: query!: unknown guard operation
use bumbledb_query::query;
bumbledb::schema! { pub Guards; relation Trial { when: event } }
fn main() {
    let _ = query!(Guards { (out: Guard(Maybe(p,g))) | Trial(when:p); });
}
