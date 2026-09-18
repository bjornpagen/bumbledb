//! Controlled finite transitions and explicit strategy witnesses, represented
//! throughout as ordinary Event relations and partitions. No hidden-state
//! visibility, probability policy or belief-memory update is inferred here.
use crate::product::require_same_map;
use crate::{
    BoolOp4, Control, Error, Event, EventProgram, EventProgramBuilder, FibreProduct,
    FixedPointLimits, FixedPointProgram, FixedPointResult, LayeredFixedPointResult, MapOp, ModalOp,
    PartitionLimits, RelationalProduct, Result, Space, WorldRelation,
};

/// A checked Step(state, action, outcome) view. Its source is the full legal
/// state/action product; availability is inside the transition relation. The
/// outcome shares exactly the product's environment, not an independent copy.
///
/// ```
/// use bumbledb_event::{ActionArena, CoordinateMap, FibreProduct, FixedPointLimits,
///     PartitionLimits, Space, SpaceId, WorldRelation};
/// # fn main() -> bumbledb_event::Result<()> {
/// let states = Space::new(SpaceId([1; 32]), 1, &())?;
/// let choices = Space::new(SpaceId([2; 32]), 1, &())?;
/// let env = Space::new(SpaceId([3; 32]), 0, &())?;
/// let s = CoordinateMap::new(&states, &env, &[], &())?.certify_surjective(&())?;
/// let a = CoordinateMap::new(&choices, &env, &[], &())?.certify_surjective(&())?;
/// let sa = FibreProduct::new(SpaceId([4; 32]), &s, &a, &())?;
/// let pair_env = CoordinateMap::new(sa.space(), &env, &[], &())?.certify_surjective(&())?;
/// let sat = FibreProduct::new(SpaceId([5; 32]), &pair_env, &s, &())?;
/// // 0:a0→0, a1→1; state 1 stays at 1. Merely remaining in {0,1} can stall.
/// let step = WorldRelation::new(&sat, &sat.space().table(7, &[0b1110_0001], &())?, &())?;
/// let arena = ActionArena::new(&sa, &step, &())?;
/// let goal = states.coordinate(0, &())?;
/// let strategy = arena.winning_reach(&goal, FixedPointLimits::default(),
///     PartitionLimits::default(), &())?;
/// assert!(strategy.winning().is_full());
/// assert!(!strategy.policy().region().contains(0)?); // stalling a0 refused
/// assert!(strategy.policy().region().contains(2)?); // a1 reaches goal
/// assert_eq!(strategy.ranked().layers().locate(0, &())?, Some(1));
/// assert_eq!(strategy.ranked().layers().locate(1, &())?, Some(0));
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ActionArena {
    actions: FibreProduct,
    transition: WorldRelation,
}

/// Finite fully observed reachability with retained first-entry ranks and all
/// enabled actions whose outcomes strictly decrease rank. Any subsequent choice
/// among those actions makes progress. Goal states stop and have no policy edge.
#[derive(Debug, Clone)]
pub struct ReachStrategy {
    arena: ActionArena,
    goal: Event,
    ranked: LayeredFixedPointResult,
    policy: WorldRelation,
}

/// Finite fully observed safety with a continuing enabled action at every
/// winning state. Every retained action keeps all outcomes inside the invariant.
#[derive(Debug, Clone)]
pub struct SafetyStrategy {
    arena: ActionArena,
    invariant: Event,
    fixed: FixedPointResult,
    policy: WorldRelation,
}

impl ActionArena {
    /// Admit the state/action product and its transition to an outcome face.
    /// The transition input must be that named product, and its environment
    /// readout must equal the product's retained environment pointwise.
    /// # Errors
    /// Refuses context/role/environment mismatches, cancellation or resources.
    pub fn new(
        actions: &FibreProduct,
        transition: &WorldRelation,
        control: &dyn Control,
    ) -> Result<Self> {
        transition
            .input()
            .full()
            .align_to(actions.space(), control)?;
        let environment = actions
            .left()
            .map()
            .then(actions.left_environment().map(), control)?;
        require_same_map(
            &environment,
            transition.product().left_environment().map(),
            control,
        )?;
        Ok(Self {
            actions: actions.clone(),
            transition: transition.clone(),
        })
    }

    #[must_use]
    pub fn actions(&self) -> &FibreProduct {
        &self.actions
    }
    #[must_use]
    pub fn transition(&self) -> &WorldRelation {
        &self.transition
    }
    #[must_use]
    pub fn states(&self) -> &Space {
        self.actions.left().map().target()
    }
    #[must_use]
    pub fn choices(&self) -> &Space {
        self.actions.right().map().target()
    }
    #[must_use]
    pub fn outcomes(&self) -> &Space {
        self.transition.output()
    }

