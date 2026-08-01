# `backbone-employee` — spec

> The people master. Read by every other workforce module + `backbone-payroll`. Source:
> `salt-laravel-employee` (13 tables). 13 aggregates, 2 reference masters.
> Decision: [ADR-004](../../adr/ADR-004-decompose-into-six-workforce-modules.md).

**Reads:** `organization` (Company, Department, Position, Level — logical FKs), `sapiens` (User — actors).
**Exposes ports:** `resolve_employee(id)`, `employee_ptkp(id)`.
**Owns:** `employee_id` — the logical FK every module references.

---

## `index.model.yaml`

```yaml
module: employee
version: 2
schema: employee
description: "the people master — identity, employment lifecycle, payroll identity, dependents"

config:
  database: postgresql
  soft_delete: true
  audit: true
  default_timestamps: true
  generators:
    disabled: [graphql, grpc, proto]

external_imports:
  - module: sapiens
    types: [User]

shared_types:
  Timestamps:
    created_at: { type: datetime, attributes: ["@default(now)"], description: "Record creation timestamp" }
    updated_at: { type: datetime, attributes: ["@updated_at"], description: "Last update timestamp" }
    deleted_at: { type: datetime?, description: "Soft delete timestamp" }
  Actors:
    created_by: { type: uuid?, attributes: ["@foreign_key(sapiens.User.id)"], description: "User who created" }
    updated_by: { type: uuid?, attributes: ["@foreign_key(sapiens.User.id)"], description: "User who last updated" }
    deleted_by: { type: uuid?, attributes: ["@foreign_key(sapiens.User.id)"], description: "User who deleted" }
  Metadata: [Timestamps, Actors]

imports:
  - employee.model.yaml
  - employee_identity.model.yaml
  - employment.model.yaml
  - employee_family.model.yaml
  - employee_contact.model.yaml
  - employee_education.model.yaml
  - employee_certification.model.yaml
  - employee_work_experience.model.yaml
  - employee_bank_account.model.yaml
  - employee_tax.model.yaml
  - employee_bpjs.model.yaml
  - employment_history.model.yaml
  - data_consent.model.yaml
  - data_subject_request.model.yaml
  - pii_access_log.model.yaml
  - religion.model.yaml
  - bank.model.yaml
```

---

## `employee.model.yaml` — the master

```yaml
models:
  - name: Employee
    collection: employees
    description: "The canonical people master. Owns employee_id (logical FK every module references)."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"], description: "Unique employee id" }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "Org owner # logical FK organization.Company.id" }
      employee_number: { type: string, attributes: ["@required", "@max(40)"], description: "Human code (unique per company)" }
      user_id: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "Login identity # logical FK sapiens.User.id" }
      first_name: { type: string, attributes: ["@required", "@max(120)"], description: "Given name" }
      last_name: { type: string?, attributes: ["@max(120)"], description: "Family name" }
      email: { type: string?, attributes: ["@max(255)"], description: "Email" }
      mobile_phone: { type: string?, attributes: ["@max(40)"], description: "Mobile" }
      phone: { type: string?, attributes: ["@max(40)"], description: "Landline" }
      birth_place: { type: string?, attributes: ["@max(120)"], description: "Birthplace" }
      birth_date: { type: date?, description: "Date of birth" }
      gender: { type: Gender?, description: "Gender" }
      marital_status: { type: MaritalStatus?, description: "Marital status" }
      blood_type: { type: BloodType?, description: "Blood type" }
      religion_id: { type: uuid?, attributes: ["@foreign_key(Religion.id)"], description: "Religion # FK religion.religions.id (in-module)" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"], description: "Audit metadata" }
    indexes:
      - { type: index, fields: [company_id] }
      - { type: index, fields: [user_id] }
      # Race-proof per-company employee_number uniqueness (ADR-001).
      - { type: unique, fields: [company_id, employee_number], where: "deleted_at IS NULL" }
```

## `employee_identity.model.yaml` — KTP / passport

