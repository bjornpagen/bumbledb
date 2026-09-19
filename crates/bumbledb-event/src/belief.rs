//! Perfect-recall possibility memory. A memory state is an ordinary Event of
//! hidden states consistent with the visible history. Equal Events share future
//! behavior under the supplied Markov transition model. No probability is used.
use std::collections::HashMap;
use std::sync::Arc;

use crate::product::require_same_map;
use crate::{
    ActionArena, BoolOp4, Capacity, Control, CoordinateMap, Error, Event, EventPartition,
    FibreProduct, Result, Space, SpaceId, WorldRelation,
};

/// Reachable subset construction can be exponential. Limits refuse the whole
/// construction, never truncate the graph or silently merge different beliefs.
#[derive(Debug, Clone, Copy)]
pub struct BeliefLimits {
    pub states: usize,
    pub transitions: usize,
    /// Cumulative admission, expansion and observation-splitting steps. Each
    /// underlying symbolic operation also retains its own kernel allowance.
    pub steps: usize,
}
impl Default for BeliefLimits {
    fn default() -> Self {
        Self {
            states: 100_000,
            transitions: 1_000_000,
            steps: 10_000_000,
        }
    }
}

/// Public roster indices are local to the owning memory. A transition retains
/// the observed cell even when several observations follow the same action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BeliefTransition {
    pub action: usize,
    pub observation: usize,
    pub target: usize,
}

#[derive(Debug, Clone)]
pub struct BeliefState {
    possible: Event,
    observation: usize,
    transitions: Vec<BeliefTransition>,
}
impl BeliefState {
    #[must_use]
    pub fn possible(&self) -> &Event {
        &self.possible
    }
    #[must_use]
    pub fn observation(&self) -> usize {
        self.observation
    }
    /// Sorted by (action, observation). Missing actions are not uniformly enabled.
    #[must_use]
    pub fn transitions(&self) -> &[BeliefTransition] {
        &self.transitions
    }
}

#[derive(Debug)]
struct Memory {
    actions: Vec<WorldRelation>,
    observations: EventPartition,
    given: Event,
    initial: Vec<Option<usize>>,
    states: Vec<BeliefState>,
}

/// Exact reachable memory for a roster of public action labels and a full
/// observation partition. Each action is an endorelation on the same hidden
/// state/environment. Its label is known to the actor; its outcome is not.
/// The supplied state must include everything affecting future transitions.
#[derive(Debug, Clone)]
pub struct BeliefMemory(Arc<Memory>);

struct Build<'a> {
    states: Vec<BeliefState>,
    index: HashMap<Event, usize>,
    limits: BeliefLimits,
    steps: usize,
    transitions: usize,
    control: &'a dyn Control,
}
impl Build<'_> {
    fn step(&mut self) -> Result<()> {
        self.control.checkpoint()?;
        if self.steps == self.limits.steps {
            return Err(Error::Capacity(Capacity::BeliefSteps));
        }
        self.steps += 1;
        Ok(())
    }

    fn intern(&mut self, possible: Event, observation: usize) -> Result<Option<usize>> {
        self.step()?;
        if possible.is_empty() {
            return Ok(None);
        }
        if let Some(&index) = self.index.get(&possible) {
            return Ok(Some(index));
        }
        if self.states.len() == self.limits.states {
            return Err(Error::Capacity(Capacity::BeliefStates));
        }
        self.states.try_reserve(1)?;
        self.index.try_reserve(1)?;
        let index = self.states.len();
        self.index.insert(possible.clone(), index);
        self.states.push(BeliefState {
            possible,
            observation,
            transitions: Vec::new(),
        });
        Ok(Some(index))
    }

    fn expand(
        &mut self,
        actions: &[WorldRelation],
        domains: &[Event],
        observations: &EventPartition,
    ) -> Result<()> {
        let mut source = 0;
        while source < self.states.len() {
            let possible = self.states[source].possible.clone();
            for (action, (relation, enabled)) in actions.iter().zip(domains).enumerate() {
                self.step()?;
                if !possible.signature(enabled, self.control)?.included() {
                    continue;
                }
                let post = relation
                    .post(&possible, self.control)?
                    .align_to(&possible.space(), self.control)?;
                for (observation, cell) in observations.cells().iter().enumerate() {
                    self.step()?;
                    let next = post.apply(BoolOp4::AND, cell, self.control)?;
                    if let Some(target) = self.intern(next, observation)? {
                        if self.transitions == self.limits.transitions {
                            return Err(Error::Capacity(Capacity::BeliefTransitions));
                        }
                        self.states[source].transitions.try_reserve(1)?;
                        self.states[source].transitions.push(BeliefTransition {
                            action,
                            observation,
                            target,
                        });
                        self.transitions += 1;
                    }
                }
            }
            source += 1;
        }
        Ok(())
    }
}

