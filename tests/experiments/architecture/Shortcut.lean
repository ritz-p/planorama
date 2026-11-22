import Std

namespace Shortcut

abbrev Point := Int × Int
abbrev Path := List Point

def segments (p : Path) := p.zip p.tail

def orthogonal (p : Path) : Bool := (segments p).all fun (a, b) =>
  (a.1 == b.1) != (a.2 == b.2)

def length (p : Path) : Nat := ((segments p).map fun (a, b) =>
  (a.1 - b.1).natAbs + (a.2 - b.2).natAbs).sum

structure Constraints where
  ports : Path → Bool
  obstacles : Path → Bool
  boundaries : Path → Path → Bool
  bends : Path → Nat
  cost : Path → Nat

def valid (c : Constraints) (p : Path) : Bool :=
  p.length ≥ 2 && orthogonal p && c.ports p && c.obstacles p

def endpoints (p : Path) := (p.head?, p.getLast?)

def applyCandidate (c : Constraints) (p candidate : Path) : Path :=
  if endpoints candidate = endpoints p ∧ valid c candidate ∧ c.boundaries p candidate ∧
    candidate.length ≤ p.length ∧ length candidate ≤ length p ∧
    (candidate.length < p.length ∨ length candidate < length p) ∧
    c.bends candidate ≤ c.bends p ∧ (c.bends candidate < c.bends p ∨ length candidate < length p) ∧
    c.cost candidate < c.cost p
  then candidate else p

theorem preserves (c : Constraints) (p candidate : Path) (hv : valid c p)
    (hb : c.boundaries p p) :
    let r := applyCandidate c p candidate
    endpoints r = endpoints p ∧ valid c r ∧ c.boundaries p r ∧
    length r ≤ length p ∧ c.bends r ≤ c.bends p ∧ c.cost r ≤ c.cost p := by
  dsimp only
  unfold applyCandidate
  split
  next h =>
    obtain ⟨he, hv', hb', _, hl, _, hn, _, hc⟩ := h
    exact ⟨he, hv', hb', hl, hn, Nat.le_of_lt hc⟩
  next => exact ⟨rfl, hv, hb, Nat.le_refl _, Nat.le_refl _, Nat.le_refl _⟩

def dogleg : Path := [(100,100),(160,100),(160,160),(260,160),(260,100),(320,100)]
def straight : Path := [(100,100),(320,100)]

def fixture (boundaryOK : Bool) : Constraints where
  ports := fun p => endpoints p == endpoints dogleg
  obstacles := fun _ => true
  boundaries := fun p q => p == q || boundaryOK
  bends := fun p => p.length - 2
  cost := length

example : applyCandidate (fixture true) dogleg straight = straight := by decide
example : applyCandidate (fixture false) dogleg straight = dogleg := by decide
example : length straight < length dogleg := by decide
example : orthogonal [(0,0),(1,1)] = false := by decide

end Shortcut
