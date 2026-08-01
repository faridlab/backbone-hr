# Workforce Modules — Schema Design (6 modules)

> Design doc for [ADR-004](../adr/ADR-004-decompose-into-six-workforce-modules.md). Maps the proven
> `salt-laravel-employee*` packages → backbone schema YAML. **Review this before any crate is created.**
> Source tables verified against `frameworks/salt-laravel-employee*/database/migrations/`.

## Shared conventions (all 6 modules)

- **Tenancy:** every tenant-scoped table carries `company_id` (logical FK → `organization.Company.id`),
  matching existing backbone modules. *(salt-laravel used `organization_id`; standardized to `company_id`.)*
- **Audit:** every table carries `Metadata` = `Timestamps` (`created_at`/`updated_at`/`deleted_at`) +
  `Actors` (`created_by`/`updated_by`/`deleted_by` → `sapiens.User.id`). Soft-delete on by default.
- **Cross-module refs are logical FKs** (`@exclude_from_foreign_key_check`) — no DB constraint across
  schemas, exactly as `backbone-hr` reads `organization`/`sapiens` today.
- **Naming:** PascalCase entity, snake_case plural collection. UUID PKs.
- **External imports:** `sapiens` (User) for actors; `organization` (Company/Department/Position/Level)
  for the org graph. `employee` (Employee) for everything that references a person.

---

## Module 1 — `backbone-employee`  *(the people master; 13 aggregates)*

Source: `salt-laravel-employee`. Read by **every** other workforce module + `backbone-payroll`.

| Entity | Collection | Key fields | Notes |
|---|---|---|---|
| `Employee` | `employees` | `employee_number`, `first_name`, `last_name?`, `email?`, `mobile_phone?`, `phone?`, `birth_place?`, `birth_date?`, `gender?`, `marital_status?`, `blood_type?`, `religion_id?` | The master. Owns `employee_id` (the logical FK all modules reference). Per-company `employee_number` unique (partial-unique index, ADR-001). |
| `EmployeeIdentity` | `employee_identities` | `employee_id`, `identity_type` (ID/passport), `identity_number`, `identity_expiry_date?`, `is_permanent` | KTP / passport. Multiple per employee. |
| `Employment` | `employments` | `employee_id`, `employment_status`, `join_date`, `end_join_date?`, `company_id`, `department_id?`, `level_id?`, `position_id?`, `direct_manager_id?`, `status` (active/inactive) | The org placement + lifecycle. Reads org graph (dept/level/position) via `EmployeePort`-style seam. |
| `EmployeeFamily` | `employee_families` | `employee_id`, `name`, **`relationship`** (spouse/child/parent/sibling/other), `birth_date?` | ⚠️ **enriched** — salt-laravel had only `name`. `relationship` drives PTKP (see below). |
| `EmployeeContact` | `employee_contacts` | `employee_id`, `name`, `phone?`, `email?` | Emergency / alternate contacts. |
| `EmployeeEducation` | `employee_educations` | `employee_id`, `institution_name`, `major?`, `field?`, `score?`, `start_year?`, `end_year?` | *(typo `instituion` fixed → `institution`)* |
| `EmployeeCertification` | `employee_certifications` | `employee_id`, `name`, `issuing_organization?`, `start_date?`, `end_date?`, `description?` | Track + expiry (feeds compliance). |
| `EmployeeWorkExperience` | `employee_work_experiences` | `employee_id`, `company_name`, `job_position?`, `start_date?`, `end_date?` | Pre-joining history. |
| `EmployeeBankAccount` | `employee_bank_accounts` | `employee_id`, `bank_id`, `account_number`, `account_name?` | Disbursement for net pay. |
| `EmployeeTax` | `employee_taxes` | `employee_id`, `npwp_number?`, `tax_method` (gross/gross_up/netto), `tax_salary` (taxable/non_taxable), `taxable_date?`, `beginning_netto?`, `pph21_paid?`, **`ptkp_override?`** | ⚠️ `ptkp_status` is no longer a free enum — see PTKP rule. `ptkp_override` allows manual override of the derived tier. |
| `EmployeeBpjs` | `employee_bpjs` | `employee_id`, `bpjs_ketenagakerjaan_number?`, `npp_bpjs_ketenagakerjaan?`, `bpjs_ketenagakerjaan_date?`, `bpjs_kesehatan_number?`, `bpjs_kesehatan_family?` (0–10), `bpjs_kesehatan_date?`, `jaminan_pensiun_date?` | Statutory contributions (read by payroll). |
| `Religion` | `religions` | `name` | Reference master (calendar scopes by religion). |
| `Bank` | `banks` | `name`, `code?` | Reference master. |

