//@ error: unexpected token in guard expression
use bumbledb_query::query;
bumbledb::schema! { pub Guards; relation Trial { when:event, identity:bytes<32> } }
fn run() { let _ = query!(Guards { (e:Guard(Holds(Sign(1,4),Existing(when,identity)))) | Trial(when,identity); }); }
