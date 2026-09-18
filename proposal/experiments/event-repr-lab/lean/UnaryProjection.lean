import Std

namespace UnaryProjection

abbrev Reach := Bool × Bool
abbrev Transform := Bool × Bool
def lookup (p : Reach) (b : Bool) : Bool := if b then p.2 else p.1
def eval (t : Transform) (b : Bool) : Bool := if b then t.2 else t.1
def compose (t u : Transform) : Transform := (eval t u.1,eval t u.2)
def image (t : Transform) (p : Reach) : Reach :=
  ((p.1 && !t.1) || (p.2 && !t.2), (p.1 && t.1) || (p.2 && t.2))
def union (p q : Reach) : Reach := (p.1 || q.1,p.2 || q.2)
def domain (p : Reach) : Bool := p.1 || p.2
def bounds (p : Reach) : Bool × Bool := (p.2,!p.1)

/-- Reach stores possible false/true, including the empty fibre (false,false). -/
theorem image_exact (t : Transform) (p : Reach) (b : Bool) :
    lookup (image t p) b = true ↔ ∃ a, lookup p a = true ∧ eval t a = b := by
  rcases t with ⟨t0,t1⟩; rcases p with ⟨p0,p1⟩
  cases t0 <;> cases t1 <;> cases p0 <;> cases p1 <;> cases b <;> decide

theorem image_composes (t u : Transform) (p : Reach) :
    image t (image u p) = image (compose t u) p := by
  rcases t with ⟨t0,t1⟩; rcases u with ⟨u0,u1⟩; rcases p with ⟨p0,p1⟩
  cases t0 <;> cases t1 <;> cases u0 <;> cases u1 <;> cases p0 <;> cases p1 <;> decide

theorem image_empty (t : Transform) : image t (false,false) = (false,false) := by
  rcases t with ⟨a,b⟩; cases a <;> cases b <;> decide

theorem image_domain (t : Transform) (p : Reach) : domain (image t p) = domain p := by
  rcases t with ⟨a,b⟩; rcases p with ⟨c,d⟩
  cases a <;> cases b <;> cases c <;> cases d <;> decide

theorem image_negation (p : Reach) : image (true,false) p = (p.2,p.1) := by
  rcases p with ⟨a,b⟩; cases a <;> cases b <;> decide

/-- The occupancy calculation works on arbitrary fibres, including empty and
    infinite ones, once their two occupancy propositions have been represented. -/
theorem image_preserves_occupancy {α : Type} (f : α → Bool) (p : Reach)
    (correct : ∀ b, lookup p b = true ↔ ∃ y, f y = b) (t : Transform) (b : Bool) :
    lookup (image t p) b = true ↔ ∃ y, eval t (f y) = b := by
  rw [image_exact]
  constructor
  · rintro ⟨a,ha,ht⟩
    obtain ⟨y,hy⟩ := (correct a).mp ha
    exact ⟨y,by rw [hy]; exact ht⟩
  · rintro ⟨y,hy⟩
    exact ⟨f y,(correct (f y)).mpr ⟨y,rfl⟩,hy⟩

inductive Letter where
  | guard | guardNot | reverseGuard | saturate | xor
def act (l : Letter) (x g : Bool) : Bool :=
  match l with
  | .guard => x && g
  | .guardNot => x && !g
  | .reverseGuard => !x && g
  | .saturate => x || g
  | .xor => x ^^ g
def known (l : Letter) (x : Bool) (p : Reach) : Reach :=
  image (act l x false,act l x true) p
def hidden (l : Letter) (p : Reach) : Reach :=
  match l with
  | .guard | .reverseGuard => (domain p,p.2)
  | .guardNot => (domain p,p.1)
  | .saturate => (p.1,domain p)
  | .xor => (domain p,domain p)

theorem hidden_is_union (l : Letter) (p : Reach) :
    hidden l p = union (known l false p) (known l true p) := by
  rcases p with ⟨a,b⟩; cases l <;> cases a <;> cases b <;> decide

def knownBounds (l : Letter) (x : Bool) (may all : Bool) : Bool × Bool :=
  match l with
  | .guard => (x && may,x && all)
  | .guardNot => (x && !all,x && !may)
  | .reverseGuard => (!x && may,!x && all)
  | .saturate => (x || may,x || all)
  | .xor => (if x then !all else may, if x then !may else all)
def hiddenBounds (l : Letter) (may all : Bool) : Bool × Bool :=
  match l with
  | .guard | .reverseGuard => (may,false)
  | .guardNot => (!all,false)
  | .saturate => (true,all)
  | .xor => (true,false)

/-- Fast bound steps require an inhabited fibre. The raw Boolean cube supplies
    it; arbitrary supported observation fibres need their own certificate. -/
theorem known_nonempty (l : Letter) (x : Bool) (p : Reach) (h : domain p = true) :
    bounds (known l x p) = knownBounds l x (bounds p).1 (bounds p).2 := by
  rcases p with ⟨a,b⟩; cases a <;> cases b <;> cases l <;> cases x <;>
    simp_all [domain,bounds,known,image,act,knownBounds]

theorem hidden_nonempty (l : Letter) (p : Reach) (h : domain p = true) :
    bounds (hidden l p) = hiddenBounds l (bounds p).1 (bounds p).2 := by
  rcases p with ⟨a,b⟩; cases a <;> cases b <;> cases l <;>
    simp_all [domain,bounds,hidden,hiddenBounds]

theorem empty_fibre_obstruction :
    bounds (known .saturate true (false,false)) = (false,true) ∧
    knownBounds .saturate true false true = (true,true) := by decide

def adjusted : Letter → Letter
  | .guard => .guardNot
  | .guardNot => .guard
  | .reverseGuard => .saturate
  | .saturate => .reverseGuard
  | .xor => .xor
def outerFlip : Letter → Bool
  | .guard | .guardNot => false
  | _ => true

theorem normalize_complement (l : Letter) (x g : Bool) :
    act l x (!g) = (act (adjusted l) x g ^^ outerFlip l) := by
  cases l <;> cases x <;> cases g <;> decide

end UnaryProjection

#print axioms UnaryProjection.image_exact
#print axioms UnaryProjection.image_composes
#print axioms UnaryProjection.image_empty
#print axioms UnaryProjection.image_domain
#print axioms UnaryProjection.image_negation
#print axioms UnaryProjection.image_preserves_occupancy
#print axioms UnaryProjection.hidden_is_union
#print axioms UnaryProjection.known_nonempty
#print axioms UnaryProjection.hidden_nonempty
#print axioms UnaryProjection.empty_fibre_obstruction
#print axioms UnaryProjection.normalize_complement
