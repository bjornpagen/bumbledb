//! Pointwise FD/IND contract over exact Event carriers, independent of probability.
use super::carrier::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Summary {
    pub covered: Id,
    pub conflict: Id,
}
impl Summary {
    pub fn empty<C: Carrier>(c: &C) -> Self {
        Self {
            covered: c.empty(),
            conflict: c.empty(),
        }
    }
    pub fn leaf<C: Carrier>(c: &C, event: Id) -> Self {
        Self {
            covered: event,
            conflict: c.empty(),
        }
    }
    // Children must own disjoint sets of distinct fact identities.
    pub fn merge<C: Carrier>(self, other: Self, c: &mut C) -> Self {
        let across = c.op(8, self.covered, other.covered);
        let within = c.op(14, self.conflict, other.conflict);
        Self {
            covered: c.op(14, self.covered, other.covered),
            conflict: c.op(14, within, across),
        }
    }
}

#[derive(Clone)]
pub struct SummaryTree {
    width: usize,
    nodes: Vec<Summary>,
}
impl SummaryTree {
    pub fn new<C: Carrier>(events: &[Id], c: &mut C) -> Self {
        let width = events.len().max(1).next_power_of_two();
        let mut nodes = vec![Summary::empty(c); width * 2];
        for (i, &event) in events.iter().enumerate() {
            nodes[width + i] = Summary::leaf(c, event);
        }
        for i in (1..width).rev() {
            nodes[i] = nodes[i * 2].merge(nodes[i * 2 + 1], c);
        }
        Self { width, nodes }
    }
    pub fn root(&self) -> Summary {
        self.nodes[1]
    }
    pub fn set<C: Carrier>(&mut self, slot: usize, event: Id, c: &mut C) {
        assert!(slot < self.width);
        let mut i = self.width + slot;
        self.nodes[i] = Summary::leaf(c, event);
        while i > 1 {
            i /= 2;
            self.nodes[i] = self.nodes[i * 2].merge(self.nodes[i * 2 + 1], c);
        }
    }
    pub fn bytes(&self) -> usize {
        self.nodes.capacity() * std::mem::size_of::<Summary>()
    }
}

// Full logical fixture identity includes the region, not just group and payload.
// Empty masks denote absent fixture branches; they are not attempted stored rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fact {
    pub group: u64,
    pub payload: u64,
    pub region: u64,
}
#[derive(Clone)]
pub struct Case {
    pub support: u64,
    pub rows: [Vec<Fact>; 2],
}
pub type Verdict = [BTreeSet<Fact>; 4];

impl Case {
    pub fn normalized(&self) -> [Vec<Fact>; 2] {
        self.rows.each_ref().map(|rows| {
            rows.iter()
                .map(|r| Fact {
                    region: r.region & self.support,
                    ..*r
                })
                .filter(|r| r.region != 0)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect()
        })
    }
    pub fn oracle(&self) -> Verdict {
        let rows = self.normalized();
        let mut bad: Verdict = std::array::from_fn(|_| BTreeSet::new());
        for side in 0..2 {
            for (i, fact) in rows[side].iter().enumerate() {
                // Direct pointwise ownership counts, independent of Summary::merge.
                for w in 0..64 {
                    if fact.region >> w & 1 == 0 {
                        continue;
                    }
                    if rows[side]
                        .iter()
                        .enumerate()
                        .any(|(j, r)| i != j && r.group == fact.group && r.region >> w & 1 != 0)
                    {
                        bad[side].insert(*fact);
                    }
                    if !rows[1 - side]
                        .iter()
                        .any(|r| r.group == fact.group && r.region >> w & 1 != 0)
                    {
                        bad[side + 2].insert(*fact);
                    }
                }
            }
        }
        bad
    }
}

pub fn cases() -> Vec<Case> {
    let f = |group, payload, region| Fact {
        group,
        payload,
        region,
    };
    let mut out = vec![];
    // All triples of four-world regions; equal full source facts collapse.
    for a in 0..16 {
        for b in 0..16 {
            for d in 0..16 {
                out.push(Case {
                    support: 15,
                    rows: [vec![f(0, 7, a), f(0, 7, b)], vec![f(0, 9, d)]],
                });
            }
        }
    }
    // Restricted supports, both conflicting sides, distinct groups and duplicates.
    for support in 1..16u64 {
        for seed in 0..64u64 {
            let mut rows: [Vec<Fact>; 2] = std::array::from_fn(|side| {
                (0..5)
                    .map(|i| {
                        f(
                            (seed + i) % 3,
                            i % 2,
                            mix(seed * 101 + i * 37 + side as u64 * 13),
                        )
                    })
                    .collect()
            });
            let duplicate = rows[0][0];
            rows[0].push(duplicate);
            rows[1].reverse();
            out.push(Case { support, rows });
        }
    }
    // Whole-word boundaries and partial occurrence partitions.
    for support in [u64::MAX, 0xaaaa_aaaa_aaaa_aaaa, 1u64 << 63] {
        let a = support & 0xcccc_cccc_cccc_cccc;
        let b = support & !a;
        out.push(Case {
            support,
            rows: [vec![f(1, 7, a), f(1, 8, b)], vec![f(1, 9, support)]],
        });
        out.push(Case {
            support,
            rows: [vec![f(1, 7, a), f(1, 7, support)], vec![f(1, 9, a)]],
        });
    }
    out
}

