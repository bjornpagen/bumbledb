import Lean.Data.Json
import Bumbledb.Query.Aggregates
import Bumbledb.Query.Membership

/-!
# Conformance — the denotation executes as the third oracle (PRD 13)

The pure half of the conformance lane: decode one interchange-format
case (`lean/conformance/README.md`) into the tree's own types, evaluate
it, canonically sort, and compare against the recorded engine answers.
`lean/Main.lean` is the IO shell (`lake exe conformance cases/`);
`crates/bumbledb-bench/src/conformance.rs` is the serializer, the
corpus builder, and the Rust comparator.

**The evaluation is the DENOTATION.** The projection fragment runs
the join of `evalList` with the **surface anti-join**
(`surfaceMatchesB` / `Atom.lowerNegated`): positive atoms are the
serializer's pre-lowered `Matches`/`PointIn` form, and negated atoms
— membership bindings included — reject by AntiProbe, the engine's
`normalize.rs::lower_atom` reading (`membership_lowering_preserves_negated`,
`surface_antiprobe_filters`). `eval_sound` still names the
membership-free-negation fragment; this lane does not silently fence
the rest. So the third oracle compares the engine and the naive model
against the SPEC itself, not against a third implementation — the
class a shared misreading of the docs cannot survive.

## The aggregate glue (recorded composition)

The head shapes beyond plain projection evaluate as compositions of
the PROVED pieces, following the aggregation contract the Lean theorems
characterize: the fold domain is the distinct full binding set of the
rule (`agg_over_distinct_bindings`; `dedup`), grouping is the fibering
by the key positions (`Group`), an empty binding set yields the empty
answer set (`empty_global_no_answer` — the glue folds groups, and no
binding means no group), sums are checked (`checkedSum`), Pack is the
coalescing fold (`pack` — `pack_canonical`/`pack_extensional`), Allen masks
classify through the DEFINED classifier (`classifyRefined`), and the
measure poisons on rays (`Value.measure?` reads `none` — the lane's
corpus excludes engine-error executions, so a `none` here is a
disagreement, never a silent drop). Order keys compare as encoded
words (`Value.orderWord` — the order embeddings make word order value
order). The multi-rule aggregate head folds the union of the rules'
head-projected binding sets — the HAND-WRITTEN multi-rule law the
executor's spanning seen-set realizes (`union_regime_agg_heads`,
`Exec/Dedup.lean` — the aggregate-head coverage; `evalUnion` below is
its executable composition). Surface `or` is fold-transparent (ruled
2026-07-23, R2): a DNF-derived rule set re-keys the union dedup on
the shared slot array, and `dnf_rekey_transparent`
(`Exec/Dedup.lean`) is the proved law — the re-keyed union fold of a
lowering equals the written rule's aggregate denotation, with
`dnf_rekey_stream` the stream-level spec the executable dedup meets.
The interchange format carries the serializer's DERIVATION MARK
(`"dnf": true` beside the tagged `cq` payload — the rule list is one
written rule's DNF lowering); `dnfBindings` below re-keys marked sets
on the shared
width, so the OR+aggregate case class evaluates the written rule's
own fold domain (R2, discharged). The serializer also carries each
rule's SURFACE WIDTH (`"width"` — the written rule's variable count,
below the membership lowering's fresh mints): the fold domain reads
`fullRow` at that width, so a minted interval variable is
fold-invisible exactly as it is answer-invisible
(`membership_lowering_preserves_fold`, `Exec/Dedup.lean` — the
2026-07-23 audit's finding 087, discharged; the corpus fence
`AggregateMembership` lifts with it). Both keys are optional in the
format: absent, the mark is false and the width is the decoded rule's
own variable ceiling.

## Law-4 record: `Lean.Data.Json`

This module adopts core `Lean.Data.Json` (parser + accessors) for the
interchange format — core Lean, not a package dependency; recorded here
per the campaign's law 4 (the specific need: one hand-readable JSON
surface shared with the hand-rolled Rust serializer). Nothing else of
the `Lean` package is used.

## Shape notes

