//! Owned, typed, acyclic Event programs. Construction is separate from execution;
//! no host callback or source allocation can occur inside the instruction set.
use std::sync::Arc;

use crate::{
    BoolOp4, Capacity, Control, CoordinateMap, Error, Event, Result, Space, WorldRelation,
};

/// Directions a value may change when the program input grows. `Mixed` means
/// this compositional analysis has not proved a direction, not that every input
/// exhibits both changes. Correlated subexpressions may require a rewrite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variance {
    Independent,
    Increasing,
    Decreasing,
    Mixed,
}

impl Variance {
    pub(crate) fn allows(self, before: bool, after: bool) -> bool {
        before == after
            || match self {
                Self::Independent => false,
                Self::Increasing => after,
                Self::Decreasing => before,
                Self::Mixed => true,
            }
    }

    pub(crate) fn classify<const N: usize>(
        inputs: [Self; N],
        truth: impl Fn([bool; N]) -> bool,
    ) -> Self {
        let mut increasing = false;
        let mut decreasing = false;
        for before in 0..(1usize << N) {
            for after in 0..(1usize << N) {
                let old = std::array::from_fn(|i| before & (1 << i) != 0);
                let new = std::array::from_fn(|i| after & (1 << i) != 0);
                if (0..N).all(|i| inputs[i].allows(old[i], new[i])) {
                    increasing |= !truth(old) && truth(new);
                    decreasing |= truth(old) && !truth(new);
                }
            }
        }
        match (increasing, decreasing) {
            (false, false) => Self::Independent,
            (true, false) => Self::Increasing,
            (false, true) => Self::Decreasing,
            (true, true) => Self::Mixed,
        }
    }
}

/// These operations preserve Event inclusion, including on unreachable fibres.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapOp {
    Pullback,
    Image,
    UniversalImage,
    NonvacuousImage,
    Possible,
    Guaranteed,
}

impl MapOp {
    fn spaces(self, map: &CoordinateMap) -> (&Space, &Space) {
        match self {
            Self::Pullback => (map.target(), map.source()),
            Self::Image | Self::UniversalImage | Self::NonvacuousImage => {
                (map.source(), map.target())
            }
            Self::Possible | Self::Guaranteed => (map.source(), map.source()),
        }
    }

    fn evaluate(self, map: &CoordinateMap, value: &Event, control: &dyn Control) -> Result<Event> {
        match self {
            Self::Pullback => map.pullback(value, control),
            Self::Image => map.image(value, control),
            Self::UniversalImage => map.universal_image(value, control),
            Self::NonvacuousImage => map.nonvacuous_image(value, control),
            Self::Possible => map.possible(value, control),
            Self::Guaranteed => map.guaranteed(value, control),
        }
    }
}

/// Modalities of a captured, immutable relation. Must requires a successor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalOp {
    May,
    All,
    Must,
    Post,
}

impl ModalOp {
    fn spaces(self, relation: &WorldRelation) -> (&Space, &Space) {
        if self == Self::Post {
            (relation.input(), relation.output())
        } else {
            (relation.output(), relation.input())
        }
    }

    fn evaluate(
        self,
        relation: &WorldRelation,
        value: &Event,
        control: &dyn Control,
    ) -> Result<Event> {
        match self {
            Self::May => relation.may(value, control),
            Self::All => relation.all(value, control),
            Self::Must => relation.must(value, control),
            Self::Post => relation.post(value, control),
        }
    }
}

/// Read-only instruction data. Child indices always address an earlier row in
/// the same prepared program. Constructing this enum does not admit a program;
/// the builder is the only public path to executable instructions.
#[derive(Debug, Clone)]
pub enum ProgramOp {
    Input,
    Constant(Event),
    Not(usize),
    Binary {
        operation: BoolOp4,
        left: usize,
        right: usize,
    },
    Ite {
        condition: usize,
        high: usize,
        low: usize,
    },
    Map {
        operation: MapOp,
        map: CoordinateMap,
        input: usize,
    },
    Modal {
        operation: ModalOp,
        relation: WorldRelation,
        input: usize,
    },
}

impl ProgramOp {
    fn inputs(&self) -> [Option<usize>; 3] {
        match *self {
            Self::Input | Self::Constant(_) => [None; 3],
            Self::Not(i) | Self::Map { input: i, .. } | Self::Modal { input: i, .. } => {
                [Some(i), None, None]
            }
            Self::Binary { left, right, .. } => [Some(left), Some(right), None],
            Self::Ite {
                condition,
                high,
                low,
            } => [Some(condition), Some(high), Some(low)],
        }
    }

