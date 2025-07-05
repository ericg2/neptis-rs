-- Add migration script here
ALTER TABLE transfer_auto_jobs
    ADD COLUMN enabled BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE transfer_auto_schedules
    ADD COLUMN user_password TEXT;
ALTER TABLE transfer_auto_schedules
    ADD COLUMN backup_on_finish BOOLEAN NOT NULL DEFAULT FALSE;