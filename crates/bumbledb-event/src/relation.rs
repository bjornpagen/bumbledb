//! Typed views over the same owned Event carrier. Products admit all compatible
//! endpoint states; relations add behavioral restrictions inside those products.
use crate::product::require_same_map;
use crate::{
    BoolOp4, Control, CoordinateMap, Error, Event, FaceProduct, FibreProduct, Result, Space,
    SpaceId, SurjectiveMap,
};

/// A relation on two legal endpoint faces of a checked full fibre product.
/// Its Event remains inspectable and usable in the ordinary region algebra.
#[derive(Debug, Clone)]
pub struct WorldRelation {
    product: FibreProduct,
    region: Event,
}

/// Prepared lift/conjoin/project maps for S→T, T→U and S→U. One finite
/// three-face workspace preserves one T witness and one shared environment.
/// Composition and both residuals use exactly the same admitted maps.
///
/// ```
/// use bumbledb_event::{CoordinateMap, Error, FibreProduct, RelationalProduct,
///     Space, SpaceId, WorldRelation};
/// let environment = Space::new(SpaceId([1; 32]), 0, &())?;
/// let states = Space::new(SpaceId([2; 32]), 1, &())?;
/// let base = CoordinateMap::new(&states, &environment, &[], &())?
///     .certify_surjective(&())?;
/// let pairs = FibreProduct::new(SpaceId([3; 32]), &base, &base, &())?;
/// let plan = RelationalProduct::new(SpaceId([4; 32]), &pairs, &pairs, &pairs, &())?;
/// let transition = WorldRelation::new(&pairs, &pairs.space().full(), &())?;
/// let permitted = pairs.right().map().pullback(&states.coordinate(0, &())?, &())?;
/// let bound = WorldRelation::new(&pairs, &permitted, &())?;
/// let continuation = plan.left_residual(&transition, &bound, &())?;
/// assert!(plan.compose(&transition, &continuation, &())?.included_in(&bound, &())?);
/// # Ok::<(), Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct RelationalProduct {
    st: FibreProduct,
    tu: FibreProduct,
    su: FibreProduct,
    workspace: FaceProduct,
    left_view: SurjectiveMap,
    right_view: SurjectiveMap,
    result_view: SurjectiveMap,
}

impl WorldRelation {
    /// Adopt a region on the exact pair context; every coordinate is a declared
    /// endpoint coordinate. Use `from_lifted` to check roles in a larger space.
    /// # Errors
    /// Refuses a context mismatch, cancellation or unavailable resources.
    pub fn new(product: &FibreProduct, region: &Event, control: &dyn Control) -> Result<Self> {
        Ok(Self {
            product: product.clone(),
            region: region.align_to(product.space(), control)?,
        })
    }

    /// Certify that a lifted Event only reads its declared pair of faces.
    /// The onto pair view establishes full endpoint coverage; descent checks
    /// the separate membership FD, including on empty/full Events.
    /// # Errors
    /// Refuses a wrong pair target, an undeclared dependency, cancellation or capacity.
    pub fn from_lifted(
        product: &FibreProduct,
        pair_view: &SurjectiveMap,
        region: &Event,
        control: &dyn Control,
    ) -> Result<Self> {
        pair_view
            .map()
            .target()
            .full()
            .align_to(product.space(), control)?;
        Self::new(product, &pair_view.descend(region, control)?, control)
    }

    #[must_use]
    pub fn product(&self) -> &FibreProduct {
        &self.product
    }

    #[must_use]
    pub fn region(&self) -> &Event {
        &self.region
    }

    #[must_use]
    pub fn input(&self) -> &Space {
        self.product.left().map().target()
    }

    #[must_use]
    pub fn output(&self) -> &Space {
        self.product.right().map().target()
    }

    #[must_use]
    pub fn complement(&self) -> Self {
        Self {
            product: self.product.clone(),
            region: self.region.complement(),
        }
    }

    /// Exchange declared endpoint roles without rebuilding the Event.
    #[must_use]
    pub fn converse(&self) -> Self {
        Self {
            product: self.product.converse(),
            region: self.region.clone(),
        }
    }

    /// Reindex a relation into another checked presentation of the same roles.
    /// # Errors
    /// Refuses differing endpoint/environment meanings or unavailable resources.
    pub fn in_product(&self, target: &FibreProduct, control: &dyn Control) -> Result<Self> {
        let map = target.map_to(&self.product, control)?;
        Self::new(target, &map.pullback(&self.region, control)?, control)
    }

    /// Boolean construction after checking/reindexing both pair views.
    /// # Errors
    /// Refuses different roles/environments, cancellation or resource exhaustion.
    pub fn apply(&self, op: BoolOp4, other: &Self, control: &dyn Control) -> Result<Self> {
        let other = other.in_product(&self.product, control)?;
        Self::new(
            &self.product,
            &self.region.apply(op, &other.region, control)?,
            control,
        )
    }

