/*
 * Neptis
 *
 * No description provided (generated by Openapi Generator https://github.com/openapitools/openapi-generator)
 *
 * The version of the OpenAPI document: v1
 *
 * Generated by: https://openapi-generator.tech
 */

use super::{Error, configuration};
use crate::{apis::ResponseContent, models};
use reqwest;
use serde::{Deserialize, Serialize};

/// struct for typed errors of method [`cancel_one_job`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CancelOneJobError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`get_all_jobs`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetAllJobsError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`get_one_job`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetOneJobError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`start_one_backup`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StartOneBackupError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`start_one_restore`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StartOneRestoreError {
    UnknownValue(serde_json::Value),
}

pub async fn cancel_one_job(
    configuration: &configuration::Configuration,
    id: &str,
) -> Result<bool, Error<CancelOneJobError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_id = id;

    let uri_str = format!(
        "{}/api/jobs/{id}",
        configuration.base_path,
        id = crate::apis::urlencode(p_id)
    );
    configuration
        .execute(reqwest::Method::DELETE, &uri_str, None, None)
        .await
}

pub async fn get_all_jobs(
    configuration: &configuration::Configuration,
) -> Result<Vec<models::RepoDataJobDto>, Error<GetAllJobsError>> {
    let uri_str = format!("{}/api/jobs", configuration.base_path);
    configuration
        .execute(reqwest::Method::GET, &uri_str, None, None)
        .await
}

pub async fn get_one_job(
    configuration: &configuration::Configuration,
    id: &str,
) -> Result<models::RepoDataJobDto, Error<GetOneJobError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_id = id;

    let uri_str = format!(
        "{}/api/jobs/{id}",
        configuration.base_path,
        id = crate::apis::urlencode(p_id)
    );
    configuration
        .execute(reqwest::Method::GET, &uri_str, None, None)
        .await
}

pub async fn start_one_backup(
    configuration: &configuration::Configuration,
    job_backup_dto: Option<models::JobBackupDto>,
) -> Result<models::RepoDataJobDto, Error<StartOneBackupError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_job_backup_dto = job_backup_dto;

    let uri_str = format!("{}/api/jobs/backup", configuration.base_path);
    configuration
        .execute(
            reqwest::Method::POST,
            &uri_str,
            Some(p_job_backup_dto),
            None,
        )
        .await
}

pub async fn start_one_restore(
    configuration: &configuration::Configuration,
    job_restore_dto: Option<models::JobRestoreDto>,
) -> Result<models::RepoDataJobDto, Error<StartOneRestoreError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_job_restore_dto = job_restore_dto;

    let uri_str = format!("{}/api/jobs/restore", configuration.base_path);
    configuration
        .execute(
            reqwest::Method::POST,
            &uri_str,
            Some(p_job_restore_dto),
            None,
        )
        .await
}
