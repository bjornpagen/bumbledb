//! Real engine execution, compiled only inside the disposable engine's tests.
use crate::exec::{
    colt::Colt,
    run::{Bindings, Executor, Flow, LeafBatch, LeafSource, NoopCounters, Sink},
};
use crate::image::{RelationImage, cache::ImageCache, testsupport::TestSource, view::apply};
use crate::ir::{
    Value, VarId,
    normalize::{NormalizedQuery, OccBind, OccId, Occurrence, Role, SlotWidth},
};
use crate::plan::{
    fj::{ValidatedPlan, binary2fj, factor, validate},
    planner::JoinOrder,
};
use crate::schema::{Schema, ValidateDescriptor};
use bumbledb_theory::schema::{
    FieldDescriptor, FieldId, RelationDescriptor, RelationId, SchemaDescriptor, ValueType,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub type Row = [u64; 4]; // two ordinary keys; EventKey.scope and EventKey.region
pub struct Native {
    plan: ValidatedPlan,
    colts: Vec<Colt>,
    executor: Executor,
    bindings: Bindings,
    slots: [usize; 5],
    _images: Vec<Arc<RelationImage>>,
}
struct Callback<'a, F: FnMut([u64; 5])> {
    callback: &'a mut F,
    slots: [usize; 5],
}
impl<F: FnMut([u64; 5])> Sink for Callback<'_, F> {
    fn emit(&mut self, b: &Bindings) -> Flow {
        (self.callback)(self.slots.map(|s| b.get(s)));
        Flow::Continue
    }
    fn emit_batch(&mut self, b: &LeafBatch<'_>) -> Flow {
        let sources = self.slots.map(|s| b.source_of(s));
        for &entry in b.survivors {
            let row = std::array::from_fn(|i| match sources[i] {
                LeafSource::Key(k) => b.key(entry, k),
                LeafSource::Outer => b.bindings.get(self.slots[i]),
            });
            (self.callback)(row);
        }
        Flow::Continue
    }
    // Defaults forbid suffix skipping, distinct traversal and fused leaf scans.
}
impl Native {
    pub fn new(data: &[Vec<Row>; 3], shape: &str) -> Self {
        let schema: Schema = SchemaDescriptor {
            relations: (0..3)
                .map(|r| RelationDescriptor {
                    extension: None,
                    name: format!("R{r}").into(),
                    fields: ["a", "b", "scope", "region"]
                        .into_iter()
                        .map(|s| FieldDescriptor {
                            name: s.into(),
                            value_type: ValueType::U64,
                        })
                        .collect(),
                })
                .collect(),
            statements: vec![],
        }
        .validate()
        .unwrap();
        let mappings = if shape == "triangle" {
            [[0, 1, 3, 4], [1, 2, 3, 5], [0, 2, 3, 6]]
        } else {
            [[0, 1, 4, 5], [0, 2, 4, 6], [0, 3, 4, 7]]
        };
        let occurrences: Vec<_> = mappings
            .iter()
            .enumerate()
            .map(|(i, vars)| Occurrence {
                occ_id: OccId(i as u16),
                bind: OccBind::Edb(RelationId(i as u32)),
                role: Role::Positive,
                vars: vars
                    .iter()
                    .enumerate()
                    .map(|(f, &v)| (FieldId(f as u16), VarId(v)))
                    .collect(),
                filters: vec![],
                point_vars: vec![],
            })
            .collect();
        let slot_widths: BTreeMap<_, _> = occurrences
            .iter()
            .flat_map(|o| o.vars.iter().map(|(_, v)| (*v, SlotWidth::ONE)))
            .collect();
        let sinks: BTreeSet<_> = slot_widths.keys().copied().collect();
        let normalized = NormalizedQuery {
            dead: None,
            occurrences,
            residuals: vec![],
            word_residuals: vec![],
            allen_residuals: vec![],
            anti_probes: vec![],
            slot_widths,
        };
        let order = JoinOrder {
            order: vec![OccId(0), OccId(1), OccId(2)],
            estimates: vec![0; 3],
        };
        let mut fj = binary2fj(&normalized, &order);
        factor(&mut fj);
        let plan = validate(&fj, &normalized, &schema, &sinks).unwrap();
        let rows: Vec<_> = data
            .iter()
            .enumerate()
            .map(|(i, rs)| {
                (
                    RelationId(i as u32),
                    rs.iter()
                        .map(|r| r.iter().copied().map(Value::U64).collect())
                        .collect(),
                )
            })
            .collect();
        let source = TestSource::new(&schema, &rows);
        let cache = ImageCache::new(&schema);
        let images: Vec<_> = (0..3)
            .map(|i| source.image(&cache, RelationId(i)))
            .collect();
        let colts = plan
            .occurrences()
            .iter()
            .map(|o| {
                let columns = o
                    .trie_schema
                    .iter()
                    .map(|level| {
                        level
                            .iter()
                            .map(|v| {
                                let (field, _) = o.vars.iter().find(|(_, var)| var == v).unwrap();
                                o.spans[field.0 as usize].first_column as usize
                            })
                            .collect()
                    })
                    .collect();
                let image = &images[o.bind.edb().unwrap().0 as usize];
                Colt::new(
                    apply(image, &o.filters, &[], vec![], image.generation().text_eq()).unwrap(),
                    &[],
                    columns,
                )
            })
            .collect();
        let vars = if shape == "triangle" {
            [0, 3, 4, 5, 6]
        } else {
            [0, 4, 5, 6, 7]
        };
        let slots = vars.map(|v| plan.slot_of(VarId(v)));
        let executor = Executor::new(&plan);
        let bindings = Bindings::new(plan.slot_count());
        Self {
            plan,
            colts,
            executor,
            bindings,
            slots,
            _images: images,
        }
    }
    pub fn run(&mut self, mut f: impl FnMut([u64; 5])) {
        let mut sink = Callback {
            callback: &mut f,
            slots: self.slots,
        };
        self.executor
            .execute(
                &self.plan,
                &mut self.colts,
                &mut self.bindings,
                &mut sink,
                &mut NoopCounters,
            )
            .unwrap();
    }
    pub fn plan_nodes(&self) -> usize {
        self.plan.nodes().len()
    }
    pub fn retained_colt_bytes(&self) -> usize {
        self.colts.iter().map(Colt::retained_bytes).sum()
    }
}

