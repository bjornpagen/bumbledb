//! Read-only joint observation. Aliases are never counted as admitted worlds.
//! The pair is (original support, completed Event); no conjunction is interned.
use super::essential_raw::{Arena, Ref, axes};
use super::observation::{CountPlan, Spectrum};
use super::occupancy::Restricted;
use rustc_hash::FxHashMap;

fn variables<const K: u32>(a: &Arena<K>, views: [Restricted; 2]) -> u64 {
    views[0].remaining(a) | views[1].remaining(a)
}
fn local<const K: u32>(
    a: &Arena<K>,
    views: [Restricted; 2],
    vars: u64,
    mut accept: impl FnMut(u64),
) {
    let coords = axes(vars);
    for i in 0..1usize << coords.len() {
        let w = coords
            .iter()
            .enumerate()
            .fold(0, |w, (k, &v)| w | (((i >> k) & 1) as u64) << v);
        if views.iter().all(|r| a.evaluate(r.root, w | r.values)) {
            accept(w);
        }
    }
}
pub fn count<const K: u32>(a: &Arena<K>, order: &[u32], support: Ref, event: Ref) -> u64 {
    fn visit<const K: u32>(
        a: &Arena<K>,
        order: &[u32],
        views: [Restricted; 2],
        memo: &mut FxHashMap<[Restricted; 2], u64>,
    ) -> u64 {
        if views.iter().any(|r| r.root == 0) {
            return 0;
        }
        if let Some(&out) = memo.get(&views) {
            return out;
        }
        let vars = variables(a, views);
        let mut out = 0;
        if vars.count_ones() <= K {
            local(a, views, vars, |_| out += 1);
        } else {
            let v = *order.iter().find(|&&v| vars >> v & 1 != 0).unwrap();
            for high in [false, true] {
                let child = views.map(|r| r.cofactor(a, v, high));
                let free = vars.count_ones() - 1 - variables(a, child).count_ones();
                out += visit(a, order, child, memo) << free;
            }
        }
        memo.insert(views, out);
        out
    }
    if support == 1 {
        return a.count(event);
    }
    let views = [support, event].map(Restricted::new);
    visit(a, order, views, &mut FxHashMap::default())
        << (order.len() as u32 - variables(a, views).count_ones())
}
pub fn spectrum<const K: u32>(
    a: &Arena<K>,
    order: &[u32],
    support: Ref,
    event: Ref,
    plan: &CountPlan,
) -> Spectrum {
    fn visit<const K: u32>(
        a: &Arena<K>,
        order: &[u32],
        views: [Restricted; 2],
        plan: &CountPlan,
        memo: &mut FxHashMap<[Restricted; 2], Spectrum>,
    ) -> Spectrum {
        if views.iter().any(|r| r.root == 0) {
            return plan.zero();
        }
        if let Some(out) = memo.get(&views) {
            return out.clone();
        }
        let vars = variables(a, views);
        let mut out = plan.zero();
        if vars.count_ones() <= K {
            local(a, views, vars, |w| out[plan.index(w as usize)] += 1);
        } else {
            let v = *order.iter().find(|&&v| vars >> v & 1 != 0).unwrap();
            for high in [false, true] {
                let child = views.map(|r| r.cofactor(a, v, high));
                let mut part = visit(a, order, child, plan, memo);
                plan.smooth(&mut part, axes(vars & !(1u64 << v) & !variables(a, child)));
                let shift = if high {
                    plan.strides[plan.group_of[v as usize]]
                } else {
                    0
                };
                plan.add_shifted(&mut out, &part, shift, 1);
            }
        }
        memo.insert(views, out.clone());
        out
    }
    assert_eq!(order.len(), plan.group_of.len());
    let views = [support, event].map(Restricted::new);
    let mut out = visit(a, order, views, plan, &mut FxHashMap::default());
    let vars = variables(a, views);
    plan.smooth(
        &mut out,
        (0..order.len() as u32).filter(|v| vars >> v & 1 == 0),
    );
    out
}
