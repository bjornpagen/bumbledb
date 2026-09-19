//@ error: is not bound
use bumbledb_query::query;
bumbledb::schema! { pub Guards; relation Trial { when: event } }
fn run(plan: &bumbledb::PredicateGuardPlan) {
    let _ = query!(Guards { use guard g = plan; (out: Guard(Holds(Sign(1,4),Common(g,missing)))) | Trial(when); });
}
