//! Canonical completed-function carrier: sixteen-byte records, essential local
//! tables, signed references and ordered symbolic splits. Only checked owned
//! values can publish references. Hashes select candidates, never equality.
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::{BoolOp4, Capacity, Control, Error, Limits, Result};

pub(crate) type Ref = u32;
const TABLE: u64 = 1 << 63;
const CHILD_MASK: u64 = (1 << 28) - 1;
const END: Ref = Ref::MAX;
pub(crate) const LOCAL_COORDINATES: u32 = 9;
pub(crate) const MAX_COORDINATES: u8 = 62;
const MAX_EXPLICIT_COORDINATES: u32 = 20;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct Record {
    variables: u64,
    payload: u64,
}
const _: () = assert!(std::mem::size_of::<Record>() == 16);

#[derive(Clone, Copy)]
pub(crate) enum View<'a> {
    Constant(bool),
    Table {
        variables: u64,
        words: &'a [u64],
        complemented: bool,
    },
    Split {
        variable: u8,
        low: Ref,
        high: Ref,
    },
}

#[derive(Debug)]
pub(crate) struct Arena {
    order: Vec<u8>,
    records: Vec<Record>,
    words: Vec<u64>,
    links: Vec<Ref>,
    index: HashMap<u64, Ref>,
    limits: Limits,
    fingerprint_mask: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Memo {
    Ite(Ref, Ref, Ref),
    Cofactor(Ref, u8, bool),
    Exists(Ref, u64),
}

pub(crate) struct Operation<'a> {
    pub arena: &'a mut Arena,
    control: &'a dyn Control,
    steps: usize,
    memo: HashMap<Memo, Ref>,
    counts: HashMap<Ref, u64>,
}

pub(crate) fn scatter(mut index: usize, mut variables: u64) -> u64 {
    let mut world = 0;
    while variables != 0 {
        world |= (index as u64 & 1) << variables.trailing_zeros();
        variables &= variables - 1;
        index >>= 1;
    }
    world
}

fn gather(world: u64, mut variables: u64) -> usize {
    let mut index = 0;
    let mut bit = 0;
    while variables != 0 {
        index |= usize::from(world & (1 << variables.trailing_zeros()) != 0) << bit;
        variables &= variables - 1;
        bit += 1;
    }
    index
}

fn cell(words: &[u64], index: usize) -> bool {
    words[index / 64] >> (index % 64) & 1 != 0
}

fn put(words: &mut [u64], index: usize, value: bool) {
    if value {
        words[index / 64] |= 1 << (index % 64);
    }
}

fn tail_mask(coordinates: u32) -> u64 {
    if coordinates < 6 {
        (1 << (1 << coordinates)) - 1
    } else {
        u64::MAX
    }
}

impl Arena {
    pub fn new(order: &[u8], limits: Limits) -> Result<Self> {
        if order.len() > usize::from(MAX_COORDINATES) {
            return Err(Error::Capacity(Capacity::Coordinates));
        }
        let mut seen = 0u64;
        for &v in order {
            if usize::from(v) >= order.len() || seen & (1 << v) != 0 {
                return Err(Error::InvalidOrder);
            }
            seen |= 1 << v;
        }
        if limits.records == 0 {
            return Err(Error::Capacity(Capacity::Records));
        }
        let mut owned_order = Vec::new();
        owned_order.try_reserve_exact(order.len())?;
        owned_order.extend_from_slice(order);
        let mut records = Vec::new();
        records.try_reserve(1)?;
        records.push(Record {
            variables: 0,
            payload: 0,
        });
        let mut links = Vec::new();
        links.try_reserve(1)?;
        links.push(END);
        Ok(Self {
            order: owned_order,
            records,
            words: Vec::new(),
            links,
            index: HashMap::new(),
            limits,
            fingerprint_mask: u64::MAX,
        })
    }

