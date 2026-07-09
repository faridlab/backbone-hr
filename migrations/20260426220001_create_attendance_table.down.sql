-- Down: drop hr.attendances table
DROP TABLE IF EXISTS hr.attendances CASCADE;
DROP FUNCTION IF EXISTS hr.attendances_audit_timestamp() CASCADE;
