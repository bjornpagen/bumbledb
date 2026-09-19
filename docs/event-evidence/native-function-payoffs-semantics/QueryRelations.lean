import Std

/-!
Representation independence of typed relation query programs. World s is the
already-admitted legal domain of face s, at ONE fixed shared environment. Apply
these results separately in each environment fibre; they do not license replacing
a joint product by its marginals. Native role admission must establish those
domains, original support, actual environment maps and legal-code bijections.

The program is indexed by endpoint roles. Rust's shape checker is not extracted
from it. Bound Event expressions instantiate the atom/predicate denotations;
scope admission and syntactic demand collection remain separate obligations.
-/
namespace QueryRelations

structure View (A B : Type) where
  encode : A → B
  decode : B → A
  decode_encode : ∀ a, decode (encode a) = a
  encode_decode : ∀ b, encode (decode b) = b

abbrev Rel (A B : Type) := A → B → Prop

def transport {A B X Y : Type} (a : View A X) (b : View B Y) (r : Rel A B) : Rel X Y :=
  fun x y => r (a.decode x) (b.decode y)

def compose {A B C : Type} (r : Rel A B) (q : Rel B C) : Rel A C :=
  fun a c => ∃ b, r a b ∧ q b c

def leftResidual {A B C : Type} (r : Rel A B) (v : Rel A C) : Rel B C :=
  fun b c => ∀ a, r a b → v a c

def rightResidual {A B C : Type} (v : Rel A C) (q : Rel B C) : Rel A B :=
  fun a b => ∀ c, q b c → v a c

variable {A B C X Y Z : Type}

theorem compose_transport (a : View A X) (b : View B Y) (c : View C Z)
    (r : Rel A B) (q : Rel B C) (x : X) (z : Z) :
    compose (transport a b r) (transport b c q) x z ↔ transport a c (compose r q) x z := by
  constructor
  · rintro ⟨y, hr, hq⟩; exact ⟨b.decode y, hr, hq⟩
  · rintro ⟨y, hr, hq⟩
    exact ⟨b.encode y, by simpa [transport, b.decode_encode] using hr,
      by simpa [transport, b.decode_encode] using hq⟩

theorem left_residual_transport (a : View A X) (b : View B Y) (c : View C Z)
    (r : Rel A B) (v : Rel A C) (y : Y) (z : Z) :
    leftResidual (transport a b r) (transport a c v) y z ↔
      transport b c (leftResidual r v) y z := by
  constructor
  · intro h u good
    have result := h (a.encode u) (by simpa [transport, a.decode_encode] using good)
    simpa [transport, a.decode_encode] using result
  · intro h x good; exact h (a.decode x) good

theorem right_residual_transport (a : View A X) (b : View B Y) (c : View C Z)
    (v : Rel A C) (q : Rel B C) (x : X) (y : Y) :
    rightResidual (transport a c v) (transport b c q) x y ↔
      transport a b (rightResidual v q) x y := by
  constructor
  · intro h u good
    have result := h (c.encode u) (by simpa [transport, c.decode_encode] using good)
    simpa [transport, c.decode_encode] using result
  · intro h z good; exact h (c.decode z) good

theorem identity_transport (a : View A X) (x y : X) : x = y ↔ a.decode x = a.decode y := by
  constructor
  · intro h; cases h; rfl
  · intro h
    calc x = a.encode (a.decode x) := (a.encode_decode x).symm
         _ = a.encode (a.decode y) := congrArg a.encode h
         _ = y := a.encode_decode y

inductive Path {A : Type} (r : Rel A A) : A → A → Prop where
  | refl a : Path r a a
  | step {a b c} : r a b → Path r b c → Path r a c

theorem path_encode (a : View A X) (r : Rel A A) {s t : A} (path : Path r s t) :
    Path (transport a a r) (a.encode s) (a.encode t) := by
  induction path with
  | refl _ => exact .refl _
  | step edge _ ih => exact .step (by simpa [transport, a.decode_encode] using edge) ih

theorem star_transport (a : View A X) (r : Rel A A) (x y : X) :
    Path (transport a a r) x y ↔ Path r (a.decode x) (a.decode y) := by
  constructor
  · intro path
    induction path with
    | refl _ => exact .refl _
    | step edge _ ih => exact .step edge ih
  · intro path
    simpa [a.encode_decode] using path_encode a r path

inductive Program (Scope : Type) (Atom : Scope → Scope → Type) (Predicate : Scope → Type) :
    Scope → Scope → Type where
  | atom {s t} (value : Atom s t) : Program Scope Atom Predicate s t
  | identity s : Program Scope Atom Predicate s s
  | test {s} (value : Predicate s) : Program Scope Atom Predicate s s
  | neg {s t} : Program Scope Atom Predicate s t → Program Scope Atom Predicate s t
  | apply {s t} (op : Prop → Prop → Prop) : Program Scope Atom Predicate s t →
      Program Scope Atom Predicate s t → Program Scope Atom Predicate s t
  | converse {s t} : Program Scope Atom Predicate s t → Program Scope Atom Predicate t s
  | compose {s t u} : Program Scope Atom Predicate s t → Program Scope Atom Predicate t u →
      Program Scope Atom Predicate s u
  | leftResidual {s t u} : Program Scope Atom Predicate s t → Program Scope Atom Predicate s u →
      Program Scope Atom Predicate t u
  | rightResidual {s t u} : Program Scope Atom Predicate s u → Program Scope Atom Predicate t u →
      Program Scope Atom Predicate s t
  | star {s} : Program Scope Atom Predicate s s → Program Scope Atom Predicate s s