    pub fn dimensions(&self) -> usize {
        self.order.len()
    }
    pub fn order(&self) -> &[u8] {
        &self.order
    }
    pub fn mask(&self) -> u64 {
        (1 << self.dimensions()) - 1
    }
    pub fn variables(&self, root: Ref) -> u64 {
        self.records[(root / 2) as usize].variables
    }
    pub fn records(&self) -> usize {
        self.records.len()
    }
    pub fn words(&self) -> usize {
        self.words.len()
    }
    pub fn retained_bytes(&self) -> usize {
        self.records.capacity() * size_of::<Record>()
            + self.words.capacity() * 8
            + self.links.capacity() * size_of::<Ref>()
            + self.index.capacity() * (size_of::<(u64, Ref)>() + 1)
            + self.order.capacity()
    }

    fn top(&self, variables: u64) -> u8 {
        *self
            .order
            .iter()
            .find(|&&v| variables & (1 << v) != 0)
            .expect("nonconstant canonical function has a coordinate")
    }

    pub fn view(&self, root: Ref) -> View<'_> {
        if root < 2 {
            return View::Constant(root != 0);
        }
        let record = self.records[(root / 2) as usize];
        if record.payload & TABLE != 0 {
            let start = (record.payload & !TABLE) as usize;
            let len = (1usize << record.variables.count_ones()).div_ceil(64);
            View::Table {
                variables: record.variables,
                words: &self.words[start..start + len],
                complemented: root & 1 != 0,
            }
        } else {
            View::Split {
                variable: u8::try_from((record.payload >> 56) & 63).expect("six-bit coordinate"),
                low: u32::try_from(record.payload & CHILD_MASK).expect("28-bit reference")
                    ^ (root & 1),
                high: u32::try_from((record.payload >> 28) & CHILD_MASK).expect("28-bit reference")
                    ^ (root & 1),
            }
        }
    }

    pub fn evaluate(&self, mut root: Ref, world: u64) -> bool {
        loop {
            match self.view(root) {
                View::Constant(value) => return value,
                View::Table {
                    variables,
                    words,
                    complemented,
                } => return cell(words, gather(world, variables)) ^ complemented,
                View::Split {
                    variable,
                    low,
                    high,
                } => {
                    root = if world & (1 << variable) == 0 {
                        low
                    } else {
                        high
                    }
                }
            }
        }
    }

    pub fn witness(&self, mut root: Ref) -> Option<u64> {
        let mut world = 0;
        loop {
            match self.view(root) {
                View::Constant(value) => return value.then_some(world),
                View::Table {
                    variables,
                    words,
                    complemented,
                } => {
                    let index = (0..(1usize << variables.count_ones()))
                        .find(|&i| cell(words, i) ^ complemented)?;
                    return Some(world | scatter(index, variables));
                }
                View::Split {
                    variable,
                    low,
                    high,
                } => {
                    if low != 0 {
                        root = low;
                    } else {
                        world |= 1 << variable;
                        root = high;
                    }
                }
            }
        }
    }

    #[cfg(test)]
    pub fn collide_all(&mut self) {
        assert_eq!(self.records.len(), 1);
        self.fingerprint_mask = 0;
    }
}

impl<'a> Operation<'a> {
    pub fn new(arena: &'a mut Arena, control: &'a dyn Control) -> Result<Self> {
        control.checkpoint()?;
        Ok(Self {
            arena,
            control,
            steps: 0,
            memo: HashMap::new(),
            counts: HashMap::new(),
        })
    }

    pub(crate) fn step(&mut self) -> Result<()> {
        self.control.checkpoint()?;
        if self.steps >= self.arena.limits.operation_steps {
            return Err(Error::Capacity(Capacity::OperationSteps));
        }
        self.steps += 1;
        Ok(())
    }

    fn remember(&mut self, key: Memo, root: Ref) -> Result<Ref> {
        if self.memo.len() >= self.arena.limits.memo_entries {
            return Err(Error::Capacity(Capacity::MemoEntries));
        }
        self.memo.try_reserve(1)?;
        self.memo.insert(key, root);
        Ok(root)
    }