* Decoding is TOTAL: condition trees decode under an explicit fuel
  bound (64 — the engine's `MAX_CONDITION_DEPTH`); no `partial` in this
  module (the PRD allows `partial` in the IO shell only).
* Canonical order: each answer row renders to its compact tagged form
  (`renderValue` — byte-identical to the serializer's) and rows sort by
  that rendering; both sides of every comparison are re-rendered HERE,
  so the comparison is value-level and cross-language format drift
  cannot silently pass or fail a case.
* Facts decode to the tree's `Fact` (`FieldId → Value`) with the
  out-of-arity default `⟨.bool, false⟩` — the same filler the naive
  model uses for unbound positions, never readable by an accepted
  query.
-/

namespace Bumbledb
namespace Conformance

open Lean (Json)

/-! ## Canonical rendering — the serializer's byte format, mirrored -/

/-- One value in the tagged compact form (the interchange format's
value spelling; `lean/conformance/README.md`). -/
def renderValue : Value → String
  | { type := .bool, val := b } =>
    "{\"bool\":" ++ cond b "true" "false" ++ "}"
  | { type := .u64, val := x } => "{\"u64\":" ++ toString x.val ++ "}"
  | { type := .i64, val := x } => "{\"i64\":" ++ toString x.val ++ "}"
  | { type := .str, val := s } => "{\"str\":" ++ toString s.id ++ "}"
  | { type := .fixedBytes _, val := bs } =>
    "{\"bytes\":[" ++
      String.intercalate "," (bs.val.map fun b => toString b.val) ++ "]}"
  | { type := .interval .u64, val := iv } =>
    "{\"interval_u64\":[" ++ toString iv.start.val ++ "," ++
      toString iv.«end».val ++ "]}"
  | { type := .interval .i64, val := iv } =>
    "{\"interval_i64\":[" ++ toString iv.start.val ++ "," ++
      toString iv.«end».val ++ "]}"
  -- The fixed-width family: start + width render (injective — the
  -- width is the type's, and the tag separates the family from the
  -- general spelling). Carried by the corpus: the fixed judgment
  -- fixtures (`judgment-fixed-*`) and the mixed-width Allen query
  -- case (`hand-allen-mixed-width`).
  | { type := .intervalFixed .u64 w, val := v } =>
    "{\"interval_u64_fixed\":[" ++ toString v.val.val ++ "," ++
      toString w ++ "]}"
  | { type := .intervalFixed .i64 w, val := v } =>
    "{\"interval_i64_fixed\":[" ++ toString v.val.val ++ "," ++
      toString w ++ "]}"

/-- One answer row: a value array. -/
def renderRow (row : List Value) : String :=
  "[" ++ String.intercalate "," (row.map renderValue) ++ "]"

/-- Byte order on rendered rows — the canonical answer order. -/
def strLE (a b : String) : Bool := a.compare b != .gt

/-- Adjacent-duplicate elimination over a sorted list. -/
def dedupSorted : List String → List String
  | [] => []
  | [x] => [x]
  | x :: y :: rest =>
    if x == y then dedupSorted (y :: rest) else x :: dedupSorted (y :: rest)

/-- The canonical form of an answer set: rows rendered, sorted by the
rendering, duplicates collapsed (set semantics at the comparison
surface). -/
def canonical (rows : List (List Value)) : List String :=
  dedupSorted ((rows.map renderRow).mergeSort strLE)

/-- Distinct rows, by rendered identity (the rendering is injective on
values), keeping the row values — the executable distinct-binding-set
construction the fold domain reads. -/
def dedupRows (rows : List (List Value)) : List (List Value) :=
  let sorted := (rows.map fun r => (renderRow r, r)).mergeSort
    fun a b => strLE a.1 b.1
  (go sorted).map (·.2)
where
  /-- Drops adjacent render-equal rows. -/
  go : List (String × List Value) → List (String × List Value)
    | [] => []
    | [x] => [x]
    | x :: y :: rest =>
      if x.1 == y.1 then go (y :: rest) else x :: go (y :: rest)

/-! ## Decoding — the interchange format into the tree's types -/

/-- One head position of a decoded rule: a projected variable, the
measure, or an aggregate op (PRD 05's `AggOp` — the head-shape row). -/
inductive CFind where
  | var (v : Query.VarId)
  | measure (v : Query.VarId)
  | agg (op : Query.AggOp)

/-- One decoded rule: the head positions plus the BODY as the tree's
own `Query.Rule` (its `finds` empty — the head lives in `finds` here,
because PRD 04's rule head is the plain-variable narrowing). -/
structure CRule where
  finds : List CFind
  body : Query.Rule
  /-- The written rule's SURFACE variable width (the serializer's
  `"width"` key): every serializer mint — the membership lowering's
  fresh interval variables — sits at or above it, and the fold domain
  reads `fullRow` at exactly this width, so a mint is fold-invisible
  precisely as it is answer-invisible
  (`membership_lowering_preserves_fold`, `Exec/Dedup.lean` — finding
  087). Absent in a case file: the decoded rule's own ceiling. -/
  width : Option Nat

/-- One decoded query-case: the `cq` arm (empty interiors) plus the
head-shape row, R2 `dnf` mark, and optional surface width that
`Query` does not carry. -/
structure CQuery where
  rules : List CRule
  /-- The serializer's derivation mark (`"dnf": true`): the rule list
  is ONE written rule's DNF lowering, and the union dedup re-keys on
  the shared width (ruled 2026-07-23, R2 — `dnf_rekey_transparent`).
  Absent: hand-written rules, the head-projection law. -/
  dnf : Bool

/-- The proved-type body: empty-prefix `.cq`, one rule list. Head
shapes stay on `CRule.finds`; `Rule.finds` stays the PRD 04 narrowing. -/
def CQuery.body (q : CQuery) : Query.Query :=
  .cq [] (q.rules.map (·.body))

/-- One positional parameter. -/
inductive PVal where
  | scalar (v : Value)
  | set (vs : List Value)

/-- One decoded case: the world (open instance + ground axioms merged
— a closed relation's extension is ordinary facts to the matching
equation), the sealed field-type header (the `theory.relations`
block — AntiProbe typing), the query, the parameter environment, and
the recorded engine answers. -/
structure Case where
  world : Query.ListInstance
  header : Header
  query : CQuery
  env : Query.ParamEnv
  expected : List (List Value)

/-- A key's presence as an `Option` (`Except`-flattened). -/
def objKey? (j : Json) (key : String) : Option Json :=
  (j.getObjVal? key).toOption

/-- `Nat` under a named key. -/
def natKey (j : Json) (key : String) : Except String Nat := do
  (← j.getObjVal? key).getNat?

def decodeU64 (n : Nat) : Except String U64 :=
  if h : n < 2 ^ 64 then .ok ⟨n, h⟩ else .error "u64 out of range"

def decodeI64 (x : Int) : Except String I64 :=
  if h : -(2 ^ 63) ≤ x ∧ x < 2 ^ 63 then .ok ⟨x, h⟩
  else .error "i64 out of range"

def decodeIntervalU64 (j : Json) : Except String (Interval U64) := do
  match (← j.getArr?).toList with
  | [s, e] =>
    let s ← decodeU64 (← s.getNat?)
    let e ← decodeU64 (← e.getNat?)
    if h : s < e then .ok ⟨s, e, h⟩ else .error "empty u64 interval"
  | _ => .error "interval expects [start, end]"

def decodeIntervalI64 (j : Json) : Except String (Interval I64) := do
  match (← j.getArr?).toList with
  | [s, e] =>
    let s ← decodeI64 (← s.getInt?)
    let e ← decodeI64 (← e.getInt?)
    if h : s < e then .ok ⟨s, e, h⟩ else .error "empty i64 interval"
  | _ => .error "interval expects [start, end]"

/-- One tagged value. -/
def decodeValue (j : Json) : Except String Value := do
  if let some b := objKey? j "bool" then
    return ⟨.bool, ← b.getBool?⟩
  if let some n := objKey? j "u64" then
    return ⟨.u64, ← decodeU64 (← n.getNat?)⟩
  if let some n := objKey? j "i64" then
    return ⟨.i64, ← decodeI64 (← n.getInt?)⟩
  if let some n := objKey? j "str" then
    return ⟨.str, ⟨← n.getNat?⟩⟩
  if let some bs := objKey? j "bytes" then
    let nats ← (← bs.getArr?).toList.mapM (·.getNat?)
    let bytes ← nats.mapM fun n =>
      if h : n < 256 then .ok ⟨n, h⟩ else .error "byte out of range"
    return ⟨.fixedBytes bytes.length, ⟨bytes, rfl⟩⟩
  if let some iv := objKey? j "interval_u64" then
    return ⟨.interval .u64, ← decodeIntervalU64 iv⟩
  if let some iv := objKey? j "interval_i64" then
    return ⟨.interval .i64, ← decodeIntervalI64 iv⟩
  -- The fixed-width family, `[start, width]` — decode-as-corruption at
  -- the format boundary: a zero width or a start at or past the Q2
  -- bound (`start + w < maxEnd`; at-bound derives the ray sentinel,
  -- unconstructible in the fixed family) REFUSES, exactly as
  -- `crate::encoding::decode_fixed_interval_start` convicts the same
  -- bytes (the ceiling negatives are `#guard`-pinned below).
  if let some iv := objKey? j "interval_u64_fixed" then
    match (← iv.getArr?).toList with
    | [s, w] =>
      let s ← decodeU64 (← s.getNat?)
      let w ← w.getNat?
      if h : 0 < w ∧ s.val + w < U64.maxEnd.val then
        return ⟨.intervalFixed .u64 w, ⟨s, h⟩⟩
      else .error "fixed u64 value violates the Q2 bound"
    | _ => .error "fixed interval expects [start, width]"
  if let some iv := objKey? j "interval_i64_fixed" then
    match (← iv.getArr?).toList with
    | [s, w] =>
      let s ← decodeI64 (← s.getInt?)
      let w ← w.getNat?
      if h : 0 < w ∧ s.val + w < I64.maxEnd.val then
        return ⟨.intervalFixed .i64 w, ⟨s, h⟩⟩
      else .error "fixed i64 value violates the Q2 bound"
    | _ => .error "fixed interval expects [start, width]"
  .error "unknown value tag"

/-! The Q2 ceiling negatives, pinned at the decode boundary (both
element domains, at-bound and past-bound starts, plus the zero
width) — and the boundary POSITIVES one below each ceiling. -/

#guard (decodeValue (Json.mkObj [("interval_u64_fixed",
  Json.arr #[Json.num ⟨2 ^ 64 - 7, 0⟩,
             Json.num 5])])).isOk  -- start + 5 = maxEnd − 1: legal