```yaml
models:
  - name: EmployeeIdentity
    collection: employee_identities
    description: "Government identity documents (KTP / passport). Multiple per employee."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK organization.Company.id" }
      employee_id: { type: uuid, attributes: ["@required", "@foreign_key(Employee.id)"], description: "# FK employees.id" }
      identity_type: { type: IdentityType, attributes: ["@required"], description: "id (KTP) / passport" }
      identity_number: { type: string, attributes: ["@required", "@max(64)"], description: "Document number" }
      identity_expiry_date: { type: date?, description: "Expiry (null if permanent)" }
      is_permanent: { type: boolean, attributes: ["@default(false)"], description: "No expiry" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [employee_id] }
```

## `employment.model.yaml` — lifecycle + org placement

```yaml
models:
  - name: Employment
    collection: employments
    description: "Employment lifecycle + organization placement (dept/level/position/manager)."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK organization.Company.id" }
      employee_id: { type: uuid, attributes: ["@required", "@foreign_key(Employee.id)"], description: "# FK employees.id" }
      employment_status: { type: EmploymentStatus, attributes: ["@required", "@default(permanent)"], description: "permanent/contract/probation/associate" }
      join_date: { type: date, attributes: ["@required"], description: "Start date" }
      end_join_date: { type: date?, description: "Contract/probation end" }
      department_id: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK organization.Department.id" }
      level_id: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK organization.Level.id (Position/Level to be added to org — see note)" }
      position_id: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK organization.Position.id" }
      direct_manager_id: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK employee.Employee.id (self-ref)" }
      status: { type: EmploymentState, attributes: ["@required", "@default(active)"], description: "active / inactive" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [company_id, status] }
      - { type: index, fields: [employee_id] }
      - { type: index, fields: [department_id] }
```

## `employment_history.model.yaml` — role/level history (append-only)

> Add `employment_history.model.yaml` to the module's `index.model.yaml` imports.
> `Employment` holds the *current* placement; this is the append-only log of changes (covers role
> history + the role-side of promotions/transfers).

```yaml
models:
  - name: EmploymentHistory
    collection: employment_histories
    description: "Append-only log of role/level/department changes over a career."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK organization.Company.id" }
      employee_id: { type: uuid, attributes: ["@required", "@foreign_key(Employee.id)"], description: "# FK employees.id" }
      effective_date: { type: date, attributes: ["@required"] }
      action: { type: EmploymentAction, attributes: ["@required"], description: "hire/transfer/promotion/demotion/role_change" }
      position_id_from: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK organization.Position.id" }
      position_id_to: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK organization.Position.id" }
      level_id_from: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK organization.Level.id" }
      level_id_to: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK organization.Level.id" }
      department_id_from: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK organization.Department.id" }
      department_id_to: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK organization.Department.id" }
      reference_id: { type: uuid?, description: "# the triggering workflow (e.g. lifecycle.Promotion id)" }
      note: { type: text? }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [employee_id, effective_date] }

enums:
  - { name: EmploymentAction, variants: [hire, transfer, promotion, demotion, role_change, reinstatement] }
```

## `employee_family.model.yaml` — dependents (drives PTKP)

```yaml
models:
  - name: EmployeeFamily
    collection: employee_families
    description: "Family members / dependents. relationship drives PTKP tax-tier derivation."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"] }
      employee_id: { type: uuid, attributes: ["@required", "@foreign_key(Employee.id)"] }
      name: { type: string, attributes: ["@required", "@max(120)"] }
      relationship: { type: FamilyRelationship, attributes: ["@required"], description: "spouse/child/parent/sibling/other — spouse+child count drive PTKP" }
      birth_date: { type: date?, description: "DOB" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [employee_id] }
```

## `employee_contact.model.yaml`

```yaml
models:
  - name: EmployeeContact
    collection: employee_contacts
    description: "Emergency / alternate contacts."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"] }
      employee_id: { type: uuid, attributes: ["@required", "@foreign_key(Employee.id)"] }
      name: { type: string, attributes: ["@required", "@max(120)"] }
      phone: { type: string?, attributes: ["@max(40)"] }
      email: { type: string?, attributes: ["@max(255)"] }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [employee_id] }
```

## `employee_education.model.yaml`