    pub(crate) fn limits(&self) -> Limits {
        self.arena.limits
    }

    fn intern(&mut self, variables: u64, payload: u64, words: Option<&[u64]>) -> Result<Ref> {
        self.step()?;
        let mut hasher = DefaultHasher::new();
        variables.hash(&mut hasher);
        if let Some(data) = words {
            data.hash(&mut hasher);
        } else {
            payload.hash(&mut hasher);
        }
        let fingerprint = hasher.finish() & self.arena.fingerprint_mask;
        let mut at = self.arena.index.get(&fingerprint).copied().unwrap_or(END);
        while at != END {
            self.step()?;
            let record = self.arena.records[(at / 2) as usize];
            if record.variables == variables {
                let equal = if let Some(data) = words {
                    matches!(self.arena.view(at), View::Table { words: old, .. } if old == data)
                } else {
                    record.payload == payload
                };
                if equal {
                    return Ok(at);
                }
            }
            at = self.arena.links[(at / 2) as usize];
        }
        let len = self.arena.records.len();
        if len >= self.arena.limits.records || len >= (1 << 27) {
            return Err(Error::Capacity(Capacity::Records));
        }
        let added = words.map_or(0, <[u64]>::len);
        let end = self
            .arena
            .words
            .len()
            .checked_add(added)
            .ok_or(Error::Capacity(Capacity::TableWords))?;
        if end > self.arena.limits.table_words || end >= (1usize << 63) {
            return Err(Error::Capacity(Capacity::TableWords));
        }
        // Reserve all correlated columns before changing any logical state.
        self.arena.records.try_reserve(1)?;
        self.arena.links.try_reserve(1)?;
        self.arena.words.try_reserve(added)?;
        self.arena.index.try_reserve(1)?;
        let root = u32::try_from(len * 2).map_err(|_| Error::Capacity(Capacity::Records))?;
        let payload = words.map_or(payload, |_| TABLE | self.arena.words.len() as u64);
        if let Some(data) = words {
            self.arena.words.extend_from_slice(data);
        }
        let previous = self.arena.index.insert(fingerprint, root).unwrap_or(END);
        self.arena.records.push(Record { variables, payload });
        self.arena.links.push(previous);
        Ok(root)
    }

    fn table(&mut self, mut variables: u64, mut data: [u64; 8]) -> Result<Ref> {
        self.step()?;
        let mut count = variables.count_ones();
        debug_assert!(count <= LOCAL_COORDINATES);
        data[0] &= tail_mask(count);
        let mut position = 0;
        while position < count {
            let lower = (1usize << position) - 1;
            let independent = (0..(1usize << (count - 1))).all(|i| {
                let low = (i & lower) | ((i >> position) << (position + 1));
                cell(&data, low) == cell(&data, low | (1 << position))
            });
            if independent {
                let mut reduced = [0; 8];
                for i in 0..(1usize << (count - 1)) {
                    let low = (i & lower) | ((i >> position) << (position + 1));
                    put(&mut reduced, i, cell(&data, low));
                }
                let coordinate = scatter(1 << position, variables);
                variables &= !coordinate;
                count -= 1;
                data = reduced;
            } else {
                position += 1;
            }
        }
        if count == 0 {
            return Ok(u32::from(data[0] & 1 != 0));
        }
        let polarity = data[0] & 1 != 0;
        let len = (1usize << count).div_ceil(64);
        if polarity {
            for word in &mut data[..len] {
                *word = !*word;
            }
        }
        data[0] &= tail_mask(count);
        Ok(self.intern(variables, 0, Some(&data[..len]))? ^ u32::from(polarity))
    }

    pub fn variable(&mut self, coordinate: u8) -> Result<Ref> {
        if usize::from(coordinate) >= self.arena.dimensions() {
            return Err(Error::InvalidCoordinate(coordinate));
        }
        let mut data = [0; 8];
        data[0] = 2;
        self.table(1 << coordinate, data)
    }