impl BeliefMemory {
    /// Split initial evidence by visible cells, then close under all uniformly
    /// enabled actions and possible observations. B --a,o--> Post(a,B) ∩ Cell(o).
    /// Actions with a dead end at any possible state are unavailable throughout B.
    /// Empty evidence has no initial state, rather than a vacuously informed one.
    /// # Errors
    /// Refuses partial observation coverage, mismatched endoroles/environments,
    /// cancellation or resource exhaustion. All action contexts are checked even
    /// when evidence is empty. No partial memory escapes a failure.
    pub fn new(
        actions: &[WorldRelation],
        observations: &EventPartition,
        given: &Event,
        limits: BeliefLimits,
        control: &dyn Control,
    ) -> Result<Self> {
        Self::new_with_parameters(
            actions,
            observations,
            given,
            limits,
            crate::ParameterSourceLimits::default(),
            &mut crate::ExactArithmetic::new(crate::ArithmeticLimits::default(), control),
        )
    }

    /// Build memory with one arithmetic allowance for parameter-preserving action
    /// normalization. Symbolic expansion retains its separate graph/kernel bounds.
    /// # Errors
    /// Has `new`'s contract, plus parameter-source and arithmetic capacities.
    pub fn new_with_parameters(
        actions: &[WorldRelation],
        observations: &EventPartition,
        given: &Event,
        limits: BeliefLimits,
        parameters: crate::ParameterSourceLimits,
        work: &mut crate::ExactArithmetic<'_>,
    ) -> Result<Self> {
        let control = work.control();
        control.checkpoint()?;
        let space = observations.parent().space();
        let given = given.align_to(&space, control)?;
        if !observations.parent().is_full() {
            return Err(Error::PartitionGap);
        }
        let mut build = Build {
            states: Vec::new(),
            index: HashMap::new(),
            limits,
            steps: 0,
            transitions: 0,
            control,
        };
        let mut retained = Vec::new();
        let mut domains = Vec::new();
        for action in actions {
            build.step()?;
            action.input().full().align_to(&space, control)?;
            action.output().full().align_to(&space, control)?;
            require_same_map(
                action.product().left_environment().map(),
                action.product().right_environment().map(),
                control,
            )?;
            let action =
                action.in_product_with_parameters(actions[0].product(), parameters, work)?;
            domains.try_reserve(1)?;
            domains.push(action.domain(control)?.align_to(&space, control)?);
            retained.try_reserve(1)?;
            retained.push(action);
        }
        let mut initial = Vec::new();
        for (observation, cell) in observations.cells().iter().enumerate() {
            build.step()?;
            let possible = given.apply(BoolOp4::AND, cell, control)?;
            initial.try_reserve(1)?;
            initial.push(build.intern(possible, observation)?);
        }
        build.expand(&retained, &domains, observations)?;
        control.checkpoint()?;
        Ok(Self(Arc::new(Memory {
            actions: retained,
            observations: observations.clone(),
            given,
            initial,
            states: build.states,
        })))
    }

