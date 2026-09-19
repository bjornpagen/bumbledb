//@ error: unknown predicate quantifier
use bumbledb_query::query;
bumbledb::schema! { pub Predicates; relation Trial { id: u64 } }
fn main() {
    let _ = query!(Predicates { (p: PredicateTest(Maybe(1>0))) | Trial(id); });
}