    fn split(&mut self, variable: u8, low: Ref, high: Ref) -> Result<Ref> {
        if low == high {
            return Ok(low);
        }
        let variables = self.arena.variables(low) | self.arena.variables(high) | (1 << variable);
        if variables.count_ones() <= LOCAL_COORDINATES {
            let mut data = [0; 8];
            for i in 0..(1usize << variables.count_ones()) {
                let world = scatter(i, variables);
                let root = if world & (1 << variable) == 0 {
                    low
                } else {
                    high
                };
                put(&mut data, i, self.arena.evaluate(root, world));
            }
            return self.table(variables, data);
        }
        debug_assert_eq!(self.arena.top(variables), variable);
        let polarity = low & 1;
        let payload = (u64::from(variable) << 56)
            | (u64::from(high ^ polarity) << 28)
            | u64::from(low ^ polarity);
        Ok(self.intern(variables, payload, None)? ^ polarity)
    }

    fn aligned(&self, root: Ref, variables: u64) -> [u64; 8] {
        let mut data = [0; 8];
        for i in 0..(1usize << variables.count_ones()) {
            put(
                &mut data,
                i,
                self.arena.evaluate(root, scatter(i, variables)),
            );
        }
        data
    }

    pub fn apply(&mut self, op: BoolOp4, a: Ref, b: Ref) -> Result<Ref> {
        self.step()?;
        let variables = self.arena.variables(a) | self.arena.variables(b);
        if variables.count_ones() <= LOCAL_COORDINATES {
            let a = self.aligned(a, variables);
            let b = self.aligned(b, variables);
            let mut data = [0; 8];
            for i in 0..(1usize << variables.count_ones()).div_ceil(64) {
                data[i] = op.word(a[i], b[i]);
            }
            return self.table(variables, data);
        }
        let unary = |bits| match bits {
            0 => 0,
            1 => b ^ 1,
            2 => b,
            _ => 1,
        };
        self.ite(a, unary(op.bits() >> 2), unary(op.bits() & 3))
    }

    pub fn ite(&mut self, mut condition: Ref, mut high: Ref, mut low: Ref) -> Result<Ref> {
        self.step()?;
        if condition == 0 {
            return Ok(low);
        }
        if condition == 1 || high == low {
            return Ok(high);
        }
        if (high, low) == (1, 0) {
            return Ok(condition);
        }
        if (high, low) == (0, 1) {
            return Ok(condition ^ 1);
        }
        if condition & 1 != 0 {
            condition ^= 1;
            std::mem::swap(&mut high, &mut low);
        }
        let polarity = low & 1;
        high ^= polarity;
        low ^= polarity;
        let key = Memo::Ite(condition, high, low);
        if let Some(&root) = self.memo.get(&key) {
            return Ok(root ^ polarity);
        }
        let variables = self.arena.variables(condition)
            | self.arena.variables(high)
            | self.arena.variables(low);
        let root = if variables.count_ones() <= LOCAL_COORDINATES {
            let c = self.aligned(condition, variables);
            let h = self.aligned(high, variables);
            let l = self.aligned(low, variables);
            let mut data = [0; 8];
            for i in 0..(1usize << variables.count_ones()).div_ceil(64) {
                data[i] = (c[i] & h[i]) | (!c[i] & l[i]);
            }
            self.table(variables, data)?
        } else {
            let v = self.arena.top(variables);
            let cl = self.cofactor(condition, v, false)?;
            let ch = self.cofactor(condition, v, true)?;
            let hl = self.cofactor(high, v, false)?;
            let hh = self.cofactor(high, v, true)?;
            let ll = self.cofactor(low, v, false)?;
            let lh = self.cofactor(low, v, true)?;
            let left = self.ite(cl, hl, ll)?;
            let right = self.ite(ch, hh, lh)?;
            self.split(v, left, right)?
        };
        Ok(self.remember(key, root)? ^ polarity)
    }

