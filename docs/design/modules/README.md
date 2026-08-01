# Workforce Module Specs

Per-module schema specifications for the six-module decomposition ([ADR-004](../../adr/ADR-004-decompose-into-six-workforce-modules.md)).
Each doc contains the full `.model.yaml` (the source of truth codegen will consume), enums, read ports,
and design notes. When a crate is created, copy its spec's YAML into `schema/models/` and run codegen.

**Workforce-time modules** (decompose `backbone-hr` — [ADR-004](../../adr/ADR-004-decompose-into-six-workforce-modules.md)):

| Module | Spec | Source (salt-laravel) | Reads |
|---|---|---|---|
| `backbone-employee` | [spec](backbone-employee.md) | salt-laravel-employee | organization, sapiens |
| `backbone-attendance` | [spec](backbone-attendance.md) | salt-laravel-employee-attendance | employee, schedule, calendar |
| `backbone-calendar` | [spec](backbone-calendar.md) | salt-laravel-employee-calendar | organization, employee |
| `backbone-schedule` | [spec](backbone-schedule.md) | salt-laravel-employee-schedule | organization, employee |
| `backbone-timeoff` | [spec](backbone-timeoff.md) | salt-laravel-employee-timeoff | employee, calendar |
| `backbone-timesheet` | [spec](backbone-timesheet.md) | salt-laravel-employee-timesheet | employee, project |

**HRIS sibling modules** (broader HRIS — consume `backbone-employee`; only payroll exists today):

| Module | Spec | Status | Reads |
|---|---|---|---|
| `backbone-payroll` | [spec](backbone-payroll.md) | ✅ exists | employee (PTKP/NPWP/bank), attendance, timeoff, calendar → posts GL to accounting |
| `backbone-recruitment` | [spec](backbone-recruitment.md) | ❌ greenfield | organization, employee → on hire, creates Employee |
| `backbone-performance` | [spec](backbone-performance.md) | ❌ greenfield | employee, organization |
| `backbone-learning` | [spec](backbone-learning.md) | ❌ greenfield | employee (→ records held certs) |
| `backbone-lifecycle` | [spec](backbone-lifecycle.md) | ❌ greenfield | employee, payroll, timeoff → flips employment.status |

**Cross-cutting context** (coherence fix — [ADR-005](../../adr/ADR-005-hris-coherence-fixes.md)):

| Module | Spec | Status | Reads |
|---|---|---|---|
| `backbone-approvals` | [spec](backbone-approvals.md) | ❌ greenfield (12th) | employee, organization → consumed by lifecycle / timeoff / timesheet / recruitment / performance |

## Conventions (shared across all six)

- **Tenancy:** tenant-scoped tables carry `company_id` — logical FK → `organization.Company.id`, marked
  `@exclude_from_foreign_key_check`. *(salt-laravel's `organization_id` standardized to `company_id`.)*
  Universal reference data (Religion, Bank) is **global** (no `company_id`).
- **Audit:** every table has `metadata: Metadata` (`Timestamps` + `Actors` → `sapiens.User.id`).
  Soft-delete (`deleted_at`) on by default.
- **Cross-module refs** = logical FKs (`@exclude_from_foreign_key_check`), no DB constraint across
  schemas. **Within-module refs** = real FKs (same Postgres schema).
- **External imports:** `sapiens` (User, for actors). Org graph (Company/Department/Position/Level/
  Branch) is read via logical FKs, not type imports — same pattern as `backbone-hr` today.
- **No Cargo edges between modules.** Cross-module reads go through ports wired by the HR
  backend-service (the composer).

## Cross-module read-port contracts

Defined per-module in each spec; collected summary:

| Port (owner) | Signature | Consumers |
|---|---|---|
| `backbone-employee` | `resolve_employee(id) → EmployeeSnapshot` | onboarding, payroll, all |
| `backbone-employee` | `employee_ptkp(id) → PtkpTier` | payroll |
| `backbone-attendance` | `present_days(emp, from, to) → Vec<NaiveDate>` | payroll (composer derives absences) |
| `backbone-calendar` | `is_holiday(company, emp, date) → bool` | timeoff, attendance, payroll |
| `backbone-calendar` | `working_days(company, emp, from, to) → u32` | timeoff |
| `backbone-schedule` | `planned_shift(emp, date) → Option<Shift>` | attendance |
| `backbone-timeoff` | `approved_leave_days(emp, from, to) → Vec<NaiveDate>` | payroll |
| `backbone-timesheet` | `logged_hours(emp, from, to) → HoursSummary` | payroll/billing |
| `backbone-recruitment` | `hire(offer_id) → HireEvent` | employee (creates Employee), lifecycle |
| `backbone-performance` | `latest_appraisal(emp)` / `goals(emp)` | payroll (merit), talent |
| `backbone-learning` | `training_history(emp)` / `competency_matrix(emp)` / `skills(emp)` | performance, staffing, compliance |
| `backbone-lifecycle` | `onboarding_status(emp)` / `offboarding_status(emp)` | composer |

| `backbone-payroll` | `current_salary(emp)` / `salary_history(emp)` | lifecycle (pesangon), performance (merit) |
| `backbone-approvals` | `request(resource)` / `status(id)` / `decide(...)` | lifecycle, timeoff, timesheet, recruitment, performance |

> `backbone-payroll` owns `unpaid_days` (ADR-004 — a consumer of attendance+timeoff) and posts money to
> `backbone-accounting`; it also *exposes* salary/compensation reads (above).

## The payroll read contract (ADR-004)

`backbone-payroll` owns the no-double-counted-payable-days invariant:
`unpaid_days = working_days − present_days − paid_leave`
(`calendar.working_days` + `attendance.present_days` + `timeoff.paid_leave_days`), each port company-scoped.
Attendance exposes `present_days` (it owns *presence*); absences are derived — a consequence of dropping
the `AttendanceStatus` enum (no stored `absent` flag).
