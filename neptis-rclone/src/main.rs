use crate::rclone::{RCloneClient, RCloneSettings};
use neptis_lib::prelude::{DbController, IPC_PORT};
use rocket::{catch, catchers, get, routes, Config};
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::thread;
use tokio::runtime::Runtime;

mod errors;
mod macros;
mod rclone;
mod schema;
mod handlers;

#[get("/")]
fn ping() -> &'static str {
    "Pong!"
}

#[rocket::launch]
fn rocket() -> _ {
    let rt = Arc::new(Runtime::new().unwrap());
    let settings = RCloneSettings::new(neptis_lib::get_working_dir());
    let db = Arc::new(DbController::new(rt.clone()));
    let client = Arc::new(RCloneClient::new(settings, db, rt.clone()));

    let mut config = Config::release_default();
    config.port = IPC_PORT;
    config.address = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    rocket::custom(&config)
        .manage(client.clone())
        .register("/", catchers![not_found, unauthorized])
        .mount("/jobs", handlers::get_routes())
        .mount("/ping", routes![ping])
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
