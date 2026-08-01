# `backbone-timeoff` — spec

> Leave / absence entitlement. Source: `salt-laravel-employee-timeoff` (3 tables). Decision: [ADR-004](../../adr/ADR-004-decompose-into-six-workforce-modules.md).
>
> Carries the **load-bearing balance-drawdown invariant** ([ADR-001 §3](../../adr/ADR-001-hr-boundary-and-leave-engine.md)):
> `approve` claims `pending → approved` and draws the balance down in one transaction, gated so
> `used + days ≤ allocated`. ⚠️ salt-laravel's `timeoff_employees` was too sparse — enriched here with
> `allocated`/`used` + DB CHECKs (the invariant backstop).

**Reads:** `employee` (Employee), `calendar` (working-day count via port) — via logical FKs/ports.
**Exposes ports:** `approved_leave_days(emp, from, to)`, `balance(...)`.

---

## `index.model.yaml`

```yaml
module: timeoff
version: 2
schema: timeoff
description: "leave / absence: types, requests, balances (+ drawdown invariant)"

config: { database: postgresql, soft_delete: true, audit: true, default_timestamps: true, generators: { disabled: [graphql, grpc, proto] } }
external_imports: [{ module: sapiens, types: [User] }]
imports: [timeoff_type.model.yaml, timeoff_request.model.yaml, timeoff_balance.model.yaml]
```

## `timeoff_type.model.yaml`

```yaml
models:
  - name: TimeoffType
    collection: timeoff_types
    description: "Leave policy/type (annual, sick, maternity, ...)."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK organization.Company.id" }
      name: { type: string, attributes: ["@required", "@max(120)"] }
      code: { type: string?, attributes: ["@max(40)"], description: "Stable code (annual/sick/...)" }
      is_paid: { type: boolean, attributes: ["@default(true)"], description: "Paid leave? (feeds payroll payable-days)" }
      allow_carry_forward: { type: boolean, attributes: ["@default(false)"], description: "Carry unused balance to next period" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: unique, fields: [company_id, code], where: "deleted_at IS NULL" }
```

## `timeoff_request.model.yaml`

```yaml
models:
  - name: TimeoffRequest
    collection: timeoff_requests
    description: "A leave request. pending→approved triggers the balance drawdown (custom service)."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"] }
      timeoff_type_id: { type: uuid, attributes: ["@required", "@foreign_key(TimeoffType.id)"] }
      employee_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK employee.Employee.id" }
      date_start: { type: date, attributes: ["@required"] }
      date_end: { type: date, attributes: ["@required"] }
      note: { type: text? }
      approval_employee_id: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK employee.Employee.id (approver)" }
      note_reject: { type: text? }
      status: { type: TimeoffRequestStatus, attributes: ["@required", "@default(pending)"], description: "⚠️ added (salt-laravel had none)" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [company_id, employee_id, date_start] }
      - { type: index, fields: [status] }

enums:
  - name: TimeoffRequestStatus
    variants:
      - { name: pending, default: true }
      - approved
      - rejected
      - cancelled
```

## `timeoff_balance.model.yaml` — the invariant

```yaml
models:
  - name: TimeoffBalance
    collection: timeoff_balances
    description: "Leave entitlement per employee per period. Load-bearing: used + days ≤ allocated on approve."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"] }
      timeoff_type_id: { type: uuid, attributes: ["@required", "@foreign_key(TimeoffType.id)"] }
      employee_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK employee.Employee.id" }
      period: { type: string, attributes: ["@required", "@max(10)"], description: "Year or period key (e.g. 2026)" }
      allocated: { type: decimal, attributes: ["@required", "@precision(18,2)", "@non_negative"], description: "Days allocated" }
      used: { type: decimal, attributes: ["@required", "@default(0)", "@precision(18,2)", "@non_negative"], description: "Days consumed" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: unique, fields: [company_id, employee_id, timeoff_type_id, period], where: "deleted_at IS NULL" }
    constraints:
      # DB CHECK backstops the drawdown invariant against ANY writer (ADR-001, proven-by-revert).
      - "used >= 0"
      - "used <= allocated"
```

## Custom service logic (NOT regenerated — `timeoff_service_custom.rs`)

```text
approve_request(request_id):
  tx:
    req = lock timeoff_requests(id) for update
    assert req.status == pending
    days = calendar.working_days(company, employee, req.date_start, req.date_end)   # read port
    bal  = lock timeoff_balances(employee, type, period) for update
    assert bal.used + days <= bal.allocated            # gated drawdown
    bal.used += days
    req.status = approved
cancel_request(request_id):                            # restores balance
  tx:
    assert req.status == approved
    bal = lock ...
    assert bal.used >= days                             # gated restore
    bal.used -= days
    req.status = cancelled
```

## Read ports

```rust
#[async_trait]
pub trait TimeoffPort: Send + Sync {
    /// Dates in [from, to] covered by an APPROVED request (used by payroll's unpaid_days).
    async fn approved_leave_days(&self, company_id: Uuid, employee_id: Uuid, from: NaiveDate, to: NaiveDate) -> Result<Vec<NaiveDate>, TimeoffRejected>;
    async fn balance(&self, company_id: Uuid, employee_id: Uuid, timeoff_type_id: Uuid, period: String) -> Result<BalanceSnapshot, TimeoffRejected>;
}
```

## Notes

- **Day-count uses `calendar.working_days`** (read port) — not a raw calendar span. This is the read-path
  dependency on `backbone-calendar` (acyclic: timeoff → calendar, calendar reads nothing in timeoff).
- The `used <= allocated` CHECK + gated drawdown is the exact pattern proven in backbone-hr (ADR-001),
  relocated into this module's custom service.
