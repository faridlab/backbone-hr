<!--
Date: 2026-08-01 · Repo type: module · Unit: HRIS-constellation (11-module design) · Focus: coherence
Roster (seated): chair, skeptic, steelman, yagni-business, ddd-bounded-context, contract-seat,
  + invited HR business experts: indonesia-compliance, talent-performance, hr-operations.
Subagent seats: steelman, skeptic, chair.
-->

# Council — module:HRIS-constellation — focus: coherence

## Best call

**Accept the skeptic's verdict: the 11 modules are a coherent DOMAIN LIBRARY, not a complete or deployable HRIS. The single corrective move is to extract the missing Approvals bounded context (a 12th spec) NOW, while all five workflow-bearing modules (lifecycle, recruitment, onboarding, offboarding, clearance) are still zero-code.**

The three decisive probes settle this:
- (a) grep consent/retention/PDP across all specs = 0 hits — backbone-employee stores the full UU PDP Article-16 PII set (NIK, NPWP, religion, family names+DOB, bank, mobile, email) with no consent, retention, access-audit, or data-subject-rights fields. Non-deployable the day it ships.
- (b) There is no ApprovalRequest aggregate — approval state lives as a `status` enum + `approved_by` field embedded inside Promotion, Onboarding, OnboardingTask, Offboarding, and ClearanceItem. That is an aggregate scattered across five entities with no home. The composer wires; it does not invent aggregates that already own data.
- (c) The post-failure state of `Promotion.effective` on partial write (role succeeds, salary fails) is unspecified — two append-only tables in two schemas, linked only by a nullable `reference_id`, no outbox, no transaction, no idempotency, no compensation.

The steelman's D7 assumption ("completeness scoped to module layer; approvals/ESS/MSS are Phase C application composition") is the load-bearing error, and it is wrong on axis (b) specifically: approvals are not unbuilt application features — they are data-owning state machines already living inside five module specs as enums. That is a missing bounded context at the module layer, which is exactly the coherence question.

**Residual negative value (concrete, what this move does NOT close):**
- PDP legal exposure stays unfenced: if any consumer wires backbone-employee and ships, exposure is up to 2% annual revenue + criminal liability under UU PDP Art 67. This is a one-way legal door, not a feature gap.
- Compound-event divergence stays open: every cross-module write (promotion, onboarding, offboarding, clearance, leave) is a silent-divergence bet — the career thread closes logically but not transactionally. Cost falls on the (unbuilt) composer to retrofit outbox/saga under pressure.
- Statutory exports absent (DPK annual, BPJS monthly, PPh21 SPT/ebupot) — modules compute the figures but cannot discharge the reporting obligation.

**Reversibility:** Easy. All five affected modules are zero-code; extracting a spec is reversible by deleting it. No migration, no cargo edge, no deploy.

**What would flip this:** A probe showing the five workflow statuses never require delegation, SLA, or escalation semantics (then a full ApprovalRequest aggregate is over-engineering and the enum suffices). Cheap probe: HR-Ops already testified "manager approves in five places" — which confirms a shared engine with delegation is wanted, so the context is real. Nothing flips it as stated.

## Disagreement map

**1. The crux — is the missing layer "application composition" or "data-owning bounded contexts"?**
Steelman holds D7: ESS/MSS/approvals/notifications/dashboards are Phase C, built by the composer on top of complete modules. Skeptic + DDD seat hold the opposite: approvals already own data inside the module specs today (status enums, approved_by), so they are a missing bounded context, not unbuilt application code. The composer wires contexts; it cannot retroactively own aggregates that are already persisting state inside other modules. This is the disagreement that decides whether "11 modules" is a complete answer. **Side owned: skeptic + DDD.** A context whose state machine hides in another entity's enum is the textbook definition of an unidentified bounded context.

**2. Is UU PDP a deployment/Phase-C concern or a design-level legal precondition?**
Steelman scopes it to Phase C. Indonesia-compliance expert + skeptic hold that it is a legal fence: the regulated PII is already in backbone-employee's schema with zero consent scaffolding, making the module non-deployable as-designed — not incomplete-in-a-later-phase. **Side owned: skeptic + Indonesia compliance.** A law in force since 2022 with criminal liability is not a Phase C feature; it is a precondition on the data model itself.

