# backbone-hr — PRD

Tier 5a · People pillar · posts **no GL** · the people master downstream modules reference.

## Why
An Indonesia SMB that runs its own payroll needs a **people master** first: who is employed, their
Indonesia payroll identity (NIK, NPWP, PTKP tax status, bank), and their leave/attendance — the inputs a
payroll run consumes. This is the lean HR core: onboard employees, run a correct **leave-balance engine**
(you cannot approve leave you don't have), track attendance. It owns `employee_id` — the logical FK
payroll, timesheets, and journal party lines all reference.

## Scope (KEEP — tier5-deferred.md §3)
- **Employee** — the canonical people master + employment lifecycle (join → resign/terminate), carrying
  the Indonesia payroll identity (NIK, NPWP, `tax_status` = PTKP, bank account, base salary). Its
  **department** is verified against `backbone-organization` at onboarding; its `user_id` (`sapiens`) is a
  logical FK (verification is a follow-on — not load-bearing for payroll compute).
- **Payroll-facing output** — `period_summary(employee, from, to)` reconciles approved leave (split
  paid/unpaid) + uncovered attendance absences into the days that scale pay; `LeaveApproved` carries
  `is_paid`. This is the input `backbone-payroll` consumes — the reason HR is built first.
- **LeaveType** — a kind of leave with an annual quota (statutory annual = 12 days).
- **LeaveBalance** — per employee/type/year: `allocated` / `used`; `available = allocated − used`.
- **LeaveApplication** — a request over a date range; approving **draws the balance down, gated on
  availability, atomic with the transition**; cancelling an approved one restores it.
- **Attendance** — one presence record per employee per day (feeds payroll working/absent days).

## Non-goals (CUT / DEFER — tier5-deferred.md §3)
- Recruitment / ATS, performance / appraisal cycles, LMS / training.
- Expense-claim workflows (a lean claim rides billing later), shift-scheduling optimization.
- A Designation master (free-text for now), holiday-calendar-aware leave-day exclusion (calendar days v1).
- **Payroll itself** — the salary run, statutory deductions (BPJS, PPh 21), and GL posting live in
  `backbone-payroll` (Tier 5b), which reads this master.

## Success criteria
- The leave balance is exact and never over-drawn: `used ≤ allocated` under retry/concurrency (golden +
  integrity).
- Onboarding verifies a real organization department (proven against REAL backbone-organization).
- Zero normal Cargo edge; survives a full codegen regen (§5).
