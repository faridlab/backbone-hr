-- Down: drop hr.leave_applications table
DROP TABLE IF EXISTS hr.leave_applications CASCADE;
DROP FUNCTION IF EXISTS hr.leave_applications_audit_timestamp() CASCADE;