    pub fn cofactor(&mut self, root: Ref, variable: u8, high: bool) -> Result<Ref> {
        self.step()?;
        if self.arena.variables(root) & (1 << variable) == 0 {
            return Ok(root);
        }
        let key = Memo::Cofactor(root, variable, high);
        if let Some(&out) = self.memo.get(&key) {
            return Ok(out);
        }
        let out = match self.arena.view(root) {
            View::Constant(_) => unreachable!("constant has no essential coordinates"),
            View::Table { variables, .. } => {
                let remaining = variables & !(1 << variable);
                let mut data = [0; 8];
                for i in 0..(1usize << remaining.count_ones()) {
                    let world = scatter(i, remaining) | (u64::from(high) << variable);
                    put(&mut data, i, self.arena.evaluate(root, world));
                }
                self.table(remaining, data)?
            }
            View::Split {
                variable: v,
                low,
                high: upper,
            } => {
                if v == variable {
                    if high { upper } else { low }
                } else {
                    let l = self.cofactor(low, variable, high)?;
                    let h = self.cofactor(upper, variable, high)?;
                    self.split(v, l, h)?
                }
            }
        };
        self.remember(key, out)
    }

    pub fn exists(&mut self, root: Ref, hidden: u64) -> Result<Ref> {
        self.step()?;
        let hidden = hidden & self.arena.variables(root);
        if hidden == 0 {
            return Ok(root);
        }
        let key = Memo::Exists(root, hidden);
        if let Some(&out) = self.memo.get(&key) {
            return Ok(out);
        }
        let v = self.arena.top(self.arena.variables(root));
        let low = self.cofactor(root, v, false)?;
        let high = self.cofactor(root, v, true)?;
        let low = self.exists(low, hidden)?;
        let high = self.exists(high, hidden)?;
        let out = if hidden & (1 << v) != 0 {
            self.apply(BoolOp4::OR, low, high)?
        } else {
            self.split(v, low, high)?
        };
        self.remember(key, out)
    }

    pub fn import_table(&mut self, variables: u64, words: &[u64]) -> Result<Ref> {
        self.step()?;
        let count = variables.count_ones();
        if count > MAX_EXPLICIT_COORDINATES {
            return Err(Error::Capacity(Capacity::ExplicitTable));
        }
        if variables & !self.arena.mask() != 0
            || words.len() != (1usize << count).div_ceil(64)
            || words[0] & !tail_mask(count) != 0
        {
            return Err(Error::InvalidTable);
        }
        if count <= LOCAL_COORDINATES {
            let mut data = [0; 8];
            data[..words.len()].copy_from_slice(words);
            return self.table(variables, data);
        }
        let v = self.arena.top(variables);
        let remaining = variables & !(1 << v);
        let position = (variables & ((1 << v) - 1)).count_ones();
        let lower = (1usize << position) - 1;
        let mut halves = [Vec::new(), Vec::new()];
        for (bit, half) in halves.iter_mut().enumerate() {
            half.try_reserve_exact((1usize << (count - 1)).div_ceil(64))?;
            half.resize((1usize << (count - 1)).div_ceil(64), 0);
            for i in 0..(1usize << (count - 1)) {
                if i.is_multiple_of(512) {
                    self.step()?;
                }
                let source = (i & lower) | (bit << position) | ((i >> position) << (position + 1));
                put(half, i, cell(words, source));
            }
        }
        let low = self.import_table(remaining, &halves[0])?;
        let high = self.import_table(remaining, &halves[1])?;
        self.split(v, low, high)
    }

