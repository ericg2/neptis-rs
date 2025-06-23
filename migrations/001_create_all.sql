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
    PRIMARY KEY (schedule_name)
);

CREATE TABLE transfer_auto_jobs
(
    id            TEXT NOT NULL,
    batch_id      TEXT NOT NULL,
    schedule_name TEXT NOT NULL,
    smb_user_name TEXT NOT NULL,
    smb_password  TEXT NOT NULL,
    smb_folder    TEXT NOT NULL,
    local_folder  TEXT NOT NULL,
    PRIMARY KEY (id)
);