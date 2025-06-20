use crate::rclone::{RCloneClient, RCloneJobLaunchInfo, RCloneSettings};
mod rclone;
mod dtos;
mod models;
mod errors;
mod macros;

fn main() {
    let settings = RCloneSettings::new("C:\\Users\\Eric\\rclone");
    let mut client = RCloneClient::new(settings);
    let job = client.create_job(RCloneJobLaunchInfo {
        host: "192.168.1.149",
        user_name: "eric-smb",
        password: "Rugratse124!",
        local_folder: "C:\\Users\\Eric\\Downloads\\test",
        remote_folder: "eric-storage-data/Downloads"
    }).unwrap();
    job.start().unwrap();
    loop {}
}
