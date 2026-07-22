-- ADR-001 parking lot — employee_number uniqueness was app-checked only (a write-path probe +
-- insert), so two concurrent onboarding requests in the same company could both pass the probe
-- and both insert the same employee_number. This partial unique index makes the invariant
-- DB-enforced against ANY writer, concurrent or not. Partial on live rows: a soft-deleted
-- employee (metadata.deleted_at set) does not occupy its number, so a re-onboard of the same
-- number after deletion is allowed. Hand-authored migration (no generator marker) — preserved
-- across regen; the matching index declaration lives in schema/models/employee.model.yaml.
CREATE UNIQUE INDEX IF NOT EXISTS idx_employees_company_id_employee_number
  ON hr.employees (company_id, employee_number)
  WHERE (metadata ->> 'deleted_at') IS NULL;