    /// State/action pairs with at least one legal outcome. Availability never
    /// follows from vacuous universal satisfaction at a dead pair.
    /// # Errors
    /// Refuses cancellation or kernel capacity.
    pub fn enabled(&self, control: &dyn Control) -> Result<WorldRelation> {
        WorldRelation::new(&self.actions, &self.transition.domain(control)?, control)
    }

    /// All enabled state/action pairs whose every legal outcome meets `goal`.
    /// # Errors
    /// Refuses a wrong outcome context, cancellation or resources.
    pub fn good(&self, goal: &Event, control: &dyn Control) -> Result<WorldRelation> {
        WorldRelation::new(
            &self.actions,
            &self.transition.must(goal, control)?,
            control,
        )
    }

    /// Existential action choice outside universal outcome quantification.
    /// This is a fully observed predecessor, not a uniform hidden-state policy.
    /// # Errors
    /// Has `good`'s endpoint and resource contract.
    pub fn controllable_predecessor(&self, goal: &Event, control: &dyn Control) -> Result<Event> {
        self.actions
            .left()
            .map()
            .image(self.good(goal, control)?.region(), control)
    }

    /// Inspectable function from an outcome Event to its controlled predecessor.
    /// States and outcomes may have different contexts for this one-step program.
    /// # Errors
    /// Refuses cancellation or construction capacity.
    pub fn predecessor_program(&self, control: &dyn Control) -> Result<EventProgram> {
        let mut builder = EventProgramBuilder::new(self.outcomes(), control)?;
        let good = builder.modal(ModalOp::Must, &self.transition, &builder.input(), control)?;
        let root = builder.map(MapOp::Image, self.actions.left().map(), &good, control)?;
        builder.finish(&root, control)
    }

    fn fixed_program(
        &self,
        event: &Event,
        op: BoolOp4,
        control: &dyn Control,
    ) -> Result<FixedPointProgram> {
        require_same_map(
            self.actions.left_environment().map(),
            self.transition.product().right_environment().map(),
            control,
        )?;
        let mut builder = EventProgramBuilder::new(self.states(), control)?;
        let constant = builder.constant(event, control)?;
        let good = builder.modal(ModalOp::Must, &self.transition, &builder.input(), control)?;
        let pre = builder.map(MapOp::Image, self.actions.left().map(), &good, control)?;
        let root = builder.apply(op, &constant, &pre, control)?;
        builder.finish(&root, control)?.fixed_points(control)
    }

    /// The monotone reachability program X ↦ goal | CPre(X).
    /// # Errors
    /// Refuses unequal state/outcome contexts or environments, a foreign goal or resources.
    pub fn reach_program(&self, goal: &Event, control: &dyn Control) -> Result<FixedPointProgram> {
        self.fixed_program(goal, BoolOp4::OR, control)
    }

    /// The monotone continuing-safety program X ↦ invariant & CPre(X).
    /// # Errors
    /// Has `reach_program`'s endocontext and resource contract.
    pub fn safe_program(
        &self,
        invariant: &Event,
        control: &dyn Control,
    ) -> Result<FixedPointProgram> {
        self.fixed_program(invariant, BoolOp4::AND, control)
    }

    fn at_states(
        &self,
        choices: &WorldRelation,
        states: &Event,
        control: &dyn Control,
    ) -> Result<WorldRelation> {
        let choices = choices.in_product(&self.actions, control)?;
        let selected = self.actions.left().map().pullback(states, control)?;
        WorldRelation::new(
            &self.actions,
            &choices.region().apply(BoolOp4::AND, &selected, control)?,
            control,
        )
    }

    /// Solve a finite fully observed reachability game, retaining progress ranks
    /// and every enabled action that strictly decreases them. A model/application
    /// may choose among these actions without defeating the progress guarantee.
    /// # Errors
    /// Refuses invalid endoroles, fixed-point/layer/kernel budgets or cancellation.
    pub fn winning_reach(
        &self,
        goal: &Event,
        limits: FixedPointLimits,
        layer_limits: PartitionLimits,
        control: &dyn Control,
    ) -> Result<ReachStrategy> {
        let fixed = self.reach_program(goal, control)?;
        let goal = goal.align_to(self.states(), control)?;
        let ranked = fixed.least_with_layers(limits, layer_limits, control)?;
        let layers = ranked.layers().cells();
        if layers.first().is_some_and(|first| *first != goal)
            || (layers.is_empty() && !goal.is_empty())
        {
            return Err(Error::FixedPointInvariant);
        }
        let mut earlier = goal.clone();
        let mut policy = self.actions.space().empty();
        for layer in layers.iter().skip(1) {
            let choices = self.at_states(&self.good(&earlier, control)?, layer, control)?;
            policy = policy.apply(BoolOp4::OR, choices.region(), control)?;
            earlier = earlier.apply(BoolOp4::OR, layer, control)?;
        }
        let policy = WorldRelation::new(&self.actions, &policy, control)?;
        let required = ranked
            .result()
            .event()
            .apply(BoolOp4::DIFFERENCE, &goal, control)?;
        if policy.domain(control)?.align_to(self.states(), control)? != required {
            return Err(Error::FixedPointInvariant);
        }
        Ok(ReachStrategy {
            arena: self.clone(),
            goal,
            ranked,
            policy,
        })
    }

