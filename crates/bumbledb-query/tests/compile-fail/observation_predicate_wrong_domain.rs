//@ error: OnDomain requires a declared `use number_domain` import
use bumbledb_query::query;
bumbledb::schema! { pub Predicates; relation Trial { id: u64 } }
fn main() {
    let _ = query!(Predicates { (p: Predicate(OnDomain(1>0,missing))) | Trial(id); });
}
