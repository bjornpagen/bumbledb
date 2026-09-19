//@ error: Common's nonempty predicate roster
use bumbledb_query::query;
bumbledb::schema! { pub Guards; relation Trial { when: event } }
fn main() {
    let _ = query!(Guards { (out: Guard(Holds(Sign(1,4),Common(g)))) | Trial(when); });
}
