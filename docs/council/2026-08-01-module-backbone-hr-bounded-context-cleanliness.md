<!--
Date: 2026-08-01 · Repo type: module · Unit: backbone-hr · Focus: bounded-context-cleanliness
Roster (seated): chair, skeptic, steelman, yagni-business, ddd-bounded-context, contract-seat, domain-expert (invited).
Subagent seats: steelman, skeptic, chair. Decisive probe run in-source by the orchestrator (salt-laravel attendance source).
-->

# Council — module:backbone-hr — focus: bounded-context-cleanliness

## Best call

**Two-way split: extract `backbone-employee` as a peer module now; keep `attendance + calendar + schedule + timeoff + timesheet + leave` unified inside `backbone-hr` as one "time/workforce" context. Defer fissioning the time cluster until a real consumer forces port-behavior contracts.**

This is not a midpoint between the steelman and the skeptic. It is two independent decisions, each owned by *different* evidence:
- **Time stays unified** *because the probe proved state-dependence* (the `count_uncovered_absences` anti-semi-join reaches into `hr.leave_applications`; the Laravel `schedule_type ∈ {schedule,calendar,timeoff}` vocabulary is itself the signature of one context). Splitting it has no honest owner.
- **Employee splits out** *because the DDD + domain-expert seats independently proved a clean read-side master with real aggregate structure* (salt-laravel's 13 employee tables; PTKP tax tier depends on `employee_families` — currently flattened into a free `tax_status` enum, a latent payroll-correctness bug). The seam is clean: nothing in the time cluster mutates employee identity; everything reads it.

The 6-way split is dead on the time side — the steelman's load-bearing assumptions C1 (acyclicity survives port translation), C2 (convergence stays read-only), and C5 (1:1 Laravel map) were all falsified by the probe. C1/C2 specifically: a SQL `NOT EXISTS` anti-semi-join is not a read-layer seam, it is state-dependence; under separate Postgres schemas it becomes either a cross-schema JOIN (distributed monolith) or an unfenced in-memory set-diff (loses the `company_scope::fetch_one_scalar_scoped` RLS/tenant fence at attendance_repository.rs:99 → cross-tenant leak risk).

**Residual negative value (concrete):**
- *Employee extraction cost:* ~2–4 dev-weeks — schema re-cut of `employee.model.yaml` into ~13 aggregate tables, migration, read-port widening for existing flattened-field consumers. This is real, but it is the *only* piece of work here that fixes a live correctness problem (PTKP/dependents/BPJS) rather than anticipating one.
- *Time-module bloat:* +5 entities (calendar/schedule/timeoff/timesheet/attendance) remain in one crate. This is honest cost, not artificial — the probe proved they ARE one context. Future split is local (schema re-cut) and cheap.
- *period_summary contract debt:* when `backbone-payroll` arrives and consumes this cross-service, the function at hr_write_service.rs:430 must be reshaped into port *behaviors* (`is_working_day`, `is_on_approved_leave`, `planned_shift`) per the CONTRACT seat. Deferred, not lost.
- *ADR-002 reconciliation:* one-paragraph edit. The 6-way split contradicts ADR-002 §1; this verdict *agrees* with it.

**Reversibility:** Easy on the "wait" half (schema re-cut to split the time cluster later is local, no external consumers yet). Costly-but-unlikely-to-need on the "extract employee" half (re-merging 13 aggregate tables is real work, but the aggregate structure is correct regardless of crate boundary).

**What would flip this:**
- A *second team* arrives needing to own calendar/schedule/timeoff independently — forces the boundary to be drawn by org, not by guessing. None today.
- Discovery of a clean internal seam in the time cluster currently invisible (e.g., distinct ownership of write paths). No evidence in the probe.
- A real `backbone-payroll` consumer lands and demands period_summary as a cross-service contract — this becomes the forcing function for port behaviors and may justify fissioning time then.

## Disagreement map

**1. The seam between attendance and leave/timeoff/calendar — read-layering or state-dependence?**
- *Crux:* can the convergence be a read port, or is it one invariant with no cross-schema owner?
- *Skeptic + probe (decides it):* state-dependence. `count_uncovered_absences` opens `hr.leave_applications` in raw SQL; Laravel carried the same fact as a `schedule_type` JSON snapshot stamped at write time. The Rust `AttendanceStatus::{on_leave,holiday}` enum denormalizes sibling state — proof of one context.
- *Steelman (overruled):* read-layering. Assumed acyclicity survives translation and convergence stays read-only. The probe invalidated exactly these assumptions (C1/C2/C5).

**2. Is the employee boundary clean enough to split today?**
- *Crux:* does employee have a real, non-speculative reason to be its own module, or is it just "the master"?
- *DDD + domain-expert (decides it):* yes — 13 real aggregates in the proven Laravel system, and PTKP correctness literally depends on `employee_families` that backbone-hr currently lacks. Payroll (HR's stated reason to exist per ADR-001) needs the enriched identity. Domain-driven, not speculative.
- *CONTRACT (compatible caution):* the period_summary read is the actual contract surface; when extracted it must be a port *behavior*, not a table reach. This shapes the future port, not the split decision.

**3. Speculative end-state vs YAGNI on the time cluster.**
- *Crux:* is there a forcing function for the 6-way split today?
- *YAGNI / business:* no. One consumer (the future HR service), no payroll/recruitment/performance modules built, no second team. Port behaviors guessed now will be wrong; a real consumer drives them.
- *Steelman:* the dependency DAG is acyclic and a proven port exists. True but insufficient — acyclicity is necessary, not sufficient; the invariant-owner problem (tension 1) sinks it.

## Recommendations (ranked by leverage)

| # | Move | Leverage | Residual negative | Reversibility | Evidence to flip |
|---|------|----------|-------------------|---------------|------------------|
| 1 | **Two-way split: extract `backbone-employee` now; unify time cluster in `backbone-hr`; defer time fission** | High — fixes the live PTKP/dependents correctness gap; honors the probe; aligns with ADR-002 §1 | ~2–4 wks extraction work; time module holds 5 extra entities (honest); period_summary port-behavior debt deferred | Easy (wait half) / costly-unlikely (extract half) | Second team, or a real payroll consumer forcing the time seam |
| 2 | Keep one module; only enrich `employee` into real aggregates (in-place, no crate boundary) | Medium — fixes the aggregate correctness bug without extraction cost | No context boundary; employee reads keep paying cross-concern coupling tax as HR grows; org boundary stays opaque | Easy | Second team needing independent versioning of employee |
| 3 | Status quo (do nothing) | Zero — accumulates debt on all axes | PTKP correctness bug persists; AttendanceStatus enum divergence persists; ADR-002 stays unreconciled with reality | One-way (debt compounds) | None — never the right call |
| 4 | Full 6-way split (mirror Laravel packages) | Negative — distributed monolith on the time cluster | Cross-schema JOIN or unfenced in-memory set-diff (RLS/tenant-fence leak risk); no owner for the no-double-counted-payable-days invariant | Costly — re-merge across Postgres schemas once deployed | Probe would need to be wrong about the anti-semi-join; it isn't (line 105-108 verified) |

## Parking lot

- **`AttendanceStatus` enum divergence from Laravel JSON-snapshot design.** The Rust port invented `present/absent/half_day/on_leave/holiday` plus a cross-table join; Laravel stamped `schedule_type` at write time. This is a correctness/maintainability fix independent of the decomposition call — align regardless. (Out of focus: implementation correctness, not bounded-context-cleanliness.)
- **Generic CRUD PATCH on leave status/days** (ADR-001 parking lot, ADR-002 reaffirms "non-blocking"). Compounds double-draw/staleness risk on the leave write path regardless of how contexts are cut. File separately.
- **period_summary port-behavior extraction** when `backbone-payroll` lands — `is_working_day` / `is_on_approved_leave` / `planned_shift` behaviors per the CONTRACT seat. Belongs to the payroll-consumer decision, not this one.
- **PTKP tax-tier-vs-dependents invariant** — surfaces only after `employee_families` exists; will need its own rule once backbone-employee is extracted. Track under the employee-extraction work, not here.
