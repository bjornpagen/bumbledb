//@ error: OnDomain requires a declared `use number_domain` import
use bumbledb_query::query;
bumbledb::schema! { pub Numbers; relation Trial { id: u64 } }
fn main() {
    let bytes = ();
    let _ = query!(Numbers { use number n = &bytes; (out: Number(OnDomain(1,n))) | Trial(id); });
}