    /// Supported relational inclusion, with explicit checked role reindexing.
    /// # Errors
    /// Refuses differing roles/environments, cancellation or resource exhaustion.
    pub fn included_in(&self, other: &Self, control: &dyn Control) -> Result<bool> {
        let other = other.in_product(&self.product, control)?;
        Ok(self.region.signature(&other.region, control)?.included())
    }

    /// Supported relation equality; probability and raw root equality are not used.
    /// # Errors
    /// Refuses differing roles/environments, cancellation or resource exhaustion.
    pub fn equivalent(&self, other: &Self, control: &dyn Control) -> Result<bool> {
        let other = other.in_product(&self.product, control)?;
        self.region.equivalent(&other.region)
    }

    /// # Errors
    /// Refuses cancellation or resource exhaustion.
    pub fn domain(&self, control: &dyn Control) -> Result<Event> {
        self.product.left().map().image(&self.region, control)
    }

    /// # Errors
    /// Refuses cancellation or resource exhaustion.
    pub fn range(&self, control: &dyn Control) -> Result<Event> {
        self.product.right().map().image(&self.region, control)
    }

    /// Source states with at least one related target in the supplied Event.
    /// # Errors
    /// Refuses a target mismatch, cancellation or unavailable resources.
    pub fn may(&self, target: &Event, control: &dyn Control) -> Result<Event> {
        let lifted = self.product.right().map().pullback(target, control)?;
        let selected = self.region.apply(BoolOp4::AND, &lifted, control)?;
        self.product.left().map().image(&selected, control)
    }

    /// Source states whose every successor satisfies the target, including dead ends.
    /// # Errors
    /// Has `may`'s context and resource contract.
    pub fn all(&self, target: &Event, control: &dyn Control) -> Result<Event> {
        Ok(self.may(&target.complement(), control)?.complement())
    }

    /// Universal guarantee with a required successor.
    /// # Errors
    /// Has `may`'s context and resource contract.
    pub fn must(&self, target: &Event, control: &dyn Control) -> Result<Event> {
        let all = self.all(target, control)?;
        self.domain(control)?.apply(BoolOp4::AND, &all, control)
    }

    /// Target states with a predecessor in the supplied source Event.
    /// # Errors
    /// Refuses a source mismatch, cancellation or unavailable resources.
    pub fn post(&self, source: &Event, control: &dyn Control) -> Result<Event> {
        self.converse().may(source, control)
    }

    /// Construct a total function's graph inside a compatible full pair product.
    /// Environment preservation is checked before the product mask is applied;
    /// masking must not turn a bad total function into a partial relation.
    /// # Errors
    /// Refuses endpoint/environment mismatches, cancellation or unavailable resources.
    pub fn graph(
        product: &FibreProduct,
        map: &CoordinateMap,
        control: &dyn Control,
    ) -> Result<Self> {
        map.source()
            .full()
            .align_to(product.left().map().target(), control)?;
        map.target()
            .full()
            .align_to(product.right().map().target(), control)?;
        let environment = map.then(product.right_environment().map(), control)?;
        require_same_map(&environment, product.left_environment().map(), control)?;
        let mut region = product.space().full();
        for (readout, target) in map.readouts().iter().zip(product.right().map().readouts()) {
            let value = product.left().map().pullback(readout, control)?;
            let equal = value.apply(BoolOp4::EQUIVALENCE, target, control)?;
            region = region.apply(BoolOp4::AND, &equal, control)?;
        }
        Self::new(product, &region, control)
    }

    /// Symbolic diagonal, without enumerating endpoint states.
    /// # Errors
    /// Refuses different endpoint/environment spaces, cancellation or exhaustion.
    pub fn identity(product: &FibreProduct, control: &dyn Control) -> Result<Self> {
        require_same_map(
            product.left_environment().map(),
            product.right_environment().map(),
            control,
        )?;
        let identity = CoordinateMap::identity(product.left().map().target(), control)?;
        Self::graph(product, &identity, control)
    }

    /// Identity restricted to the supplied predicate on its source face.
    /// # Errors
    /// Has `identity`'s contract and checks the predicate's context.
    pub fn test(product: &FibreProduct, event: &Event, control: &dyn Control) -> Result<Self> {
        let selected = product.left().map().pullback(event, control)?;
        let identity = Self::identity(product, control)?;
        Self::new(
            product,
            &identity.region.apply(BoolOp4::AND, &selected, control)?,
            control,
        )
    }

    /// Recover a deterministic readout from a total functional relation. Each
    /// target bit is constructed with May; disjoint true/false preimages certify
    /// functionality without enumerating states or choosing arbitrary witnesses.
    /// # Errors
    /// Refuses missing outputs, multiple outputs for one input, cancellation or capacity.
    pub fn readout(&self, control: &dyn Control) -> Result<CoordinateMap> {
        if !self.domain(control)?.is_full() {
            return Err(Error::PartialRelation);
        }
        let mut readouts = Vec::new();
        readouts.try_reserve_exact(usize::from(self.output().dimensions()))?;
        for coordinate in 0..self.output().dimensions() {
            let bit = self.output().coordinate(coordinate, control)?;
            let high = self.may(&bit, control)?;
            let low = self.may(&bit.complement(), control)?;
            if !high.signature(&low, control)?.disjoint() {
                return Err(Error::NonFunctionalRelation);
            }
            readouts.push(high);
        }
        CoordinateMap::new(self.input(), self.output(), &readouts, control)
    }
}

