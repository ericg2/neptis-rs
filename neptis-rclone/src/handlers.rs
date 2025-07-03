use crate::errors::ApiError;
use crate::rclone::RCloneClient;
use neptis_lib::db::sync_models::TransferJobDto;
use neptis_lib::prelude::PostForAutoScheduleStartDto;
use rocket::State;
use rocket::serde::json::Json;
use rocket::{Route, routes};
use rocket::{delete, get, post, put};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

#[get("/<id>")]
async fn get_one_job(
    id: String,
    nb: &State<Arc<RCloneClient>>,
) -> Result<Json<TransferJobDto>, ApiError> {
    let sid = Uuid::from_str(&id).map_err(|_| ApiError::BadRequest("ID is not valid!".into()))?;
    Ok(Json(nb.get_job(sid).await.ok_or(ApiError::BadRequest(
        "Job does not exist!".into(),
    ))?))
}

#[get("/")]
async fn get_all_jobs(
    nb: &State<Arc<RCloneClient>>,
) -> Result<Json<Vec<TransferJobDto>>, ApiError> {
    let ret = nb
        .get_all_jobs()
        .await
        .ok_or(ApiError::InternalError("Failed to load jobs!".into()))?;
    Ok(Json(ret))
}

#[delete("/<id>")]
fn cancel_one_job(id: String, nb: &State<Arc<RCloneClient>>) -> Result<(), ApiError> {
    let sid = Uuid::from_str(&id).map_err(|_| ApiError::BadRequest("ID is not valid!".into()))?;
    nb.cancel_job(sid)
}

#[post("/start", data = "<data>")]
fn start_one_auto_job(
    data: Json<PostForAutoScheduleStartDto>,
    nb: &State<Arc<RCloneClient>>,
) -> Result<(), ApiError> {
    nb.start_auto_job(data.0)
}

pub fn get_routes() -> Vec<Route> {
    routes![
        get_one_job,
        cancel_one_job,
        start_one_auto_job,
        get_all_jobs
    ]
}
