import Std

namespace Reference

inductive Knowledge | known | unknown | sensitive deriving DecidableEq
structure Identity where
  address : Nat
  provider : Nat
  state : Nat
  managed : Bool
  current : Bool
  deriving DecidableEq

structure Input where
  source : Identity
  selected : List Identity
  candidates : List Identity
  knowledge : Knowledge
  complete : Bool
  scopeCompatible : Bool

def admissible (i : Input) (t : Identity) : Prop :=
  i.source ∈ i.selected ∧ t ∈ i.selected ∧ i.knowledge = .known ∧
  i.complete = true ∧ i.scopeCompatible = true ∧ t.provider = i.source.provider ∧
  t.state = i.source.state ∧ t.managed = true ∧ t.current = true

instance (i : Input) (t : Identity) : Decidable (admissible i t) := by
  unfold admissible
  infer_instance

def resolve (i : Input) : Option Identity :=
  match i.candidates with
  | [t] => if admissible i t then some t else none
  | _ => none

theorem sound {i : Input} {t : Identity} (h : resolve i = some t) :
    i.candidates = [t] ∧ admissible i t := by
  unfold resolve at h
  split at h
  next u he =>
    split at h
    next ha => cases h; exact ⟨he, ha⟩
    next => contradiction
  next => contradiction

theorem reject {i : Input}
    (h : ∀ t, i.candidates ≠ [t] ∨ ¬ admissible i t) : resolve i = none := by
  cases he : resolve i with
  | none => rfl
  | some t =>
    obtain ⟨one, valid⟩ := sound he
    rcases h t with hn | hn
    · exact False.elim (hn one)
    · exact False.elim (hn valid)

theorem reject_unavailable (i : Input)
    (h : i.knowledge ≠ .known ∨ i.complete ≠ true ∨ i.scopeCompatible ≠ true) :
    resolve i = none := by
  apply reject
  intro t
  right
  intro valid
  obtain ⟨_, _, known, complete, compatible, _⟩ := valid
  rcases h with no | no | no
  · exact no known
  · exact no complete
  · exact no compatible

theorem endpoints_and_namespace {i : Input} {t : Identity} (h : resolve i = some t) :
    i.source ∈ i.selected ∧ t ∈ i.selected ∧
      t.provider = i.source.provider ∧ t.state = i.source.state := by
  obtain ⟨_, hs, ht, _, _, _, hp, hstate, _⟩ := sound h
  exact ⟨hs, ht, hp, hstate⟩

def fixture (count provider : Nat) : Bool := Id.run do
  let source : Identity := ⟨0, 0, 0, true, true⟩
  let candidates := (List.range count).map fun n => {source with address := n + 1, provider}
  return (resolve ⟨source, source :: candidates, candidates, .known, true, true⟩).isSome

end Reference

def main (args : List String) : IO Unit := do
  let [path] := args | throw (IO.userError "expected fixture path")
  for line in (← IO.FS.readFile path).trim.splitOn "\n" do
    let [count, provider, expected] := line.splitOn " " | throw (IO.userError "invalid fixture")
    let some count := count.toNat? | throw (IO.userError "invalid count")
    let some provider := provider.toNat? | throw (IO.userError "invalid provider")
    unless Reference.fixture count provider == (expected == "accept") do
      throw (IO.userError line)
