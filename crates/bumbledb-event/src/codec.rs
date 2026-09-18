//! Versioned, allocation/order/decoder-independent encoding of unmeasured
//! finite Event spaces. The two semantic roots are legal support and membership
//! intersected with support. Fixed semantic-order Shannon reduction determines
//! one forest; low-first postorder determines its wire indices.
use std::collections::{HashMap, HashSet};

use crate::arena::{Arena, MAX_COORDINATES, Operation, Ref};
use crate::{BoolOp4, Capacity, Control, Error, Limits, Result, SpaceId};

const VERSION: u8 = 1;
const HEADER: usize = 52;
const NODE_BYTES: usize = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Node {
    coordinate: u8,
    low: Ref,
    high: Ref,
}

struct Encoder<'a, 'b> {
    operation: &'a mut Operation<'b>,
    nodes: Vec<Node>,
    unique: HashMap<Node, Ref>,
    translated: HashMap<Ref, Ref>,
}

impl Encoder<'_, '_> {
    fn root(&mut self, root: Ref) -> Result<Ref> {
        self.operation.step()?;
        if root < 2 {
            return Ok(root);
        }
        let regular = root & !1;
        if let Some(&wire) = self.translated.get(&regular) {
            return Ok(wire ^ (root & 1));
        }
        // Semantic order is increasing coordinate ID, independent of resident
        // order and the size/decomposition of any resident local table.
        let coordinate = u8::try_from(self.operation.arena.variables(regular).trailing_zeros())
            .map_err(|_| Error::InvalidEncoding)?;
        let low = self.operation.cofactor(regular, coordinate, false)?;
        let high = self.operation.cofactor(regular, coordinate, true)?;
        let low = self.root(low)?;
        let high = self.root(high)?;
        let wire = if low == high {
            low
        } else {
            let polarity = low & 1;
            let node = Node {
                coordinate,
                low: low ^ polarity,
                high: high ^ polarity,
            };
            let reference = if let Some(&reference) = self.unique.get(&node) {
                reference
            } else {
                if self.nodes.len() >= self.operation.limits().records {
                    return Err(Error::Capacity(Capacity::Records));
                }
                let reference = u32::try_from(self.nodes.len() + 1)
                    .ok()
                    .and_then(|n| n.checked_mul(2))
                    .ok_or(Error::Capacity(Capacity::Records))?;
                self.nodes.try_reserve(1)?;
                self.unique.try_reserve(1)?;
                self.nodes.push(node);
                self.unique.insert(node, reference);
                reference
            };
            reference ^ polarity
        };
        if self.translated.len() >= self.operation.limits().memo_entries {
            return Err(Error::Capacity(Capacity::MemoEntries));
        }
        self.translated.try_reserve(1)?;
        self.translated.insert(regular, wire);
        Ok(wire ^ (root & 1))
    }
}

pub(crate) fn encode(
    identity: SpaceId,
    dimensions: u8,
    support: Ref,
    event: Ref,
    arena: &mut Arena,
    control: &dyn Control,
) -> Result<Vec<u8>> {
    let mut operation = Operation::new(arena, control)?;
    let membership = operation.apply(BoolOp4::AND, support, event)?;
    let mut encoder = Encoder {
        operation: &mut operation,
        nodes: Vec::new(),
        unique: HashMap::new(),
        translated: HashMap::new(),
    };
    let support = encoder.root(support)?;
    let membership = encoder.root(membership)?;
    let length = encoder
        .nodes
        .len()
        .checked_mul(NODE_BYTES)
        .and_then(|n| n.checked_add(HEADER))
        .ok_or(Error::Allocation)?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(length)?;
    bytes.extend_from_slice(b"BEVT");
    bytes.extend_from_slice(&[VERSION, dimensions, 0, 0]); // unmeasured; reserved
    bytes.extend_from_slice(&identity.0);
    let count =
        u32::try_from(encoder.nodes.len()).map_err(|_| Error::Capacity(Capacity::Records))?;
    bytes.extend_from_slice(&count.to_le_bytes());
    bytes.extend_from_slice(&support.to_le_bytes());
    bytes.extend_from_slice(&membership.to_le_bytes());
    for node in encoder.nodes {
        control.checkpoint()?;
        bytes.push(node.coordinate);
        bytes.extend_from_slice(&node.low.to_le_bytes());
        bytes.extend_from_slice(&node.high.to_le_bytes());
    }
    control.checkpoint()?;
    Ok(bytes)
}

pub(crate) struct Decoded {
    pub identity: SpaceId,
    pub dimensions: u8,
    pub support: Ref,
    pub event: Ref,
    pub arena: Arena,
}

fn word(bytes: &[u8], offset: usize) -> Result<u32> {
    let array = bytes
        .get(offset..offset + 4)
        .ok_or(Error::InvalidEncoding)?
        .try_into()
        .map_err(|_| Error::InvalidEncoding)?;
    Ok(u32::from_le_bytes(array))
}

