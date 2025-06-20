CREATE TABLE server_items (
    server_name TEXT NOT NULL,
    server_endpoint TEXT NOT NULL,
    server_password TEXT,
    user_name TEXT,
    user_password TEXT,
    arduino_endpoint TEXT,
    arduino_password TEXT,
    auto_fuse BOOLEAN NOT NULL,
    is_default BOOLEAN NOT NULL,
    PRIMARY KEY (server_name)
);

CREATE TABLE auto_transfers (
    id TEXT PRIMARY KEY NOT NULL,
    server_name TEXT NOT NULL,
    user_name TEXT NOT NULL,
    user_password TEXT NOT NULL,
    point_name TEXT NOT NULL,
    cron_schedule TEXT NOT NULL,
    last_ran TEXT
);