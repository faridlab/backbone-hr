# `backbone-payroll` — spec

> Salary processing + Indonesia statutory deductions + GL posting. **This module already exists**
> (Cargo-depends on `backbone-hr` today; reads employee unpaid-days via its `GlPostSink` port).
> This doc reflects the **existing** schema and marks **proposed** additions. Decision context: [ADR-004](../../adr/ADR-004-decompose-into-six-workforce-modules.md).

**Consumes (read ports):** `employee` (identity, derived PTKP, NPWP, bank, salary), `attendance`
(`absences`), `timeoff` (`approved_leave_days`), `calendar` (working days). **Posts GL to**
`accounting`. Per ADR-004 it owns `unpaid_days = attendance.absences − timeoff.approved_leave_days`.

---

## Existing entities (✅ in `backbone-payroll/schema/models/` today)

| Entity | Collection | Key fields | Status |
|---|---|---|---|
| `PayrollEntry` | `payroll_entries` | `company_id`, `period_year`, `period_month`, `posting_date?`, `status` (PayrollStatus: draft→processed→posted/cancelled), `salary_expense_account_id?`, `salary_payable_account_id?` | ✅ exists — a payroll run; posts one balanced salary journal to the GL |
| `SalarySlip` | `salary_slips` | `payroll_entry_id`, `company_id`, `employee_id`, `structure_id?`, `working_days` (payable, from HR), `unpaid_days` (from HR — reduce gross), `gross_pay`, … net | ✅ exists — one employee's pay for one run (gross − deductions = net) |
| `SalaryStructure` | `salary_structures` | `company_id`, `name`, `is_active` | ✅ exists — reusable pay template |
| `SalaryComponent` | `salary_components` | `structure_id`, (earning/deduction), GL account | ✅ exists — one line in a structure |

**`SalarySlip.unpaid_days`** is the ADR-004 contract surface: today sourced from `backbone-hr.period_summary`;
post-decomposition it is computed in payroll from `attendance.absences − timeoff.approved_leave_days`.

## Proposed additions (🇮🇩 Indonesia statutory)

These are **proposed** (not yet present) — the computation backbone-employee was built to feed:

| Entity / logic | Purpose | Reads |
|---|---|---|
| `TaxComponent` (PPh 21) | Progressive income-tax line on the slip | `employee.employee_ptkp` (derived tier), `employee.npwp` (missing → surcharge), YTD `pph21_paid` |
| `BpjsComponent` (Kesehatan + Ketenagakerjaan) | Employee + employer contribution lines | `employee_bpjs` (numbers, `bpjs_kesehatan_family`), UMK floors |
| `ThrComponent` (holiday allowance) | Mandatory THR (1× religion-holiday pay, pro-rated by tenure) | `employee.join_date`, `calendar` (holiday timing) |

These can be modeled as typed `SalaryComponent` rows + custom calc in `payroll_service_custom.rs`, or as
first-class entities. **Recommendation:** start as custom calc reading the employee port; promote to
entities only if configuration beyond code is needed.

## `CompensationChange` — salary history *(proposed)*

> Append-only compensation log. **Current salary = latest entry.** Read by pesangon
> (`backbone-lifecycle`) and merit (`backbone-performance`) via a port. The salary-side of a promotion
> lands here (the role-side lands in `employee.EmploymentHistory`).

```yaml
models:
  - name: CompensationChange
    collection: compensation_changes
    description: "Append-only salary/compensation history per employee."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK organization.Company.id" }
      employee_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK employee.Employee.id" }
      effective_date: { type: date, attributes: ["@required"] }
      change_type: { type: CompensationChangeType, attributes: ["@required"], description: "initial/raise/promotion/adjustment/reduction" }
      previous_amount: { type: decimal?, attributes: ["@precision(18,2)", "@non_negative"] }
      new_amount: { type: decimal, attributes: ["@required", "@precision(18,2)", "@non_negative"] }
      frequency: { type: PayFrequency, attributes: ["@required", "@default(monthly)"] }
      currency: { type: string?, attributes: ["@max(3)"], description: "ISO 4217 (IDR)" }
      reason: { type: string?, attributes: ["@max(255)"] }
      reference_id: { type: uuid?, description: "# the triggering workflow (e.g. lifecycle.Promotion id)" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [employee_id, effective_date] }

enums:
  - { name: CompensationChangeType, variants: [initial, raise, promotion, adjustment, reduction, correction] }
  - { name: PayFrequency, variants: [{name: monthly, default: true}, annual, hourly, daily] }
```

Payroll exposes a read port for the current/historical salary:

```rust
async fn current_salary(company_id, employee_id) -> Result<CompensationSnapshot, PayrollRejected>; // latest entry
async fn salary_history(company_id, employee_id) -> Result<Vec<CompensationChange>, PayrollRejected>;
```

## Enums

```yaml
enums:
  - name: PayrollStatus                # ✅ exists
    variants: [{name: draft, default: true}, processed, posted, cancelled]
  - name: SalaryComponentType          # proposed (if components are typed)
    variants: [earning, deduction, tax, contribution]
```

## Read contract consumed (ADR-004)

```rust
// In payroll's GlPostSink adapter (post-decomposition):
async fn unpaid_days(company_id, employee_id, from, to) -> decimal {
    let absences       = attendance.absences(company_id, employee_id, from, to).await?;   // Vec<NaiveDate>
    let approved_leave = timeoff.approved_leave_days(company_id, employee_id, from, to).await?;
    (absences.len() - approved_leave.len()) as decimal      // company-scoped throughout
}
```

## Notes

- **Existing Cargo edge to `backbone-hr`** (`Cargo.toml`) is dissolved in Phase 5 of the
  [rollout plan](../rollout-plan.md) — the `GlPostSink` adapter is the single seam to repoint.
- Payroll is a **consumer**, not a port-provider. It posts money to `backbone-accounting` (GL).
- Indonesian floors: UMK/UMR (regional minimum wage) and BPJS caps should live as config, not schema.