    fn remap(&mut self, indices: &[usize]) {
        match self {
            Self::Input | Self::Constant(_) => {}
            Self::Not(i) | Self::Map { input: i, .. } | Self::Modal { input: i, .. } => {
                *i = indices[*i];
            }
            Self::Binary { left, right, .. } => {
                *left = indices[*left];
                *right = indices[*right];
            }
            Self::Ite {
                condition,
                high,
                low,
            } => {
                *condition = indices[*condition];
                *high = indices[*high];
                *low = indices[*low];
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProgramInstruction {
    space: Space,
    variance: Variance,
    operation: ProgramOp,
}

impl ProgramInstruction {
    #[must_use]
    pub fn space(&self) -> &Space {
        &self.space
    }
    #[must_use]
    pub fn variance(&self) -> Variance {
        self.variance
    }
    #[must_use]
    pub fn operation(&self) -> &ProgramOp {
        &self.operation
    }
}

/// A checked reference belonging to exactly one live builder, including when
/// another builder has the same input space and coincident numeric indices.
#[derive(Debug, Clone)]
pub struct ProgramValue {
    owner: Arc<()>,
    index: usize,
}

#[derive(Debug)]
pub struct EventProgramBuilder {
    owner: Arc<()>,
    input: Space,
    instructions: Vec<ProgramInstruction>,
    limit: usize,
}

impl EventProgramBuilder {
    /// Create a single-input program, allowing at most 100,000 authored nodes.
    /// # Errors
    /// Refuses cancellation or unavailable resources.
    pub fn new(input: &Space, control: &dyn Control) -> Result<Self> {
        Self::with_limit(input, 100_000, control)
    }

    /// Set the construction bound; the input counts as one authored node.
    /// # Errors
    /// Refuses zero capacity, cancellation or allocation failure.
    pub fn with_limit(input: &Space, limit: usize, control: &dyn Control) -> Result<Self> {
        control.checkpoint()?;
        let mut builder = Self {
            owner: Arc::new(()),
            input: input.clone(),
            instructions: Vec::new(),
            limit,
        };
        builder.push(input, Variance::Increasing, ProgramOp::Input, control)?;
        Ok(builder)
    }

    #[must_use]
    pub fn input(&self) -> ProgramValue {
        self.value(0)
    }

    fn value(&self, index: usize) -> ProgramValue {
        ProgramValue {
            owner: self.owner.clone(),
            index,
        }
    }

    fn get(&self, value: &ProgramValue) -> Result<&ProgramInstruction> {
        if !Arc::ptr_eq(&self.owner, &value.owner) {
            return Err(Error::ProgramMismatch);
        }
        self.instructions
            .get(value.index)
            .ok_or(Error::ProgramMismatch)
    }

    fn push(
        &mut self,
        space: &Space,
        variance: Variance,
        operation: ProgramOp,
        control: &dyn Control,
    ) -> Result<ProgramValue> {
        control.checkpoint()?;
        if self.instructions.len() >= self.limit {
            return Err(Error::Capacity(Capacity::ProgramNodes));
        }
        self.instructions.try_reserve(1)?;
        let result = self.value(self.instructions.len());
        self.instructions.push(ProgramInstruction {
            space: space.clone(),
            variance,
            operation,
        });
        Ok(result)
    }

    /// Capture an owned constant in its original context. Intermediate contexts
    /// may differ from the input; every consuming operation checks its endpoints.
    /// # Errors
    /// Refuses cancellation or construction capacity/allocation failure.
    pub fn constant(&mut self, value: &Event, control: &dyn Control) -> Result<ProgramValue> {
        self.push(
            &value.space(),
            Variance::Independent,
            ProgramOp::Constant(value.clone()),
            control,
        )
    }

    /// # Errors
    /// Refuses a foreign builder reference, cancellation or unavailable resources.
    pub fn complement(
        &mut self,
        value: &ProgramValue,
        control: &dyn Control,
    ) -> Result<ProgramValue> {
        let instruction = self.get(value)?;
        let space = instruction.space.clone();
        let variance = Variance::classify([instruction.variance], |[a]| !a);
        self.push(&space, variance, ProgramOp::Not(value.index), control)
    }

    /// Apply any of the sixteen truth functions, validating both operands before
    /// constant truth functions or variance analysis can simplify the result.
    /// # Errors
    /// Refuses foreign references/contexts, cancellation or capacity.
    pub fn apply(
        &mut self,
        operation: BoolOp4,
        left: &ProgramValue,
        right: &ProgramValue,
        control: &dyn Control,
    ) -> Result<ProgramValue> {
        let left_node = self.get(left)?;
        let right_node = self.get(right)?;
        right_node
            .space
            .full()
            .align_to(&left_node.space, control)?;
        let space = left_node.space.clone();
        let variance = Variance::classify([left_node.variance, right_node.variance], |[a, b]| {
            operation.evaluate(a, b)
        });
        self.push(
            &space,
            variance,
            ProgramOp::Binary {
                operation,
                left: left.index,
                right: right.index,
            },
            control,
        )
    }

    /// A conditional expression; every operand is validated and participates in
    /// execution. The program performs algebra, not branch-dependent side effects.
    /// # Errors
    /// Refuses foreign references/contexts, cancellation or capacity.
    pub fn ite(
        &mut self,
        condition: &ProgramValue,
        high: &ProgramValue,
        low: &ProgramValue,
        control: &dyn Control,
    ) -> Result<ProgramValue> {
        let c = self.get(condition)?;
        let h = self.get(high)?;
        let l = self.get(low)?;
        h.space.full().align_to(&c.space, control)?;
        l.space.full().align_to(&c.space, control)?;
        let space = c.space.clone();
        let variance =
            Variance::classify(
                [c.variance, h.variance, l.variance],
                |[c, h, l]| if c { h } else { l },
            );
        self.push(
            &space,
            variance,
            ProgramOp::Ite {
                condition: condition.index,
                high: high.index,
                low: low.index,
            },
            control,
        )
    }

    /// Lift, quantify or saturate through a captured checked map.
    /// # Errors
    /// Refuses incorrect endpoints, foreign builder references, cancellation or capacity.
    pub fn map(
        &mut self,
        operation: MapOp,
        map: &CoordinateMap,
        input: &ProgramValue,
        control: &dyn Control,
    ) -> Result<ProgramValue> {
        let node = self.get(input)?;
        let (source, target) = operation.spaces(map);
        node.space.full().align_to(source, control)?;
        self.push(
            target,
            node.variance,
            ProgramOp::Map {
                operation,
                map: map.clone(),
                input: input.index,
            },
            control,
        )
    }

    /// Apply a modality of a fixed, owned relation.
    /// # Errors
    /// Refuses incorrect endpoints, foreign builder references, cancellation or capacity.
    pub fn modal(
        &mut self,
        operation: ModalOp,
        relation: &WorldRelation,
        input: &ProgramValue,
        control: &dyn Control,
    ) -> Result<ProgramValue> {
        let node = self.get(input)?;
        let (source, target) = operation.spaces(relation);
        node.space.full().align_to(source, control)?;
        self.push(
            target,
            node.variance,
            ProgramOp::Modal {
                operation,
                relation: relation.clone(),
                input: input.index,
            },
            control,
        )
    }

    /// Seal an acyclic program containing only nodes reachable from the root.
    /// Unused authored nodes are not executed. Used operands remain participating
    /// even when a truth function is constant. Output may have a different space;
    /// fixed-point admission separately requires an increasing endoprogram.
    /// # Errors
    /// Refuses a foreign root, cancellation or unavailable resources.
    pub fn finish(self, root: &ProgramValue, control: &dyn Control) -> Result<EventProgram> {
        self.get(root)?;
        control.checkpoint()?;
        let extent = root.index + 1;
        let mut needed = Vec::new();
        needed.try_reserve_exact(extent)?;
        needed.resize(extent, false);
        needed[root.index] = true;
        for index in (0..extent).rev() {
            control.checkpoint()?;
            if needed[index] {
                for input in self.instructions[index]
                    .operation
                    .inputs()
                    .into_iter()
                    .flatten()
                {
                    needed[input] = true;
                }
            }
        }
        let mut indices = Vec::new();
        indices.try_reserve_exact(extent)?;
        indices.resize(extent, 0);
        let mut instructions = Vec::new();
        instructions.try_reserve_exact(needed.iter().filter(|&&keep| keep).count())?;
        for (index, mut node) in self.instructions.into_iter().take(extent).enumerate() {
            control.checkpoint()?;
            if needed[index] {
                node.operation.remap(&indices);
                indices[index] = instructions.len();
                instructions.push(node);
            }
        }
        control.checkpoint()?;
        Ok(EventProgram(Arc::new(Program {
            input: self.input,
            instructions,
            root: indices[root.index],
        })))
    }
}

#[derive(Debug)]
struct Program {
    input: Space,
    instructions: Vec<ProgramInstruction>,
    root: usize,
}

/// A reusable single-input Event function. Instructions/maps/relations and all
/// contexts are owned and inspectable. There are no unvalidated operation hooks.
///
/// A rotating readout propagates a seed around three one-hot legal worlds.
/// The least solution contains all three; four applications include the final
/// equality check. Iteration remains symbolic, without enumerating the carrier.
/// ```
/// use bumbledb_event::{
///     BoolOp4, CoordinateMap, EventProgramBuilder, FixedPointLimits, MapOp,
///     Space, SpaceId,
/// };
/// # fn main() -> bumbledb_event::Result<()> {
/// let raw = Space::new(SpaceId([1; 32]), 3, &())?;
/// let legal = raw.table(0b111, &[0b0001_0110], &())?; // 001, 010, 100
/// let space = raw.restrict(&legal, &())?;
/// let rotate = CoordinateMap::coordinates(&space, &space, &[1, 2, 0], &())?;
/// let mut builder = EventProgramBuilder::new(&space, &())?;
/// let seed = builder.constant(&space.coordinate(0, &())?, &())?;
/// let next = builder.map(MapOp::Pullback, &rotate, &builder.input(), &())?;
/// let root = builder.apply(BoolOp4::OR, &seed, &next, &())?;
/// let fixed = builder.finish(&root, &())?.fixed_points(&())?;
/// let result = fixed.least(FixedPointLimits::default(), &())?;
/// assert_eq!(fixed.carrier().worlds(), 3);
/// assert!(result.event().is_full());
/// assert_eq!(result.iterations(), 4);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct EventProgram(Arc<Program>);

impl EventProgram {
    #[must_use]
    pub fn input(&self) -> &Space {
        &self.0.input
    }
    #[must_use]
    pub fn output(&self) -> &Space {
        &self.0.instructions[self.0.root].space
    }
    #[must_use]
    pub fn variance(&self) -> Variance {
        self.0.instructions[self.0.root].variance
    }
    #[must_use]
    pub fn instructions(&self) -> &[ProgramInstruction] {
        &self.0.instructions
    }
    #[must_use]
    pub fn root(&self) -> usize {
        self.0.root
    }

    /// Execute once, validating the input even for a constant program.
    /// Each retained instruction is evaluated exactly once in dependency order.
    /// # Errors
    /// Refuses input mismatch, cancellation or kernel/workspace capacity.
    pub fn evaluate(&self, input: &Event, control: &dyn Control) -> Result<Event> {
        self.evaluate_counted(input, &mut 0, u64::MAX, control)
    }

    pub(crate) fn evaluate_counted(
        &self,
        input: &Event,
        steps: &mut u64,
        limit: u64,
        control: &dyn Control,
    ) -> Result<Event> {
        let input = input.align_to(self.input(), control)?;
        let mut values: Vec<Event> = Vec::new();
        values.try_reserve_exact(self.0.instructions.len())?;
        for instruction in &self.0.instructions {
            control.checkpoint()?;
            if *steps >= limit {
                return Err(Error::Capacity(Capacity::ProgramSteps));
            }
            *steps += 1;
            let value = match &instruction.operation {
                ProgramOp::Input => input.clone(),
                ProgramOp::Constant(value) => value.clone(),
                ProgramOp::Not(i) => values[*i].complement(),
                ProgramOp::Binary {
                    operation,
                    left,
                    right,
                } => {
                    let right = values[*right].align_to(&instruction.space, control)?;
                    values[*left].apply(*operation, &right, control)?
                }
                ProgramOp::Ite {
                    condition,
                    high,
                    low,
                } => {
                    let high = values[*high].align_to(&instruction.space, control)?;
                    let low = values[*low].align_to(&instruction.space, control)?;
                    values[*condition].ite(&high, &low, control)?
                }
                ProgramOp::Map {
                    operation,
                    map,
                    input,
                } => operation.evaluate(map, &values[*input], control)?,
                ProgramOp::Modal {
                    operation,
                    relation,
                    input,
                } => operation.evaluate(relation, &values[*input], control)?,
            };
            values.push(value);
        }
        control.checkpoint()?;
        Ok(values[self.0.root].clone())
    }
}