```yaml
models:
  - name: EmployeeEducation
    collection: employee_educations
    description: "Education history."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"] }
      employee_id: { type: uuid, attributes: ["@required", "@foreign_key(Employee.id)"] }
      institution_name: { type: string, attributes: ["@required", "@max(180)"], description: "(typo instituion fixed)" }
      major: { type: string?, attributes: ["@max(120)"] }
      field: { type: string?, attributes: ["@max(120)"] }
      score: { type: decimal?, attributes: ["@precision(5,2)"], description: "GPA" }
      start_year: { type: integer?, description: "Year" }
      end_year: { type: integer?, description: "Year" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [employee_id] }
```

## `employee_certification.model.yaml`

```yaml
models:
  - name: EmployeeCertification
    collection: employee_certifications
    description: "Certifications (track + expiry feeds compliance)."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"] }
      employee_id: { type: uuid, attributes: ["@required", "@foreign_key(Employee.id)"] }
      name: { type: string, attributes: ["@required", "@max(180)"] }
      issuing_organization: { type: string?, attributes: ["@max(180)"] }
      start_date: { type: date? }
      end_date: { type: date?, description: "Expiry" }
      description: { type: text? }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [employee_id] }
```

## `employee_work_experience.model.yaml`

```yaml
models:
  - name: EmployeeWorkExperience
    collection: employee_work_experiences
    description: "Pre-joining work history."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"] }
      employee_id: { type: uuid, attributes: ["@required", "@foreign_key(Employee.id)"] }
      company_name: { type: string, attributes: ["@required", "@max(180)"] }
      job_position: { type: string?, attributes: ["@max(120)"] }
      start_date: { type: date? }
      end_date: { type: date? }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [employee_id] }
```

## `employee_bank_account.model.yaml` — payroll disbursement

```yaml
models:
  - name: EmployeeBankAccount
    collection: employee_bank_accounts
    description: "Disbursement accounts for net pay (read by payroll)."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"] }
      employee_id: { type: uuid, attributes: ["@required", "@foreign_key(Employee.id)"] }
      bank_id: { type: uuid, attributes: ["@required", "@foreign_key(Bank.id)"], description: "# FK banks.id (in-module)" }
      account_number: { type: string, attributes: ["@required", "@max(40)"] }
      account_name: { type: string?, attributes: ["@max(120)"] }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [employee_id] }
```

## `employee_tax.model.yaml` — Indonesia payroll identity (PTKP fix)

```yaml
models:
  - name: EmployeeTax
    collection: employee_taxes
    description: "Indonesia tax identity. PTKP tier is DERIVED from employee_families (see PTKP rule); ptkp_override allows manual override."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"] }
      employee_id: { type: uuid, attributes: ["@required", "@foreign_key(Employee.id)"] }
      npwp_number: { type: string?, attributes: ["@max(30)"], description: "Tax number (missing NPWP → surcharged by payroll)" }
      ptkp_override: { type: PtkpTier?, description: "Manual override of derived PTKP; null = derive from dependents" }
      tax_method: { type: TaxMethod, attributes: ["@required", "@default(gross)"], description: "gross / gross_up / netto" }
      tax_salary: { type: TaxSalary, attributes: ["@required", "@default(taxable)"], description: "taxable / non_taxable" }
      taxable_date: { type: date?, description: "Date tax status takes effect" }
      beginning_netto: { type: decimal?, attributes: ["@precision(18,2)", "@non_negative"], description: "Cumulative netto carryforward" }
      pph21_paid: { type: decimal?, attributes: ["@precision(18,2)", "@non_negative"], description: "YTD PPh 21 paid" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [employee_id] }
```

## `employee_bpjs.model.yaml` — statutory contributions

