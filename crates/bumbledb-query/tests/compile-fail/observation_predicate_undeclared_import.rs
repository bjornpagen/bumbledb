//@ error: undeclared
use bumbledb_query::query;
bumbledb::schema! { pub Predicates; relation Trial { id: u64 } }
fn main() {
    let _ = query!(Predicates { (p: Predicate(Imported(missing))) | Trial(id); });
}