#guard !(decodeValue (Json.mkObj [("interval_u64_fixed",
  Json.arr #[Json.num ⟨2 ^ 64 - 6, 0⟩,
             Json.num 5])])).isOk  -- start + 5 = maxEnd: the ray sentinel
#guard !(decodeValue (Json.mkObj [("interval_u64_fixed",
  Json.arr #[Json.num ⟨2 ^ 64 - 1, 0⟩,
             Json.num 5])])).isOk  -- past the bound: overflow
#guard (decodeValue (Json.mkObj [("interval_i64_fixed",
  Json.arr #[Json.num ⟨2 ^ 63 - 7, 0⟩,
             Json.num 5])])).isOk  -- i64 twin, one below the bound
#guard !(decodeValue (Json.mkObj [("interval_i64_fixed",
  Json.arr #[Json.num ⟨2 ^ 63 - 6, 0⟩,
             Json.num 5])])).isOk  -- i64 at-bound
#guard !(decodeValue (Json.mkObj [("interval_u64_fixed",
  Json.arr #[Json.num 3, Json.num 0])])).isOk  -- w = 0 denotes nothing
#guard (decodeValue (Json.mkObj [("bytes",
  Json.arr #[Json.num 7, Json.num 0, Json.num 255])])).isOk
#guard !(decodeValue (Json.mkObj [("bytes",
  Json.arr #[Json.num 256])])).isOk

/-- The thirteen Allen relation names, as the format spells them. -/
def relOfName : String → Except String Query.AllenRel
  | "before" => .ok .before
  | "meets" => .ok .meets
  | "overlaps" => .ok .overlaps
  | "starts" => .ok .starts
  | "during" => .ok .during
  | "finishes" => .ok .finishes
  | "equals" => .ok .equals
  | "finished_by" => .ok .finishedBy
  | "contains" => .ok .contains
  | "started_by" => .ok .startedBy
  | "overlapped_by" => .ok .overlappedBy
  | "met_by" => .ok .metBy
  | "after" => .ok .after
  | other => .error s!"unknown Allen relation {other}"

def decodeMask (j : Json) : Except String Query.AllenMask := do
  (← j.getArr?).toList.mapM fun name => do relOfName (← name.getStr?)

/-- One term. -/
def decodeTerm (j : Json) : Except String Query.Term := do
  if let some v := objKey? j "var" then
    return .var ⟨← v.getNat?⟩
  if let some p := objKey? j "param" then
    return .param ⟨← p.getNat?⟩
  if let some p := objKey? j "param_set" then
    return .paramSet ⟨← p.getNat?⟩
  if let some v := objKey? j "lit" then
    return .lit (← decodeValue v)
  if let some v := objKey? j "measure" then
    return .measure ⟨← v.getNat?⟩
  .error "unknown term tag"

