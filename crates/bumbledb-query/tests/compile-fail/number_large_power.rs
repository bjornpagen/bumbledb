//@ error: power exceeds u32
use bumbledb_query::query;
bumbledb::schema! { pub Numbers; relation Trial { id: u64 } }
fn main() {
    let _ = query!(Numbers { (n: Number(Pow(1,4294967296))) | Trial(id); });
}
