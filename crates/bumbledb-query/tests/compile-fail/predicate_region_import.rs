//@ error: requires a declared `use parameter_region` import
use bumbledb::{schema, query};
schema! { pub Example; relation Row { id:u64 } }
fn main() {
    let _ = query!(Example { use number_domain d = &panic!("not run"); (p: Predicate(Region(d,unknown))) | Row(id); });
}
