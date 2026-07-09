-- Down: drop hr.leave_types table
DROP TABLE IF EXISTS hr.leave_types CASCADE;
DROP FUNCTION IF EXISTS hr.leave_types_audit_timestamp() CASCADE;
