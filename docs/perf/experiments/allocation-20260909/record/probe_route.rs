use super::*;
use crate::exec::colt::KeyCount;
use crate::exec::run::Counters;
use std::collections::BTreeMap;

crate::schema! {
    pub DisplacedWorld;
    relation Hub { id: u64 as HubId, tag: u64, }
    relation Spoke { id: u64 as SpokeId, hub: u64 as HubId, val: u64, }
    Hub(id) -> Hub;
    Spoke(id) -> Spoke;
    Spoke(hub) <= Hub(id);
}

#[derive(Debug, Default)]
struct Routes {
    carried: Vec<Vec<bool>>,
    covers: BTreeMap<(usize, usize), u64>,
    batches: BTreeMap<(usize, usize), (u64, u64)>,
    probes: BTreeMap<(usize, usize), (u64, u64)>,
}

impl Routes {
    fn of(plan: &crate::plan::fj::ValidatedPlan) -> Self {
        let mut levels = vec![0; plan.occurrences().len()];
        let mut result = Self::default();
        for (i, node) in plan.nodes().iter().enumerate() {
            result.carried.push(node.subatoms.iter().map(|s| levels[usize::from(s.occ.0)] > 0).collect());
            println!("NODE {i} {node:?} entry={levels:?}");
            for sub in &node.subatoms {
                levels[usize::from(sub.occ.0)] += 1;
            }
        }
        result
    }
}

impl Counters for Routes {
    fn node_entry(&mut self, _: usize) {}
    fn batch(&mut self, _: usize, _: usize) {}
    fn cover_choice(&mut self, n: usize, s: usize, _: KeyCount) {
        *self.covers.entry((n, s)).or_default() += 1;
    }
    fn probe_hash(&mut self, _: usize, _: usize) {}
    fn probe(&mut self, n: usize, s: usize, hit: bool) {
        let row = self.probes.entry((n, s)).or_default();
        row.0 += 1;
        row.1 += u64::from(hit);
    }
    fn probe_batch(&mut self, n: usize, s: usize, len: usize) {
        let row = self.batches.entry((n, s)).or_default();
        row.0 += 1;
        row.1 += len as u64;
    }
    fn residual(&mut self, _: usize, _: bool) {}
    fn anti_probe(&mut self, _: usize, _: bool) {}
    fn emit(&mut self) {}
    fn skip(&mut self, _: usize) {}
}

fn mix(rel: u64, row: u64) -> u64 {
    let mut z = 1 ^ (rel << 56) ^ row;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[test]
#[ignore = "local saved-corpus route audit, not a timing or trace workload"]
fn saved_displaced_route() {
    let path = std::env::var_os("BUMBLEDB_ROUTE_DB").expect("private copy path");
    let work = crate::WorkContext::new();
    let db = crate::Db::open(std::path::Path::new(&path), DisplacedWorld, work.clone()).unwrap();
    let v = |id| Term::Var(VarId(id));
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Aggregate { op: crate::FoldOp::Sum, over: VarId(1) }],
        atoms: vec![
            Atom { source: AtomSource::Edb(RelationId(1)), bindings: vec![(FieldId(0), v(2)), (FieldId(1), v(3)), (FieldId(2), v(1))] },
            Atom { source: AtomSource::Edb(RelationId(0)), bindings: vec![(FieldId(0), v(3)), (FieldId(1), v(0))] },
        ],
        negated: vec![], conditions: vec![],
    });
    let mut expected = BTreeMap::<u64, u64>::new();
    for row in 0..1 << 20 {
        let m = mix(1, row);
        let tag = mix(0, m % (1 << 19)) % 1024;
        *expected.entry(tag).or_default() += (m >> 32) % 997;
    }
    let pin = db.owned_read().unwrap();
    let frame = pin.frame(&work);
    let mut prepared = frame.prepare(&query).unwrap();
    let source = super::super::source::QuerySource::store(frame.snapshot(), &work);
    let cache = std::sync::Arc::clone(&prepared.cache);
    let images = crate::image::SourceImages::bind(&source, &cache);
    let mut out = Answers::new();
    for pass in 0..3 {
        let [PreparedRule::FreeJoin(rule)] = prepared.pipeline.main_rules() else { panic!("Free Join"); };
        let mut routes = Routes::of(&rule.plan);
        (&[] as &[BindValue]).bind(&mut prepared, &work).unwrap();
        prepared.sink.begin_execution(Some(work.clone()));
        out.begin(prepared.signature.columns.len());
        let ran = prepared.run_rules(&images, &mut routes).unwrap();
        prepared.finish_sink(&images, ran, &mut out).unwrap();
        let actual: BTreeMap<_, _> = (0..out.len()).map(|i| {
            let (AnswerValue::U64(tag), AnswerValue::U64(sum)) = (out.get(i, 0), out.get(i, 1)) else { panic!("u64 output"); };
            (tag, sum)
        }).collect();
        assert_eq!(actual, expected);
        let mut counts = [0u64; 2];
        for (&(n, s), &(_, keys)) in &routes.batches {
            counts[usize::from(routes.carried[n][s])] += keys;
        }
        println!("PASS {pass}: root_keys={} carried_keys={} {routes:?}", counts[0], counts[1]);
        assert!(counts[0] + counts[1] > 0);
    }
}