/-- One comparison (the operator flattened into the `cmp` object). -/
def decodeCmp (j : Json) : Except String Query.Comparison := do
  let name ← (← j.getObjVal? "op").getStr?
  let lhs ← decodeTerm (← j.getObjVal? "lhs")
  let rhs ← decodeTerm (← j.getObjVal? "rhs")
  let op : Query.CmpOp ←
    match name with
    | "eq" => pure .eq
    | "ne" => pure .ne
    | "lt" => pure .lt
    | "le" => pure .le
    | "gt" => pure .gt
    | "ge" => pure .ge
    | "point_in" => pure .pointIn
    | "allen" =>
      if let some m := objKey? j "mask" then
        pure (.allen (← decodeMask m))
      else .error "allen expects a literal mask"
    | other => .error s!"unknown comparison op {other}"
  return { op, lhs, rhs }

/-- One condition-tree node, under the depth fuel (the engine's
`MAX_CONDITION_DEPTH` = 64 — decoding stays total). -/
def decodeCondition : Nat → Json → Except String Query.Condition
  | 0, _ => .error "condition tree deeper than the boundary cap"
  | fuel + 1, j => do
    if let some c := objKey? j "cmp" then
      return .leaf (← decodeCmp c)
    if let some cs := objKey? j "and" then
      return .and (← (← cs.getArr?).toList.mapM (decodeCondition fuel))
    if let some cs := objKey? j "or" then
      return .or (← (← cs.getArr?).toList.mapM (decodeCondition fuel))
    .error "unknown condition node"

/-- One atom: `edb` or `interior` — one spelling of `AtomSource`. -/
def decodeAtom (j : Json) : Except String Query.Atom := do
  let source : Query.AtomSource ←
    if let some r := objKey? j "edb" then
      pure (.edb ⟨← r.getNat?⟩)
    else if let some c := objKey? j "interior" then
      pure (.interior ⟨← c.getNat?⟩)
    else .error "atom expects edb or interior"
  let bindings ← (← (← j.getObjVal? "bindings").getArr?).toList.mapM
    fun pair => do
      match (← pair.getArr?).toList with
      | [f, t] => return ((⟨← f.getNat?⟩ : FieldId), ← decodeTerm t)
      | _ => .error "binding expects [field, term]"
  return { source, bindings }

/-- One head position. -/
def decodeFind (j : Json) : Except String CFind := do
  if let some v := objKey? j "var" then
    return .var ⟨← v.getNat?⟩
  if let some v := objKey? j "measure" then
    return .measure ⟨← v.getNat?⟩
  if let some a := objKey? j "agg" then
    let op ← (← a.getObjVal? "op").getStr?
    match op with
    | "count" => return .agg .count
    | "sum" => return .agg (.sum ⟨← natKey a "over"⟩)
    | "min" => return .agg (.min ⟨← natKey a "over"⟩)
    | "max" => return .agg (.max ⟨← natKey a "over"⟩)
    | "pack" => return .agg (.pack ⟨← natKey a "over"⟩)
    | other => .error s!"unknown aggregate op {other}"
  if let some a := objKey? j "agg_measure" then
    let op ← (← a.getObjVal? "op").getStr?
    let over : Query.VarId := ⟨← natKey a "over"⟩
    match op with
    | "sum" => return .agg (.measureFold .sum over)
    | "min" => return .agg (.measureFold .min over)
    | "max" => return .agg (.measureFold .max over)
    | other => .error s!"unknown measure fold {other}"
  .error "unknown find tag"

/-- One rule (head positions + the body as `Query.Rule` + the
optional surface width). -/
def decodeRule (j : Json) : Except String CRule := do
  let finds ← (← (← j.getObjVal? "finds").getArr?).toList.mapM decodeFind
  let atoms ← (← (← j.getObjVal? "atoms").getArr?).toList.mapM decodeAtom
  let negated ←
    (← (← j.getObjVal? "negated").getArr?).toList.mapM decodeAtom
  let conditions ← (← (← j.getObjVal? "conditions").getArr?).toList.mapM
    (decodeCondition 64)
  return { finds, body := { finds := [], atoms, negated, conditions },
           width := (natKey j "width").toOption }

/-- The tagged Query JSON: `{ "cq": payload } | { "reach": payload }`.
Derivation marks (`dnf`) sit beside the payload. -/
inductive QueryTag where
  | cq (payload : Json)
  | reach (payload : Json)

/-- One tagged encoding of `Query.cq | Query.reach`. No `rec: null`,
no omitted-`rec` constructor. -/
def taggedQueryPayload (j : Json) : Except String QueryTag :=
  match objKey? j "cq", objKey? j "reach" with
  | some p, none => .ok (.cq p)
  | none, some p => .ok (.reach p)
  | some _, some _ => .error "query carries both cq and reach tags"
  | none, none => .error "query expects a cq or reach tag"

/-- JSON `head` is the find list (required). Lean sizes heads from
`finds.length`; the name `arity` is unrepresentable. -/
def decodeHead (j : Json) : Except String Unit := do
  let _ ← (← j.getObjVal? "head").getArr?
  return ()

/-- Query-case documents are the `cq` arm. `dnf` sits beside the
payload; `width` stays on each rule. -/
def decodeQuery (j : Json) : Except String CQuery := do
  match ← taggedQueryPayload j with
  | .reach _ => .error "query-case document is the cq arm"
  | .cq payload =>
    decodeHead payload
    let _ ← (← payload.getObjVal? "interiors").getArr?
    let rules ← (← (← payload.getObjVal? "rules").getArr?).toList.mapM
      decodeRule
    let dnf :=
      match objKey? j "dnf" with
      | some b => b.getBool?.toOption.getD false
      | none => false
    return { rules, dnf }

/-- One positional parameter. -/
def decodeParam (j : Json) : Except String PVal := do
  if let some v := objKey? j "scalar" then
    return .scalar (← decodeValue v)
  if let some vs := objKey? j "set" then
    return .set (← (← vs.getArr?).toList.mapM decodeValue)
  .error "unknown param tag"

/-- A field-type spelling (`theory.relations[].fields`). -/
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

