import Std

namespace LayoutSelection

structure Candidate where
  id : Nat
  span : Nat
  area : Nat
  width : Nat
  cost : Nat
  deriving Repr, DecidableEq

def shapeLE (a b : Candidate) : Prop :=
  a.span < b.span ∨ a.span = b.span ∧
    (a.area < b.area ∨ a.area = b.area ∧ a.width ≤ b.width)

def keyLE (a b : Candidate) : Prop :=
  a.cost < b.cost ∨ a.cost = b.cost ∧ shapeLE a b

instance (a b : Candidate) : Decidable (shapeLE a b) := by unfold shapeLE; infer_instance
instance (a b : Candidate) : Decidable (keyLE a b) := by unfold keyLE; infer_instance

def eligible (compact c : Candidate) : Bool :=
  decide (10 * c.span ≤ 11 * compact.span ∧ 4 * c.area ≤ 5 * compact.area)

def best (le : Candidate → Candidate → Prop) [DecidableRel le] :
    List Candidate → Option Candidate
  | [] => none
  | head :: tail =>
    match best le tail with
    | none => some head
    | some next => if le head next then some head else some next

def select (xs : List Candidate) : Option Candidate := do
  let compact ← best shapeLE xs
  best keyLE (xs.filter (eligible compact))

private theorem best_none (le) [DecidableRel le] (xs : List Candidate) :
    best le xs = none ↔ xs = [] := by
  cases xs with
  | nil => simp [best]
  | cons head tail =>
    cases h : best le tail with
    | none => simp [best, h]
    | some next => by_cases c : le head next <;> simp [best, h, c]

def FirstMinimum (le : Candidate → Candidate → Prop) (w : Candidate) (xs : List Candidate) :=
  ∃ earlier later, xs = earlier ++ w :: later ∧
    (∀ c ∈ earlier, ¬ le c w) ∧ (∀ c ∈ later, le w c)

theorem first_minimum (le) [DecidableRel le]
    (total : ∀ a b, le a b ∨ le b a)
    (trans : ∀ a b c, le a b → le b c → le a c)
    {xs : List Candidate} {w : Candidate} (h : best le xs = some w) :
    FirstMinimum le w xs := by
  induction xs generalizing w with
  | nil => simp [best] at h
  | cons a xs ih =>
    cases hn : best le xs with
    | none =>
      have empty := (best_none le xs).mp hn
      subst xs
      simp [best] at h
      subst w
      exact ⟨[], [], rfl, by simp, by simp⟩
    | some n =>
      obtain ⟨before, after, split, earlier, later⟩ := ih hn
      by_cases ha : le a n
      · have eq : a = w := by simpa [best, hn, ha] using h
        subst w
        refine ⟨[], xs, rfl, by simp, ?_⟩
        intro c hc
        rw [split] at hc
        rcases List.mem_append.mp hc with hb | ht
        · exact trans _ _ _ ha ((total c n).resolve_left (earlier c hb))
        · rcases List.mem_cons.mp ht with rfl | ht
          · exact ha
          · exact trans _ _ _ ha (later c ht)
      · have eq : n = w := by simpa [best, hn, ha] using h
        subst w
        refine ⟨a :: before, after, by simp [split], ?_, later⟩
        intro c hc
        rcases List.mem_cons.mp hc with rfl | hb
        · exact ha
        · exact earlier c hb

private theorem shape_total (a b : Candidate) : shapeLE a b ∨ shapeLE b a := by
  unfold shapeLE; omega

private theorem key_total (a b : Candidate) : keyLE a b ∨ keyLE b a := by
  unfold keyLE shapeLE; omega

theorem select_nonempty {xs : List Candidate} (nonempty : xs ≠ []) :
    ∃ w, select xs = some w := by
  cases hb : best shapeLE xs with
  | none => exact False.elim (nonempty ((best_none _ _).mp hb))
  | some compact =>
    obtain ⟨before, after, split, _, _⟩ := first_minimum shapeLE shape_total
      (fun _ _ _ => by unfold shapeLE; omega) hb
    have member : compact ∈ xs := by simp [split]
    have legal : eligible compact compact = true := by simp [eligible]; omega
    have kept := List.mem_filter.mpr ⟨member, legal⟩
    cases hw : best keyLE (xs.filter (eligible compact)) with
    | some w => exact ⟨w, by simp [select, hb, hw]⟩
    | none =>
      rw [(best_none _ _).mp hw] at kept
      simp at kept

theorem selection_policy {xs : List Candidate} {w : Candidate} (chosen : select xs = some w) :
    ∃ compact, best shapeLE xs = some compact ∧ w ∈ xs ∧ eligible compact w = true ∧
      FirstMinimum keyLE w (xs.filter (eligible compact)) := by
  cases hb : best shapeLE xs with
  | none => simp [select, hb] at chosen
  | some compact =>
    have hw : best keyLE (xs.filter (eligible compact)) = some w := by
      simpa [select, hb] using chosen
    have minimum := first_minimum keyLE key_total
      (fun _ _ _ => by unfold keyLE shapeLE; omega) hw
    have mem : w ∈ xs.filter (eligible compact) := by
      obtain ⟨before, after, split, _, _⟩ := minimum
      simp [split]
    exact ⟨compact, rfl, (List.mem_filter.mp mem).1, (List.mem_filter.mp mem).2, minimum⟩

theorem least_eligible {xs : List Candidate} {w compact c : Candidate}
    (chosen : select xs = some w) (baseline : best shapeLE xs = some compact)
    (member : c ∈ xs) (legal : eligible compact c = true) : keyLE w c := by
  obtain ⟨actual, found, _, _, before, after, split, earlier, later⟩ := selection_policy chosen
  rw [baseline] at found
  cases found
  have kept := List.mem_filter.mpr ⟨member, legal⟩
  rw [split] at kept
  rcases List.mem_append.mp kept with hb | ha
  · exact (key_total c w).resolve_left (earlier c hb)
  · rcases List.mem_cons.mp ha with rfl | ha
    · exact (key_total _ _).elim id id
    · exact later c ha

end LayoutSelection

private def number (s : String) : IO Nat :=
  match s.toNat? with
  | some n => pure n
  | none => throw (IO.userError s!"invalid integer: {s}")

def main (args : List String) : IO Unit := do
  let [file] := args | throw (IO.userError "expected the shared packing cases")
  let lines := (← IO.FS.readFile file).trim.splitOn "\n"
  for line in lines do
    let name :: expected :: entries := line.splitOn " " | throw (IO.userError "invalid case")
    let xs ← entries.zipIdx.mapM fun (entry, i) => do
      let [s,a,w,c] := entry.splitOn "/" | throw (IO.userError "invalid candidate")
      pure ({id := i+1, span := ← number s, area := ← number a, width := ← number w, cost := ← number c} : LayoutSelection.Candidate)
    let actual := ((LayoutSelection.select xs).map (·.id)).getD 0
    unless actual == (← number expected) do throw (IO.userError s!"{name}: got {actual}")
  IO.println s!"{lines.length} shared packing cases passed"
