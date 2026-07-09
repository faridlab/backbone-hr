-- Maturity council 2026-07-08: the leave-balance invariant was 100% application-enforced — `@non_negative`
-- on `used` was never translated to a DB CHECK, so a tampered `days` (via the generic PATCH) restored on
-- cancel could drive `used` negative and manufacture phantom entitlement (the approve gate reads a
-- corrupted `used`). These CHECKs make the invariant DB-enforced against ANY writer, not just the gated
-- write path. Hand-authored migration (no generator marker) — preserved across regen.
ALTER TABLE hr.leave_balances
  ADD CONSTRAINT leave_balances_used_non_negative CHECK (used >= 0);
ALTER TABLE hr.leave_balances
  ADD CONSTRAINT leave_balances_used_within_allocated CHECK (used <= allocated);
