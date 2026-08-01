# `backbone-timesheet` — spec

> Time logging + approvals. Source: `salt-laravel-employee-timesheet` (2 tables + settings). Decision: [ADR-004](../../adr/ADR-004-decompose-into-six-workforce-modules.md).
>
> Cleanly separable — reads only `employee` (+ `project`/`task`). Standalone module per the design defaults.

**Reads:** `employee` (Employee), `project` (Project, Task) — via logical FKs.
**Exposes port:** `logged_hours(emp, from, to)`.

---

## `index.model.yaml`

```yaml
module: timesheet
version: 2
schema: timesheet
description: "timesheet entries + approval cycle"

config: { database: postgresql, soft_delete: true, audit: true, default_timestamps: true, generators: { disabled: [graphql, grpc, proto] } }
external_imports: [{ module: sapiens, types: [User] }]
imports: [timesheet.model.yaml, timesheet_approval.model.yaml]
```

## `timesheet.model.yaml`

```yaml
models:
  - name: Timesheet
    collection: timesheets
    description: "A logged time entry (work or overtime) against a project/task."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK organization.Company.id" }
      employee_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK employee.Employee.id" }
      project_id: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK project.Project.id" }
      task_id: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK project.Task.id" }
      year: { type: integer, attributes: ["@required"] }
      month: { type: integer, attributes: ["@required"], description: "1–12" }
      date: { type: date, attributes: ["@required"] }
      remark: { type: string?, attributes: ["@max(255)"] }
      time_start: { type: datetime? }
      time_end: { type: datetime? }
      type: { type: TimesheetType, attributes: ["@required", "@default(work)"], description: "work / overtime" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [company_id, employee_id, date] }
      - { type: index, fields: [project_id] }

enums:
  - name: TimesheetType
    variants: [{name: work, default: true}, overtime]
```

## `timesheet_approval.model.yaml`

```yaml
models:
  - name: TimesheetApproval
    collection: timesheet_approvals
    description: "Approval cycle for a timesheet period (per employee/month)."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"] }
      employee_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK employee.Employee.id" }
      approver_id: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK employee.Employee.id" }
      year: { type: integer, attributes: ["@required"] }
      month: { type: integer, attributes: ["@required"] }
      remark: { type: text? }
      billable_time: { type: decimal?, attributes: ["@precision(18,2)"], description: "Approved billable hours" }
      billable_cost: { type: decimal?, attributes: ["@precision(12,2)"], description: "Approved cost" }
      status: { type: TimesheetApprovalStatus, attributes: ["@required", "@default(pending)"] }
      data: { type: json?, description: "Extensible payload" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [company_id, employee_id, year, month] }

enums:
  - name: TimesheetApprovalStatus
    variants: [{name: pending, default: true}, approved, rejected]
```

## Read port

```rust
#[async_trait]
pub trait TimesheetPort: Send + Sync {
    async fn logged_hours(&self, company_id: Uuid, employee_id: Uuid, from: NaiveDate, to: NaiveDate) -> Result<HoursSummary, TimesheetRejected>;
}

// HoursSummary { work_hours, overtime_hours, billable_hours, cost }
```

## Notes

- **Placement:** standalone (default). Alternative — fold into `backbone-project` (it references
  `project_id`/`task_id`). Kept standalone to mirror salt-laravel and keep project focused on
  deliverables; revisit if timesheet approvals and project billing converge.
- Two approval-ish modules (timeoff + timesheet) each carry their own `status` enum for now (YAGNI a
  shared approval engine); extract one if a third approval surface appears.