/-- The sealed header the AntiProbe typing reads: `id → field types`. -/
def decodeHeader (j : Json) : Except String Header := do
  let rels ← (← j.getArr?).toList.mapM fun r => do
    let id ← natKey r "id"
    let fields ← (← (← r.getObjVal? "fields").getArr?).toList.mapM
      fun t => do typeOfName (← t.getStr?)
    pure (id, fields)
  return { sig := fun R =>
    match rels.find? (fun e => e.1 == R.id) with
    | some e => e.2
    | none => [] }

/-- The environment the positional parameters denote (an id's unused
faces read defaults no accepted query consults). -/
def paramEnv (params : List PVal) : Query.ParamEnv where
  scalar p :=
    match params.getD p.id (.scalar ⟨.bool, false⟩) with
    | .scalar v => v
    | _ => ⟨.bool, false⟩
  set p :=
    match params.getD p.id (.set []) with
    | .set vs => vs
    | _ => []

/-- One fact: a value array read as the tree's total field assignment
(out-of-arity fields carry the never-read filler). -/
def decodeFact (j : Json) : Except String Fact := do
  let vals ← (← j.getArr?).toList.mapM decodeValue
  return fun i => vals.getD i.id ⟨.bool, false⟩

/-- One relation block (`{relation, facts}` — the instance's and the
ground axioms' shared shape). -/
def decodeRelationFacts (j : Json) :
    Except String (RelId × List Fact) := do
  let relation ← natKey j "relation"
  let facts ← (← (← j.getObjVal? "facts").getArr?).toList.mapM decodeFact
  return (⟨relation⟩, facts)

/-- One full case document. -/
def decodeCase (j : Json) : Except String Case := do
  let theory ← j.getObjVal? "theory"
  let open_ ← (← (← j.getObjVal? "instance").getArr?).toList.mapM
    decodeRelationFacts
  let closed ← (← (← theory.getObjVal?
    "ground_axioms").getArr?).toList.mapM decodeRelationFacts
  let header ← decodeHeader (← theory.getObjVal? "relations")
  let query ← decodeQuery (← j.getObjVal? "query")
  let params ← (← (← j.getObjVal? "params").getArr?).toList.mapM decodeParam
  let expected ← (← (← j.getObjVal? "answers").getArr?).toList.mapM
    fun row => do (← row.getArr?).toList.mapM decodeValue
  return { world := ⟨open_ ++ closed⟩, header, query,
           env := paramEnv params, expected }

/-! ## Evaluation — join + surface anti-join (AntiProbe) plus the aggregate glue -/

/-- The classifier: PRD 05's DEFINED refinement of the abstract
parameter — the lane evaluates the real thirteen-way classification. -/
def theClassify : Query.Classify := Query.classifyRefined

/-- A variable is scalar-anchored when a positive atom binds it on a
non-interval field — the bivalent-resolution rule the serializer and
the engine share (`conformance.rs::scalar_anchors`). -/
def scalarAnchored (h : Header) (r : Query.Rule) (v : Query.VarId) :
    Bool :=
  r.atoms.any fun a =>
    a.bindings.any fun b =>
      match b.2 with
      | .var u => decide (u = v) &&
          !(match a.source with
            | .edb R => h.isInterval R b.1
            | .interior _ => false)
      | _ => false

/-- The typing witness `SurfaceMatches` / AntiProbe consult: field
types from the sealed header, params from the environment, variables
`.u64` when scalar-anchored else interval-shaped (only `isInterval`
matters). -/
def typingOf (h : Header) (ρ : Query.ParamEnv) (r : Query.Rule) :
    Query.Typing where
  header := h
  var v := if scalarAnchored h r v then .u64 else .interval .u64
  param p := (ρ.scalar p).type

/-- One rule body's surviving states: the join over the positive atoms
(`Matches` / already-lowered `PointIn`), the **surface** anti-join
(`surfaceMatchesB` — AntiProbe, membership bindings included), the
condition filter. On membership-free negated atoms this coincides
with `evalList`'s `matchesB` anti-join. -/
def ruleStates (h : Header) (W : Query.ListInstance) (ρ : Query.ParamEnv)
    (r : Query.Rule) : List Query.PartialAssign :=
  let Γ := typingOf h ρ r
  (Query.joinAtoms W Query.InteriorTables.empty ρ r.atoms [[]]).filter fun σp =>
    (r.negated.all fun a =>
      (Query.factsOf W Query.InteriorTables.empty a.source).all fun f =>
        ! Query.surfaceMatchesB Γ ρ (Query.totalize σp) a f) &&
    (r.conditions.all fun t =>
      Query.condHoldsB theClassify ρ (Query.totalize σp) t)

/-- The variable width of one rule (max mentioned id + 1): the body's
sites plus the head's (`over`/`key` variables included). -/
def varCount (r : CRule) : Nat :=
  let bodyMax := r.body.allVars.foldl (fun n v => max n (v.id + 1)) 0
  r.finds.foldl
    (fun n f =>
      match f with
      | .var v | .measure v => max n (v.id + 1)
      | .agg .count => n
      | .agg (.sum v) | .agg (.min v)
      | .agg (.max v) | .agg (.pack v)
      | .agg (.measureFold _ v) => max n (v.id + 1))
    bodyMax

/-- One state as the full binding row over `0..n` (unbound positions
carry the naive model's filler, never read by an accepted query). -/
def fullRow (n : Nat) (σp : Query.PartialAssign) : List Value :=
  (List.range n).map fun i => Query.totalize σp ⟨i⟩

/-- One rule's fold-row width: the serializer's surface width, else
the decoded rule's own ceiling. Reading `fullRow` at the SURFACE
width projects every serializer mint away before the dedup, so the
membership lowering's fresh interval variable is fold-invisible
(`membership_lowering_preserves_fold` — finding 087). -/
def ruleWidth (r : CRule) : Nat :=
  r.width.getD (varCount r)

/-- The distinct full binding set of one rule — the fold domain
(`agg_over_distinct_bindings`: no fold observes a duplicate), read at
the surface width. -/
def ruleBindings (h : Header) (W : Query.ListInstance) (ρ : Query.ParamEnv)
    (r : CRule) : List (List Value) :=
  dedupRows ((ruleStates h W ρ r.body).map (fullRow (ruleWidth r)))

/-- The re-keyed fold domain of a DNF-derived rule set (ruled
2026-07-23, R2): the disjuncts share one variable vocabulary and one
layout, so the fold domain is the deduplicated union of their full
binding rows over the shared width — the WRITTEN rule's own domain
(`dnf_rekey_transparent`; `dnf_rekey_stream` is the stream this dedup
realizes). -/
def dnfBindings (h : Header) (W : Query.ListInstance) (ρ : Query.ParamEnv)
    (rules : List CRule) : List (List Value) :=
  let n := rules.foldl (fun m r => max m (ruleWidth r)) 0
  dedupRows (rules.flatMap fun r =>
    (ruleStates h W ρ r.body).map (fullRow n))

/-! Tagged Query decode, pinned: `cq` payload carries `interiors` /
`head` / `rules`; `dnf` sits beside the tag; the product (`rec: null`,
`arity`, untagged `rules`) is unrepresentable. -/

#guard ((decodeQuery (Json.mkObj [
    ("cq", Json.mkObj [
      ("interiors", Json.arr #[]),
      ("head", Json.arr #[]),
      ("rules", Json.arr #[Json.mkObj [
        ("finds", Json.arr #[]), ("atoms", Json.arr #[]),
        ("negated", Json.arr #[]), ("conditions", Json.arr #[]),
        ("width", Json.num 3)]])]),
    ("dnf", Json.bool true)])).toOption.map
  fun q => (q.dnf, q.rules.map ruleWidth)) == some (true, [3])