pub fn data(shape: &str, groups: usize, fanout: usize, bank: usize) -> [Vec<Row>; 3] {
    std::array::from_fn(|r| {
        let d = if shape == "triangle" && r == 2 {
            fanout * 2 - 1
        } else {
            fanout
        };
        (0..groups)
            .flat_map(|x| {
                (0..d).map(move |j| {
                    let y = if shape == "triangle" {
                        (x + j) % groups
                    } else {
                        r * 10000 + j
                    };
                    [
                        x as u64,
                        y as u64,
                        1,
                        ((x * 17 + j * 13 + r * 7) % bank) as u64,
                    ]
                })
            })
            .collect()
    })
}

pub fn oracle(
    data: &[Vec<Row>; 3],
    shape: &str,
    bank: &[Vec<u64>],
    groups: usize,
) -> (Vec<Vec<u64>>, usize) {
    let mut out = vec![vec![0; bank[0].len()]; groups];
    let mut count = 0;
    for a in &data[0] {
        for b in &data[1] {
            if (if shape == "triangle" { a[1] } else { a[0] }) != b[0] {
                continue;
            }
            for c in &data[2] {
                if c[0] != a[0] || (shape == "triangle" && c[1] != b[1]) {
                    continue;
                }
                count += 1;
                for (i, w) in out[a[0] as usize].iter_mut().enumerate() {
                    *w |=
                        bank[a[3] as usize][i] & !(bank[b[3] as usize][i] | bank[c[3] as usize][i]);
                }
            }
        }
    }
    (out, count)
}
