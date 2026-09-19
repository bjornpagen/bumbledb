//@ error: requires a declared `use number_domain` import
use bumbledb::{schema, query};
schema! { pub Example; relation Row { id:u64 } }
fn main() {
    let _ = query!(Example { (p: Predicate(Region(unknown,r))) | Row(id); });
}