```yaml
models:
  - name: EmployeeBpjs
    collection: employee_bpjs
    description: "BPJS Ketenagakerjaan + Kesehatan identity (read by payroll)."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"] }
      employee_id: { type: uuid, attributes: ["@required", "@foreign_key(Employee.id)"] }
      bpjs_ketenagakerjaan_number: { type: string?, attributes: ["@max(30)"] }
      npp_bpjs_ketenagakerjaan: { type: string?, attributes: ["@max(30)"], description: "Employer NPP" }
      bpjs_ketenagakerjaan_date: { type: date?, description: "Registration date" }
      bpjs_kesehatan_number: { type: string?, attributes: ["@max(30)"] }
      bpjs_kesehatan_family: { type: integer?, attributes: ["@default(0)"], description: "Family members covered (0–10)" }
      bpjs_kesehatan_date: { type: date? }
      jaminan_pensiun_date: { type: date?, description: "Jaminan Pensiun start" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [employee_id] }
```

## `religion.model.yaml` + `bank.model.yaml` — reference masters (global)

```yaml
models:
  - name: Religion
    collection: religions
    description: "Religion reference master (global; scopes calendar holidays)."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      name: { type: string, attributes: ["@required", "@max(60)"] }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: unique, fields: [name], where: "deleted_at IS NULL" }

  - name: Bank
    collection: banks
    description: "Bank reference master (global)."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      name: { type: string, attributes: ["@required", "@max(120)"] }
      code: { type: string?, attributes: ["@max(20)"], description: "Bank code (e.g. SWIFT/BCA code)" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: unique, fields: [name], where: "deleted_at IS NULL" }
```

---

## Enums

```yaml
enums:
  - { name: Gender,         variants: [male, female] }
  - { name: MaritalStatus,  variants: [single, married, widow, widower] }
  - { name: BloodType,      variants: [A, B, AB, O] }
  - name: IdentityType
    variants:
      - { name: id, description: "KTP national ID" }
      - { name: passport }
  - name: EmploymentStatus
    description: "Employment relationship"
    variants:
      - { name: permanent, default: true }
      - { name: contract }
      - { name: probation }
      - { name: associate, description: "(typo assosiate fixed)" }
  - { name: EmploymentState, variants: [{name: active, default: true}, inactive] }
  - name: TaxMethod
    variants: [{name: gross, default: true}, gross_up, netto]
  - name: TaxSalary
    variants: [{name: taxable, default: true}, non_taxable]
  - name: FamilyRelationship
    variants: [spouse, child, parent, sibling, other]
  - name: PtkpTier
    description: "PTKP — Penghasilan Tidak Kena Pajak. DERIVED (not stored on tax by default)."
    variants: [TK0, TK1, TK2, TK3, K0, K1, K2, K3]
```

---

## ⚠️ PTKP derivation rule (the live correctness fix)

`EmployeeTax.ptkp_override` is nullable; when null, the tier is **derived** from `EmployeeFamily`:

```
married    = EXISTS EmployeeFamily(employee, relationship = spouse)
dependents = COUNT EmployeeFamily(employee, relationship = child)   // capped at 3
ptkp       = (married ? "K" : "TK") + min(dependents, 3)            // ∈ TK0..3, K0..3
```

This replaces salt-laravel's free `ptkp_status` enum (decoupled from dependents → could not honor
"add a child → PPh 21 relief tier changes"). The fix lets `employee_families` drive tax correctly.

## ⚠️ UU PDP compliance fence (🇮🇩 data privacy — legal precondition)

> **`backbone-employee` is NON-DEPLOYABLE in Indonesia until this fence exists.** UU PDP (UU 13/2022,
> in force) regulates the PII this module stores — NIK/KTP, NPWP, religion, family names+DOB, bank,
> mobile, email. Storing it without consent / retention / access-audit / data-subject-rights is
> non-compliant by default (sanction up to 2% annual revenue + criminal liability, Art 67). This is a
> legal fence on the data model, **not** a Phase-C feature.
> ([coherence council](../council/2026-08-01-module-hris-constellation-coherence.md), finding 2)

### `data_consent.model.yaml` — lawful basis + consent + retention

