import Std

namespace Ports

inductive Side | left | right | top | bottom deriving DecidableEq
inductive Role | source | target deriving DecidableEq
structure Endpoint where
  edge : Nat
  owner : Nat
  side : Side
  role : Role
  bundle : Option Nat
  reserved : Bool
  deriving DecidableEq

def mayShare (a b : Endpoint) : Prop :=
  a.owner = b.owner ∧ a.side = b.side ∧ a.role = b.role ∧
  a.reserved = false ∧ b.reserved = false ∧ ∃ trunk, a.bundle = some trunk ∧ b.bundle = some trunk

structure Slot where
  endpoint : Endpoint
  left : Nat
  width : Nat
  deriving DecidableEq

def center (s : Slot) := s.left + (s.width + 1) / 2

def compatible (a b : Slot) : Prop :=
  a.endpoint.owner ≠ b.endpoint.owner ∨ a.endpoint.side ≠ b.endpoint.side ∨
  a.left + a.width ≤ b.left ∨ b.left + b.width ≤ a.left ∨ mayShare a.endpoint b.endpoint

theorem exclusive_centers {a b : Slot} (ok : compatible a b)
    (owner : a.endpoint.owner = b.endpoint.owner) (side : a.endpoint.side = b.endpoint.side)
    (wa : 0 < a.width) (wb : 0 < b.width) (forbidden : ¬mayShare a.endpoint b.endpoint) :
    center a ≠ center b := by
  unfold center
  rcases ok with ho | hs | ab | ba | shared
  · contradiction
  · contradiction
  · omega
  · omega
  · contradiction

def relocate (s : Slot) (offset : Nat) : Slot := {s with left := offset}

theorem relocate_preserves_reservation (s : Slot) (n : Nat) :
    (relocate s n).endpoint = s.endpoint ∧ (relocate s n).width = s.width := ⟨rfl,rfl⟩

def start (used gap : Nat) := 20 + used + gap

theorem ordered_intervals (used width gap nextGap : Nat) (h : gap ≤ nextGap) :
    start used gap + width ≤ start (used + width) nextGap := by unfold start; omega

theorem gap_monotone (slot extra count : Nat) :
    (slot + 1) * extra / (count + 1) ≤ (slot + 2) * extra / (count + 1) :=
  (Nat.le_div_iff_mul_le (Nat.succ_pos count)).2 (Nat.le_trans (Nat.div_mul_le_self _ _) (Nat.mul_le_mul_right extra (by omega)))

def offsets (widths : List Nat) (height : Nat) := Id.run do
  let extra := height - (41 + widths.sum)
  let mut used := 0
  let mut ports := #[]
  for (w,i) in widths.zipIdx do
    ports := ports.push (start used ((i + 1) * extra / (widths.length + 1)) + (w + 1) / 2)
    used := used + w
  return ports

example : offsets [30,38,46] 155 = #[35,69,111] := by decide
example : offsets [24,24,24] 113 = #[32,56,80] := by decide

def trunk : Endpoint := ⟨3, 1, .right, .source, some 8, false⟩
example : mayShare trunk {trunk with edge := 4} := ⟨rfl,rfl,rfl,rfl,rfl,8,rfl,rfl⟩
example : ¬mayShare trunk {trunk with role := .target} := by simp [mayShare, trunk]
example : ¬mayShare trunk {trunk with reserved := true} := by simp [mayShare, trunk]

end Ports
