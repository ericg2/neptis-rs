CREATE TABLE server_items
(
    server_name      TEXT    NOT NULL,
    server_endpoint  TEXT    NOT NULL,
    server_password  TEXT,
    user_name        TEXT,
    user_password    TEXT,
    arduino_endpoint TEXT,
    arduino_password TEXT,
    auto_fuse        BOOLEAN NOT NULL,
    is_default       BOOLEAN NOT NULL,
    PRIMARY KEY (server_name)
);

CREATE TABLE transfer_auto_schedules
(
    schedule_name TEXT NOT NULL,
    server_name   TEXT NOT NULL,
    cron_schedule TEXT NOT NULL,
    smb_user_name TEXT NOT NULL,
    smb_password  TEXT NOT NULL,
    PRIMARY KEY (schedule_name, server_name)
);

CREATE TABLE transfer_auto_jobs
(
    schedule_name TEXT NOT NULL,
    server_name   TEXT NOT NULL,
    action_name   TEXT NOT NULL,
    smb_folder    TEXT NOT NULL,
    local_folder  TEXT NOT NULL,
    PRIMARY KEY (schedule_name, server_name, action_name)
);

-- Add migration script here
CREATE TABLE IF NOT EXISTS transfer_jobs_internal
(
    job_id        TEXT NOT NULL,
    auto_job      TEXT,
    server_name   TEXT NOT NULL,
    smb_user_name TEXT NOT NULL,
    smb_password  TEXT NOT NULL,
    smb_folder    TEXT NOT NULL,
    local_folder  TEXT NOT NULL,
    last_stats    TEXT,
    start_date    TEXT,
    end_date      TEXT,
    fatal_errors  TEXT NOT NULL,
    warnings      TEXT NOT NULL,
    last_updated  TEXT NOT NULL,
    PRIMARY KEY (job_id)
);

ALTER TABLE transfer_jobs_internal
    RENAME COLUMN auto_job TO auto_job_action_name;

ALTER TABLE transfer_jobs_internal
    ADD COLUMN auto_job_schedule_name TEXT;

ALTER TABLE transfer_auto_schedules
    ADD COLUMN last_updated TEXT;

UPDATE transfer_auto_schedules
SET last_updated = '07-02-2025 00:00:00'
WHERE last_updated = '';   

ALTER TABLE transfer_auto_jobs
    ADD COLUMN enabled BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE transfer_auto_schedules
    ADD COLUMN user_password TEXT;
ALTER TABLE transfer_auto_schedules
    ADD COLUMN backup_on_finish BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE transfer_jobs_internal
    ADD COLUMN init_hash TEXT;