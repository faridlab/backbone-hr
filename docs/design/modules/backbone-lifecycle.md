# `backbone-lifecycle` — spec

> Onboarding & offboarding workflows, clearance, exit interviews, 🇮🇩 final settlement (pesangon).
> **Greenfield.** Owns the joiner/leaver journeys around `backbone-employee`.

**Reads:** `employee` (Employment lifecycle, join/exit dates), `payroll` (salary for final settlement),
`timeoff` (unused-leave payout), `attendance`. **Drives:** `employee.employment.status`
(active↔inactive) via event/port, never a direct cross-schema write.

---

## `index.model.yaml`

```yaml
module: lifecycle
version: 2
schema: lifecycle
description: "employee lifecycle — onboarding, offboarding, clearance, exit, final settlement"

config: { database: postgresql, soft_delete: true, audit: true, default_timestamps: true, generators: { disabled: [graphql, grpc, proto] } }
external_imports: [{ module: sapiens, types: [User] }]
imports:
  - onboarding.model.yaml
  - onboarding_task.model.yaml
  - offboarding.model.yaml
  - clearance_item.model.yaml
  - exit_interview.model.yaml
  - final_settlement.model.yaml
```

## Entities

| Entity | Collection | Key fields |
|---|---|---|
| `Onboarding` | `onboardings` | `company_id`, `employee_id`, `start_date`, `status` (pending/in_progress/completed), `completed_at?`, `template_id?` |
| `OnboardingTask` | `onboarding_tasks` | `company_id`, `onboarding_id`, `title`, `category?` (document/equipment/account/policy), `owner_employee_id?`, `due_date?`, `status` (pending/done/skipped) |
| `Offboarding` | `offboardings` | `company_id`, `employee_id`, `reason` (resignation/termination/end_of_contract/retirement), `notice_date`, `last_working_day`, `status` (initiated/in_progress/cleared/closed) |
| `ClearanceItem` | `clearance_items` | `company_id`, `offboarding_id`, `title`, `clearer_employee_id?`, `status` (pending/cleared/blocked) |
| `ExitInterview` | `exit_interviews` | `company_id`, `employee_id`, `offboarding_id?`, `conducted_by?` (employee FK), `responses` (json), `would_recommend?` |
| `FinalSettlement` | `final_settlements` | `company_id`, `employee_id`, `offboarding_id`, `period`, `base_pay`, `unused_leave_payout?`, `pesangon_amount?` (🇮🇩), `tax_deduction?`, `net_payable`, `status` (draft/confirmed/paid) |

## `Promotion` — promotion / transfer / demotion workflow

> An employment-**change** workflow (a "move"), the same shape as onboarding/offboarding. Performance
> supplies the merit justification (`appraisal_id`); on `effective` it **emits** two changes: a role
> change → `employee.EmploymentHistory` and a salary change → `payroll.CompensationChange`. Lifecycle
> never writes those schemas directly (no Cargo edge) — the composer applies the events.

```yaml
models:
  - name: Promotion
    collection: promotions
    description: "Promotion / transfer / demotion workflow. On effective, emits role + salary changes."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK organization.Company.id" }
      employee_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK employee.Employee.id" }
      type: { type: PromotionType, attributes: ["@required"], description: "promotion/transfer/demotion/lateral" }
      position_id_from: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK organization.Position.id" }
      position_id_to: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK organization.Position.id" }
      level_id_from: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK organization.Level.id" }
      level_id_to: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK organization.Level.id" }
      department_id_from: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"] }
      department_id_to: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"] }
      proposed_salary: { type: decimal?, attributes: ["@precision(18,2)", "@non_negative"], description: "New salary → payroll.CompensationChange on effective" }
      effective_date: { type: date, attributes: ["@required"] }
      status: { type: PromotionStatus, attributes: ["@required", "@default(draft)"] }
      requested_by: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK employee.Employee.id" }
      approved_by: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK employee.Employee.id" }
      appraisal_id: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK performance.Appraisal.id (merit justification)" }
      reason: { type: text? }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [company_id, employee_id, effective_date] }
      - { type: index, fields: [status] }

enums:
  - { name: PromotionType, variants: [promotion, transfer, demotion, lateral] }
  - { name: PromotionStatus, variants: [{name: draft, default: true}, pending, approved, rejected, effective, cancelled] }
```

## Enums

```yaml
enums:
  - { name: OnboardingStatus, variants: [pending, in_progress, completed, abandoned] }
  - { name: TaskCategory,     variants: [document, equipment, account_access, policy_ack, induction] }
  - { name: TaskStatus,       variants: [pending, done, skipped, blocked] }
  - { name: OffboardingReason,variants: [resignation, termination, end_of_contract, retirement, death] }
  - { name: OffboardingStatus,variants: [initiated, in_progress, cleared, closed] }
  - { name: SettlementStatus, variants: [draft, confirmed, paid, disputed] }
```

## Ports

```rust
#[async_trait]
pub trait LifecyclePort: Send + Sync {
    async fn onboarding_status(&self, company_id: Uuid, employee_id: Uuid) -> Result<OnboardingSummary, LifecycleRejected>;
    async fn offboarding_status(&self, company_id: Uuid, employee_id: Uuid) -> Result<Option<OffboardingSummary>, LifecycleRejected>;
}
```

## Notes

- **🇮🇩 Pesangon** (severance) calculation lives in `final_settlement_service_custom.rs`: reads tenure
  (from `employee.employment.join_date`), `reason` (termination vs resignation changes the formula),
  and last salary (from `payroll`). It is computation over reads, not owned master data.
- **Unused-leave payout** reads `timeoff.balance` (remaining allocation) at the last working day.
- Lifecycle **emits** `Onboarded` / `Offboarded` events; the composer flips `employee.employment.status`
  — lifecycle never writes the employee schema directly (no Cargo edge).
