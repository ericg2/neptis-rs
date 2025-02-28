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

/// struct for typed errors of method [`get_system_summary`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetSystemSummaryError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`get_valid_notify_methods`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetValidNotifyMethodsError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`get_valid_notify_subscriptions`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetValidNotifySubscriptionsError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`get_valid_permissions`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetValidPermissionsError {
    UnknownValue(serde_json::Value),
}

pub async fn get_system_summary(
    configuration: &configuration::Configuration,
) -> Result<models::SystemStatusDto, Error<GetSystemSummaryError>> {
    let uri_str = format!("{}/api/infos/summary", configuration.base_path);
    ApiBuilder::new(configuration, reqwest::Method::GET, &uri_str)
        .execute()
        .await
}

pub async fn get_valid_notify_methods(
    configuration: &configuration::Configuration,
) -> Result<Vec<String>, Error<GetValidNotifyMethodsError>> {
    let uri_str = format!("{}/api/infos/validnotifymethods", configuration.base_path);
    ApiBuilder::new(configuration, reqwest::Method::GET, &uri_str)
        .execute()
        .await
}

pub async fn get_valid_notify_subscriptions(
    configuration: &configuration::Configuration,
) -> Result<Vec<String>, Error<GetValidNotifySubscriptionsError>> {
    let uri_str = format!("{}/api/infos/validsubscriptions", configuration.base_path);
    ApiBuilder::new(configuration, reqwest::Method::GET, &uri_str)
        .execute()
        .await
}

pub async fn get_valid_permissions(
    configuration: &configuration::Configuration,
) -> Result<Vec<String>, Error<GetValidPermissionsError>> {
    let uri_str = format!("{}/api/infos/validperms", configuration.base_path);
    ApiBuilder::new(configuration, reqwest::Method::GET, &uri_str)
        .execute()
        .await
}
