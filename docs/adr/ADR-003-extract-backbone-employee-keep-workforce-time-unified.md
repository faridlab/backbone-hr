# ADR-003 — Extract `backbone-employee`; keep workforce-time unified in `backbone-hr`

Status: proposed · 2026-08-01 · Tier 5a (People pillar; posts no GL)

> Adjudicated by the council run of 2026-08-01 (focus: bounded-context-cleanliness).
> Full record: [`docs/council/2026-08-01-module-backbone-hr-bounded-context-cleanliness.md`](../council/2026-08-01-module-backbone-hr-bounded-context-cleanliness.md).

## Context

[ADR-002](ADR-002-hris-scope-and-module-fanout.md) set the direction — "the HRIS is a constellation of
modules, not one" — but left *who owns what* implicit: its §1 said `backbone-hr` is the
"people + time + leave" context. Two questions followed:

1. **Is `backbone-hr` a module or an application/service?** (i.e. is "HR" one bounded context, or a
   composition of several?)
2. **Should `backbone-hr` be decomposed to mirror the proven salt-laravel split** — six packages:
   `employee`, `calendar`, `schedule`, `timeoff`, `timesheet`, `attendance`?

A council run adjudicated this with a decisive probe against the salt-laravel source we are porting
from.

## Decision

**Two-way split — not six. Extract the people master; keep the time cluster unified.**

1. **`backbone-hr` remains a *module*** — it is **not** an application/service. It narrows to the
   **workforce-time bounded context**: attendance + leave + calendar + schedule + timesheet. The
   thing called the "HR application" is the *composing backend-service* that wires
   `backbone-employee` + `backbone-hr` (+ future payroll/recruitment) — that composer is the
   application layer, not `backbone-hr`.
2. **Extract `backbone-employee` as a peer module now.** It owns the people master as real aggregates
   (mirroring salt-laravel's 13 tables: employees, identities, employments, banks, bank_accounts,
   taxes, bpjs, families, contacts, educations, certifications, work_experiences, religions).
   `backbone-hr` then reads `employee_id` through a read-port, exactly as it reads
   `organization.Department` via `OrgPort` today (ADR-001 §2).
3. **Reject the full 6-way mirror.** The workforce-time sub-domains (attendance + leave + calendar +
   schedule + timesheet) are **one bounded context**, not five peers. Do not split them.
4. **Defer any fission of the time cluster** until a forcing function arrives (a real
   `backbone-payroll` consumer demanding `period_summary` as cross-service port-behaviors, or a
   second team owning a slice).

## Evidence (the probe)

The council's skeptic named a falsifiable assumption — "the leave↔attendance no-double-counted-
payable-days invariant is a pure READ" — and the probe confirmed it **false**:

- **salt-laravel `Attendances` has no `status` enum.** Its columns are `date`, `schedule(json)`,
  `clockin`, `clockout`, `time_debt(json)`, `timeoff(json)`. It resolves "what governs this workday"
  at **write time** and stamps a `schedule_type ∈ {schedule, calendar, timeoff}` JSON onto the row.
  That single vocabulary is itself the signature of **one** bounded context.
- **The Rust port *invented* `AttendanceStatus::{present, absent, half_day, on_leave, holiday}`** plus
  a cross-table SQL anti-join into `hr.leave_applications`
  (`attendance_repository.rs::count_uncovered_absences`). Neither exists in the proven Laravel design.
- Under separate Postgres schemas that anti-join becomes either a **cross-schema JOIN** (distributed
  monolith — every `leave_applications` change breaks attendance) or an unfenced in-memory set-diff
  (loses the `company_scope` RLS/tenant fence → **cross-tenant leak risk**).

The edges between attendance / leave / calendar / schedule are **state-dependence, not read-layering**.
Acyclicity of the dependency DAG is necessary but not sufficient for safe fission — the
no-double-counted-payable-days invariant has no single owner once split.

## Why `backbone-employee` *does* split cleanly

- It is a pure **read-side master**: nothing in the time cluster mutates employee identity; everything
  reads `employee_id`.
- It has **real aggregate structure** (13 tables in the proven Laravel system), not a flat row.
- **Live correctness driver:** `tax_status` (PTKP) is currently a free enum decoupled from dependents,
  so the model cannot honor "add a dependent → tax tier changes → PPh 21 relief changes." Deriving
  PTKP from `employee_families` requires the aggregate to exist. Payroll — HR's stated reason to exist
  (ADR-001) — needs this.

## Consequences

- `backbone-hr` narrows: it loses the employee master and reads it via a port. Its scope becomes
  attendance + leave + calendar + schedule + timesheet.
- `backbone-employee` becomes the foundation that payroll, recruitment, performance, and the time
  module all read — the same role `organization` plays today.
- The time cluster stays unified and will grow (the ADR-002 Phase-A items — holiday calendar,
  shift/overtime, accrual job, attendance regularization, timesheet — now correctly scoped as
  *one* module's work).
- Future split of the time cluster is **gated**, not abandoned: re-evaluate when a forcing function
  appears. Re-cutting schema boundaries later is local and cheap (no external consumers yet).
- **Parked (correctness, independent of this split):** the `AttendanceStatus` enum divergence from
  Laravel's `schedule_type` snapshot design — align regardless, in a separate change.

## Alternatives considered (from the council, ranked)

| # | Move | Verdict |
|---|---|---|
| 1 | **Two-way split: employee out, time unified** *(this ADR)* | ✅ Best call — fixes the live PTKP bug, honors the probe, clean seam. |
| 2 | Keep one module; enrich `employee` in-place (no crate boundary) | Viable — fixes the aggregate bug cheaply, but loses the clean employee context boundary every future consumer needs. |
| 3 | Status quo | ✗ PTKP bug persists; attendance divergence persists; ADR-002 unreconciled with reality. |
| 4 | Full 6-way mirror of salt-laravel packages | ✗ Distributed monolith on the time cluster; cross-tenant leak risk; no owner for the payable-days invariant. |

## References

- [Council 2026-08-01 — bounded-context-cleanliness](../council/2026-08-01-module-backbone-hr-bounded-context-cleanliness.md) — the full adjudication + probe.
- [ADR-001](ADR-001-hr-boundary-and-leave-engine.md) — HR boundary, leave invariant, `OrgPort` seam.
- [ADR-002](ADR-002-hris-scope-and-module-fanout.md) — HRIS-as-constellation direction (refined here: §1's "people + time + leave" → workforce-time only).
- salt-laravel source: `frameworks/salt-laravel-employee*` (the proven decomposition being ported).