**Enums:** `Gender` (male/female), `MaritalStatus` (single/married/widow/widower), `BloodType` (A/B/AB/O),
`EmploymentStatus` (permanent/contract/probation/**associate** — *typo `assosiate` fixed*),
`EmploymentState` (active/inactive), `IdentityType` (id/passport), `TaxMethod` (gross/gross_up/netto),
`FamilyRelationship` (spouse/child/parent/sibling/other).

### ⚠️ The PTKP-from-dependents fix (the live correctness bug)

salt-laravel stored `employee_taxes.ptkp_status` as a free enum decoupled from dependents — so the
model could not honor *"add a child → tax tier changes → PPh 21 relief changes."* In `backbone-employee`
the tier is **derived** from `EmployeeFamily`:

```
married    = EXISTS EmployeeFamily(employee, relationship = spouse)
dependents = COUNT EmployeeFamily(employee, relationship = child)   // capped at 3
ptkp       = (married ? "K" : "TK") + "/" + min(dependents, 3)      // ∈ TK/0..3, K/0..3
```

- Default: derived. `EmployeeTax.ptkp_override` (nullable) lets payroll/HR force a tier for edge cases
  (e.g. spouse has separate income → `K/I/0`-style override).
- Exposed to consumers via a read port: `employee_ptkp(employee_id) → PtkpTier`.

**Read ports exposed:** `resolve_employee(id) → EmployeeSnapshot` (identity + employment + payroll-
identity + family, what onboarding/payroll need); `employee_ptkp(id) → PtkpTier`.

---

## Module 2 — `backbone-attendance`  *(presence)*

Source: `salt-laravel-employee-attendance`. Reads `employee`, `schedule`, `calendar`.

| Entity | Collection | Key fields | Notes |
|---|---|---|---|
| `Attendance` | `attendances` | `company_id`, `employee_id`, `date`, `schedule` (json snapshot), `clockin?`, `clockout?`, `time_debt?` (json), `timeoff?` (json) | One per employee per day. **No `status` enum** — aligns with Laravel's `schedule_type` snapshot design (council parking lot: drop the invented `AttendanceStatus`). |
| `AttendanceClock` | `attendance_clocks` | `attendance_id`, `company_id`, `employee_id`, `date`, `clock` | Raw clock events (in/out punches). |

**Read ports exposed:** `absences(employee_id, from, to) → Vec<NaiveDate>` (days absent with no clock
record and not a scheduled day off); `clock_events(employee_id, from, to)`. *(No `period_summary` here —
that cross-cutting computation moves to payroll, ADR-004.)*

---

## Module 3 — `backbone-calendar`  *(holiday / working-day reference)*

Source: `salt-laravel-employee-calendar`. A calendar is scoped to org dimensions (which
branches/religions/departments/levels/positions/statuses/employees it applies to).

| Entity | Collection | Key fields | Notes |
|---|---|---|---|
| `Calendar` | `calendars` | `company_id`, `name`, `date_start`, `date_end`, `is_holiday`, `can_everyone_view`, `note?` | A holiday or working-day window. |
| `CalendarBranch` | `calendar_branches` | `calendar_id`, `company_id`, `branch_id?` | Scope: applies to these branches. |
| `CalendarReligion` | `calendar_religions` | `calendar_id`, `religion_id` | Scope: religion-specific holidays. |
| `CalendarDepartment` | `calendar_departments` | `calendar_id`, `department_id` | Scope. |
| `CalendarLevel` | `calendar_levels` | `calendar_id`, `level_id` | Scope. |
| `CalendarPosition` | `calendar_positions` | `calendar_id`, `position_id` | Scope. |
| `CalendarEmployeeStatus` | `calendar_employee_statuses` | `calendar_id`, `employment_status` | Scope. |
| `CalendarEmployee` | `calendar_employees` | `calendar_id`, `employee_id` | Scope: explicit per-employee. |

**Read ports exposed:** `is_holiday(company_id, employee_id, date) → bool` (resolves all scope
dimensions); `holidays(company_id, from, to) → Vec<HolidayDay>`; `working_days(company_id, employee_id,
from, to) → u32` (used by timeoff day-counting).

---

## Module 4 — `backbone-schedule`  *(shifts / roster)*

Source: `salt-laravel-employee-schedule`.

| Entity | Collection | Key fields | Notes |
|---|---|---|---|
| `Schedule` | `schedules` | `company_id`, `name`, `is_default`, `order_number`, `start_date?`, `end_date?`, `time_in`, `time_out`, `is_override_holiday` | Shift definition. |
| `ScheduleWeekday` | `schedule_weekdays` | `schedule_id`, `sun`/`mon`/…/`sat` (weekday/weekend) | Weekly recurrence per day. |
| `ScheduleOrganization` | `schedule_organizations` | `company_id`, `structure_id?`, `name?`, `order_number`, `start_date?`, `end_date?`, `time_in`, `time_out` | Org-level assignment with override times. |
| `ScheduleEmployee` | `schedule_employees` | `employee_id`, `schedule_date`, `order_number`, `time_in`, `time_out` | Per-employee roster override. |

**Read ports exposed:** `planned_shift(employee_id, date) → Option<Shift>` (time_in/time_out or none).

---

## Module 5 — `backbone-timeoff`  *(leave)*

Source: `salt-laravel-employee-timeoff`. Carries the **balance-drawdown invariant** (ADR-001 §3).

| Entity | Collection | Key fields | Notes |
|---|---|---|---|
| `TimeoffType` | `timeoff_types` | `company_id`, `name`, `is_paid?`, `allow_carry_forward?`, `code?` | The leave policy (annual/sick/maternity…). |
| `TimeoffRequest` | `timeoff_requests` | `company_id`, `timeoff_type_id`, `employee_id`, `date_start`, `date_end`, `note?`, `approval_employee_id?`, `note_reject?`, **`status`** (pending/approved/rejected/cancelled) | ⚠️ **`status` added** (salt-laravel had none; backbone-hr's leave_application does). The `pending→approved` transition triggers the drawdown. |
| `TimeoffBalance` | `timeoff_balances` | `company_id`, `timeoff_type_id`, `employee_id`, `period`, `allocated`, `used`, **CHECK (`used >= 0`, `used <= allocated`)** | ⚠️ **enriched** — salt-laravel's `timeoff_employees` was sparse (no allocated/used). This carries the load-bearing invariant: `approve` draws `used + days ≤ allocated` in one tx (ADR-001 §3). |

**Read ports exposed:** `approved_leave_days(employee_id, from, to) → Vec<NaiveDate>` (days covered by
an approved request); `balance(employee_id, timeoff_type_id, period) → BalanceSnapshot`.

**Depends on (read):** `backbone-calendar` (working-day count for `date_start..date_end`).

---

## Module 6 — `backbone-timesheet`  *(time logging)*

Source: `salt-laravel-employee-timesheet`. Cleanly separable — reads only `employee` (+ `project`/`task`).

| Entity | Collection | Key fields | Notes |
|---|---|---|---|
| `Timesheet` | `timesheets` | `company_id`, `employee_id`, `project_id?`, `task_id?`, `year`, `month`, `date`, `remark?`, `time_start?`, `time_end?`, `type` (work/overtime) | Logged hours. `project_id`/`task_id` → `backbone-project` (logical FK). |
| `TimesheetApproval` | `timesheet_approvals` | `company_id`, `employee_id`, `approver_id`, `year`, `month`, `remark?`, `billable_time?`, `billable_cost?`, `status` (pending/approved/rejected), `data?` (json) | Approval cycle. |

**Read ports exposed:** `logged_hours(employee_id, from, to) → HoursSummary`.

---

## Cross-cutting — the payroll read contract (ADR-004)

`backbone-payroll` owns the no-double-counted-payable-days invariant, computed from three port reads
(replacing the old `backbone-hr.period_summary`):

```
absences       = attendance.absences(emp, from, to)          // company-scoped
approved_leave = timeoff.approved_leave_days(emp, from, to)  // company-scoped
unpaid_days    = absences − approved_leave                   // computed in payroll
```

Each port query takes `company_id` + `employee_id` so the tenant fence is preserved (the RLS concern
the council raised). No module owns another's invariant.

---

## Migration map — what happens to `backbone-hr`

| Current `backbone-hr` entity | → new module |
|---|---|
| `employee` (+ enums: EmploymentType, EmployeeStatus, TaxStatus) | `backbone-employee` (expanded to 13 aggregates) |
| `attendance` (+ AttendanceStatus) | `backbone-attendance` (drop the invented status enum) |
| `leave_type` / `leave_application` / `leave_balance` | `backbone-timeoff` (TimeoffType / TimeoffRequest / TimeoffBalance) |
| `OrgPort`, `hr_write_service`, `hr_events`, `hr_ports` | dissolve — read ports move into each module; the write-service logic (leave drawdown) moves into `backbone-timeoff`'s custom service. |

`backbone-hr` is **removed** once its content is migrated and payroll is repointed.

---

## Open design questions (resolve before scaffolding)

1. **`Position` / `Level` / org `Structure` have no home.** ✅ RESOLVED (probe): `backbone-organization`
   owns `Company`, `Department`, `Branch`, `Industry` — but **not** `Position`, `Level`, or `Structure`,
   which `employments`, `calendar_*`, and `schedule_organizations` all reference. Recommend **adding
   `Position` + `Level` (and a `Structure`/org-unit concept) to `backbone-organization**` — they are
   org-design / job-architecture concepts, not employee-specific. **Blocks** employee + calendar design.
2. **`ScheduleWeekday` shape** — keep 7 boolean-ish enum columns (sun..sat) or normalize to a
   `weekday_recurrence` pattern? (salt-laravel's 7-column form is simple but rigid.)
3. **Timesheet placement** — standalone `backbone-timesheet`, or fold into `backbone-project` (it
   references `project_id`/`task_id`)? Council leaned standalone; confirm.
4. **PTKP edge cases** — do we need `K/I/0` (spouse with separate income) tiers, or are TK/0–3 + K/0–3
   sufficient? (salt-laravel had only those 8.)
5. **Approval workflow engine** — timeoff + timesheet both have approvals; share one approval module, or
   keep per-module status fields for now?
