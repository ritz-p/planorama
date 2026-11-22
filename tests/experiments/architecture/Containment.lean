import Std

namespace Containment

abbrev Region := List Nat
def Parent (p c : Region) := ∃ localId, c = localId :: p

inductive Ancestor : Region → Region → Prop
| parent (h : Parent p c) : Ancestor p c
| step (h : Ancestor p q) (hq : Parent q c) : Ancestor p c

theorem unique_parent (ha : Parent a c) (hb : Parent b c) : a = b := by
  obtain ⟨i, rfl⟩ := ha
  obtain ⟨j, h⟩ := hb
  exact (List.cons.inj h).2

theorem depth_increases (h : Ancestor p c) : p.length < c.length := by
  induction h with
  | parent h => obtain ⟨i, rfl⟩ := h; simp
  | step _ h ih => obtain ⟨i, rfl⟩ := h; simp; omega

theorem acyclic (p : Region) : ¬Ancestor p p := fun h => Nat.lt_irrefl _ (depth_increases h)

structure Rect where
  left : Int
  top : Int
  right : Int
  bottom : Int

def encloses (p c : Rect) := p.left ≤ c.left ∧ p.top ≤ c.top ∧ c.right ≤ p.right ∧ c.bottom ≤ p.bottom

theorem enclosing_ancestors (bounds : Region → Rect)
    (nested : ∀ p c, Parent p c → encloses (bounds p) (bounds c))
    (h : Ancestor p c) : encloses (bounds p) (bounds c) := by
  induction h with
  | parent h => exact nested _ _ h
  | step _ h ih =>
    obtain ⟨l,t,r,b⟩ := ih
    obtain ⟨l',t',r',b'⟩ := nested _ _ h
    exact ⟨Int.le_trans l l', Int.le_trans t t', Int.le_trans r' r, Int.le_trans b' b⟩

inductive Crossing | leaveAt (step : Nat) | enterAt (step : Nat) | stay (inside : Bool)

def expected (policy : Crossing) (i : Nat) : Bool :=
  match policy with
  | .leaveAt n => decide (i < n)
  | .enterAt n => decide (n ≤ i)
  | .stay inside => inside

def accepts (policy : Crossing) (trace : List Bool) : Bool :=
  let ends := match policy with
    | .leaveAt _ => (true,false)
    | .enterAt _ => (false,true)
    | .stay inside => (inside,inside)
  (trace.head? == some ends.1 && trace.getLast? == some ends.2) &&
    trace.zipIdx.all (fun (inside, i) => inside == expected policy i)

theorem agrees {policy : Crossing} {trace : List Bool} (h : accepts policy trace)
    (seen : (b,i) ∈ trace.zipIdx) : b = expected policy i := by
  simp only [accepts, Bool.and_eq_true] at h
  exact beq_iff_eq.mp (List.all_eq_true.mp h.2 _ seen)

theorem no_source_reentry {trace : List Bool} (valid : accepts (.leaveAt cut) trace)
    (outside : (false,i) ∈ trace.zipIdx) (later : (true,j) ∈ trace.zipIdx) : j < i := by
  have hi := agrees valid outside
  have hj := agrees valid later
  simp [expected] at hi hj
  omega

theorem no_destination_exit {trace : List Bool} (valid : accepts (.enterAt cut) trace)
    (inside : (true,i) ∈ trace.zipIdx) (outside : (false,j) ∈ trace.zipIdx) : j < i := by
  have hi := agrees valid inside
  have hj := agrees valid outside
  simp [expected] at hi hj
  omega

theorem excludes_unrelated {trace : List Bool} (h : accepts (.stay false) trace) :
    (true,i) ∉ trace.zipIdx := by
  intro seen
  have bad := agrees h seen
  contradiction

example : accepts (.leaveAt 1) [true,false,false] = true := by decide
example : accepts (.leaveAt 1) [true,false,true] = false := by decide
example : accepts (.enterAt 2) [false,false,true] = true := by decide
example : accepts (.stay false) [false,true,false] = false := by decide

end Containment
