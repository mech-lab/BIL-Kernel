import Mathlib

theorem bil_phase5_commutes (a b : Nat) : a + b = b + a := by
  exact Nat.add_comm a b
