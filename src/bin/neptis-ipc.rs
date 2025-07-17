use neptis_rs::ipc::handlers;
use neptis_rs::ipc::rclone::{RCloneClient, RCloneSettings};
use neptis_rs::prelude::{DbController, IPC_PORT, WebApi};
use rocket::{Config, catch, catchers, get, routes};
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::thread;
use tokio::runtime::Runtime;
use neptis_rs::ipc::msg_receiver::IPCMessageReceiver;

#[get("/")]
fn ping() -> &'static str {
    "Pong!"
}

#[rocket::launch]
fn rocket() -> _ {
    let rt = Arc::new(Runtime::new().unwrap());

    if rt.block_on(async { WebApi::ipc_ping().await }).is_ok() {
        panic!(
            "The background service port ({}) is already in-use. Is there another instance of \
            this program running, or a separate service using this port?",
            IPC_PORT
        )
    }

    let settings = RCloneSettings::new(neptis_rs::get_working_dir());
    let db = Arc::new(DbController::new(rt.clone()));
    let sync_client = Arc::new(RCloneClient::new(settings, db.clone(), rt.clone()));
    
    let mut msg_client = IPCMessageReceiver::new(db, rt);
    let mut config = Config::release_default();
    config.port = IPC_PORT;
    config.address = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    rocket::custom(&config)
        .manage(sync_client.clone())
        .register("/", catchers![not_found, unauthorized])
        .mount("/jobs", handlers::get_routes())
        .mount("/ping", routes![ping])
        .attach(rocket::fairing::AdHoc::on_liftoff("Test", move |_| {
            let nb_clone = sync_client.clone();
            Box::pin(async move {
                let _ = thread::spawn(move || {
                    nb_clone.handle_blocking();
                });
            })
        }))
        .attach(rocket::fairing::AdHoc::on_liftoff("Test2", move |_| {
            Box::pin(async move {
                let _ = thread::spawn(move || {
                    msg_client.handle_blocking();
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