pub fn judge<C: Carrier>(case: &Case, c: &mut C) -> Verdict {
    let rows = case.normalized();
    let mut groups: [BTreeMap<u64, Vec<(Fact, Id)>>; 2] = std::array::from_fn(|_| BTreeMap::new());
    for side in 0..2 {
        for fact in &rows[side] {
            let event = c.import(&[fact.region]);
            groups[side]
                .entry(fact.group)
                .or_default()
                .push((*fact, event));
        }
    }
    let mut summaries: [BTreeMap<u64, Summary>; 2] = std::array::from_fn(|_| BTreeMap::new());
    for side in 0..2 {
        for (&group, facts) in &groups[side] {
            let events: Vec<_> = facts.iter().map(|(_, e)| *e).collect();
            let tree = SummaryTree::new(&events, c);
            summaries[side].insert(group, tree.root());
        }
    }
    let mut bad: Verdict = std::array::from_fn(|_| BTreeSet::new());
    for side in 0..2 {
        for (&group, facts) in &groups[side] {
            let own = summaries[side][&group];
            let target = summaries[1 - side]
                .get(&group)
                .map_or(c.empty(), |s| s.covered);
            for &(fact, event) in facts {
                if c.op(8, event, own.conflict) != c.empty() {
                    bad[side].insert(fact);
                }
                if c.op(4, event, target) != c.empty() {
                    bad[side + 2].insert(fact);
                }
            }
        }
    }
    bad
}

pub fn verify<C: Carrier>() {
    let cases = cases();
    for case in &cases {
        let mut c = C::new(&[case.support], 64);
        assert_eq!(
            judge(case, &mut c),
            case.oracle(),
            "{} pointwise contract",
            C::NAME
        );
    }
    // Exhaustive associativity and commutativity of singleton summaries.
    let mut c = C::new(&[15], 64);
    let events: Vec<_> = (0..16).map(|w| c.import(&[w])).collect();
    for &a in &events {
        for &b in &events {
            for &d in &events {
                let (a, b, d) = (
                    Summary::leaf(&c, a),
                    Summary::leaf(&c, b),
                    Summary::leaf(&c, d),
                );
                let ab = a.merge(b, &mut c);
                let bd = b.merge(d, &mut c);
                assert_eq!(ab.merge(d, &mut c), a.merge(bd, &mut c));
                assert_eq!(ab, b.merge(a, &mut c));
            }
        }
    }
    // Deletion cannot be implemented by subtracting a contributor from coverage.
    let a = c.import(&[7]);
    let b = c.import(&[6]);
    let d = c.import(&[8]);
    let mut tree = SummaryTree::new(&[a, b, d], &mut c);
    assert_eq!(c.export(tree.root().conflict), vec![6]);
    let wrong = c.op(4, tree.root().covered, a);
    let empty = c.empty();
    tree.set(0, empty, &mut c);
    assert_eq!(c.export(tree.root().covered), vec![14]);
    assert_ne!(tree.root().covered, wrong);
    assert_eq!(tree.root().conflict, empty);
    // A remove+insert transaction is judged only after the final replacement.
    tree.set(1, empty, &mut c);
    let replacement = c.import(&[7]);
    tree.set(0, replacement, &mut c);
    assert_eq!(c.export(tree.root().covered), vec![15]);
    assert_eq!(tree.root().conflict, empty);
    // Mutable slots stand for distinct fixture facts, not public database IDs.
    // Check every final root after mixed removals/replacements against per-world counts.
    let mut c = C::new(&[u64::MAX], 64);
    let mut masks = vec![0u64; 32];
    let mut tree = SummaryTree::new(&vec![c.empty(); 32], &mut c);
    for step in 0..512u64 {
        let slot = (mix(step) % 32) as usize;
        masks[slot] = if step % 3 == 0 { 0 } else { mix(step + 91) };
        let event = c.import(&[masks[slot]]);
        tree.set(slot, event, &mut c);
        let mut cover = 0;
        let mut conflict = 0;
        for w in 0..64 {
            let count = masks.iter().filter(|m| *m >> w & 1 != 0).count();
            if count > 0 {
                cover |= 1u64 << w;
            }
            if count > 1 {
                conflict |= 1u64 << w;
            }
        }
        assert_eq!(c.export(tree.root().covered), vec![cover]);
        assert_eq!(c.export(tree.root().conflict), vec![conflict]);
    }
    println!(
        "EVENT_LAB {{\"kind\":\"dependency_contract\",\"candidate\":\"{}\",\"cases\":{},\"summary_triples\":4096,\"updates\":512,\"passed\":true}}",
        C::NAME,
        cases.len()
    );
}
