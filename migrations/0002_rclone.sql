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