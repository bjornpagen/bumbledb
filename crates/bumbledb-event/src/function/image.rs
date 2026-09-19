//! Weighted direct image. Each source coordinate is summed exactly once, after
//! its last remaining readout use. Existential abstraction would lose mass;
//! summing an already forgotten bit again would invent an extra factor of two.
use super::{
    FiniteFunction, FunctionLimits, FunctionPiece,
    raw::{self, Budget, Term},
};
use crate::arena::{Arena, MAX_COORDINATES, Operation, Ref};
use crate::{Capacity, CoordinateMap, Error, ExactArithmetic, Result};
use std::collections::HashMap;

impl FiniteFunction {
    /// Substitute a checked deterministic readout into this scalar function.
    /// # Errors
    /// Foreign target context, graph/arithmetic/function limits or cancellation.
    pub fn pullback(
        &self,
        map: &CoordinateMap,
        limits: FunctionLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.space.full().align_to(map.target(), work.control())?;
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(self.terms.len())?;
        for (region, value) in self.pieces() {
            pieces.push(FunctionPiece {
                region: map.pullback(&region, work.control())?,
                value: value.clone(),
            });
        }
        Self::new(map.source(), &pieces, limits, work)
    }

    /// Sum the scalar values of *all* legal source worlds in each target fibre.
    /// This is weighted direct image, not existential Event image, conditional
    /// averaging or a claim about the target's previously designated law.
    /// Source and target can each have 62 coordinates without a combined arena.
    /// # Errors
    /// Context, graph/arithmetic/function/memo limits or cancellation.
    pub fn pushforward(
        &self,
        map: &CoordinateMap,
        limits: FunctionLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let aligned = self.align_to(map.source(), limits, work)?;
        let control = work.control();
        map.source()
            .with_arena_pair(map.target(), |source, target| {
                let mut image = Image::new(source, target, map, limits, work)?;
                let input = aligned.supported(&mut image.source, &mut image.budget)?;
                let hidden = image.source.arena.mask() & !image.suffix[0];
                let input = raw::abstract_sum(
                    input,
                    hidden,
                    &mut image.source,
                    &mut image.budget,
                    image.work,
                )?;
                let terms = image.run(0, input)?;
                match &mut image.target {
                    Some(op) => {
                        Self::publish(map.target(), &terms, op, &mut image.budget, image.work)
                    }
                    None => Self::publish(
                        map.target(),
                        &terms,
                        &mut image.source,
                        &mut image.budget,
                        image.work,
                    ),
                }
            })
            .and_then(|result| {
                control.checkpoint()?;
                Ok(result)
            })
    }
}

struct Image<'s, 't, 'w, 'c> {
    source: Operation<'s>,
    target: Option<Operation<'t>>,
    order: Vec<u8>,
    readouts: [Ref; MAX_COORDINATES as usize],
    suffix: Vec<u64>,
    budget: Budget,
    work: &'w mut ExactArithmetic<'c>,
    memo: HashMap<(usize, Vec<Term>), Vec<Term>>,
}
impl<'s, 't, 'w, 'c> Image<'s, 't, 'w, 'c>
where
    'c: 's,
    's: 't,
{
    fn new(
        source: &'s mut Arena,
        target: Option<&'t mut Arena>,
        map: &CoordinateMap,
        limits: FunctionLimits,
        work: &'w mut ExactArithmetic<'c>,
    ) -> Result<Self> {
        let control = work.control();
        let mut order = Vec::new();
        order.try_reserve_exact(map.readouts().len())?;
        order.extend_from_slice(target.as_deref().unwrap_or(source).order());
        let mut readouts = [0; MAX_COORDINATES as usize];
        for (root, event) in readouts.iter_mut().zip(map.readouts()) {
            *root = event.root;
        }
        let mut suffix = Vec::new();
        suffix.try_reserve_exact(order.len() + 1)?;
        suffix.resize(order.len() + 1, 0);
        for position in (0..order.len()).rev() {
            suffix[position] =
                suffix[position + 1] | source.variables(readouts[usize::from(order[position])]);
        }
        Ok(Self {
            source: Operation::new(source, control)?,
            target: target
                .map(|arena| Operation::new(arena, control))
                .transpose()?,
            order,
            readouts,
            suffix,
            budget: Budget::new(limits, control)?,
            work,
            memo: HashMap::new(),
        })
    }
    fn run(&mut self, position: usize, input: Vec<Term>) -> Result<Vec<Term>> {
        self.budget.step(self.work.control())?;
        if input.is_empty() {
            return Ok(input);
        }
        if position == self.order.len() {
            if input.len() != 1 || input[0].root != 1 {
                return Err(Error::FunctionInvariant);
            }
            return Ok(input);
        }
        let key = (position, input);
        if let Some(terms) = self.memo.get(&key) {
            return Ok(terms.clone());
        }
        let coordinate = self.order[position];
        let readout = self.readouts[usize::from(coordinate)];
        let hidden = self.suffix[position] & !self.suffix[position + 1];
        let low = raw::mask(&key.1, readout ^ 1, &mut self.source, &mut self.budget)?;
        let high = raw::mask(&key.1, readout, &mut self.source, &mut self.budget)?;
        let low = raw::abstract_sum(low, hidden, &mut self.source, &mut self.budget, self.work)?;
        let high = raw::abstract_sum(high, hidden, &mut self.source, &mut self.budget, self.work)?;
        let low = self.run(position + 1, low)?;
        let high = self.run(position + 1, high)?;
        let terms = if let Some(op) = &mut self.target {
            let bit = op.variable(coordinate)?;
            raw::select(bit, &low, &high, op, &mut self.budget, self.work)?
        } else {
            let bit = self.source.variable(coordinate)?;
            raw::select(
                bit,
                &low,
                &high,
                &mut self.source,
                &mut self.budget,
                self.work,
            )?
        };
        if self.memo.len() >= self.budget.limits.memo_entries {
            return Err(Error::Capacity(Capacity::FunctionMemo));
        }
        self.memo.try_reserve(1)?;
        self.memo.insert(key, terms.clone());
        Ok(terms)
    }
}
