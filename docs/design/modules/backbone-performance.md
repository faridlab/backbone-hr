# `backbone-performance` — spec

> Performance management: goals/OKRs, appraisal cycles, 360 feedback, 9-box. **Greenfield.**

**Reads:** `employee` (employee, manager/reviewer), `organization` (Department, Position, Level).

---

## `index.model.yaml`

```yaml
module: performance
version: 2
schema: performance
description: "performance management — goals, appraisal cycles, 360 feedback, talent matrix"

config: { database: postgresql, soft_delete: true, audit: true, default_timestamps: true, generators: { disabled: [graphql, grpc, proto] } }
external_imports: [{ module: sapiens, types: [User] }]
imports:
  - appraisal_cycle.model.yaml
  - goal.model.yaml
  - appraisal.model.yaml
  - feedback.model.yaml
  - talent_matrix_entry.model.yaml
```

## Entities

| Entity | Collection | Key fields |
|---|---|---|
| `AppraisalCycle` | `appraisal_cycles` | `company_id`, `name` (e.g. "H1 2026"), `period_start`, `period_end`, `status` (draft/open/closed), `type?` (annual/quarterly/mid-year) |
| `Goal` | `goals` | `company_id`, `employee_id`, `cycle_id?`, `title`, `description?`, `weight?` (decimal, 0–100), `progress?` (decimal 0–100), `parent_goal_id?` (cascade alignment — self-ref), `status` (draft/active/achieved/missed) |
| `Appraisal` | `appraisals` | `company_id`, `employee_id`, `cycle_id`, `reviewer_id` (employee FK), `status` (draft/submitted/calibrated/finalized), `overall_rating?`, `submitted_at?` |
| `Feedback` | `performance_feedback` | `company_id`, `cycle_id?`, `from_employee_id`, `to_employee_id`, `content`, `is_anonymous`, `relationship?` (peer/manager/report) |
| `TalentMatrixEntry` | `talent_matrix_entries` | `company_id`, `employee_id`, `cycle_id`, `performance_score` (1–9), `potential_score` (1–9), `box?` (derived: e.g. "star"/"workhorse"/"question") |

## Enums

```yaml
enums:
  - { name: CycleStatus,    variants: [draft, open, calibration, closed] }
  - { name: GoalStatus,     variants: [draft, active, achieved, missed, cancelled] }
  - { name: AppraisalStatus,variants: [draft, self_review, submitted, calibrated, finalized] }
  - { name: FeedbackRelationship, variants: [peer, manager, direct_report, cross_functional] }
```

## `Reward` — recognition / bonus

> Recognition or bonus awarded to an employee. **Non-monetary recognition lives here;** if monetary,
> the *disbursement* stays in `backbone-payroll` (linked via `payroll_component_id`). A performance-
> driven bonus is the typical trigger for a `payroll.CompensationChange`.

```yaml
models:
  - name: Reward
    collection: rewards
    description: "Recognition or bonus awarded to an employee."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK organization.Company.id" }
      employee_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK employee.Employee.id" }
      cycle_id: { type: uuid?, attributes: ["@foreign_key(AppraisalCycle.id)"], description: "# FK appraisal_cycles.id (in-module)" }
      reward_type: { type: RewardType, attributes: ["@required"] }
      title: { type: string, attributes: ["@required", "@max(180)"] }
      description: { type: text? }
      amount: { type: decimal?, attributes: ["@precision(18,2)", "@non_negative"], description: "Monetary value (if any)" }
      awarded_by: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK employee.Employee.id" }
      awarded_at: { type: date, attributes: ["@required"] }
      payroll_component_id: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK payroll.SalaryComponent.id (disbursed via payroll if monetary)" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [company_id, employee_id, awarded_at] }

enums:
  - { name: RewardType, variants: [bonus, recognition, gift, leave_credit, certificate] }
```

## Ports

```rust
#[async_trait]
pub trait PerformancePort: Send + Sync {
    async fn latest_appraisal(&self, company_id: Uuid, employee_id: Uuid) -> Result<Option<AppraisalSummary>, PerformanceRejected>;
    async fn goals(&self, company_id: Uuid, employee_id: Uuid, cycle_id: Option<Uuid>) -> Result<Vec<Goal>, PerformanceRejected>;
}
```

## Notes

- **Goal cascade** via `parent_goal_id` (self-ref) supports OKR alignment (company → team → individual).
- **9-box / talent matrix** is `performance_score × potential_score`; `box` is derived, not stored
  verbatim (like PTKP) — store scores, derive placement.
- `overall_rating` can drive `backbone-payroll` merit increases (a future read port) — keep the
  boundary as an event/port, not a direct write.
