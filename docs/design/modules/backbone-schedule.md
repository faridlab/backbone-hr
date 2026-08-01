# `backbone-schedule` — spec

> Shifts / roster. Source: `salt-laravel-employee-schedule` (4 tables). Decision: [ADR-004](../../adr/ADR-004-decompose-into-six-workforce-modules.md).

**Reads:** `organization` (Company, Structure), `employee` (Employee) — via logical FKs.
**Exposes port:** `planned_shift(emp, date)`. Consumed by attendance (actual-vs-planned).

---

## `index.model.yaml`

```yaml
module: schedule
version: 2
schema: schedule
description: "shift / roster definitions and assignment"

config: { database: postgresql, soft_delete: true, audit: true, default_timestamps: true, generators: { disabled: [graphql, grpc, proto] } }
external_imports: [{ module: sapiens, types: [User] }]
imports: [schedule.model.yaml, schedule_weekday.model.yaml, schedule_organization.model.yaml, schedule_employee.model.yaml]
```

## `schedule.model.yaml`

```yaml
models:
  - name: Schedule
    collection: schedules
    description: "A shift definition (times + recurrence + holiday override)."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK organization.Company.id" }
      name: { type: string, attributes: ["@required", "@max(120)"] }
      is_default: { type: boolean, attributes: ["@default(false)"] }
      order_number: { type: integer, attributes: ["@default(0)"] }
      start_date: { type: date? }
      end_date: { type: date? }
      time_in: { type: time, attributes: ["@required"] }
      time_out: { type: time, attributes: ["@required"] }
      is_override_holiday: { type: boolean, attributes: ["@default(false)"], description: "Work this shift even on holidays" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [company_id] }
```

## `schedule_weekday.model.yaml`

```yaml
models:
  - name: ScheduleWeekday
    collection: schedule_weekdays
    description: "Weekly recurrence for a schedule (which days are working days)."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      schedule_id: { type: uuid, attributes: ["@required", "@foreign_key(Schedule.id)"] }
      sun: { type: WeekdayType, attributes: ["@default(weekend)"] }
      mon: { type: WeekdayType, attributes: ["@default(weekday)"] }
      tue: { type: WeekdayType, attributes: ["@default(weekday)"] }
      wed: { type: WeekdayType, attributes: ["@default(weekday)"] }
      thu: { type: WeekdayType, attributes: ["@default(weekday)"] }
      fri: { type: WeekdayType, attributes: ["@default(weekday)"] }
      sat: { type: WeekdayType, attributes: ["@default(weekend)"] }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [schedule_id] }

enums:
  - name: WeekdayType
    variants: [{name: weekday, default: true}, weekend]
```

## `schedule_organization.model.yaml` + `schedule_employee.model.yaml`

```yaml
models:
  - name: ScheduleOrganization
    collection: schedule_organizations
    description: "Org-level schedule assignment with override times."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"] }
      structure_id: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK organization.Structure.id (Structure not yet in org — see note)" }
      name: { type: string?, attributes: ["@max(120)"] }
      order_number: { type: integer, attributes: ["@default(0)"] }
      start_date: { type: date? }
      end_date: { type: date? }
      time_in: { type: time, attributes: ["@required"] }
      time_out: { type: time, attributes: ["@required"] }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes: [{ type: index, fields: [company_id] }]

  - name: ScheduleEmployee
    collection: schedule_employees
    description: "Per-employee roster override for a specific date."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"] }
      employee_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK employee.Employee.id" }
      schedule_date: { type: date, attributes: ["@required"] }
      order_number: { type: integer, attributes: ["@default(0)"] }
      time_in: { type: time, attributes: ["@required"] }
      time_out: { type: time, attributes: ["@required"] }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [employee_id, schedule_date] }
```

## Read port

```rust
#[async_trait]
pub trait SchedulePort: Send + Sync {
    /// The planned shift for an employee on a date (override > org assignment > default schedule). None = day off.
    async fn planned_shift(&self, company_id: Uuid, employee_id: Uuid, date: NaiveDate) -> Result<Option<Shift>, ScheduleRejected>;
}
```

## Notes

- **`Structure`** (referenced by `schedule_organizations.structure_id`) is not yet in `backbone-organization`
  — same gap as Position/Level; add to org as part of the org-design masters.
- **ScheduleWeekday** keeps salt-laravel's 7-column form (simple, matches source). Normalize later if
  recurrence rules grow beyond weekly.