    /// Solve finite continuing safety. Dead states are excluded even if the
    /// invariant holds there; model terminal success with an explicit self-loop
    /// when continued behavior is required.
    /// # Errors
    /// Refuses invalid endoroles, resource exhaustion or cancellation.
    pub fn winning_safe(
        &self,
        invariant: &Event,
        limits: FixedPointLimits,
        control: &dyn Control,
    ) -> Result<SafetyStrategy> {
        let fixed = self
            .safe_program(invariant, control)?
            .greatest(limits, control)?;
        let invariant = invariant.align_to(self.states(), control)?;
        let policy = self.at_states(&self.good(fixed.event(), control)?, fixed.event(), control)?;
        if policy.domain(control)?.align_to(self.states(), control)? != *fixed.event() {
            return Err(Error::FixedPointInvariant);
        }
        Ok(SafetyStrategy {
            arena: self.clone(),
            invariant,
            fixed,
            policy,
        })
    }
}

fn checked_policy(
    candidate: &WorldRelation,
    permitted: &WorldRelation,
    control: &dyn Control,
) -> Result<WorldRelation> {
    let candidate = candidate.in_product(permitted.product(), control)?;
    if !candidate.included_in(permitted, control)? {
        return Err(Error::UnsafePolicy);
    }
    let source = permitted.input();
    if candidate.domain(control)?.align_to(source, control)?
        != permitted.domain(control)?.align_to(source, control)?
    {
        return Err(Error::IncompletePolicy);
    }
    Ok(candidate)
}

impl ReachStrategy {
    #[must_use]
    pub fn arena(&self) -> &ActionArena {
        &self.arena
    }
    #[must_use]
    pub fn goal(&self) -> &Event {
        &self.goal
    }
    #[must_use]
    pub fn ranked(&self) -> &LayeredFixedPointResult {
        &self.ranked
    }
    #[must_use]
    pub fn winning(&self) -> &Event {
        self.ranked.result().event()
    }
    #[must_use]
    pub fn policy(&self) -> &WorldRelation {
        &self.policy
    }

    /// Retain a chosen policy only if it covers every required non-goal state
    /// and uses only this strategy's rank-decreasing actions. Nondeterministic
    /// choices are allowed; every remaining choice still guarantees progress.
    /// # Errors
    /// Refuses wrong roles, unsafe/partial choices, resources or cancellation.
    pub fn with_policy(&self, policy: &WorldRelation, control: &dyn Control) -> Result<Self> {
        let policy = checked_policy(policy, &self.policy, control)?;
        Ok(Self {
            policy,
            ..self.clone()
        })
    }
}

impl SafetyStrategy {
    #[must_use]
    pub fn arena(&self) -> &ActionArena {
        &self.arena
    }
    #[must_use]
    pub fn invariant(&self) -> &Event {
        &self.invariant
    }
    #[must_use]
    pub fn result(&self) -> &FixedPointResult {
        &self.fixed
    }
    #[must_use]
    pub fn winning(&self) -> &Event {
        self.fixed.event()
    }
    #[must_use]
    pub fn policy(&self) -> &WorldRelation {
        &self.policy
    }

    /// Restrict choices while retaining an enabled safe action at every winning
    /// state. The selected policy may remain nondeterministic.
    /// # Errors
    /// Refuses wrong roles, unsafe/partial choices, resources or cancellation.
    pub fn with_policy(&self, policy: &WorldRelation, control: &dyn Control) -> Result<Self> {
        let policy = checked_policy(policy, &self.policy, control)?;
        Ok(Self {
            policy,
            ..self.clone()
        })
    }
}

impl RelationalProduct {
    /// Largest one-step permission relation on inhabited information cases.
    /// Plan roles are S→O, O→A, S→A; `information` is I:O→S and `good` is S→A.
    /// Good must include actual action enabledness, e.g. `ActionArena::good`.
    /// All hidden states in a case share one action; empty cases permit none.
    /// # Errors
    /// Refuses wrong roles/environments, cancellation or resources.
    pub fn uniform_permissions(
        &self,
        information: &WorldRelation,
        good: &WorldRelation,
        control: &dyn Control,
    ) -> Result<WorldRelation> {
        let information = information
            .converse()
            .in_product(self.products()[0], control)?;
        let permissions = self.left_residual(&information, good, control)?;
        let inhabited = information.range(control)?;
        let gate = permissions
            .product()
            .left()
            .map()
            .pullback(&inhabited, control)?;
        WorldRelation::new(
            permissions.product(),
            &permissions.region().apply(BoolOp4::AND, &gate, control)?,
            control,
        )
    }
}
