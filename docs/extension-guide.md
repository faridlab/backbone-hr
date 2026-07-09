# backbone-hr — Extension Guide

## Public surface (stable)
- **Events** (`application::service::hr_events`): `EmployeeOnboarded`, `EmployeeExited`, `LeaveApproved`,
  the `HrEvent` union, and `HrEventSink` (a consuming service supplies its own — bus, outbox, …). Payroll
  subscribes to build its roster + paid/unpaid day computation.
- **Port** (`application::service::hr_ports`): `OrgPort` + DTOs (`DepartmentRef`, `HrRejected`) — the
  onboarding department-verification seam a composing service implements over backbone-organization. Zero
  normal Cargo edge.
- **Write path** (`application::service::hr_write_service::HrWriteService`): the guarded verbs
  (`onboard_employee`, `exit_employee`, `create_leave_type`, `allocate_leave`, `apply_leave`,
  `approve_leave`, `reject_leave`, `cancel_leave`, `mark_attendance`).

## How a consuming service (e.g. payroll) uses HR
Read the Employee master by `employee_id` for the salary/tax identity; call
`HrWriteService::period_summary(employee, from, to)` for the reconciled `PeriodSummary
{paid_leave_days, unpaid_leave_days, absent_days}` — the payroll input (deduct `unpaid_leave_days +
absent_days`). Subscribe to `LeaveApproved` (carries `is_paid`) for real-time reaction. Payroll never
mutates HR state.

## Not a contract
- The 12 generated CRUD endpoints per entity are convenience scaffolding. Do **not** mutate a
  `leave_balance` or a `leave_application` status through the generic PATCH surface — it bypasses the
  gated draw + the transition atomicity. Use `HrWriteService`.
- `// <<< CUSTOM` blocks preserve local edits only; not a cross-module extension point.

## Invariants a consumer must not break
- A leave balance's `used` never exceeds `allocated`; only `approve_leave`/`cancel_leave` move it.
- A leave is approved at most once; the draw is atomic with the transition.
- The Indonesia payroll identity (`tax_status`, NIK, NPWP) is the *input* to payroll's statutory math —
  don't recompute PTKP relief in HR.
