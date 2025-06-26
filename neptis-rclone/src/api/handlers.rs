use crate::api::dtos::TransferJobDto;
use crate::errors::ApiError;
use crate::rclone::RCloneClient;
use rocket::State;
use rocket::serde::json::Json;
use rocket::{Route, routes};
use rocket::{delete, get, post, put};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

#[get("/<id>")]
fn get_one_job(
    id: String,
    nb: &State<Arc<RCloneClient>>,
) -> Result<Json<TransferJobDto>, ApiError> {
    let sid = Uuid::from_str(&id).map_err(|_| ApiError::BadRequest("ID is not valid!".into()))?;
    Ok(Json(nb.get_job(sid).ok_or(ApiError::BadRequest(
        "Job does not exist!".into(),
    ))?))
}

#[delete("/<id>")]
fn cancel_one_job(
    id: String,
    nb: &State<Arc<RCloneClient>>,
) -> Result<(), ApiError> {
    let sid = Uuid::from_str(&id).map_err(|_| ApiError::BadRequest("ID is not valid!".into()))?;
    nb.cancel_job(sid)
}

pub fn get_routes() -> Vec<Route> {
    routes![get_one_job, cancel_one_job]
}
