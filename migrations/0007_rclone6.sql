-- Add migration script here
ALTER TABLE transfer_jobs_internal
    ADD COLUMN init_hash TEXT;