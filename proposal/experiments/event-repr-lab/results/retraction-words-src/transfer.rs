//! Structural transport, exact support maps and retained owners.
//! Packet bytes are NOT representation-independent canonical fact bytes.
use super::carrier::*;
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

pub type Ref = u32; // 0=false, 1=true; other references have a complement bit.
const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_NODES: usize = 2_000_000;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Node {
    // Coordinates are local little-endian axes. Children exhaust all assignments.
    Split {
        coords: Vec<u32>,
        children: Vec<Ref>,
    },
    Table {
        coords: Vec<u32>,
        words: Vec<u64>,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packet {
    pub names: Vec<u64>,
    pub order: Vec<u32>,
    pub nodes: Vec<Node>,
    pub support: Ref,
    pub roots: Vec<Ref>,
}
pub struct Builder {
    names: Vec<u64>,
    order: Vec<u32>,
    nodes: Vec<Node>,
    unique: FxHashMap<Node, Ref>,
}
impl Builder {
    pub fn new(names: Vec<u64>, order: Vec<u32>) -> Self {
        Self {
            names,
            order,
            nodes: vec![],
            unique: FxHashMap::default(),
        }
    }
    fn intern(&mut self, node: Node) -> Ref {
        if let Some(&r) = self.unique.get(&node) {
            return r;
        }
        assert!(self.nodes.len() < MAX_NODES, "RESOURCE_CAP: transfer nodes");
        let r = (self.nodes.len() as Ref + 1) * 2;
        self.nodes.push(node.clone());
        self.unique.insert(node, r);
        r
    }
    pub fn split(&mut self, coords: Vec<u32>, children: Vec<Ref>) -> Ref {
        assert_eq!(children.len(), 1usize << coords.len());
        if children.iter().all(|&r| r == children[0]) {
            return children[0];
        }
        self.intern(Node::Split { coords, children })
    }
    pub fn table(&mut self, coords: Vec<u32>, words: Vec<u64>) -> Ref {
        let cells = 1usize << coords.len();
        assert_eq!(words.len(), cells.div_ceil(64));
        if words.iter().all(|&w| w == 0) {
            return 0;
        }
        if words
            .iter()
            .enumerate()
            .all(|(i, &w)| w == cell_mask(cells, i))
        {
            return 1;
        }
        self.intern(Node::Table { coords, words })
    }
    pub fn finish(self, support: Ref, roots: Vec<Ref>) -> Packet {
        let p = Packet {
            names: self.names,
            order: self.order,
            nodes: self.nodes,
            support,
            roots,
        };
        p.validate()
            .expect("carrier emitted an invalid structural view");
        p
    }
}
fn cell_mask(cells: usize, word: usize) -> u64 {
    let n = cells.saturating_sub(word * 64).min(64);
    if n == 64 { !0 } else { (1u64 << n) - 1 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Format,
    Limit,
    Coordinate,
    Order,
    Reference,
    Shape,
    Map,
    EmptySupport,
    OutsideSource,
    NotTotal,
    Scope,
    Unpublished,
}
impl Packet {
    pub fn validate(&self) -> Result<(), Error> {
        let n = self.names.len();
        if n >= 63 || self.nodes.len() > MAX_NODES || self.roots.len() > MAX_NODES {
            return Err(Error::Limit);
        }
        if self.names.iter().collect::<FxHashSet<_>>().len() != n || self.order.len() != n {
            return Err(Error::Coordinate);
        }
        let mut rank = vec![usize::MAX; n];
        for (i, &v) in self.order.iter().enumerate() {
            if v as usize >= n || rank[v as usize] != usize::MAX {
                return Err(Error::Coordinate);
            }
            rank[v as usize] = i;
        }
        let mut tops = vec![usize::MAX];
        for (index, node) in self.nodes.iter().enumerate() {
            let coords = match node {
                Node::Split { coords, .. } | Node::Table { coords, .. } => coords,
            };
            if coords.len() > 20 {
                return Err(Error::Limit);
            }
            let mut previous = None;
            for &c in coords.iter().rev() {
                if c as usize >= n {
                    return Err(Error::Coordinate);
                }
                let r = rank[c as usize];
                if previous.is_some_and(|p| p >= r) {
                    return Err(Error::Order);
                }
                previous = Some(r);
            }
            let last = previous;
            let cells = 1usize << coords.len();
            match node {
                Node::Split { children, .. } => {
                    if coords.is_empty() || coords.len() > 6 || children.len() != cells {
                        return Err(Error::Shape);
                    }
                    for &r in children {
                        if r as usize / 2 > index {
                            return Err(Error::Reference);
                        }
                        if last.unwrap() >= tops[r as usize / 2] {
                            return Err(Error::Order);
                        }
                    }
                }
                Node::Table { words, .. } => {
                    if words.len() != cells.div_ceil(64)
                        || words
                            .iter()
                            .enumerate()
                            .any(|(i, &w)| w & !cell_mask(cells, i) != 0)
                    {
                        return Err(Error::Shape);
                    }
                }
            }
            tops.push(coords.last().map_or(usize::MAX, |&c| rank[c as usize]));
        }
        if std::iter::once(&self.support)
            .chain(&self.roots)
            .any(|&r| r as usize / 2 > self.nodes.len())
        {
            return Err(Error::Reference);
        }
        Ok(())
    }
    pub fn encode(&self) -> Vec<u8> {
        self.validate().unwrap();
        let mut out = b"EVTP\x01".to_vec();
        put32(&mut out, self.names.len() as u32);
        for &name in &self.names {
            out.extend(name.to_be_bytes());
        }
        for &axis in &self.order {
            put32(&mut out, axis);
        }
        put32(&mut out, self.nodes.len() as u32);
        for node in &self.nodes {
            let (tag, coords) = match node {
                Node::Split { coords, .. } => (0, coords),
                Node::Table { coords, .. } => (1, coords),
            };
            out.push(tag);
            out.push(coords.len() as u8);
            for &c in coords {
                put32(&mut out, c);
            }
            match node {
                Node::Split { children, .. } => {
                    for &r in children {
                        put32(&mut out, r);
                    }
                }
                Node::Table { words, .. } => {
                    for &w in words {
                        out.extend(w.to_be_bytes());
                    }
                }
            }
        }
        put32(&mut out, self.support);
        put32(&mut out, self.roots.len() as u32);
        for &r in &self.roots {
            put32(&mut out, r);
        }
        assert!(out.len() <= MAX_BYTES, "RESOURCE_CAP: transfer bytes");
        out
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_BYTES {
            return Err(Error::Limit);
        }
        let mut r = Reader { bytes, at: 0 };
        if r.take(5)? != b"EVTP\x01" {
            return Err(Error::Format);
        }
        let n = r.word()? as usize;
        if n >= 63 {
            return Err(Error::Limit);
        }
        let names = (0..n).map(|_| r.long()).collect::<Result<_, _>>()?;
        let order = (0..n).map(|_| r.word()).collect::<Result<_, _>>()?;
        let count = r.word()? as usize;
        if count > MAX_NODES {
            return Err(Error::Limit);
        }
        let mut nodes = vec![];
        for _ in 0..count {
            let tag = r.take(1)?[0];
            let k = r.take(1)?[0] as usize;
            if k > 20 || (tag == 0 && k > 6) {
                return Err(Error::Limit);
            }
            let coords = (0..k).map(|_| r.word()).collect::<Result<_, _>>()?;
            let node = match tag {
                0 => Node::Split {
                    coords,
                    children: (0..1usize << k)
                        .map(|_| r.word())
                        .collect::<Result<_, _>>()?,
                },
                1 => Node::Table {
                    coords,
                    words: (0..(1usize << k).div_ceil(64))
                        .map(|_| r.long())
                        .collect::<Result<_, _>>()?,
                },
                _ => return Err(Error::Format),
            };
            nodes.push(node);
        }
        let support = r.word()?;
        let count = r.word()? as usize;
        if count > MAX_NODES {
            return Err(Error::Limit);
        }
        let roots = (0..count).map(|_| r.word()).collect::<Result<_, _>>()?;
        if r.at != bytes.len() {
            return Err(Error::Format);
        }
        let p = Self {
            names,
            order,
            nodes,
            support,
            roots,
        };
        p.validate()?;
        Ok(p)
    }
}
fn put32(out: &mut Vec<u8>, v: u32) {
    out.extend(v.to_be_bytes());
}
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.at.checked_add(n).ok_or(Error::Limit)?;
        let v = self.bytes.get(self.at..end).ok_or(Error::Format)?;
        self.at = end;
        Ok(v)
    }
    fn word(&mut self) -> Result<u32, Error> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn long(&mut self) -> Result<u64, Error> {
        Ok(u64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }
}

fn select<C: Carrier>(c: &mut C, x: Id, lo: Id, hi: Id) -> Id {
    if lo == hi {
        return lo;
    }
    let yes = c.op(8, x, hi);
    let no = c.op(4, lo, x);
    c.op(14, yes, no)
}
struct Import<'a, C: Carrier> {
    packet: &'a Packet,
    map: &'a [Option<u32>],
    c: &'a mut C,
    memo: FxHashMap<Ref, Id>,
    literals: FxHashMap<u32, Id>,
    table_mode: TableImport,
}
impl<C: Carrier> Import<'_, C> {
    fn variable(&mut self, k: u32) -> Id {
        if let Some(&r) = self.literals.get(&k) {
            return r;
        }
        let r = self.c.variable(k);
        self.literals.insert(k, r);
        r
    }
    fn combine(&mut self, coordinate: u32, lo: Id, hi: Id) -> Id {
        if let Some(k) = self.map[coordinate as usize] {
            let v = self.variable(k);
            select(self.c, v, lo, hi)
        } else {
            self.c.op(14, lo, hi)
        }
    }
    fn table(&mut self, coords: &[u32], words: &[u64], base: usize, flip: u32) -> Id {
        if coords.is_empty() {
            return if ((words[base / 64] >> (base % 64)) as u32 ^ flip) & 1 == 0 {
                self.c.empty()
            } else {
                self.c.full()
            };
        }
        let k = coords.len() - 1;
        let lo = self.table(&coords[..k], words, base, flip);
        let hi = self.table(&coords[..k], words, base | (1usize << k), flip);
        self.combine(coords[k], lo, hi)
    }
    fn branches(&mut self, coords: &[u32], children: &[Ref], base: usize, flip: u32) -> Id {
        if coords.is_empty() {
            return self.eval(children[base] ^ flip);
        }
        let k = coords.len() - 1;
        let lo = self.branches(&coords[..k], children, base, flip);
        let hi = self.branches(&coords[..k], children, base | (1usize << k), flip);
        self.combine(coords[k], lo, hi)
    }
    fn eval(&mut self, r: Ref) -> Id {
        if r < 2 {
            return if r == 0 {
                self.c.empty()
            } else {
                self.c.full()
            };
        }
        if let Some(&v) = self.memo.get(&r) {
            return v;
        }
        // Push polarity into cofactors BEFORE elimination: exists(!E) != !exists(E).
        let node = &self.packet.nodes[(r / 2 - 1) as usize];
        let v = match node {
            Node::Split { coords, children } => self.branches(coords, children, 0, r & 1),
            Node::Table { coords, words } => {
                // Direct substitution is safe only when every local axis survives.
                // Projected complements still use the cofactor path above.
                let direct = if self.table_mode != TableImport::Recursive {
                    coords
                        .iter()
                        .map(|&k| self.map[k as usize])
                        .collect::<Option<Vec<_>>>()
                        .and_then(|axes| match self.table_mode {
                            TableImport::Words => self.c.import_table_words(&axes, words),
                            _ => self.c.import_table(&axes, words),
                        })
                } else {
                    None
                };
                if let Some(id) = direct {
                    if r & 1 == 0 { id } else { self.c.not(id) }
                } else {
                    self.table(coords, words, 0, r & 1)
                }
            }
        };
        self.memo.insert(r, v);
        v
    }
}
fn evaluate<C: Carrier>(p: &Packet, r: Ref, map: &[Option<u32>], c: &mut C) -> Id {
    evaluate_mode(p, r, map, c, table_mode())
}
pub(super) fn evaluate_mode<C: Carrier>(
    p: &Packet,
    r: Ref,
    map: &[Option<u32>],
    c: &mut C,
    mode: TableImport,
) -> Id {
    Import {
        packet: p,
        map,
        c,
        memo: FxHashMap::default(),
        literals: FxHashMap::default(),
        table_mode: mode,
    }
    .eval(r)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableImport {
    Recursive,
    Direct,
    Words,
}
fn table_mode() -> TableImport {
    match std::env::var("EVENT_LAB_TRANSFER_IMPORT").as_deref() {
        Ok("recursive") => TableImport::Recursive,
        Ok("direct") => TableImport::Direct,
        Ok("words") | Err(std::env::VarError::NotPresent) => TableImport::Words,
        _ => panic!("invalid EVENT_LAB_TRANSFER_IMPORT"),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapKind {
    Extension,
    Restriction,
}
// Validate in a fresh full-product verifier so target support cannot hide lost worlds.
pub fn check_map<V: Carrier>(
    source: &Packet,
    target: &Packet,
    images: &[u32],
    kind: MapKind,
) -> Result<(), Error> {
    source.validate()?;
    target.validate()?;
    if images.len() != source.names.len() {
        return Err(Error::Map);
    }
    let mut inverse = vec![None; target.names.len()];
    for (s, &t) in images.iter().enumerate() {
        if t as usize >= inverse.len() || inverse[t as usize].is_some() {
            return Err(Error::Map);
        }
        inverse[t as usize] = Some(s as u32);
    }
    let mut verifier =
        V::full_space(source.names.len() as u32, source.order.clone()).map_err(|_| Error::Limit)?;
    let identity: Vec<_> = (0..source.names.len() as u32).map(Some).collect();
    let old = evaluate(source, source.support, &identity, &mut verifier);
    let projected = evaluate(target, target.support, &inverse, &mut verifier);
    if old == verifier.empty() || projected == verifier.empty() {
        return Err(Error::EmptySupport);
    }
    if verifier.op(4, projected, old) != verifier.empty() {
        return Err(Error::OutsideSource);
    }
    if kind == MapKind::Extension && projected != old {
        return Err(Error::NotTotal);
    }
    Ok(())
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventKey {
    pub space: u64,
    pub region: Id,
}
static NEXT_SPACE: AtomicU64 = AtomicU64::new(1);
struct State<C> {
    carrier: C,
    published: FxHashSet<Id>,
}
pub struct Space<C> {
    token: u64,
    names: Vec<u64>,
    state: Mutex<State<C>>,
}
pub struct Batch<C> {
    pub owner: Arc<Space<C>>,
    keys: Vec<EventKey>,
}
impl<C: Carrier> Space<C> {
    fn publication_class(id: Id) -> Id {
        if C::BIT_COMPLEMENT { id & !1 } else { id }
    }
    pub fn new(carrier: C, names: Vec<u64>) -> Arc<Self> {
        assert_eq!(names.len(), carrier.dimensions() as usize);
        let token = NEXT_SPACE
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .expect("scope token exhausted");
        Arc::new(Self {
            token,
            names,
            state: Mutex::new(State {
                carrier,
                published: FxHashSet::default(),
            }),
        })
    }
    // Lab-only publishing boundary: caller supplies handles constructed by this carrier.
    pub fn publish(self: &Arc<Self>, ids: &[Id]) -> Batch<C> {
        let mut state = self.state.lock().unwrap();
        state
            .published
            .extend(ids.iter().copied().map(Self::publication_class));
        Batch {
            owner: self.clone(),
            keys: ids
                .iter()
                .map(|&region| EventKey {
                    space: self.token,
                    region,
                })
                .collect(),
        }
    }
    pub fn resolve(&self, key: EventKey) -> Result<Id, Error> {
        if key.space != self.token {
            return Err(Error::Scope);
        }
        if !self
            .state
            .lock()
            .unwrap()
            .published
            .contains(&Self::publication_class(key.region))
        {
            return Err(Error::Unpublished);
        }
        Ok(key.region)
    }
    pub fn support_packet(&self) -> Packet {
        self.state.lock().unwrap().carrier.packet(&self.names, &[])
    }
    // Resolve all input keys before entering a trusted borrowed query. Even a
    // constant result must not bypass scope or publication checks. No returned
    // root is published by this inspection boundary.
    pub(super) fn inspect_inputs<R>(
        &self,
        inputs: &[EventKey],
        program: impl FnOnce(&mut C, u64) -> R,
    ) -> Result<R, Error> {
        let mut state = self.state.lock().unwrap();
        for key in inputs {
            if key.space != self.token {
                return Err(Error::Scope);
            }
            if !state
                .published
                .contains(&Self::publication_class(key.region))
            {
                return Err(Error::Unpublished);
            }
        }
        Ok(program(&mut state.carrier, self.token))
    }
    // Trusted lab program boundary. The closure must return roots constructed
    // through this borrowed arena, never raw IDs from another manager.
    pub(super) fn compute<R>(
        self: &Arc<Self>,
        program: impl FnOnce(&mut C) -> (Vec<Id>, R),
    ) -> (Batch<C>, R, usize) {
        let mut state = self.state.lock().unwrap();
        let (roots, result) = program(&mut state.carrier);
        let classes: FxHashSet<_> = roots.iter().copied().map(Self::publication_class).collect();
        let newly_published = classes
            .iter()
            .filter(|id| !state.published.contains(id))
            .count();
        state.published.extend(classes);
        let batch = Batch {
            owner: self.clone(),
            keys: roots
                .into_iter()
                .map(|region| EventKey {
                    space: self.token,
                    region,
                })
                .collect(),
        };
        (batch, result, newly_published)
    }
}
impl<C: Carrier> Batch<C> {
    pub fn complemented(&self) -> Self {
        let mut state = self.owner.state.lock().unwrap();
        let ids: Vec<_> = self
            .keys
            .iter()
            .map(|key| {
                assert_eq!(key.space, self.owner.token);
                assert!(
                    state
                        .published
                        .contains(&Space::<C>::publication_class(key.region))
                );
                if C::BIT_COMPLEMENT {
                    key.region ^ 1
                } else {
                    state.carrier.not(key.region)
                }
            })
            .collect();
        drop(state);
        self.owner.publish(&ids)
    }
    pub fn carrier_snapshot(&self) -> C {
        self.owner.state.lock().unwrap().carrier.clone()
    }
    pub fn retained_bytes(&self) -> usize {
        self.owner.state.lock().unwrap().carrier.bytes()
    }
    pub fn keys(&self) -> &[EventKey] {
        &self.keys
    }
    pub fn packet(&self) -> Packet {
        let roots: Vec<_> = self
            .keys
            .iter()
            .map(|&k| self.owner.resolve(k).unwrap())
            .collect();
        self.owner
            .state
            .lock()
            .unwrap()
            .carrier
            .packet(&self.owner.names, &roots)
    }
    pub fn export(&self) -> Vec<Vec<u64>> {
        let state = self.owner.state.lock().unwrap();
        self.keys
            .iter()
            .map(|k| state.carrier.export(k.region))
            .collect()
    }
    pub fn counts(&self) -> Vec<u64> {
        let state = self.owner.state.lock().unwrap();
        self.keys
            .iter()
            .map(|k| state.carrier.count(k.region))
            .collect()
    }
}
pub fn restore<V: Carrier, T: Carrier>(
    source: &Packet,
    target: &Arc<Space<T>>,
    images: &[u32],
    kind: MapKind,
) -> Result<Batch<T>, Error> {
    let target_packet = target.support_packet();
    check_map::<V>(source, &target_packet, images, kind)?;
    let map: Vec<_> = images.iter().copied().map(Some).collect();
    let mut state = target.state.lock().unwrap();
    let mut importer = Import {
        packet: source,
        map: &map,
        c: &mut state.carrier,
        memo: FxHashMap::default(),
        literals: FxHashMap::default(),
        table_mode: table_mode(),
    };
    let roots: Vec<_> = source.roots.iter().map(|&r| importer.eval(r)).collect();
    drop(state);
    Ok(target.publish(&roots))
}
