-- Down: drop hr.employees table
DROP TABLE IF EXISTS hr.employees CASCADE;
DROP FUNCTION IF EXISTS hr.employees_audit_timestamp() CASCADE;
