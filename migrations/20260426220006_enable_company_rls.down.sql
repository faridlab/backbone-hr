-- Down: remove the company RLS fence for hr module

-- Reverse the company RLS fence for hr.attendances
DROP POLICY IF EXISTS attendances_company_isolation ON hr.attendances;
ALTER TABLE hr.attendances NO FORCE ROW LEVEL SECURITY;
ALTER TABLE hr.attendances DISABLE ROW LEVEL SECURITY;

-- Reverse the company RLS fence for hr.employees
DROP POLICY IF EXISTS employees_company_isolation ON hr.employees;
ALTER TABLE hr.employees NO FORCE ROW LEVEL SECURITY;
ALTER TABLE hr.employees DISABLE ROW LEVEL SECURITY;

-- Reverse the company RLS fence for hr.leave_applications
DROP POLICY IF EXISTS leave_applications_company_isolation ON hr.leave_applications;
ALTER TABLE hr.leave_applications NO FORCE ROW LEVEL SECURITY;
ALTER TABLE hr.leave_applications DISABLE ROW LEVEL SECURITY;

-- Reverse the company RLS fence for hr.leave_balances
DROP POLICY IF EXISTS leave_balances_company_isolation ON hr.leave_balances;
ALTER TABLE hr.leave_balances NO FORCE ROW LEVEL SECURITY;
ALTER TABLE hr.leave_balances DISABLE ROW LEVEL SECURITY;

-- Reverse the company RLS fence for hr.leave_types
DROP POLICY IF EXISTS leave_types_company_isolation ON hr.leave_types;
ALTER TABLE hr.leave_types NO FORCE ROW LEVEL SECURITY;
ALTER TABLE hr.leave_types DISABLE ROW LEVEL SECURITY;

