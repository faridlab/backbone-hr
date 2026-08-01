# `backbone-calendar` — spec

> Org-scoped holiday / working-day reference. Source: `salt-laravel-employee-calendar` (8 tables).
> Decision: [ADR-004](../../adr/ADR-004-decompose-into-six-workforce-modules.md).
>
> A calendar is scoped to org dimensions (branch / religion / department / level / position /
> employment-status / explicit-employee) — the junction tables express "which employees this applies to."

**Reads:** `organization` (Company, Department, Position, Level, Branch), `employee` (Religion master,
Employee) — via logical FKs.
**Exposes ports:** `is_holiday`, `working_days`. Consumed by timeoff (day-count), attendance, payroll.

---

## `index.model.yaml`

```yaml
module: calendar
version: 2
schema: calendar
description: "org-scoped holiday / working-day reference"

config: { database: postgresql, soft_delete: true, audit: true, default_timestamps: true, generators: { disabled: [graphql, grpc, proto] } }
external_imports: [{ module: sapiens, types: [User] }]
imports:
  - calendar.model.yaml
  - calendar_branch.model.yaml
  - calendar_religion.model.yaml
  - calendar_department.model.yaml
  - calendar_level.model.yaml
  - calendar_position.model.yaml
  - calendar_employee_status.model.yaml
  - calendar_employee.model.yaml
```

## `calendar.model.yaml`

```yaml
models:
  - name: Calendar
    collection: calendars
    description: "A holiday or working-day window scoped to org dimensions."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK organization.Company.id" }
      name: { type: string, attributes: ["@required", "@max(120)"] }
      date_start: { type: date, attributes: ["@required"] }
      date_end: { type: date, attributes: ["@required"] }
      is_holiday: { type: boolean, attributes: ["@default(false)"], description: "true = holiday (non-working); false = special working day" }
      can_everyone_view: { type: boolean, attributes: ["@default(true)"] }
      note: { type: text? }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [company_id, date_start] }
```

## Scope junctions (each: `calendar_id` + one org dimension)

```yaml
# calendar_branch — ⚠️ salt-laravel had no branch_id; added here (logical FK organization.Branch.id)
models:
  - name: CalendarBranch
    collection: calendar_branches
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      calendar_id: { type: uuid, attributes: ["@required", "@foreign_key(Calendar.id)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"] }
      branch_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK organization.Branch.id" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes: [{ type: index, fields: [calendar_id] }]

  - name: CalendarReligion
    collection: calendar_religions
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      calendar_id: { type: uuid, attributes: ["@required", "@foreign_key(Calendar.id)"] }
      religion_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK employee.Religion.id" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes: [{ type: index, fields: [calendar_id] }]

  - name: CalendarDepartment
    collection: calendar_departments
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      calendar_id: { type: uuid, attributes: ["@required", "@foreign_key(Calendar.id)"] }
      department_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK organization.Department.id" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes: [{ type: index, fields: [calendar_id] }]

  - name: CalendarLevel
    collection: calendar_levels
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      calendar_id: { type: uuid, attributes: ["@required", "@foreign_key(Calendar.id)"] }
      level_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK organization.Level.id" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes: [{ type: index, fields: [calendar_id] }]

  - name: CalendarPosition
    collection: calendar_positions
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      calendar_id: { type: uuid, attributes: ["@required", "@foreign_key(Calendar.id)"] }
      position_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK organization.Position.id" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes: [{ type: index, fields: [calendar_id] }]

  - name: CalendarEmployeeStatus
    collection: calendar_employee_statuses
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      calendar_id: { type: uuid, attributes: ["@required", "@foreign_key(Calendar.id)"] }
      employment_status: { type: EmploymentStatus, attributes: ["@required"], description: "scope: permanent/contract/probation/associate" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes: [{ type: index, fields: [calendar_id] }]

  - name: CalendarEmployee
    collection: calendar_employees
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      calendar_id: { type: uuid, attributes: ["@required", "@foreign_key(Calendar.id)"] }
      employee_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK employee.Employee.id" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes: [{ type: index, fields: [calendar_id] }, { type: index, fields: [employee_id] }]
```

## Enums

```yaml
enums:
  - name: EmploymentStatus       # shared with backbone-employee; duplicate or import per module convention
    variants: [{name: permanent, default: true}, contract, probation, associate]
```

## Read ports

```rust
#[async_trait]
pub trait CalendarPort: Send + Sync {
    /// True if `date` is a non-working holiday for this employee (resolves all scope dimensions).
    async fn is_holiday(&self, company_id: Uuid, employee_id: Uuid, date: NaiveDate) -> Result<bool, CalendarRejected>;
    /// Count of working days in [from, to] for this employee (excludes holidays + non-working weekdays).
    async fn working_days(&self, company_id: Uuid, employee_id: Uuid, from: NaiveDate, to: NaiveDate) -> Result<u32, CalendarRejected>;
    async fn holidays(&self, company_id: Uuid, from: NaiveDate, to: NaiveDate) -> Result<Vec<HolidayDay>, CalendarRejected>;
}
```

## Notes

- **`is_holiday(company, employee, date)`** resolves an employee's full org placement (branch/dept/
  level/position/religion/status) against the calendar's scope junctions — the load-bearing read for
  timeoff day-counting and payroll payable-days.
- `EmploymentStatus` is shared with backbone-employee; define once and reference, or duplicate per
  module (current backbone convention duplicates small enums).
