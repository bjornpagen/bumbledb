//@ error: Guard requires a declared `use guard` import
use bumbledb_query::query;
bumbledb::schema! { pub Guards; relation Trial { when: event } }
fn main() {
    let _ = query!(Guards { (out: Guard(Holds(Sign(1,4),missing))) | Trial(when); });
}
