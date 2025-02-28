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

/// struct for typed errors of method [`delete_one_notification`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeleteOneNotificationError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`delete_one_notification_config`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeleteOneNotificationConfigError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`get_all_notification_configs`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetAllNotificationConfigsError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`get_all_notifications`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetAllNotificationsError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`get_one_notification`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetOneNotificationError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`update_one_notification_config`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UpdateOneNotificationConfigError {
    UnknownValue(serde_json::Value),
}

pub async fn delete_one_notification(
    configuration: &configuration::Configuration,
    id: &str,
) -> Result<bool, Error<DeleteOneNotificationError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_id = id;

    let uri_str = format!(
        "{}/api/notifications/{id}",
        configuration.base_path,
        id = crate::apis::urlencode(p_id)
    );
    ApiBuilder::new(configuration, reqwest::Method::DELETE, &uri_str)
        .execute()
        .await
}

pub async fn delete_one_notification_config(
    configuration: &configuration::Configuration,
    body: Option<&str>,
) -> Result<bool, Error<DeleteOneNotificationConfigError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_body = body;

    let uri_str = format!("{}/api/notifications/configs", configuration.base_path);
    ApiBuilder::new(configuration, reqwest::Method::DELETE, &uri_str)
        .with_body(p_body)
        .execute()
        .await
}

pub async fn get_all_notification_configs(
    configuration: &configuration::Configuration,
) -> Result<Vec<models::WsConfigItemDto>, Error<GetAllNotificationConfigsError>> {
    let uri_str = format!("{}/api/notifications/configs", configuration.base_path);
    ApiBuilder::new(configuration, reqwest::Method::GET, &uri_str)
        .execute()
        .await
}

pub async fn get_all_notifications(
    configuration: &configuration::Configuration,
    unread_only: Option<bool>,
) -> Result<Vec<models::WsNotificationDto>, Error<GetAllNotificationsError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_unread_only = unread_only;

    let uri_str = format!("{}/api/notifications", configuration.base_path);
    ApiBuilder::new(configuration, reqwest::Method::GET, &uri_str)
        .with_opt_query("unreadOnly", p_unread_only)?
        .execute()
        .await
}

pub async fn get_one_notification(
    configuration: &configuration::Configuration,
    id: &str,
) -> Result<models::WsNotificationDto, Error<GetOneNotificationError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_id = id;

    let uri_str = format!(
        "{}/api/notifications/{id}",
        configuration.base_path,
        id = crate::apis::urlencode(p_id)
    );
    ApiBuilder::new(configuration, reqwest::Method::GET, &uri_str)
        .execute()
        .await
}

pub async fn update_one_notification_config(
    configuration: &configuration::Configuration,
    ws_config_put_dto: Option<models::WsConfigPutDto>,
) -> Result<models::WsConfigItemDto, Error<UpdateOneNotificationConfigError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_ws_config_put_dto = ws_config_put_dto;

    let uri_str = format!("{}/api/notifications/configs", configuration.base_path);
    ApiBuilder::new(configuration, reqwest::Method::PUT, &uri_str)
        .with_body(p_ws_config_put_dto)
        .execute()
        .await
}
