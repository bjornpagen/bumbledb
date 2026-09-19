//@ error: Imported requires a declared `use number` import
use bumbledb_query::query;
bumbledb::schema! { pub Numbers; relation Trial { id: u64 } }
fn main() {
    let _ = query!(Numbers { (n: Number(Imported(missing))) | Trial(id); });
}
