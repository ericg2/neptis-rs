use crate::rclone::{RCloneClient, RCloneSettings};
use neptis_lib::prelude::DbController;
use rocket::{catch, catchers};
use std::sync::Arc;
use std::thread;
use tokio::io::AsyncWriteExt;
use tokio::runtime::Runtime;

mod errors;
mod macros;
mod rclone;
mod api;
mod schema;

#[rocket::launch]
fn rocket() -> _ {
    let rt = Arc::new(Runtime::new().unwrap());
    let settings = RCloneSettings::new(neptis_lib::get_working_dir());
    let db = Arc::new(DbController::new(rt));
    let client = Arc::new(RCloneClient::new(settings, db));
    
    rocket::build()
        .manage(client.clone())
        .register("/", catchers![not_found, unauthorized])
        .attach(rocket::fairing::AdHoc::on_liftoff("Test", move |_| {
            let nb_clone = client.clone();
            Box::pin(async move {
                let _ = thread::spawn(move || {
                    nb_clone.handle_blocking();
                });
            })
        }))
}

#[catch(404)]
fn not_found() -> &'static str {
    "Not Found"
}

#[catch(401)]
fn unauthorized() -> &'static str {
    "Unauthorized"
}