# ADR-001 — HR boundary, the leave-balance engine, and the onboarding seam

Status: accepted · 2026-07-08 · Tier 5a (People pillar; posts no GL)

## Context
HR is the **people master** — the first of the net-new People pillar, built before `backbone-payroll`
because payroll has nothing to run against without an employee roster, salary identity, and leave/
attendance inputs. ERPNext ships HR as a separate app with no Indonesia pack; the value here is a clean
master + a correct leave-balance engine + the Indonesia payroll identity that the payroll overlay needs.

## Decision
1. **HR owns `employee_id`; other modules hold it as a logical FK.** The Employee is the canonical people
   identity (payroll salary slips, project timesheets, journal party lines reference it). HR *reads* the
   `sapiens` user and the `organization` department — it owns neither.
2. **Onboarding verifies the department against the REAL organization** through `OrgPort` (zero normal
   Cargo edge): a given `department_id` must exist AND belong to the employee's company. A composing
   service wires the real organization behind the port; HR never imports it.
3. **The leave balance is the load-bearing invariant.** `approve_leave` claims the `pending → approved`
   transition and draws the balance down **in the same transaction, gated so `used + days ≤ allocated`**.
   Insufficient/unallocated → both roll back (fail-closed, application stays pending). Under READ
   COMMITTED the gated draw serializes concurrent approvals on the balance row, so total approved leave
   never exceeds the allocation. Cancelling an approved application restores the balance in one tx.
4. **The Indonesia payroll identity lives on the Employee, rules do not.** `tax_status` (PTKP), NIK, NPWP,
   bank, base salary are captured here; the *computation* (PPh 21, BPJS) is `backbone-payroll`'s overlay —
   HR carries the inputs, not the statutory math (mirrors the tax overlay split).
5. **Posts no GL.** Money moves only in payroll.

## Consequences
- Turn HR off and no ledger changes; it is a master + workflow that *feeds* payroll and time tracking.
- Proven end-to-end (`tests/hr_org_seam.rs` drives the REAL organization onboarding + department) and
  survives regen (§5).

## Parking lot (each with a gate)
- **No payroll-facing output** — FIXED (completeness council 2026-07-08): HR shipped the leave/attendance
  entities but nothing payroll could read — `LeaveApproved` omitted `is_paid` and there was no payable-days
  read, so payroll (HR's reason to exist) had no clean input. Added `is_paid` to `LeaveApproved` + a
  reconciled `period_summary(employee, from, to) → {paid_leave_days, unpaid_leave_days, absent_days}`
  (leave authoritative, absences uncovered-by-leave so no double-count) (HGC-5, proven-by-revert).
- **SapiensPort (verify `user_id`)** — symmetric to the department seam; deferred (not load-bearing for
  payroll compute).
- **Balance could go negative → phantom entitlement** — FIXED (maturity council 2026-07-08): the cancel
  restore was ungated and `@non_negative` was never a DB CHECK, so a `days`-tampered application cancelled
  to a negative `used` (approvable beyond the allocation). Added DB CHECKs (`used >= 0`, `used <=
  allocated`) as the backstop against any writer + gated the restore on `used >= days` (IP-6,
  proven-by-revert).
- **Generic CRUD/PATCH exposes leave status/days + balance** — a client can PATCH an approved application
  back to `pending` and re-approve for a double draw (bounded by the `used <= allocated` CHECK but not
  prevented). Gate: an authorization review of the generic mutation surface.
- **Cross-year leave draws only the from_date year's bucket** — a Dec→Jan leave debits one year's
  allocation; gate: split the draw per year or reject year-spanning applications.
- **Holiday-calendar leave days** — `days` is an inclusive calendar span; weekend/public-holiday exclusion
  needs a holiday calendar. Gate: a `HolidayCalendar` master (or a `service_day`-style config).
- **Carry-forward automation** — `allow_carry_forward` is stored but no year-rollover job applies it. Gate:
  a year-end allocation job.
- **Employee-number uniqueness is a write-path check, not a DB constraint** — a partial-unique index on
  `(company_id, employee_number)` would make it race-proof. Gate: a schema-hardening pass.
- **Designation master, expense claims, appraisal** — deferred (PRD non-goals).
