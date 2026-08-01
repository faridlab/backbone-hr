# Rollout & Migration Plan — HRIS Constellation

> Full build sequence for the **12-module HRIS** ([ADR-004](../adr/ADR-004-decompose-into-six-workforce-modules.md)
> decomposition + [ADR-005](../adr/ADR-005-hris-coherence-fixes.md) coherence fixes), with **zero
> disruption to `backbone-payroll`** (the live consumer). Per-module schemas live in [modules/](modules/).
> Covers every module spec in `modules/` — 12 modules + 2 cross-cutting gates (UU PDP fence, event spine).

## Guiding constraints

- **No Cargo edges between modules** — cross-module reads go through ports wired by the HR composer.
- **Payroll stays correct throughout** — `unpaid_days` must compute correctly at every step.
- **Each step is independently shippable + revertable** — `backbone-hr` survives until Step 8.
- **Two hard gates:** (a) `backbone-employee` is **PDP-non-deployable** until the UU PDP fence exists;
  (b) **no cross-module write ships** before the event spine (Step 11).

## Dependency DAG

```
org ─▶ employee(+PDP) ─▶ approvals ─┬─▶ timeoff ──▶ payroll(repoint) ─▶ [hr decommissioned]
                              ├─▶ calendar ──↗     ↘
                              │                   attendance ─↗
schedule ──────────────────────────────────────────↗
employee ─▶ timesheet                                      (independent)
event-spine(composer) ─▶ lifecycle, recruitment, performance, learning   (all cross-module writes)
```

## The sequence (15 steps, dependency-ordered)

### Tier 0 — Foundation *(blocks everything)*
**1. `backbone-organization`** — add Position, Level, Structure (org-design masters). Referenced by
employee/calendar/schedule/recruitment/performance but absent today. Pure prereq, no HR code.

### Tier 1 — Master + privacy + approval spine
**2. `backbone-employee`** ([spec](modules/backbone-employee.md)) — the 13 aggregates, **PTKP derived
from dependents**, and the **UU PDP fence** (DataConsent / DataSubjectRequest / PiiAccessLog).
🚫 *Non-deployable to an ID tenant until the fence is implemented + a DPO signs off on retention.*
Expose `EmployeePort` (`resolve_employee`, `employee_ptkp`). Migrate `hr.employees` → aggregates.
**3. `backbone-approvals`** ([spec](modules/backbone-approvals.md)) — `ApprovalRequest` + steps/delegation/
SLA/policy. *Built before any workflow module so timeoff/timesheet/talent use it from day 1 — no
scattered-enum rework (ADR-005).*

### Tier 2 — The time cluster + the live consumer ⭐ *(validates the core)*
**4. `backbone-calendar`** ([spec](modules/backbone-calendar.md)) — holidays/working-days; needed for
accurate leave day-count + attendance (ADR-001 gate). Ports: `is_holiday`, `working_days`.
**5. `backbone-timeoff`** ([spec](modules/backbone-timeoff.md)) — migrate `hr.leave_*`; carry the
**drawdown invariant** (`used + days ≤ allocated` in one tx); uses `approvals` + `calendar.working_days`.
Port: `approved_leave_days`.
**6. `backbone-attendance`** ([spec](modules/backbone-attendance.md)) — migrate `hr.attendance`; **drop
the invented `AttendanceStatus` enum** (align to `schedule_type` snapshot). Port: `absences`.
**7. Repoint `backbone-payroll`** — `GlPostSink`: `hr.period_summary` → `attendance.absences +
timeoff.approved_leave_days`. **Verify equivalence** (below) before Step 8.
**8. Decommission `backbone-hr`** — all content migrated; remove the crate.

### Tier 3 — Workforce completeness *(lower urgency)*
**9. `backbone-schedule`** ([spec](modules/backbone-schedule.md)) — shifts/roster; feeds attendance's
planned-vs-actual. Port: `planned_shift`.
**10. `backbone-timesheet`** ([spec](modules/backbone-timesheet.md)) — independent; reads employee/project.
Port: `logged_hours`.

