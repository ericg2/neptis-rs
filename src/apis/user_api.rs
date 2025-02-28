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
use serde_json::Value;
use crate::apis::configuration::ApiBuilder;

/// struct for typed errors of method [`create_one_user`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CreateOneUserError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`delete_one_user`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeleteOneUserError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`disable_one_permission_for_user`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DisableOnePermissionForUserError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`enable_one_permission_for_user`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EnableOnePermissionForUserError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`get_all_permissions_for_user`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetAllPermissionsForUserError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`get_all_users`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetAllUsersError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`get_one_permission_for_user`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetOnePermissionForUserError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`get_one_user`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetOneUserError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`update_all_permissions_for_user`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UpdateAllPermissionsForUserError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`update_one_user`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UpdateOneUserError {
    UnknownValue(serde_json::Value),
}

pub async fn create_one_user(
    configuration: &configuration::Configuration,
    user_create_dto: Option<models::UserCreateDto>,
) -> Result<models::UserSummaryDto, Error<CreateOneUserError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_user_create_dto = user_create_dto;

    let uri_str = format!("{}/api/users", configuration.base_path);
    ApiBuilder::new(&configuration, reqwest::Method::POST, &uri_str)
        .with_body(p_user_create_dto)
        .execute()
        .await
}

pub async fn delete_one_user(
    configuration: &configuration::Configuration,
    user_name: &str,
) -> Result<bool, Error<DeleteOneUserError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_user_name = user_name;

    let uri_str = format!(
        "{}/api/users/{userName}",
        configuration.base_path,
        userName = crate::apis::urlencode(p_user_name)
    );
    ApiBuilder::new(&configuration, reqwest::Method::DELETE, &uri_str)
        .execute()
        .await
}

pub async fn disable_one_permission_for_user(
    configuration: &configuration::Configuration,
    user_name: &str,
    permission_name: &str,
) -> Result<bool, Error<DisableOnePermissionForUserError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_user_name = user_name;
    let p_permission_name = permission_name;

    let uri_str = format!(
        "{}/api/users/{userName}/perms/{permissionName}",
        configuration.base_path,
        userName = crate::apis::urlencode(p_user_name),
        permissionName = crate::apis::urlencode(p_permission_name)
    );
    ApiBuilder::new(&configuration, reqwest::Method::DELETE, &uri_str)
        .execute()
        .await
}

pub async fn enable_one_permission_for_user(
    configuration: &configuration::Configuration,
    user_name: &str,
    permission_name: &str,
) -> Result<bool, Error<EnableOnePermissionForUserError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_user_name = user_name;
    let p_permission_name = permission_name;

    let uri_str = format!(
        "{}/api/users/{userName}/perms/{permissionName}",
        configuration.base_path,
        userName = crate::apis::urlencode(p_user_name),
        permissionName = crate::apis::urlencode(p_permission_name)
    );
    ApiBuilder::new(&configuration, reqwest::Method::POST, &uri_str)
        .execute()
        .await
}

pub async fn get_all_permissions_for_user(
    configuration: &configuration::Configuration,
    user_name: &str,
) -> Result<Vec<models::UserPermission>, Error<GetAllPermissionsForUserError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_user_name = user_name;

    let uri_str = format!(
        "{}/api/users/{userName}/perms",
        configuration.base_path,
        userName = crate::apis::urlencode(p_user_name)
    );
    ApiBuilder::new(&configuration, reqwest::Method::GET, &uri_str)
        .execute()
        .await
}

pub async fn get_all_users(
    configuration: &configuration::Configuration,
) -> Result<Vec<models::UserSummaryDto>, Error<GetAllUsersError>> {
    let uri_str = format!("{}/api/users", configuration.base_path);
    ApiBuilder::new(&configuration, reqwest::Method::GET, &uri_str)
        .execute()
        .await
}

pub async fn get_one_permission_for_user(
    configuration: &configuration::Configuration,
    user_name: &str,
    permission_name: &str,
) -> Result<models::UserPermission, Error<GetOnePermissionForUserError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_user_name = user_name;
    let p_permission_name = permission_name;

    let uri_str = format!(
        "{}/api/users/{userName}/perms/{permissionName}",
        configuration.base_path,
        userName = crate::apis::urlencode(p_user_name),
        permissionName = crate::apis::urlencode(p_permission_name)
    );
    ApiBuilder::new(&configuration, reqwest::Method::GET, &uri_str)
        .execute()
        .await
}

pub async fn get_one_user(
    configuration: &configuration::Configuration,
    user_name: &str,
) -> Result<models::UserSummaryDto, Error<GetOneUserError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_user_name = user_name;

    let uri_str = format!(
        "{}/api/users/{userName}",
        configuration.base_path,
        userName = crate::apis::urlencode(p_user_name)
    );
    ApiBuilder::new(&configuration, reqwest::Method::GET, &uri_str)
        .execute()
        .await
}

pub async fn update_all_permissions_for_user(
    configuration: &configuration::Configuration,
    user_name: &str,
    user_permission_dto: Option<Vec<models::UserPermissionDto>>,
) -> Result<Vec<models::UserPermission>, Error<UpdateAllPermissionsForUserError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_user_name = user_name;
    let p_user_permission_dto = user_permission_dto;

    let uri_str = format!(
        "{}/api/users/{userName}/perms",
        configuration.base_path,
        userName = crate::apis::urlencode(p_user_name)
    );
    ApiBuilder::new(&configuration, reqwest::Method::PUT, &uri_str)
        .with_body(p_user_permission_dto)
        .execute()
        .await
}

pub async fn update_one_user(
    configuration: &configuration::Configuration,
    user_name: &str,
    user_put_dto: Option<models::UserPutDto>,
) -> Result<models::UserSummaryDto, Error<UpdateOneUserError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_user_name = user_name;
    let p_user_put_dto = user_put_dto;

    let uri_str = format!(
        "{}/api/users/{userName}",
        configuration.base_path,
        userName = crate::apis::urlencode(p_user_name)
    );
    ApiBuilder::new(&configuration, reqwest::Method::PUT, &uri_str)
        .with_body(p_user_put_dto)
        .execute()
        .await
}
