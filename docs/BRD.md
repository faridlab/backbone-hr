# backbone-hr — BRD

## Documents
Employee (people master + Indonesia payroll identity) · LeaveType · LeaveBalance · LeaveApplication ·
Attendance. Own Postgres schema `hr`. Posts NO GL.

## Business rules

**BR-1 (onboard).** `onboard_employee` requires an `employee_number` (unique per company) + a name. A
given `department_id` is **verified against the REAL backbone-organization** (must exist AND belong to
the employee's company); a foreign or unknown department is refused. Indonesia payroll identity (NIK,
NPWP, `tax_status`=PTKP, bank) is captured for payroll. Status `active`. Emits `EmployeeOnboarded`.

**BR-2 (exit).** `exit_employee` transitions an **active** employee → `resigned`/`terminated` with an exit
date (terminal). Emits `EmployeeExited`.

**BR-3 (leave type + allocation).** `create_leave_type` defines a leave with an annual quota.
`allocate_leave` sets an employee's `allocated` entitlement for a (type, year) — idempotent, and never
below what's already `used`.

**BR-4 (apply).** `apply_leave` requires an **active** employee, an active leave type, and
`from ≤ to`; `days` = inclusive calendar-day span. Status `pending`.

**BR-5 (approve — the balance invariant).** `approve_leave` claims the `pending → approved` transition and
**draws the balance down in the SAME transaction, gated so `used + days ≤ allocated`**. If the balance is
insufficient (or unallocated) BOTH the draw and the transition roll back — the application stays pending
and nothing is drawn. A leave is approved **at most once**. Emits `LeaveApproved`.

**BR-6 (reject / cancel).** `reject_leave` refuses a pending application (no balance change).
`cancel_leave` cancels a pending one, or — for an **approved** one — restores the drawn balance in the
same transaction as the transition, **gated on `used >= days`** so a tampered application can't
over-restore. A DB CHECK (`used >= 0`, `used <= allocated`) backstops the balance against any writer,
including the generic PATCH surface (maturity council 2026-07-08).

**BR-7 (attendance).** `mark_attendance` records **one presence per (employee, date)** — a second mark
overwrites, never duplicates.

## Events
`EmployeeOnboarded`, `EmployeeExited`, `LeaveApproved` (carries `is_paid` — the payroll signal).

**BR-8 (payroll-facing output).** `period_summary(employee, from, to)` returns `{paid_leave_days,
unpaid_leave_days, absent_days}` — approved leave clamped + split by is_paid, and attendance absences not
covered by leave (no double-count). Payroll deducts `unpaid_leave_days + absent_days` (completeness
council 2026-07-08).

## Deferred (with reason)
Recruitment/appraisal/LMS, expense claims, shift scheduling, a Designation master, holiday-calendar leave
exclusion (calendar days v1); payroll (→ backbone-payroll).
