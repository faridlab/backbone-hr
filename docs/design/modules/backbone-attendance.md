# `backbone-attendance` — spec

> Presence + clock events. Source: `salt-laravel-employee-attendance` (2 tables). Decision: [ADR-004](../../adr/ADR-004-decompose-into-six-workforce-modules.md).
>
> ⚠️ **No `status` enum.** Aligns with Laravel's `schedule_type` JSON snapshot design (council parking
> lot: the Rust port's invented `AttendanceStatus` + cross-table join is dropped). Interpretation
> (absent? on_leave? holiday?) is computed by the consumer (payroll), not stored here.

**Reads:** `employee` (Employee), `schedule` (planned shift), `calendar` (holiday) — via ports.
**Exposes port:** `absences(emp, from, to)`.
**Does NOT own:** `period_summary` / no-double-count — that lives in payroll (ADR-004).

---

## `index.model.yaml`

```yaml
module: attendance
version: 2
schema: attendance
description: "employee presence + clock events"

config: { database: postgresql, soft_delete: true, audit: true, default_timestamps: true, generators: { disabled: [graphql, grpc, proto] } }
external_imports: [{ module: sapiens, types: [User] }]
# shared_types Metadata ([Timestamps, Actors]) — same shape as backbone-employee; reused per-module.
imports: [attendance.model.yaml, attendance_clock.model.yaml]
```

## `attendance.model.yaml`

```yaml
models:
  - name: Attendance
    collection: attendances
    description: "An employee's presence on a date (one per employee per day). Carries schedule/timeoff snapshots, not a status enum."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK organization.Company.id" }
      employee_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK employee.Employee.id" }
      date: { type: date, attributes: ["@required"], description: "The attendance date" }
      schedule: { type: json?, description: "Snapshot: { schedule_type: schedule|calendar|timeoff, shift, ... } resolved at write time" }
      clockin: { type: time?, description: "First clock-in" }
      clockout: { type: time?, description: "Last clock-out" }
      time_debt: { type: json?, description: "Computed shortfall/overtime vs planned shift" }
      timeoff: { type: json?, description: "Snapshot of applicable approved timeoff(s) for the day" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: unique, fields: [company_id, employee_id, date], where: "deleted_at IS NULL", description: "One presence per employee per day" }
      - { type: index, fields: [company_id, date] }
```

## `attendance_clock.model.yaml`

```yaml
models:
  - name: AttendanceClock
    collection: attendance_clocks
    description: "Raw clock events (in/out punches) — multiple per attendance."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"] }
      attendance_id: { type: uuid, attributes: ["@required", "@foreign_key(Attendance.id)"], description: "# FK attendances.id (in-module)" }
      employee_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK employee.Employee.id" }
      date: { type: date, attributes: ["@required"] }
      clock: { type: time, attributes: ["@required"], description: "Punch time" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [attendance_id] }
      - { type: index, fields: [employee_id, date] }
```

## Read port

```rust
#[async_trait]
pub trait AttendancePort: Send + Sync {
    /// Days in [from, to] the employee was absent (no valid clock record, not a scheduled day off).
    /// Company-scoped — preserves the tenant fence the period_summary query relied on.
    async fn absences(&self, company_id: Uuid, employee_id: Uuid, from: NaiveDate, to: NaiveDate) -> Result<Vec<NaiveDate>, AttendanceRejected>;
    /// Raw clock events in range (for regularization / detail views).
    async fn clock_events(&self, company_id: Uuid, employee_id: Uuid, from: NaiveDate, to: NaiveDate) -> Result<Vec<ClockEvent>, AttendanceRejected>;
}
```
