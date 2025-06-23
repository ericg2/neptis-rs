use crate::rclone::{RCloneClient, RCloneJobLaunchInfo, RCloneSettings};
use diesel::deserialize::FromSql;
use diesel::serialize::ToSql;
use neptis_lib::prelude::DbController;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::runtime::Runtime;

mod dtos;
mod errors;
mod macros;
mod models;
mod rclone;
mod schema;
mod uuid;

fn main() {
    let rt = Arc::new(Runtime::new().unwrap());
    let settings = RCloneSettings::new("C:\\Users\\Eric\\rclone");
    let db = Arc::new(DbController::new_default(rt, None));
    let mut client = RCloneClient::new_owned(settings, db);

    let batch_id = client
        .create_batch(vec![RCloneJobLaunchInfo {
            server_name: "192.168.1.149".into(),
            smb_user_name: "eric-smb".into(),
            smb_password: "Rugratse124!".into(),
            local_folder: "C:\\Users\\Eric\\Downloads\\test".into(),
            smb_folder: "eric-storage-data/Downloads".into(),
        }])
        .unwrap();
    client.start_batch(batch_id).unwrap();
    loop {}
}