impl RelationalProduct {
    /// Prepare all three pair roles on one full S,T,U workspace. Endpoint domains
    /// may differ in width and support. The caller names the workspace; no result
    /// invents a new source identity. The three faces share T and the environment.
    /// # Errors
    /// Refuses incompatible endpoints/environment maps, more than 62 workspace
    /// coordinates, cancellation or unavailable resources.
    pub fn new(
        workspace_identity: SpaceId,
        st: &FibreProduct,
        tu: &FibreProduct,
        su: &FibreProduct,
        control: &dyn Control,
    ) -> Result<Self> {
        require_same_map(
            st.right_environment().map(),
            tu.left_environment().map(),
            control,
        )?;
        require_same_map(
            st.left_environment().map(),
            su.left_environment().map(),
            control,
        )?;
        require_same_map(
            tu.right_environment().map(),
            su.right_environment().map(),
            control,
        )?;
        let workspace = FaceProduct::new(
            workspace_identity,
            &[
                st.left_environment().clone(),
                st.right_environment().clone(),
                tu.right_environment().clone(),
            ],
            control,
        )?;
        let [s, t, u] = workspace.projections() else {
            unreachable!("three declared faces")
        };
        let left_view = st
            .pair(s.map(), t.map(), control)?
            .certify_surjective(control)?;
        let right_view = tu
            .pair(t.map(), u.map(), control)?
            .certify_surjective(control)?;
        let result_view = su
            .pair(s.map(), u.map(), control)?
            .certify_surjective(control)?;
        Ok(Self {
            st: st.clone(),
            tu: tu.clone(),
            su: su.clone(),
            workspace,
            left_view,
            right_view,
            result_view,
        })
    }

    #[must_use]
    pub fn workspace(&self) -> &FaceProduct {
        &self.workspace
    }

    /// The three owned endpoint products in S×T, T×U, S×U order.
    #[must_use]
    pub fn products(&self) -> [&FibreProduct; 3] {
        [&self.st, &self.tu, &self.su]
    }

    /// Owned projections to S×T, T×U and S×U, respectively.
    #[must_use]
    pub fn views(&self) -> [&SurjectiveMap; 3] {
        [&self.left_view, &self.right_view, &self.result_view]
    }

    /// Existentially join two relations on their single shared middle state.
    /// # Errors
    /// Refuses role/environment mismatches, cancellation or unavailable resources.
    pub fn compose(
        &self,
        left: &WorldRelation,
        right: &WorldRelation,
        control: &dyn Control,
    ) -> Result<WorldRelation> {
        let left = left.in_product(&self.st, control)?;
        let right = right.in_product(&self.tu, control)?;
        let left = self.left_view.map().pullback(left.region(), control)?;
        let right = self.right_view.map().pullback(right.region(), control)?;
        let joined = left.apply(BoolOp4::AND, &right, control)?;
        WorldRelation::new(
            &self.su,
            &self.result_view.map().image(&joined, control)?,
            control,
        )
    }

    /// Largest Q:T→U with R;Q contained in V:S→U.
    /// # Errors
    /// Refuses role/environment mismatches, cancellation or unavailable resources.
    pub fn left_residual(
        &self,
        left: &WorldRelation,
        bound: &WorldRelation,
        control: &dyn Control,
    ) -> Result<WorldRelation> {
        let left = left.in_product(&self.st, control)?;
        let bound = bound.in_product(&self.su, control)?;
        let left = self.left_view.map().pullback(left.region(), control)?;
        let bound = self.result_view.map().pullback(bound.region(), control)?;
        let bad = left.apply(BoolOp4::DIFFERENCE, &bound, control)?;
        WorldRelation::new(
            &self.tu,
            &self.right_view.map().image(&bad, control)?.complement(),
            control,
        )
    }

    /// Largest R:S→T with R;Q contained in V:S→U.
    /// # Errors
    /// Refuses role/environment mismatches, cancellation or unavailable resources.
    pub fn right_residual(
        &self,
        bound: &WorldRelation,
        right: &WorldRelation,
        control: &dyn Control,
    ) -> Result<WorldRelation> {
        let bound = bound.in_product(&self.su, control)?;
        let right = right.in_product(&self.tu, control)?;
        let bound = self.result_view.map().pullback(bound.region(), control)?;
        let right = self.right_view.map().pullback(right.region(), control)?;
        let bad = right.apply(BoolOp4::DIFFERENCE, &bound, control)?;
        WorldRelation::new(
            &self.st,
            &self.left_view.map().image(&bad, control)?.complement(),
            control,
        )
    }
}
