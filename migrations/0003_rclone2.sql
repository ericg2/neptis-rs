-- Add migration script here
-- Rename column auto_job to auto_job_action_name
ALTER TABLE transfer_jobs_internal RENAME COLUMN auto_job TO auto_job_action_name;

-- Add new column auto_job_schedule_name
ALTER TABLE transfer_jobs_internal ADD COLUMN auto_job_schedule_name TEXT;