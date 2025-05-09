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