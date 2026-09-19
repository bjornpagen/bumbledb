//@ error: predicate mask is out of range
use bumbledb_query::query;
bumbledb::schema! { pub Predicates; relation Trial { id: u64 } }
fn main() {
    let _ = query!(Predicates { (p: Predicate(Bool4(16,1>0,1>0))) | Trial(id); });
}
