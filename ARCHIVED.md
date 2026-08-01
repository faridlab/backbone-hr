# ⚠️ ARCHIVED — `backbone-hr` is superseded (kept for reference + design docs)

**Status:** Dormant. Unregistered from `metaphor.yaml` on 2026-08-01 — not built by `metaphor build --all`,
not a dependency of any module (its sole consumer, `backbone-payroll`, was repointed to the new modules).
The dir + code are kept as a reference; the design docs below are the live record.

**Superseded by** the 6-module decomposition ([ADR-004](docs/adr/ADR-004-decompose-into-six-workforce-modules.md)):
- `employee` → `backbone-employee` (13 aggregates + UU PDP fence + PTKP)
- `attendance` → `backbone-attendance` (status enum dropped; `present_days` port)
- `leave_*` → `backbone-timeoff` (drawdown invariant, ported from `hr_write_service`)
- + `backbone-calendar`, `backbone-schedule`, `backbone-timesheet`, `backbone-approvals`

**The migration is proven:** `backbone-payroll`'s repointed seam test (`unpaid_days = working_days −
present_days − paid_leave`) is green — see [ADR-004](docs/adr/ADR-004-decompose-into-six-workforce-modules.md)
+ the [coherence council](docs/council/2026-08-01-module-hris-constellation-coherence.md).

**This dir's value now = `docs/`** — ADRs 001–005, two council reports, the 12 module specs, the rollout
plan, and the compound-event contracts. That's the design history for the whole HRIS; do not delete.