**3. Build all 11 vs. build core 4, spec the rest.**
YAGNI seat: 8 of 11 modules have zero consumers (only backbone-payroll exists); designing all 11 is fine but building all 11 is over-reach. Real this-month leverage is org-readiness + backbone-employee (PTKP) + time + payroll repoint. Steelman does not argue for building all 11, but treats all 11 as first-class equals in the constellation. **Side owned: YAGNI**, but this is a build-sequencing tension, not a coherence verdict — it belongs in the recommendations, not in the verdict. Designing all 11 is cheap and demonstrates the framework; the over-reach is only in building.

**4. Does the career thread actually close?**
Steelman: Appraisal → Promotion → EmploymentHistory + CompensationChange closes the loop. Contract seat + skeptic: the cross-schema write has no transaction, no outbox, no reconciliation schema — it closes on paper, not in state. **Side owned: contract seat + skeptic.** A nullable `reference_id` linking two append-only tables across schemas is not a closed loop; it is an unenforceable hope.

## Recommendations (ranked by leverage)

| # | Move | Leverage | Residual negative | Reversibility | Evidence to flip |
|---|------|----------|-------------------|---------------|------------------|
| 1 | **Extract Approvals bounded context (12th spec): ApprovalRequest aggregate owning state machine, delegation, SLA, approved_by; the five workflow modules reference it instead of embedding status enums.** | Highest — prevents the scattered-enum anti-pattern from being baked into five zero-code modules; the only structural coherence fix that is free right now. | Does not close PDP exposure or compound-event divergence (see #2, #3). ~1 spec-day. | Easy — zero code, no migration, no cargo edge. | Probe shows no delegation/SLA/escalation ever needed (already contradicted by HR-Ops). |
| 2 | **Fence backbone-employee as PDP-non-deployable; add consent, retention-period, access-audit, and DSAR fields before any consumer wires it.** | Closes the one-way legal door; without it the whole constellation carries Art 67 liability. | Requires DPO/legal sign-off on retention periods; ~2–4 spec-days + legal review. | Easy at schema stage; one-way legal door after data is populated. | A DPO certifies the current fields don't trigger Art 16 (will not happen — NIK/NPWP/religion are textbook regulated PII). |
| 3 | **Specify compound-event contracts for all cross-schema writes (Promotion, Onboarding, Offboarding, Clearance, Leave): outbox emission, idempotency key, reconciliation query, compensation semantics.** | Makes the career thread actually close in state, not just on paper. | Implementation cost falls on the unbuilt composer (~1–2 weeks of saga/outbox work when it lands). | Moderate — cheap to specify, costly to retrofit after raw dual-writes ship. | A probe proves a single-writer guarantee per event makes the tx unnecessary (none exists today). |
| 4 | **YAGNI trim: build core 4 (org-readiness + employee PTKP fix + time + payroll repoint); keep recruitment/performance/learning/lifecycle/calendar/schedule/timesheet as SPECS until a forcing function.** | Focuses real leverage on the only existing consumer (payroll); cost of waiting near-zero and reversible. | 7 specs sit unbuilt; spec-to-build lag ~weeks if a forcing function lands. | Easy — specs written; building is a decision, not a commitment. | A second backend-service consumer appears, or a real hiring/review cycle starts. |
| 5 | **Move salary-banding into backbone-organization (referenced by payroll), per DDD seat.** | Cleaner boundary — bands are org-structural; stops `proposed_salary` being a free-floating number. | Minor refactor of compensation/promotion specs. | Easy at spec stage. | Bands computed purely inside payroll with no org reference (unlikely). |

## Parking lot

Outside the coherence lens — noted, not adjudicated here:
- ESS/MSS UI, manager self-service dashboards, approval inbox, notifications wiring (genuinely Phase C application composition — the composer's job, once the contexts above exist).
- Statutory exports: DPK annual report, BPJS monthly, PPh21 SPT/ebupot (Indonesia compliance — completeness-of-reporting, not coherence-of-contexts).
- PKWT (UU 6/2023) 5-year contract cap/expiry enforcement; THR ≤7-day timing + pro-rated calculation rule (statutory-rule gaps inside existing modules).
- Succession planning, 9-box→development-plan→learning-enrollment gap closure, ExitInterview structured analytics (Talent — feature depth, not context boundaries).
- Document management: signed contracts, KTP scans, e-signature (HR-Ops — a genuine candidate context, scoped out of this pass; revisit if it owns persisted state beyond blobs).
- Composer design (saga/outbox/transactional-outbox topology) — deferred until bounded contexts are settled; it is the integration layer, not a context.
