-- Down: drop hr.leave_balances table
DROP TABLE IF EXISTS hr.leave_balances CASCADE;
DROP FUNCTION IF EXISTS hr.leave_balances_audit_timestamp() CASCADE;