variable {S : Type} {Atom : S → S → Type} {Predicate : S → Type}

def eval (W : S → Type) (atoms : ∀ s t, Atom s t → Rel (W s) (W t))
    (predicates : ∀ s, Predicate s → W s → Prop) :
    {s t : S} → Program S Atom Predicate s t → Rel (W s) (W t)
  | s, t, .atom a => atoms s t a
  | _, _, .identity _ => Eq
  | s, _, .test p => fun x y => x = y ∧ predicates s p x
  | _, _, .neg r => fun x y => ¬ eval W atoms predicates r x y
  | _, _, .apply op r q => fun x y => op (eval W atoms predicates r x y) (eval W atoms predicates q x y)
  | _, _, .converse r => fun x y => eval W atoms predicates r y x
  | _, _, .compose r q => compose (eval W atoms predicates r) (eval W atoms predicates q)
  | _, _, .leftResidual r v => leftResidual (eval W atoms predicates r) (eval W atoms predicates v)
  | _, _, .rightResidual v q => rightResidual (eval W atoms predicates v) (eval W atoms predicates q)
  | _, _, .star r => Path (eval W atoms predicates r)

theorem whole_program_transport (W V : S → Type) (view : ∀ s, View (W s) (V s))
    (atoms : ∀ s t, Atom s t → Rel (W s) (W t)) (predicates : ∀ s, Predicate s → W s → Prop)
    {s t} (e : Program S Atom Predicate s t) :
    eval V (fun s t r => transport (view s) (view t) (atoms s t r))
        (fun s p x => predicates s p ((view s).decode x)) e =
      transport (view s) (view t) (eval W atoms predicates e) := by
  induction e with
  | atom _ => rfl
  | identity s => funext x y; exact propext (identity_transport (view s) x y)
  | test p =>
      funext x y
      apply propext
      simp only [eval, transport]
      exact and_congr (identity_transport (view _) x y) Iff.rfl
  | neg r ih => simp only [eval, ih]; rfl
  | apply op r q ir iq => simp only [eval, ir, iq]; rfl
  | converse r ih => simp only [eval, ih]; rfl
  | compose r q ir iq =>
      simp only [eval, ir, iq]
      funext x z; exact propext (compose_transport _ _ _ _ _ x z)
  | leftResidual r v ir iv =>
      simp only [eval, ir, iv]
      funext y z; exact propext (left_residual_transport _ _ _ _ _ y z)
  | rightResidual v q iv iq =>
      simp only [eval, iv, iq]
      funext x y; exact propext (right_residual_transport _ _ _ _ _ x y)
  | star r ih =>
      simp only [eval, ih]
      funext x y; exact propext (star_transport _ _ x y)

def may {A B : Type} (r : Rel A B) (p : B → Prop) (a : A) := ∃ b, r a b ∧ p b
def all {A B : Type} (r : Rel A B) (p : B → Prop) (a : A) := ∀ b, r a b → p b
def must {A B : Type} (r : Rel A B) (p : B → Prop) (a : A) :=
  (∃ b, r a b) ∧ all r p a

theorem may_transport (a : View A X) (b : View B Y) (r : Rel A B) (p : B → Prop) (x : X) :
    may (transport a b r) (fun y => p (b.decode y)) x ↔ may r p (a.decode x) := by
  constructor
  · rintro ⟨y, hr, hp⟩; exact ⟨b.decode y, hr, hp⟩
  · rintro ⟨y, hr, hp⟩
    exact ⟨b.encode y, by simpa [transport, b.decode_encode] using hr,
      by simpa [b.decode_encode] using hp⟩

theorem all_transport (a : View A X) (b : View B Y) (r : Rel A B) (p : B → Prop) (x : X) :
    all (transport a b r) (fun y => p (b.decode y)) x ↔ all r p (a.decode x) := by
  constructor
  · intro h y hr
    have result := h (b.encode y) (by simpa [transport, b.decode_encode] using hr)
    simpa [b.decode_encode] using result
  · intro h y hr; exact h (b.decode y) hr

theorem must_transport (a : View A X) (b : View B Y) (r : Rel A B) (p : B → Prop) (x : X) :
    must (transport a b r) (fun y => p (b.decode y)) x ↔ must r p (a.decode x) := by
  have domain := may_transport a b r (fun _ => True) x
  simp only [may, and_true] at domain
  simp only [must, domain, all_transport]

theorem dead_end_is_not_must : all (fun _ _ : Unit => False) (fun _ => False) () ∧
    ¬ must (fun _ _ : Unit => False) (fun _ => False) () := by
  simp [all, must]

#print axioms QueryRelations.compose_transport
#print axioms QueryRelations.left_residual_transport
#print axioms QueryRelations.right_residual_transport
#print axioms QueryRelations.identity_transport
#print axioms QueryRelations.path_encode
#print axioms QueryRelations.star_transport
#print axioms QueryRelations.whole_program_transport
#print axioms QueryRelations.may_transport
#print axioms QueryRelations.all_transport
#print axioms QueryRelations.must_transport
#print axioms QueryRelations.dead_end_is_not_must

end QueryRelations
