import Bumbledb.Conformance
import Bumbledb.Decide
import Bumbledb.Exec.Fixpoint

/-!
# The conformance driver (PRD 13; judgment + recursive arms) — the IO shell

`lake exe conformance [cases-dir]`: read every `*.json` case, dispatch
on the case kind, and compare against the recorded engine verdicts.
Exit 0 on full agreement; exit 1 with the offending case files named
otherwise — a disagreement is a finding (engine bug / naive-model bug /
spec bug), triaged before anything else merges, never repaired here.

Three arms, dispatched by file name (`lean/conformance/README.md`):

* **query cases** (everything else): evaluate the denotation
  (`Bumbledb.Conformance.checkCase` — `evalList` under `eval_sound`,
  plus the recorded aggregate glue) and compare answer sets.
* **judgment cases** (`judgment-*.json`): decode `(theory, instance,
  delta)`, apply the delta by row-set arithmetic, and run the PROVED
  two-phase judge — `Txn.judgeB` (`Bumbledb/Decide.lean`), which
  agrees with the model's `Txn.judge` verdict and violation sets phase
  for phase (`Txn.judgeB_agrees`, under no premise beyond the
  closed-roster merge). The recorded verdict is compared WHOLE:
  accept, or the rejecting phase plus the per-phase violation set as
  statement indices, in the contracted citation order — ascending
  statement indices, a both-directions containment cited once
  (`RVerdict`'s doc carries the contract and its engine anchors). The
  decode glue below is IO-shell material (this file), never spec:
  `verdictOf` is one pattern match on `judgeB`'s position-tagged
  payload — the compared citation list IS the proved artifact's index
  projection (2026-07-23 audit, finding 143).
* **program cases** (`program-*.json`, the RECURSIVE arm — the
  shipping law: the oracles landed before the evaluator,
  `docs/architecture/60-validation.md` § the two oracles): decode the
  program cut
  (`Bumbledb/Query/Syntax.lean`: `Program`/`PredicateDef`/`PRule`) and
  the recorded stratification witness, run the PROVED fueled fixpoint
  `Query.evalProgram` (`Bumbledb/Exec/Fixpoint.lean`;
  `program_eval_sound` is its agreement with the stratified
  denotation), and compare answer sets against the naive-and-SQLite
  agreed answers the Rust builder recorded — the third oracle judging
  recursion before any engine driver exists.

This file is the ONE place PRD 13 allows `partial` definitions; the
modules below happen to need none — the loops are `for`s over finite
file lists and structural recursions over decoded JSON.
-/

open Bumbledb.Conformance

namespace Bumbledb
namespace JudgmentCase

open Lean (Json)
open Conformance

/-! ## Decoding — the judgment interchange format into the tree's types -/

/-- One stored row (the conformance lane's fact shape, kept as data —
`Decide.lean`'s `Row` carrier). -/
def decodeRow (j : Json) : Except String Row := do
  (← j.getArr?).toList.mapM decodeValue

/-- One relation block of rows (`{relation, facts}` — the instance's,
the ground axioms', and the delta's shared shape). -/
def decodeRelationRows (j : Json) : Except String (RelId × List Row) := do
  let relation ← natKey j "relation"
  let facts ← (← (← j.getObjVal? "facts").getArr?).toList.mapM decodeRow
  return (⟨relation⟩, facts)

/-- A field-type spelling (the query lane's `relations` block spelling,
reused verbatim). -/
def typeOfName (s : String) : Except String ValueType :=
  match s with
  | "bool" => .ok .bool
  | "u64" => .ok .u64
  | "i64" => .ok .i64
  | "str" => .ok .str
  | "interval_u64" => .ok (.interval .u64)
  | "interval_i64" => .ok (.interval .i64)
  | _ =>
    if s.startsWith "bytes<" && s.endsWith ">" then
      match ((s.drop 6).dropEnd 1).toNat? with
      | some n => .ok (.fixedBytes n)
      | none => .error s!"bad bytes width in {s}"
    -- `interval<E, w>`: the width is the type — the `bytes<N>`
    -- spelling precedent, generalized (`w = 0` denotes nothing and is
    -- refused exactly as the macro grammar refuses it).
    else if s.startsWith "interval_u64_fixed<" && s.endsWith ">" then
      match ((s.drop 19).dropEnd 1).toNat? with
      | some w => if 0 < w then .ok (.intervalFixed .u64 w)
                  else .error s!"zero width in {s}"
      | none => .error s!"bad interval width in {s}"
    else if s.startsWith "interval_i64_fixed<" && s.endsWith ">" then
      match ((s.drop 19).dropEnd 1).toNat? with
      | some w => if 0 < w then .ok (.intervalFixed .i64 w)
                  else .error s!"zero width in {s}"
      | none => .error s!"bad interval width in {s}"
    else .error s!"unknown field type {s}"

/-- One declared relation: id, closedness, sealed positional types (a
closed relation's list opens with the synthetic id, exactly as the
serializer writes it). -/
structure JRelation where
  /-- The relation id. -/
  id : Nat
  /-- Whether the relation is closed (its rows are ground axioms). -/
  closed : Bool
  /-- The sealed positional field types. -/
  fields : List ValueType

def decodeRelation (j : Json) : Except String JRelation := do
  let id ← natKey j "id"
  let closed ← (← j.getObjVal? "closed").getBool?
  let fields ← (← (← j.getObjVal? "fields").getArr?).toList.mapM
    fun t => do typeOfName (← t.getStr?)
  return { id, closed, fields }

/-- A field-id array. -/
def decodeFieldIds (j : Json) : Except String (List FieldId) := do
  (← j.getArr?).toList.mapM fun n => do
    pure (⟨← n.getNat?⟩ : FieldId)

/-- A σ: `[[field, [literal…]]…]` — the disjunctive literal sets,
`Selection.bindings`' own shape. -/
def decodeSelection (j : Json) : Except String Selection := do
  let bindings ← (← j.getArr?).toList.mapM fun pair => do
    match (← pair.getArr?).toList with
    | [f, vs] =>
      return ((⟨← f.getNat?⟩ : FieldId),
        ← (← vs.getArr?).toList.mapM decodeValue)
    | _ => Except.error "selection binding expects [field, [literals]]"
  return ⟨bindings⟩

/-- One statement side (`Bumbledb.Atom` — the schema atom, not the
query atom). -/
def decodeSide (j : Json) : Except String Bumbledb.Atom := do
  return { relation := ⟨← natKey j "relation"⟩,
           projection := ← decodeFieldIds (← j.getObjVal? "projection"),
           selection := ← decodeSelection (← j.getObjVal? "selection") }

/-- A window: `hi` absent is the `*` spelling. -/
def decodeWindow (j : Json) : Except String Window := do
  let lo ← natKey j "lo"
  match objKey? j "hi" with
  | some h => return ⟨lo, some (← h.getNat?)⟩
  | none => return ⟨lo, none⟩

/-- One declared statement, in the materialized order the file pins
(indices ARE the engine's statement ids). -/
def decodeStatement (j : Json) : Except String Statement := do
  if let some f := objKey? j "functionality" then
    return .functionality ⟨← natKey f "relation"⟩
      (← decodeFieldIds (← f.getObjVal? "projection"))
  if let some c := objKey? j "containment" then
    return .containment (← decodeSide (← c.getObjVal? "source"))
      (← decodeSide (← c.getObjVal? "target"))
  if let some c := objKey? j "cardinality" then
    return .cardinality (← decodeSide (← c.getObjVal? "source"))
      (← decodeWindow (← c.getObjVal? "window"))
      (← decodeSide (← c.getObjVal? "target"))
  .error "unknown statement form"

/-- The recorded engine verdict: accept, or the rejecting phase with
its complete violation set as statement indices. **The citation-order
contract** (the verdict is compared WHOLE — list `BEq` — so the order
is normative, not incidental): indices ascend in materialized-statement
order, and each violated statement of the failing phase is cited
exactly once — a containment violated in BOTH directions is ONE index.
The engine side: `storage/commit/judgment.rs::judge` collects the
phase's finds and the sealing constructor
(`error.rs::Violations::seal`) stable-sorts by the citation key
(`error.rs::Violation::citation` — statement id, then direction,
source before target) and dedups by it; the fixture writer's index
projection (`conformance/judgment.rs::lane_verdict`) maps each
citation to its statement index and collapses adjacent duplicates —
ascending citation order keeps a containment's two directions
adjacent, so the collapse is total. `verdictOf` below meets the same
contract by construction. -/
inductive RVerdict where
  /-- The commit was accepted. -/
  | accept
  /-- The commit was rejected in one phase with the complete cited
  statement-index set of that phase. -/
  | reject (keyPhase : Bool) (violations : List Nat)
deriving BEq

def decodeVerdict (j : Json) : Except String RVerdict := do
  if let .ok s := j.getStr? then
    if s == "accept" then return .accept
    .error s!"unknown verdict {s}"
  let r ← j.getObjVal? "reject"
  let keyPhase ← match (← (← r.getObjVal? "phase").getStr?) with
    | "key" => pure true
    | "statement" => pure false
    | other => Except.error s!"unknown phase {other}"
  let violations ← (← (← r.getObjVal? "violations").getArr?).toList.mapM
    (·.getNat?)
  return .reject keyPhase violations

/-- One decoded judgment case. -/
structure JCase where
  /-- The theory (header from the relations block, closed rosters from
  the ground axioms, the materialized statement list). -/
  theory : Theory
  /-- The pre-state world: open instance rows plus the ground-axiom
  rows (the `WorldCarriesClosed` merge, the query lane's own rule). -/
  world : List (RelId × List Row)
  /-- The delta's deletes, per relation. -/
  deletes : List (RelId × List Row)
  /-- The delta's inserts, per relation. -/
  inserts : List (RelId × List Row)
  /-- The recorded engine verdict. -/
  expected : RVerdict

/-- One full judgment-case document. -/
def decodeJCase (j : Json) : Except String JCase := do
  let theory := (← j.getObjVal? "theory")
  let rels ← (← (← theory.getObjVal? "relations").getArr?).toList.mapM
    decodeRelation
  let axioms ← (← (← theory.getObjVal? "ground_axioms").getArr?)
    |>.toList.mapM decodeRelationRows
  let statements ← (← (← theory.getObjVal? "statements").getArr?)
    |>.toList.mapM decodeStatement
  let instance_ ← (← (← j.getObjVal? "instance").getArr?).toList.mapM
    decodeRelationRows
  let delta := (← j.getObjVal? "delta")
  let deletes ← (← (← delta.getObjVal? "deletes").getArr?).toList.mapM
    decodeRelationRows
  let inserts ← (← (← delta.getObjVal? "inserts").getArr?).toList.mapM
    decodeRelationRows
  let expected ← decodeVerdict (← j.getObjVal? "verdict")
  let header : Header :=
    ⟨fun R => match rels.find? (fun r => r.id == R.id) with
      | some r => r.fields
      | none => []⟩
  let closedRows : List (Nat × List Row) :=
    axioms.map fun (R, rows) => (R.id, rows)
  let closed : RelId → Option GroundExtension := fun R =>
    match closedRows.find? (fun e => e.1 == R.id) with
    | some e => some ⟨e.2.map Query.tupleFact⟩
    | none => none
  return { theory := { header, closed, statements },
           world := instance_ ++ axioms, deletes, inserts, expected }

/-! ## The delta, applied — row-set arithmetic -/

/-- The rows a relation carries in a block list (all blocks merged —
a relation may appear once; missing is empty). -/
def rowsAt (blocks : List (RelId × List Row)) (R : RelId) : List Row :=
  (blocks.filter fun e => e.1 == R).flatMap (·.2)

/-- Set-semantics removal: every row denoting a deleted fact goes. -/
def removeAll (rows dels : List Row) : List Row :=
  rows.filter fun r => !dels.any fun d => rowEqB r d

/-- Set-semantics insertion: a present fact's re-insert is a no-op. -/
def addAll (rows ins : List Row) : List Row :=
  ins.foldl
    (fun acc r => if acc.any (fun x => rowEqB x r) then acc else acc ++ [r])
    rows

/-- First-occurrence deduplication of the touched relation ids. -/
def dedupIds : List RelId → List RelId
  | [] => []
  | R :: rest =>
    if rest.any fun S => S == R then dedupIds rest
    else R :: dedupIds rest

/-- The candidate final state: per touched or carried relation,
deletes removed then inserts added — `NaiveDb::staged`'s arithmetic,
row-level. -/
def finalWorld (c : JCase) : RowInstance :=
  let ids := dedupIds
    ((c.world ++ c.deletes ++ c.inserts).map (·.1))
  ⟨ids.map fun R =>
    (R, addAll (removeAll (rowsAt c.world R) (rowsAt c.deletes R))
      (rowsAt c.inserts R))⟩

/-! ## The verdict — `judgeB`'s payload, projected -/

/-- The executable judge's verdict with citation indices: ONE pattern
match on the PROVED artifact — `judgeB` (`judgeB_agrees`) carries the
phase and the position-tagged citations from birth, and the compared
index list is its payload's position projection, re-derived nowhere.
The filter over the index-paired statement list ascends and cites
each statement at most once, so this side meets the citation-order
contract (`RVerdict`'s doc) by construction. -/
def verdictOf (T : Theory) (W : RowInstance) : RVerdict :=
  match Txn.judgeB T W with
  | none => .accept
  | some (keyPhase, cited) => .reject keyPhase (cited.map (·.2))

/-- A verdict, rendered for the mismatch report. -/
def renderVerdict : RVerdict → String
  | .accept => "accept"
  | .reject keyPhase violations =>
    s!"reject {cond keyPhase "key" "statement"} {violations}"

/-- One judgment case, end to end: decode, apply the delta, judge,
compare whole. `none` is agreement; `some (expected, judged)` is a
disagreement (a trophy — the caller reports, never repairs). -/
def checkJudgmentCase (text : String) :
    Except String (Option (String × String)) := do
  let json ← Json.parse text
  let c ← decodeJCase json
  let judged := verdictOf c.theory (finalWorld c)
  if judged == c.expected then
    return none
  return some (renderVerdict c.expected, renderVerdict judged)

end JudgmentCase

namespace ProgramCase

open Lean (Json)
open Conformance

/-! ## Decoding — the program interchange format into the program cut -/

/-- One program atom: the source arm spelled (`edb`/`idb`), bindings as
the query lane's `[field, term]` pairs. -/
def decodePAtom (j : Json) : Except String Query.PAtom := do
  let source : Query.AtomSource ←
    if let some r := objKey? j "edb" then
      pure (.edb ⟨← r.getNat?⟩)
    else if let some p := objKey? j "idb" then
      pure (.idb ⟨← p.getNat?⟩)
    else .error "program atom expects edb or idb"
  let bindings ← (← (← j.getObjVal? "bindings").getArr?).toList.mapM
    fun pair => do
      match (← pair.getArr?).toList with
      | [f, t] => return ((⟨← f.getNat?⟩ : FieldId), ← decodeTerm t)
      | _ => Except.error "binding expects [field, term]"
  return { source, bindings }

/-- One program rule — the head is the plain variable-id list
(`PRule.finds : List VarId`; the program cut is projection-shaped, so
the corpus carries no fold heads). -/
def decodePRule (j : Json) : Except String Query.PRule := do
  let finds ← (← (← j.getObjVal? "finds").getArr?).toList.mapM
    fun n => do pure (⟨← n.getNat?⟩ : Query.VarId)
  let atoms ← (← (← j.getObjVal? "atoms").getArr?).toList.mapM decodePAtom
  let negated ←
    (← (← j.getObjVal? "negated").getArr?).toList.mapM decodePAtom
  let conditions ← (← (← j.getObjVal? "conditions").getArr?).toList.mapM
    (decodeCondition 64)
  return { finds, atoms, negated, conditions }

/-- One predicate: head arity plus deriving rules. -/
def decodePredicate (j : Json) : Except String Query.PredicateDef := do
  return { arity := ← natKey j "arity",
           rules := ← (← (← j.getObjVal? "rules").getArr?).toList.mapM
             decodePRule }

/-- One decoded program case: the world (open instance + ground axioms
merged, the query lane's own rule), the program, the recorded
stratification witness (the Rust side computes ONE witness; the
denotation is witness-independent — the recorded narrowing in
`Bumbledb/Exec/Fixpoint.lean`), the parameters, and the agreed
answers. -/
structure PCase where
  /-- The world the program's `edb` atoms read. -/
  world : Query.ListInstance
  /-- The program cut. -/
  program : Query.Program
  /-- The recorded stratification witness, `strata[p]` = predicate
  `p`'s stratum. -/
  strata : List Nat
  /-- The positional parameter environment. -/
  env : Query.ParamEnv
  /-- The recorded agreed answers (naive; SQLite-attested where the
  `WITH RECURSIVE` gate admits). -/
  expected : List (List Value)

/-- One full program-case document. -/
def decodePCase (j : Json) : Except String PCase := do
  let open_ ← (← (← j.getObjVal? "instance").getArr?).toList.mapM
    decodeRelationFacts
  let closed ← (← (← (← j.getObjVal? "theory").getObjVal?
    "ground_axioms").getArr?).toList.mapM decodeRelationFacts
  let p := (← j.getObjVal? "program")
  let predicates ← (← (← p.getObjVal? "predicates").getArr?).toList.mapM
    decodePredicate
  let output : Query.PredId := ⟨← natKey p "output"⟩
  let strata ← (← (← p.getObjVal? "strata").getArr?).toList.mapM
    (·.getNat?)
  let params ← (← (← j.getObjVal? "params").getArr?).toList.mapM
    decodeParam
  let expected ← (← (← j.getObjVal? "answers").getArr?).toList.mapM
    fun row => do (← row.getArr?).toList.mapM decodeValue
  return { world := ⟨open_ ++ closed⟩,
           program := { predicates, output },
           strata, env := paramEnv params, expected }

/-- One program case, end to end: decode, run the PROVED fueled
fixpoint (`Query.evalProgram` — `program_eval_sound` names its
agreement with the stratified denotation `programAnswers`, and
`program_den_finite` is why the run terminates), canonicalize both
sides, compare. `none` is agreement; `some (expected, evaluated)` is a
disagreement (a trophy — the caller reports, never repairs). -/
def checkProgramCase (text : String) :
    Except String (Option (List String × List String)) := do
  let json ← Json.parse text
  let c ← decodePCase json
  let strat : Query.PredId → Nat := fun P => c.strata.getD P.id 0
  let evaluated :=
    Query.evalProgram theClassify c.world c.env c.program strat
  let want := canonical c.expected
  let got := canonical evaluated
  if want == got then
    return none
  return some (want, got)

end ProgramCase
end Bumbledb

/-- Rows present in one canonical list and absent from the other — the
mismatch report's debugging surface. -/
def missingFrom (present want : List String) : List String :=
  want.filter fun row => !present.contains row

def main (args : List String) : IO UInt32 := do
  let dir := args.headD "conformance/cases"
  let started ← IO.monoMsNow
  let entries ← System.FilePath.readDir ⟨dir⟩
  let files := (entries.map (·.path)).filter
    (·.extension == some "json")
  let files := files.qsort fun a b =>
    a.toString.compare b.toString == .lt
  let mut failures : Nat := 0
  let mut judgments : Nat := 0
  let mut programs : Nat := 0
  for path in files do
    let text ← IO.FS.readFile path
    if (path.fileName.getD "").startsWith "program-" then
      programs := programs + 1
      match Bumbledb.ProgramCase.checkProgramCase text with
      | .ok none => pure ()
      | .ok (some (want, got)) =>
        failures := failures + 1
        IO.eprintln s!"MISMATCH {path}: evalProgram disagrees with the recorded agreed answers"
        IO.eprintln s!"  recorded {want.length} rows, evaluated {got.length} rows"
        for row in (missingFrom got want).take 5 do
          IO.eprintln s!"  recorded but not derived: {row}"
        for row in (missingFrom want got).take 5 do
          IO.eprintln s!"  derived but not recorded: {row}"
      | .error e =>
        failures := failures + 1
        IO.eprintln s!"ERROR {path}: {e}"
    else if (path.fileName.getD "").startsWith "judgment-" then
      judgments := judgments + 1
      match Bumbledb.JudgmentCase.checkJudgmentCase text with
      | .ok none => pure ()
      | .ok (some (want, got)) =>
        failures := failures + 1
        IO.eprintln s!"MISMATCH {path}: judgeB disagrees with the recorded engine verdict"
        IO.eprintln s!"  recorded: {want}"
        IO.eprintln s!"  judged:   {got}"
      | .error e =>
        failures := failures + 1
        IO.eprintln s!"ERROR {path}: {e}"
    else
      match checkCase text with
      | .ok none => pure ()
      | .ok (some (want, got)) =>
        failures := failures + 1
        IO.eprintln s!"MISMATCH {path}: the denotation disagrees with the recorded engine answers"
        IO.eprintln s!"  recorded {want.length} rows, evaluated {got.length} rows"
        for row in (missingFrom got want).take 5 do
          IO.eprintln s!"  recorded but not derived: {row}"
        for row in (missingFrom want got).take 5 do
          IO.eprintln s!"  derived but not recorded: {row}"
      | .error e =>
        failures := failures + 1
        IO.eprintln s!"ERROR {path}: {e}"
  let elapsed := (← IO.monoMsNow) - started
  IO.println
    s!"conformance: {files.size} cases ({judgments} judgment, {programs} program), {failures} disagreements, {elapsed} ms"
  if failures == 0 then
    return 0
  return 1