### Tier 4 — The event spine *(needed before any cross-module WRITE)*
**11. HR composer + compound-event spine** ([contract](compound-event-contracts.md)) — transactional
outbox + idempotent apply + reconciliation + compensation. *Tiers 0–3 have no cross-module writes
(payroll reads; the drawdown is intra-timeoff), so this cost is deferred until lifecycle needs it.*

### Tier 5 — Talent / ops constellation *(greenfield — defer until a forcing function)*
**12. `backbone-lifecycle`** ([spec](modules/backbone-lifecycle.md)) — onboarding/offboarding/promotion/
🇮🇩 pesangon. Needs `approvals` + the event spine (Promotion writes employee + payroll).
**13. `backbone-recruitment`** ([spec](modules/backbone-recruitment.md)) — hire → creates Employee (via
event spine).
**14. `backbone-performance`** ([spec](modules/backbone-performance.md)) — appraisals/goals/rewards;
reward → payroll (via event spine).
**15. `backbone-learning`** ([spec](modules/backbone-learning.md)) — courses/skills/competencies; cert
handoff → employee (via event spine).

## The YAGNI line — build now vs defer

| | Steps | Justification |
|---|---|---|
| **Build now** | 1–8 (Tiers 0–2) | The one live consumer (`backbone-payroll`) + the PTKP correctness fix + the UU PDP legal fence + the approval spine (no rework). This is the deployable workforce core. |
| **Defer** | 9–15 (Tiers 3–5) | No consumer yet. Keep as specs until a forcing function appears (schedule/timesheet demand; a real hiring pipeline, review cycle, or onboarding need; a second team). Cost of waiting ≈ 0 and reversible. |

## `backbone-hr` dissolution map (Steps 2/5/6/8)

| Current `backbone-hr` | → new module |
|---|---|
| `employee` (+ EmploymentType/EmployeeStatus/TaxStatus) | `backbone-employee` (13 aggregates; TaxStatus → derived PTKP; + PDP fence) |
| `attendance` (+ invented AttendanceStatus) | `backbone-attendance` (status enum dropped) |
| `leave_type` / `leave_application` / `leave_balance` | `backbone-timeoff` (TimeoffType / TimeoffRequest / TimeoffBalance) |
| `OrgPort`, `hr_write_service`, `hr_events`, `hr_ports` | dissolve — read ports move into each module; leave-drawdown → `timeoff_service_custom.rs` |

## Gates & verification

- **Step 2 gate (PDP):** `backbone-employee` carries consent/retention/access-audit before any tenant
  wires it. DPO signs off on retention periods. (One-way *legal* door — not a code door.)
- **Step 7 gate (payroll equivalence):** for a corpus of cases, `unpaid_days_new == period_summary_old`;
  each port takes `company_id`+`employee_id` (tenant fence) with an explicit cross-tenant-leak test.
- **Step 11 gate (writes):** the outbox/idempotency/reconciliation contract is live before Step 12.
  Receiving `reference_id` fields are **non-null** idempotency keys (not nullable).

## Risks & rollback

- **Rollback:** until Step 8, `backbone-hr` is the fallback — a misbehaving port → point the composer
  back at it.
- **Step 1 blocks everything** — Position/Level/Structure in `backbone-organization` first.
- **Residual risk (ADR-004):** a future *transactional* attendance↔timeoff rule can't be enforced across
  schemas → forces a merge or saga. None today.
- **Compound-event divergence (ADR-005):** mitigated by Step 11's outbox+reconciliation; until then, no
  cross-module writes exist, so the risk is latent only.

## Triggering conditions to pull Tier 3–5 forward

- A real **hiring pipeline** → pull `recruitment` (13).
- A real **appraisal/review cycle** or merit cycle → pull `performance` (14) + `lifecycle`-promotion.
- A **second team** owning a sub-domain → split is justified by org structure.
- **Statutory reporting demand** (DPK/BPJS/SPT) → pulls the reporting work (parking lot, not a module yet).