    #[must_use]
    pub fn actions(&self) -> &[WorldRelation] {
        &self.0.actions
    }
    #[must_use]
    pub fn observations(&self) -> &EventPartition {
        &self.0.observations
    }
    #[must_use]
    pub fn given(&self) -> &Event {
        &self.0.given
    }
    /// One position per observation cell, including unreachable empty cells.
    #[must_use]
    pub fn initial(&self) -> &[Option<usize>] {
        &self.0.initial
    }
    #[must_use]
    pub fn states(&self) -> &[BeliefState] {
        &self.0.states
    }

    /// Uniform enabledness. The actor cannot choose different hidden-state actions.
    /// # Errors
    /// Refuses an index outside this memory's state or action roster.
    pub fn enabled(&self, state: usize, action: usize) -> Result<bool> {
        let state = self.states().get(state).ok_or(Error::BeliefIndex)?;
        if action >= self.actions().len() {
            return Err(Error::BeliefIndex);
        }
        let index = state
            .transitions
            .partition_point(|edge| edge.action < action);
        Ok(state
            .transitions
            .get(index)
            .is_some_and(|edge| edge.action == action))
    }

    /// Update using only the retained memory, public action and observed label.
    /// None means an unavailable action or an impossible observation; `enabled`
    /// distinguishes them. No empty belief is treated as a usable state.
    /// # Errors
    /// Refuses any index outside its roster, including for unavailable actions.
    pub fn update(&self, state: usize, action: usize, observation: usize) -> Result<Option<usize>> {
        let state = self.states().get(state).ok_or(Error::BeliefIndex)?;
        if action >= self.actions().len() || observation >= self.observations().cells().len() {
            return Err(Error::BeliefIndex);
        }
        Ok(state
            .transitions
            .binary_search_by_key(&(action, observation), |edge| {
                (edge.action, edge.observation)
            })
            .ok()
            .map(|i| state.transitions[i].target))
    }
}

/// Explicit names for a finite memory presentation; labels/codes are meaningful
/// only with its retained memory roster. No hidden-world environment is exposed
/// as an observed coordinate of this fully observed arena.
#[derive(Debug, Clone, Copy)]
pub struct BeliefSpaceIds {
    pub states: SpaceId,
    pub actions: SpaceId,
    pub environment: SpaceId,
    pub state_actions: SpaceId,
    pub transitions: SpaceId,
}

/// Ordinary `ActionArena` over reachable memory states. Winning/rank/policy Events
/// use the same storage, containment and Free Join paths as other Event values.
#[derive(Debug, Clone)]
pub struct BeliefArena {
    memory: BeliefMemory,
    arena: ActionArena,
    initial: Event,
}

impl BeliefMemory {
    /// Lower the reachable memory graph into the existing Event action algebra.
    /// The retained graph already bounds work; kernel limits/cancellation also apply.
    /// # Errors
    /// Refuses empty memory or an empty action vocabulary (Space is inhabited),
    /// too many product coordinates, cancellation or kernel/allocation capacity.
    pub fn arena(&self, ids: BeliefSpaceIds, control: &dyn Control) -> Result<BeliefArena> {
        let states = code_space(ids.states, self.states().len(), control)?;
        let actions = code_space(ids.actions, self.actions().len(), control)?;
        let environment = Space::new(ids.environment, 0, control)?;
        let base = |s: &Space| {
            CoordinateMap::new(s, &environment, &[], control)?.certify_surjective(control)
        };
        let s = base(&states)?;
        let sa = FibreProduct::new(ids.state_actions, &s, &base(&actions)?, control)?;
        let steps = FibreProduct::new(ids.transitions, &base(sa.space())?, &s, control)?;
        let mut region = steps.space().empty();
        for (source, state) in self.states().iter().enumerate() {
            let source = sa
                .left()
                .map()
                .pullback(&code(&states, source, control)?, control)?;
            for edge in state.transitions() {
                control.checkpoint()?;
                let action = sa
                    .right()
                    .map()
                    .pullback(&code(&actions, edge.action, control)?, control)?;
                let pair = source.apply(BoolOp4::AND, &action, control)?;
                let pair = steps.left().map().pullback(&pair, control)?;
                let target = steps
                    .right()
                    .map()
                    .pullback(&code(&states, edge.target, control)?, control)?;
                region = region.apply(
                    BoolOp4::OR,
                    &pair.apply(BoolOp4::AND, &target, control)?,
                    control,
                )?;
            }
        }
        let arena = ActionArena::new(&sa, &WorldRelation::new(&steps, &region, control)?, control)?;
        let mut initial = states.empty();
        for &index in self.initial().iter().flatten() {
            initial = initial.apply(BoolOp4::OR, &code(&states, index, control)?, control)?;
        }
        control.checkpoint()?;
        Ok(BeliefArena {
            memory: self.clone(),
            arena,
            initial,
        })
    }
}

