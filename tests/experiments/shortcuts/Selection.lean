import Std

namespace Shortcuts

structure Candidate where
  route : Nat
  cost : Nat
  eligible : Bool
deriving DecidableEq, Repr

def improves (original : Nat) (c : Candidate) : Bool :=
  c.eligible && decide (c.cost < original)

def examined (original budget : Nat) (candidates : List Candidate) : List Candidate :=
  (candidates.dropWhile fun c => !(improves original c)).take budget

def best : List Candidate → Option Candidate
  | [] => none
  | head :: tail =>
    match best tail with
    | none => some head
    | some other => if head.cost ≤ other.cost then some head else some other

def select (original budget : Nat) (candidates : List Candidate) : Option Candidate :=
  best ((examined original budget candidates).filter (improves original))

theorem best_none_iff (xs : List Candidate) : best xs = none ↔ xs = [] := by
  cases xs with
  | nil => simp [best]
  | cons x xs =>
    cases h : best xs with
    | none => simp [best, h]
    | some y => by_cases le : x.cost ≤ y.cost <;> simp [best, h, le]

def FirstMinimum (winner : Candidate) (xs : List Candidate) : Prop :=
  ∃ before after, xs = before ++ winner :: after ∧
    (∀ c ∈ before, winner.cost < c.cost) ∧ (∀ c ∈ after, winner.cost ≤ c.cost)

private theorem FirstMinimum.le_all {w : Candidate} {xs : List Candidate}
    (h : FirstMinimum w xs) : ∀ c ∈ xs, w.cost ≤ c.cost := by
  obtain ⟨before, after, rfl, earlier, later⟩ := h
  intro c hc
  rcases List.mem_append.mp hc with hb | ha
  · exact Nat.le_of_lt (earlier c hb)
  · rcases List.mem_cons.mp ha with rfl | hrest
    · exact Nat.le_refl _
    · exact later c hrest

theorem best_first_minimum {xs : List Candidate} {w : Candidate}
    (chosen : best xs = some w) : FirstMinimum w xs := by
  induction xs generalizing w with
  | nil => simp [best] at chosen
  | cons head tail ih =>
    cases ht : best tail with
    | none =>
      have empty := (best_none_iff tail).mp ht
      subst tail
      simp [best] at chosen
      subst w
      exact ⟨[], [], rfl, by simp, by simp⟩
    | some other =>
      have minimum := ih ht
      by_cases le : head.cost ≤ other.cost
      · have eq : head = w := by simpa [best, ht, le] using chosen
        subst w
        exact ⟨[], tail, rfl, by simp, fun c hc => Nat.le_trans le (minimum.le_all c hc)⟩
      · have eq : other = w := by simpa [best, ht, le] using chosen
        subst w
        obtain ⟨before, after, htail, earlier, later⟩ := minimum
        refine ⟨head :: before, after, by simp [htail], ?_, later⟩
        intro c hc
        rcases List.mem_cons.mp hc with rfl | hb
        · exact Nat.lt_of_not_ge le
        · exact earlier c hb

theorem select_none_iff (original budget : Nat) (xs : List Candidate) :
    select original budget xs = none ↔
    ∀ c ∈ examined original budget xs, improves original c = false := by
  simp [select, best_none_iff, List.filter_eq_nil_iff]

theorem select_first_minimum {original budget : Nat} {xs : List Candidate} {w : Candidate}
    (chosen : select original budget xs = some w) :
    improves original w = true ∧
    FirstMinimum w ((examined original budget xs).filter (improves original)) := by
  have minimum := best_first_minimum chosen
  have member : w ∈ (examined original budget xs).filter (improves original) := by
    obtain ⟨before, after, split, _, _⟩ := minimum
    simp [split]
  exact ⟨(List.mem_filter.mp member).2, minimum⟩

def selectedRoute (original : Candidate) (budget : Nat) (xs : List Candidate) : Candidate :=
  (select original.cost budget xs).getD original

theorem retain_original (original : Candidate) (budget : Nat) (xs : List Candidate)
    (h : ∀ c ∈ examined original.cost budget xs, improves original.cost c = false) :
    selectedRoute original budget xs = original := by
  simp [selectedRoute, (select_none_iff _ _ _).mpr h]

theorem select_empty (original budget : Nat) : select original budget [] = none := by
  simp [select, examined, best]

theorem select_zero (original : Nat) (xs : List Candidate) : select original 0 xs = none := by
  simp [select, examined, best]

end Shortcuts

private def number (s : String) : IO Nat :=
  match s.toNat? with
  | some n => pure n
  | none => throw (IO.userError s!"invalid number: {s}")

private def checkCase (line : String) : IO Unit := do
  let name :: original :: budget :: expected :: entries := line.splitOn " " |
    throw (IO.userError s!"invalid case: {line}")
  let candidates ← entries.zipIdx.mapM fun (entry, index) => do
    let [cost, flag] := entry.splitOn "/" | throw (IO.userError s!"invalid entry: {entry}")
    let eligible ← match flag with
      | "yes" => pure true
      | "no" => pure false
      | _ => throw (IO.userError s!"invalid eligibility: {flag}")
    pure ({ route := index + 1, cost := ← number cost, eligible } : Shortcuts.Candidate)
  let incumbent : Shortcuts.Candidate := { route := 0, cost := ← number original, eligible := false }
  let actual := (Shortcuts.selectedRoute incumbent (← number budget) candidates).route
  unless actual == (← number expected) do
    throw (IO.userError s!"{name}: expected {expected}, got {actual}")

def main (args : List String) : IO Unit := do
  let [path] := args | throw (IO.userError "expected path to the shared cases")
  let contents ← IO.FS.readFile path
  let lines := contents.trim.splitOn "\n"
  for line in lines do checkCase line
  IO.println s!"{lines.length} shared cases passed"
