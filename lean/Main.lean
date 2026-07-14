import Bumbledb.Conformance

/-!
# The conformance driver (PRD 13) — the IO shell

`lake exe conformance [cases-dir]`: read every `*.json` case, evaluate
the denotation (`Bumbledb.Conformance.checkCase` — `evalList` under
`eval_sound`, plus the recorded aggregate glue), and compare against
the recorded engine answers. Exit 0 on full agreement; exit 1 with the
offending case files named otherwise — a disagreement is a TROPHY
(engine bug / naive-model bug / spec bug), triaged per the fuzzing
charter, never repaired here.

This file is the ONE place PRD 13 allows `partial` definitions; the
module below happens to need none — the loop is a `for` over a finite
file list.
-/

open Bumbledb.Conformance

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
  for path in files do
    let text ← IO.FS.readFile path
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
    s!"conformance: {files.size} cases, {failures} disagreements, {elapsed} ms"
  if failures == 0 then
    return 0
  return 1
