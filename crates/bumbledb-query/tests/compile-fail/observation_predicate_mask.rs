//@ error: predicate mask is out of range
use bumbledb_query::query;
bumbledb::schema! { pub Predicates; relation Trial { id: u64 } }
fn main() {
    let _ = query!(Predicates { (p: Predicate(Sign(1,8))) | Trial(id); });
}
