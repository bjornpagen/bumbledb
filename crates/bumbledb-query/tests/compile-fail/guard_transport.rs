//@ error: a comma before the Event input
use bumbledb_query::query;
bumbledb::schema! { pub Guards; relation Trial { when: event } }
fn main() {
    let _ = query!(Guards { (out: Guard(Lift(Sign(1,4),g))) | Trial(when); });
}
