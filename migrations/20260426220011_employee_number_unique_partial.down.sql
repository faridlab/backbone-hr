-- Reverse of 20260426220011_employee_number_unique_partial.up.sql.
-- Drops the partial unique index; per-company employee_number uniqueness reverts to
-- app-checked-only (write-path probe in HrWriteService::onboard_employee).
DROP INDEX IF EXISTS hr.idx_employees_company_id_employee_number;
