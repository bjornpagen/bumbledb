//@ error: Pow requires a natural exponent
use bumbledb_query::query;
bumbledb::schema! { pub Numbers; relation Trial { id: u64 } }
fn main() {
    let _ = query!(Numbers { (n: Number(Pow(1,-1))) | Trial(id); });
}
