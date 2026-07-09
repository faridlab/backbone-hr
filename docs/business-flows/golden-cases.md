# backbone-hr — business flows & golden cases

## Flow: onboard → allocate → apply → approve (the leave engine)
```
onboard_employee (verify department vs REAL organization) → EmployeeOnboarded
   │
   ▼  create_leave_type · allocate_leave (set allocated for employee/type/year)
   │
   ▼  apply_leave → pending (days = inclusive span)
   │
   ▼  approve_leave → [pending→approved] + draw balance (gated used+days ≤ allocated), ONE tx → LeaveApproved
   │                    └─ insufficient → BOTH roll back (stays pending, nothing drawn)
   │
   └▶ cancel_leave (approved) → restore balance
```
Separate: `mark_attendance` (one per employee/day) · `exit_employee` → EmployeeExited. Posts NO GL.

## Golden cases (`tests/hr_golden_cases.rs`)
- **HGC-1 — approve draws the balance.** 3-day leave against a 12-day allocation → `used = 3`, status
  approved, `LeaveApproved` carries days=3.
- **HGC-2 — insufficient refused.** A 5-day leave against a 2-day allocation → `InsufficientBalance`,
  nothing drawn, application stays pending.
- **HGC-3 — cancel restores.** Cancelling an approved 3-day leave returns `used` to 0.
- **HGC-4 — validation.** Employee needs a number; duplicate employee_number refused; `to < from` refused.
- **HGC-5 — payroll-facing output.** A 2-day paid + 3-day unpaid leave + 1 uncovered absence →
  `period_summary` = {paid 2, unpaid 3, absent 1}; `LeaveApproved` carries `is_paid`.

## Integrity probes (`tests/integrity_probes.rs`)
- **IP-1 — approve once.** A re-approve is refused; drawn exactly once.
- **IP-2 — balance never over-drawn.** 4+2 days against a 5-day allocation → the second is refused.
- **IP-3 — apply requires active employee.** An exited employee cannot apply.
- **IP-4 — attendance one per day.** A second mark overwrites, never duplicates.
- **IP-5 — exit terminal.** An inactive employee cannot be re-exited.

## Seam (`tests/hr_org_seam.rs`)
- **HRSEAM-1 — onboard vs REAL organization.** Onboard into a real department (same company) succeeds; a
  foreign-company department + an unknown department are both refused.

## §5 round-trip (`scripts/hr_org_seam_roundtrip.sh`)
Regen (`--force`) leaves the seam files byte-identical; the oracle + seam re-run green.
