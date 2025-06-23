use diesel::table;

table! {
    transfer_auto_schedules(batch_id) {
        batch_id -> Text,
        server_name -> Text,
        cron_schedule -> Text,
    }
}

table! {
    transfer_auto_jobs(id) {
        id -> Text,
        batch_id -> Text,
        smb_user_name -> Text,
        smb_password -> Text,
        smb_folder -> Text,
        local_folder -> Text,
    }
}