impl BeliefArena {
    #[must_use]
    pub fn memory(&self) -> &BeliefMemory {
        &self.memory
    }
    #[must_use]
    pub fn arena(&self) -> &ActionArena {
        &self.arena
    }
    #[must_use]
    pub fn initial(&self) -> &Event {
        &self.initial
    }

    /// Memory states at which every still-possible world meets `event`.
    /// Reachability of this region means the actor eventually *knows* the goal.
    /// It does not assert completeness for unobservable hidden-state objectives.
    /// # Errors
    /// Refuses a foreign hidden Event, cancellation or resources.
    pub fn known(&self, event: &Event, control: &dyn Control) -> Result<Event> {
        self.classify(event, true, control)
    }

    /// Memory states with at least one still-possible world meeting `event`.
    /// # Errors
    /// Has `known`'s context and resource contract.
    pub fn possible(&self, event: &Event, control: &dyn Control) -> Result<Event> {
        self.classify(event, false, control)
    }

    fn classify(&self, event: &Event, universal: bool, control: &dyn Control) -> Result<Event> {
        let event = event.align_to(&self.memory.given().space(), control)?;
        let mut selected = self.arena.states().empty();
        for (index, state) in self.memory.states().iter().enumerate() {
            let signature = state.possible.signature(&event, control)?;
            if if universal {
                signature.included()
            } else {
                !signature.disjoint()
            } {
                selected = selected.apply(
                    BoolOp4::OR,
                    &code(self.arena.states(), index, control)?,
                    control,
                )?;
            }
        }
        Ok(selected)
    }
}

fn code(space: &Space, value: usize, control: &dyn Control) -> Result<Event> {
    let mut result = space.full();
    for bit in 0..space.dimensions() {
        let set = space.coordinate(bit, control)?;
        result = result.apply(
            BoolOp4::AND,
            &if value & (1 << bit) == 0 {
                set.complement()
            } else {
                set
            },
            control,
        )?;
    }
    control.checkpoint()?;
    Ok(result)
}

fn code_space(id: SpaceId, count: usize, control: &dyn Control) -> Result<Space> {
    if count == 0 {
        return Err(Error::EmptySpace);
    }
    let bits = u8::try_from((count - 1).bit_width())
        .map_err(|_| Error::Capacity(Capacity::Coordinates))?;
    let raw = Space::new(id, bits, control)?;
    if count.is_power_of_two() {
        return Ok(raw);
    }
    // Exact prefix support; unused binary codes are not memory states.
    let mut less = raw.empty();
    let mut equal = raw.full();
    for bit in (0..bits).rev() {
        let set = raw.coordinate(bit, control)?;
        if count & (1 << bit) != 0 {
            less = less.apply(
                BoolOp4::OR,
                &equal.apply(BoolOp4::DIFFERENCE, &set, control)?,
                control,
            )?;
            equal = equal.apply(BoolOp4::AND, &set, control)?;
        } else {
            equal = equal.apply(BoolOp4::DIFFERENCE, &set, control)?;
        }
    }
    raw.restrict(&less, control)
}
