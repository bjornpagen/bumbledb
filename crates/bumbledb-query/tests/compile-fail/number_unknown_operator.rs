//@ error: unknown numerical operator
use bumbledb_query::query;
bumbledb::schema! { pub Numbers; relation Trial { id: u64 } }
fn main() {
    let _ = query!(Numbers { (n: Number(Round(1))) | Trial(id); });
}
