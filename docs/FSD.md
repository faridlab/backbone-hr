# backbone-hr — FSD

## Entities
Employee (`company_id`, `employee_number`, `user_id`/`department_id` logical FKs, `employment_type`,
`date_of_joining`/`date_of_exit`, `status`, + Indonesia identity `nik`/`npwp`/`tax_status`(PTKP)/
`bank_account_no`/`base_salary`) · LeaveType (`is_paid`, `annual_quota_days`, `allow_carry_forward`) ·
LeaveBalance (unique `(employee, leave_type, year)`; `allocated`/`used`) · LeaveApplication (`from_date`/
`to_date`/`days`, `status`, `approved_by`/`approved_at`) · Attendance (unique `(employee, date)`,
`status`, `working_hours`). Enums: EmploymentType {permanent, contract, probation, intern}, EmployeeStatus
{active, resigned, terminated}, TaxStatus {tk0..tk3, k0..k3} (PTKP), LeaveStatus {pending, approved,
rejected, cancelled}, AttendanceStatus {present, absent, half_day, on_leave, holiday}.

## Write path (`HrWriteService`, hand-authored, user-owned)
- `onboard_employee(NewEmployee, &dyn OrgPort, sink)` → verifies the department seam, emits
- `exit_employee(now, sink)`
- `create_leave_type` / `allocate_leave`
- `apply_leave` → pending (computes `days`)
- `approve_leave(now, sink)` → **balance draw + transition in one gated tx** (the invariant)
- `reject_leave` / `cancel_leave` (restores balance if approved)
- `mark_attendance` (one per employee/day, upsert)
- `period_summary(employee, from, to)` → `PeriodSummary {paid_leave_days, unpaid_leave_days, absent_days}`
  — the payroll-facing read (reconciles leave + attendance; the input backbone-payroll consumes)

Clock verbs take an explicit `now`. Errors: `HrError {Db, NotFound, InvalidState, Invalid,
InsufficientBalance, OrgRejected}`.

## Seam (port — zero normal Cargo edge)
- **Onboard → organization (proven, HRSEAM-1):** `onboard_employee` resolves a `department_id` through
  `OrgPort` against the REAL backbone-organization — exists + same-company; foreign/unknown refused. ADR-001.
- **Outbound:** HR emits events (`EmployeeOnboarded`/`Exited`, `LeaveApproved`) for payroll to consume; it
  posts no GL and drives no writes.

## Test oracle
`hr_golden_cases` (5: approve draws balance, approve-insufficient refused, cancel restores, validation,
HGC-5 payroll-facing period_summary),
`integrity_probes` (6: approve-once, balance-never-over-drawn, apply-requires-active-employee, attendance-
one-per-day, exit-terminal, IP-6 balance-cannot-go-negative), `hr_org_seam` (1: onboard against a REAL
organization department) + §5 round-trip. **12 tests.**

> The generated `integration_tests.rs` hits an external HTTP server (`API_BASE_URL`, default
> `127.0.0.1:3000`) and is environmental scaffolding, not part of this module's correctness gate.