```yaml
models:
  - name: DataConsent
    collection: data_consents
    description: "Lawful basis, consent, and retention for a category of an employee's PII (UU PDP Art 16/20)."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"] }
      employee_id: { type: uuid, attributes: ["@required", "@foreign_key(Employee.id)"] }
      data_category: { type: DataCategory, attributes: ["@required"], description: "identity/financial/family/contact/employment/health/biometric" }
      lawful_basis: { type: LawfulBasis, attributes: ["@required"], description: "consent/contract/legal_obligation/employment/vital_interest" }
      consent_given_at: { type: datetime? }
      consent_method: { type: string?, attributes: ["@max(40)"], description: "signed/digital/verbal" }
      privacy_notice_version: { type: string?, attributes: ["@max(40)"], description: "the privacy notice version consented to" }
      retention_until: { type: date?, description: "When the data must be purged (Art 20)" }
      withdrawn_at: { type: datetime?, description: "Consent withdrawn → triggers purge workflow" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [employee_id, data_category] }

enums:
  - { name: DataCategory, variants: [identity, financial, family, contact, employment, health, biometric] }
  - { name: LawfulBasis,  variants: [consent, contract, legal_obligation, employment, vital_interest, legitimate_interest] }
```

### `data_subject_request.model.yaml` — DSAR (data-subject rights, Art 5/16)

```yaml
models:
  - name: DataSubjectRequest
    collection: data_subject_requests
    description: "An employee's UU PDP right exercise: access / rectify / erase / export / object."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"] }
      employee_id: { type: uuid, attributes: ["@required", "@foreign_key(Employee.id)"] }
      request_type: { type: DataSubjectRight, attributes: ["@required"] }
      status: { type: DsarStatus, attributes: ["@required", "@default(pending)"] }
      requested_at: { type: datetime, attributes: ["@required", "@default(now)"] }
      fulfilled_at: { type: datetime? }
      response: { type: json?, description: "Fulfillment payload / export reference" }
      note: { type: text? }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [company_id, status] }
      - { type: index, fields: [employee_id] }

enums:
  - { name: DataSubjectRight, variants: [access, rectify, erase, export, object, restrict] }
  - { name: DsarStatus, variants: [{name: pending, default: true}, in_progress, fulfilled, rejected] }
```

### `pii_access_log.model.yaml` — access audit (accountability)

```yaml
models:
  - name: PiiAccessLog
    collection: pii_access_logs
    description: "Who accessed whose PII, when, why. APPEND-ONLY — this is the audit trail (no soft-delete)."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"] }
      employee_id: { type: uuid, attributes: ["@required", "@foreign_key(Employee.id)"], description: "Whose PII was accessed" }
      accessed_by: { type: uuid, attributes: ["@required", "@foreign_key(sapiens.User.id)"], description: "The user who accessed it" }
      data_category: { type: DataCategory, attributes: ["@required"] }
      purpose: { type: string?, attributes: ["@max(120)"] }
      accessed_at: { type: datetime, attributes: ["@required", "@default(now)"] }
    indexes:
      - { type: index, fields: [employee_id, accessed_at] }
      - { type: index, fields: [accessed_by] }
```

> **Future extraction:** as PDP compliance grows (payroll salary PII, health data, breach
> notification), these may extract to a dedicated `backbone-privacy` context consumed by all PII-holding
> modules. For now they live in `backbone-employee`, the primary regulated-PII owner.

## Read ports

```rust
#[async_trait]
pub trait EmployeePort: Send + Sync {
    /// Full identity snapshot for onboarding / payroll: identity + employment + payroll-identity + family.
    async fn resolve_employee(&self, employee_id: Uuid) -> Result<EmployeeSnapshot, EmployeeRejected>;
    /// Derived PTKP tier (override > derived-from-dependents).
    async fn employee_ptkp(&self, employee_id: Uuid) -> Result<PtkpTier, EmployeeRejected>;
}
```

## Notes / decisions

- **Religion + Bank are global** (no `company_id`) — universal reference data. Alternative: company-scoped
  if per-company bank/religion lists are needed.
- **Position / Level** are referenced by `employments` but live in `backbone-organization` (which lacks
  them today — must be added there as org-design masters; not this module's concern).
- **No salary field here.** `base_salary` / salary structure belongs to `backbone-payroll` (the tax
  overlay split, ADR-001 §4) — backbone-employee carries the *identity*, payroll carries the math.