fn check_postorder(
    root: Ref,
    nodes: &[Node],
    seen: &mut [bool],
    next: &mut usize,
    control: &dyn Control,
) -> Result<()> {
    control.checkpoint()?;
    if root < 2 {
        return Ok(());
    }
    let index = (root / 2 - 1) as usize;
    if seen[index] {
        return Ok(());
    }
    let node = nodes[index];
    check_postorder(node.low, nodes, seen, next, control)?;
    check_postorder(node.high, nodes, seen, next, control)?;
    if index != *next {
        return Err(Error::InvalidEncoding);
    }
    seen[index] = true;
    *next += 1;
    Ok(())
}

pub(crate) fn decode(
    bytes: &[u8],
    order: Option<&[u8]>,
    limits: Limits,
    control: &dyn Control,
) -> Result<Decoded> {
    control.checkpoint()?;
    if bytes.len() < HEADER || &bytes[..4] != b"BEVT" {
        return Err(Error::InvalidEncoding);
    }
    if bytes[4] != VERSION {
        return Err(Error::UnsupportedVersion(bytes[4]));
    }
    let dimensions = bytes[5];
    if dimensions > MAX_COORDINATES {
        return Err(Error::Capacity(Capacity::Coordinates));
    }
    if bytes[6..8] != [0, 0] {
        return Err(Error::InvalidEncoding);
    }
    let identity = SpaceId(
        bytes[8..40]
            .try_into()
            .map_err(|_| Error::InvalidEncoding)?,
    );
    let count = word(bytes, 40)? as usize;
    if count > limits.records {
        return Err(Error::Capacity(Capacity::Records));
    }
    if count
        .checked_mul(NODE_BYTES)
        .and_then(|n| n.checked_add(HEADER))
        != Some(bytes.len())
    {
        return Err(Error::InvalidEncoding);
    }
    let support = word(bytes, 44)?;
    let membership = word(bytes, 48)?;
    if support / 2 > u32::try_from(count).map_err(|_| Error::InvalidEncoding)?
        || membership / 2 > u32::try_from(count).map_err(|_| Error::InvalidEncoding)?
    {
        return Err(Error::InvalidEncoding);
    }
    let nodes = parse_nodes(bytes, count, dimensions, support, membership, control)?;
    let mut semantic_order = [0; MAX_COORDINATES as usize];
    for (v, coordinate) in semantic_order.iter_mut().zip(0..dimensions) {
        *v = coordinate;
    }
    let order = order.unwrap_or(&semantic_order[..usize::from(dimensions)]);
    if order.len() != usize::from(dimensions) {
        return Err(Error::InvalidOrder);
    }
    let mut arena = Arena::new(order, limits)?;
    let mut operation = Operation::new(&mut arena, control)?;
    let mut roots = Vec::new();
    roots.try_reserve_exact(count.checked_add(1).ok_or(Error::Allocation)?)?;
    roots.push(0);
    for node in nodes {
        let variable = operation.variable(node.coordinate)?;
        let low = roots[(node.low / 2) as usize] ^ (node.low & 1);
        let high = roots[(node.high / 2) as usize] ^ (node.high & 1);
        roots.push(operation.ite(variable, high, low)?);
    }
    let support = roots[(support / 2) as usize] ^ (support & 1);
    let membership = roots[(membership / 2) as usize] ^ (membership & 1);
    if support == 0 {
        return Err(Error::EmptySpace);
    }
    if operation.apply(BoolOp4::DIFFERENCE, membership, support)? != 0 {
        return Err(Error::InvalidEncoding);
    }
    let anchor = operation.arena.witness(support).ok_or(Error::EmptySpace)?;
    let anchor_value = Ref::from(operation.arena.evaluate(membership, anchor));
    let event = operation.ite(support, membership, anchor_value)?;
    control.checkpoint()?;
    Ok(Decoded {
        identity,
        dimensions,
        support,
        event,
        arena,
    })
}

fn parse_nodes(
    bytes: &[u8],
    count: usize,
    dimensions: u8,
    support: Ref,
    membership: Ref,
    control: &dyn Control,
) -> Result<Vec<Node>> {
    let mut nodes = Vec::<Node>::new();
    nodes.try_reserve_exact(count)?;
    let mut unique = HashSet::new();
    unique.try_reserve(count)?;
    for i in 0..count {
        control.checkpoint()?;
        let offset = HEADER + NODE_BYTES * i;
        let node = Node {
            coordinate: bytes[offset],
            low: word(bytes, offset + 1)?,
            high: word(bytes, offset + 5)?,
        };
        if node.coordinate >= dimensions || node.low == node.high || node.low & 1 != 0 {
            return Err(Error::InvalidEncoding);
        }
        for child in [node.low, node.high] {
            if child / 2 > u32::try_from(i).map_err(|_| Error::InvalidEncoding)? {
                return Err(Error::InvalidEncoding);
            }
            if child >= 2 && nodes[(child / 2 - 1) as usize].coordinate <= node.coordinate {
                return Err(Error::InvalidEncoding);
            }
        }
        if !unique.insert(node) {
            return Err(Error::InvalidEncoding);
        }
        nodes.push(node);
    }
    let mut seen = Vec::new();
    seen.try_reserve_exact(count)?;
    seen.resize(count, false);
    let mut next = 0;
    check_postorder(support, &nodes, &mut seen, &mut next, control)?;
    check_postorder(membership, &nodes, &mut seen, &mut next, control)?;
    if next != count {
        return Err(Error::InvalidEncoding);
    }
    Ok(nodes)
}