#guard ((decodeQuery (Json.mkObj [
    ("cq", Json.mkObj [
      ("interiors", Json.arr #[]),
      ("head", Json.arr #[]),
      ("rules", Json.arr #[Json.mkObj [
        ("finds", Json.arr #[]), ("atoms", Json.arr #[]),
        ("negated", Json.arr #[]), ("conditions", Json.arr #[])]])])])).toOption.map
  fun q => (q.dnf, q.rules.map ruleWidth)) == some (false, [0])

#guard !(decodeQuery (Json.mkObj [
    ("rules", Json.arr #[]), ("rec", Json.null),
    ("arity", Json.num 0)])).isOk

#guard (decodeAtom (Json.mkObj [
    ("edb", Json.num 1), ("bindings", Json.arr #[])])).isOk

#guard !(decodeAtom (Json.mkObj [
    ("relation", Json.num 1), ("bindings", Json.arr #[])])).isOk

#guard (match taggedQueryPayload (Json.mkObj [
    ("reach", Json.mkObj [
      ("interiors", Json.arr #[]), ("head", Json.arr #[]),
      ("rec", Json.mkObj []), ("rules", Json.arr #[])])]) with
  | .ok (.reach _) => true
  | _ => false)

#guard (match taggedQueryPayload (Json.mkObj [
    ("interiors", Json.arr #[]), ("rec", Json.null)]) with
  | .error _ => true
  | _ => false)

/-- A row's value at a variable position. -/
def rowGet (row : List Value) (v : Query.VarId) : Value :=
  row.getD v.id ⟨.bool, false⟩

/-- The measure of a bound interval, or the ray refusal (the corpus
excludes engine-error executions, so reaching the error here IS a
disagreement). -/
def measureVal (v : Value) : Except String Value :=
  match v.measure? with
  | some m => .ok m
  | none => .error "MeasureOfRay: a ray reached a measure position"

/-- The encoded order word (order keys compare as words — the order
embeddings make word order value order). -/
def orderKey (v : Value) : Except String Nat :=
  match v.orderWord with
  | some (_, w) => .ok w
  | none => .error "an order position holds a non-orderable value"

/-- Min/Max: the extreme-attaining VALUE of a nonempty list. -/
def pickExtreme (isMax : Bool) : List Value → Except String Value
  | [] => .error "an aggregate fold over an empty group"
  | v :: rest =>
    rest.foldlM
      (fun best x => do
        let kb ← orderKey best
        let kx ← orderKey x
        pure (if (if isMax then kb < kx else kx < kb) then x else best))
      v

/-- A group size as a `U64` value. -/
def natValue (n : Nat) : Except String Value :=
  if h : n < 2 ^ 64 then .ok ⟨.u64, ⟨n, h⟩⟩
  else .error "a count exceeds the u64 domain"

/-- The checked sum: `checkedSum` for the `u64` face (the PRD 05 form,
`checkedSum_sound`), the `Int` sum with the one finalize range check
for `i64` (the wide-accumulator shape). -/
def sumVals : List Value → Except String Value
  | [] => .error "an aggregate fold over an empty group"
  | vals@({ type := .u64, val := _ } :: _) => do
    let nats ← vals.mapM fun v =>
      match v with
      | { type := .u64, val := x } => Except.ok x.val
      | _ => Except.error "a mixed-type Sum input"
    match checkedSum (2 ^ 64 - 1) nats with
    | some s =>
      if h : s < 2 ^ 64 then .ok ⟨.u64, ⟨s, h⟩⟩
      else .error "Overflow: Sum(U64) out of range"
    | none => .error "Overflow: Sum(U64) out of range"
  | vals@({ type := .i64, val := _ } :: _) => do
    let ints ← vals.mapM fun v =>
      match v with
      | { type := .i64, val := x } => Except.ok x.val
      | _ => Except.error "a mixed-type Sum input"
    let s := ints.foldl (· + ·) 0
    if h : -(2 ^ 63) ≤ s ∧ s < 2 ^ 63 then .ok ⟨.i64, ⟨s, h⟩⟩
    else .error "Overflow: Sum(I64) out of range"
  | _ => .error "Sum over a non-integer input"

/-- Pack: the group's claims through PRD 05's coalescing fold. -/
def packVals : List Value → Except String (List Value)
  | [] => .error "Pack over an empty group"
  | vals@({ type := .interval .u64, val := _ } :: _) => do
    let ivs ← vals.mapM fun v =>
      match v.intervalU64 with
      | some iv => Except.ok iv
      | none => Except.error "a mixed-type Pack input"
    .ok ((pack ivs).map fun iv => ⟨.interval .u64, iv⟩)
  | vals@({ type := .interval .i64, val := _ } :: _) => do
    let ivs ← vals.mapM fun v =>
      match v.intervalI64 with
      | some iv => Except.ok iv
      | none => Except.error "a mixed-type Pack input"
    .ok ((pack ivs).map fun iv => ⟨.interval .i64, iv⟩)
  -- Fixed-width claims pack through their derived intervals
  -- (`Value.intervalU64`'s fixed arm); the packed output is the
  -- GENERAL type — coalescing does not preserve a width, so Pack's
  -- result column is `interval<E>`, exactly the engine's typing.
  | vals@({ type := .intervalFixed .u64 _, val := _ } :: _) => do
    let ivs ← vals.mapM fun v =>
      match v.intervalU64 with
      | some iv => Except.ok iv
      | none => Except.error "a mixed-type Pack input"
    .ok ((pack ivs).map fun iv => ⟨.interval .u64, iv⟩)
  | vals@({ type := .intervalFixed .i64 _, val := _ } :: _) => do
    let ivs ← vals.mapM fun v =>
      match v.intervalI64 with
      | some iv => Except.ok iv
      | none => Except.error "a mixed-type Pack input"
    .ok ((pack ivs).map fun iv => ⟨.interval .i64, iv⟩)
  | _ => .error "Pack over a non-interval input"

/-- One fold aggregate over a group of full bindings (the single-rule
fold domain; Pack and Arg take their own paths). -/
def foldAgg (op : Query.AggOp) (group : List (List Value)) :
    Except String Value :=
  match op with
  | .count => natValue group.length
  | .sum v => sumVals (group.map (rowGet · v))
  | .min v => pickExtreme false (group.map (rowGet · v))
  | .max v => pickExtreme true (group.map (rowGet · v))
  | .measureFold fold v => do
    let ms ← group.mapM fun b => measureVal (rowGet b v)
    match fold with
    | .sum => sumVals ms
    | .min => pickExtreme false ms
    | .max => pickExtreme true ms
  | .pack _ => .error "Pack takes the segment path"

/-- The group key of one binding: the values at the var and measure
head positions, in head order — grouping is the fibering by exactly
these (`Group`). -/
def keyOf (finds : List CFind) (b : List Value) :
    Except String (List Value) :=
  finds.foldlM
    (fun acc f =>
      match f with
      | .var v => pure (acc ++ [rowGet b v])
      | .measure v => do pure (acc ++ [← measureVal (rowGet b v)])
      | .agg _ => pure acc)
    []

/-- Fibers a row list by a rendered key: sort by the key's canonical
rendering, split adjacent runs. -/
def groupBy (keyOf : α → Except String (List Value)) (rows : List α) :
    Except String (List (List α)) := do
  let keyed ← rows.mapM fun r => do pure (renderRow (← keyOf r), r)
  let sorted := keyed.mergeSort fun a b => strLE a.1 b.1
  return split sorted
where
  /-- Splits adjacent key-equal runs. -/
  split : List (String × α) → List (List α)
    | [] => []
    | (k, r) :: rest => run k [r] rest
  /-- Accumulates one run. -/
  run (k : String) (acc : List α) :
      List (String × α) → List (List α)
    | [] => [acc.reverse]
    | (k', r') :: rest =>
      if k' == k then run k (r' :: acc) rest
      else acc.reverse :: run k' [r'] rest

/-- Whether the head carries a Pack position. -/
def hasPack (finds : List CFind) : Bool :=
  finds.any fun f =>
    match f with
    | .agg (.pack _) => true
    | _ => false

/-- One group's output rows (single-rule path): Pack emits one row per
maximal segment; everything else is one row of key values and folds. -/
def projectGroup (finds : List CFind) (group : List (List Value)) :
    Except String (List (List Value)) := do
  match group with
  | [] => .error "an empty group fiber"
  | b0 :: _ =>
    if hasPack finds then
      let claims ← finds.findSomeM? fun f =>
        match f with
        | .agg (.pack v) => pure (some (group.map (rowGet · v)))
        | _ => pure none
      let some claims := claims
        | .error "Pack position vanished"
      let segments ← packVals claims
      segments.mapM fun seg =>
        finds.mapM fun f =>
          match f with
          | .var v => pure (rowGet b0 v)
          | .measure v => measureVal (rowGet b0 v)
          | .agg (.pack _) => pure seg
          | .agg _ => .error "Pack mixes with no other aggregate"
    else do
      let row ← finds.mapM fun f =>
        match f with
        | .var v => pure (rowGet b0 v)
        | .measure v => measureVal (rowGet b0 v)
        | .agg op => foldAgg op group
      pure [row]

/-- The grouped aggregate path over an explicit fold domain: fibered
by the key positions, one output batch per inhabited fiber — no
binding, no group, no row (`empty_global_no_answer`). -/
def evalGrouped (finds : List CFind) (domain : List (List Value)) :
    Except String (List (List Value)) := do
  let groups ← groupBy (keyOf finds) domain
  let batches ← groups.mapM (projectGroup finds)
  return batches.flatten

/-- The single-rule aggregate/measure path: the rule's distinct full
bindings, grouped. -/
def evalSingle (h : Header) (W : Query.ListInstance) (ρ : Query.ParamEnv)
    (r : CRule) : Except String (List (List Value)) :=
  evalGrouped r.finds (ruleBindings h W ρ r)

/-- One rule's head-projected input row (the multi-rule union fold's
domain element): var and measure positions project, fold positions
carry their input value, the nullary Count carries the stable filler. -/
def headRow (finds : List CFind) (b : List Value) :
    Except String (List Value) :=
  finds.mapM fun f =>
    match f with
    | .var v => pure (rowGet b v)
    | .measure v => measureVal (rowGet b v)
    | .agg .count => pure ⟨.bool, false⟩
    | .agg (.sum v) | .agg (.min v)
    | .agg (.max v) | .agg (.pack v) => pure (rowGet b v)
    | .agg (.measureFold _ v) => measureVal (rowGet b v)

/-- The key positions of a head-projected row. -/
def headKey (finds : List CFind) (row : List Value) : List Value :=
  (finds.zip row).filterMap fun (f, v) =>
    match f with
    | .var _ | .measure _ => some v
    | .agg _ => none

/-- A column of a row group. -/
def colVals (i : Nat) (group : List (List Value)) : List Value :=
  group.map fun r => r.getD i ⟨.bool, false⟩

/-- Pairs each element with its position. -/
def indexed : Nat → List α → List (Nat × α)
  | _, [] => []
  | i, x :: xs => (i, x) :: indexed (i + 1) xs

/-- One union-fold group's output rows: fold positions fold the
group's projected column (measure positions already hold measure
values), Pack coalesces the column's claims. -/
def projectUnionGroup (finds : List CFind)
    (group : List (List Value)) : Except String (List (List Value)) := do
  match group with
  | [] => .error "an empty union group"
  | r0 :: _ =>
    let slots := indexed 0 finds
    if hasPack finds then
      let claims ← slots.findSomeM? fun (i, f) =>
        match f with
        | .agg (.pack _) => pure (some (colVals i group))
        | _ => pure none
      let some claims := claims
        | .error "Pack position vanished"
      let segments ← packVals claims
      segments.mapM fun seg =>
        slots.mapM fun (i, f) =>
          match f with
          | .var _ | .measure _ => pure (r0.getD i ⟨.bool, false⟩)
          | .agg (.pack _) => pure seg
          | .agg _ => .error "Pack mixes with no other aggregate"
    else do
      let row ← slots.mapM fun (i, f) =>
        match f with
        | .var _ | .measure _ => pure (r0.getD i ⟨.bool, false⟩)
        | .agg .count => natValue group.length
        | .agg (.sum _) | .agg (.measureFold .sum _) =>
          sumVals (colVals i group)
        | .agg (.min _) | .agg (.measureFold .min _) =>
          pickExtreme false (colVals i group)
        | .agg (.max _) | .agg (.measureFold .max _) =>
          pickExtreme true (colVals i group)
        | .agg (.pack _) => .error "Pack takes the segment path"
      pure [row]

/-- The multi-rule aggregate head: the union of the rules' distinct
head-projected binding sets, fibered by the key positions, folded per
position — the rules-IR union fold. -/
def evalUnion (h : Header) (W : Query.ListInstance) (ρ : Query.ParamEnv)
    (q : CQuery) : Except String (List (List Value)) := do
  let finds :=
    match q.rules with
    | r :: _ => r.finds
    | [] => []
  let domain ← q.rules.foldlM
    (fun acc r => do
      let rows ← (ruleBindings h W ρ r).mapM (headRow r.finds)
      pure (acc ++ rows))
    []
  let groups ← groupBy (fun row => pure (headKey finds row))
    (dedupRows domain)
  let batches ← groups.mapM (projectUnionGroup finds)
  return batches.flatten

/-- Whether a head position is a plain projected variable. -/
def CFind.plainVar? : CFind → Option Query.VarId
  | .var v => some v
  | _ => none

/-- Whether a head position is an aggregate. -/
def CFind.isAgg : CFind → Bool
  | .agg _ => true
  | _ => false

/-- The plain-projection evaluator: join + surface anti-join + head
projection — `evalList`'s pipeline with AntiProbe on negated atoms. -/
def evalPlain (h : Header) (W : Query.ListInstance) (ρ : Query.ParamEnv)
    (q : CQuery) : List (List Value) :=
  q.rules.flatMap fun r =>
    let vars := r.finds.filterMap CFind.plainVar?
    (ruleStates h W ρ r.body).map fun σp =>
      vars.map (Query.totalize σp)

/-- **The evaluator**: plain projections run the join + surface
anti-join (`evalPlain` — AntiProbe on negated membership; coincides
with `evalList` on membership-free negation), and the
aggregate/measure head shapes run the recorded glue over the same
join states. -/
def evalSeeded (h : Header) (W : Query.ListInstance) (ρ : Query.ParamEnv)
    (q : CQuery) : Except String (List (List Value)) :=
  match q.rules with
  | [] => .error "a query needs at least one rule"
  | [r] =>
    if r.finds.all fun f => (CFind.plainVar? f).isSome then
      .ok (evalPlain h W ρ q)
    else
      evalSingle h W ρ r
  | r0 :: _ =>
    if q.rules.all fun r =>
        r.finds.all fun f => (CFind.plainVar? f).isSome then
      .ok (evalPlain h W ρ q)
    else if q.dnf then
      -- The re-keyed regime (ruled 2026-07-23, R2): a DNF-derived
      -- rule set evaluates the WRITTEN rule's own denotation — the
      -- disjuncts' binding rows unioned over the shared width,
      -- grouped once.
      evalGrouped r0.finds (dnfBindings h W ρ q.rules)
    else if r0.finds.any CFind.isAgg then
      evalUnion h W ρ q
    else
      -- Measure finds, no aggregate: the union of the per-rule
      -- projections (the plain multi-rule reading, measure positions
      -- projected per rule).
      q.rules.foldlM
        (fun acc r => do pure (acc ++ (← evalSingle h W ρ r)))
        []

/-! ## The comparison -/

/-- One case, end to end: decode, evaluate, canonicalize both sides,
compare. `none` is agreement; `some (expected, evaluated)` is a
disagreement (a trophy — the caller reports, never repairs). -/
def checkCase (text : String) :
    Except String (Option (List String × List String)) := do
  let json ← Json.parse text
  let case ← decodeCase json
  let evaluated ← evalSeeded case.header case.world case.env case.query
  let want := canonical case.expected
  let got := canonical evaluated
  if want == got then
    return none
  return some (want, got)

end Conformance
end Bumbledb