    // Count assignments on exactly the root's raw essential coordinates.
    // The public caller intersects original support and smooths missing axes.
    pub fn count(&mut self, root: Ref) -> Result<u64> {
        self.step()?;
        if let Some(&count) = self.counts.get(&root) {
            return Ok(count);
        }
        let count = match self.arena.view(root) {
            View::Constant(value) => u64::from(value),
            View::Table {
                variables,
                words,
                complemented,
            } => {
                let positive: u64 = words.iter().map(|w| u64::from(w.count_ones())).sum();
                if complemented {
                    (1 << variables.count_ones()) - positive
                } else {
                    positive
                }
            }
            View::Split {
                variable,
                low,
                high,
            } => {
                let rest = self.arena.variables(root) & !(1 << variable);
                let a = self.count(low)? << (rest & !self.arena.variables(low)).count_ones();
                let b = self.count(high)? << (rest & !self.arena.variables(high)).count_ones();
                a + b
            }
        };
        if self.counts.len() >= self.arena.limits.memo_entries {
            return Err(Error::Capacity(Capacity::MemoEntries));
        }
        self.counts.try_reserve(1)?;
        self.counts.insert(root, count);
        Ok(count)
    }

    /// Import a canonical denotation, rebuilding with this arena's own order.
    /// The caller binds the memo to this one source arena for the whole import.
    pub fn transfer(
        &mut self,
        source: &Arena,
        root: Ref,
        memo: &mut HashMap<Ref, Ref>,
    ) -> Result<Ref> {
        self.step()?;
        if root < 2 {
            return Ok(root);
        }
        let regular = root & !1;
        if let Some(&result) = memo.get(&regular) {
            return Ok(result ^ (root & 1));
        }
        let result = match source.view(regular) {
            View::Constant(_) => unreachable!("nonconstant reference"),
            View::Table {
                variables, words, ..
            } => self.import_table(variables, words)?,
            View::Split {
                variable,
                low,
                high,
            } => {
                let low = self.transfer(source, low, memo)?;
                let high = self.transfer(source, high, memo)?;
                let condition = self.variable(variable)?;
                self.ite(condition, high, low)?
            }
        };
        if memo.len() >= self.limits().memo_entries {
            return Err(Error::Capacity(Capacity::MemoEntries));
        }
        memo.try_reserve(1)?;
        memo.insert(regular, result);
        Ok(result ^ (root & 1))
    }

    /// Simultaneous Boolean substitution. A missing source means this arena;
    /// node snapshots let resident substitution append without aliasing a view.
    /// The caller scopes `memo` to one source and one replacement vector.
    pub fn substitute(
        &mut self,
        source: Option<&Arena>,
        root: Ref,
        replacements: &[Ref],
        memo: &mut HashMap<Ref, Ref>,
    ) -> Result<Ref> {
        self.step()?;
        if root < 2 {
            return Ok(root);
        }
        let regular = root & !1;
        if let Some(&result) = memo.get(&regular) {
            return Ok(result ^ (root & 1));
        }
        let result = match source.unwrap_or(self.arena).view(regular) {
            View::Constant(_) => unreachable!("nonconstant reference"),
            View::Table {
                variables, words, ..
            } => {
                let mut data = [0; 8];
                data[..words.len()].copy_from_slice(words);
                self.substitute_table(variables, &data, 0, 0, replacements)?
            }
            View::Split {
                variable,
                low,
                high,
            } => {
                let low = self.substitute(source, low, replacements, memo)?;
                let high = self.substitute(source, high, replacements, memo)?;
                self.ite(replacements[usize::from(variable)], high, low)?
            }
        };
        if memo.len() >= self.limits().memo_entries {
            return Err(Error::Capacity(Capacity::MemoEntries));
        }
        memo.try_reserve(1)?;
        memo.insert(regular, result);
        Ok(result ^ (root & 1))
    }

    fn substitute_table(
        &mut self,
        variables: u64,
        data: &[u64; 8],
        position: u32,
        index: usize,
        replacements: &[Ref],
    ) -> Result<Ref> {
        self.step()?;
        if variables == 0 {
            return Ok(Ref::from(cell(data, index)));
        }
        let variable = variables.trailing_zeros() as usize;
        let remaining = variables & (variables - 1);
        let low = self.substitute_table(remaining, data, position + 1, index, replacements)?;
        let high = self.substitute_table(
            remaining,
            data,
            position + 1,
            index | (1 << position),
            replacements,
        )?;
        self.ite(replacements[variable], high, low)
    }
}
