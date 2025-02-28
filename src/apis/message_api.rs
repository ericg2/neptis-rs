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

/// struct for typed errors of method [`delete_one_message`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeleteOneMessageError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`get_all_messages`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetAllMessagesError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`get_one_message`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetOneMessageError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`post_one_message`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PostOneMessageError {
    UnknownValue(serde_json::Value),
}

pub async fn delete_one_message(
    configuration: &configuration::Configuration,
    id: &str,
) -> Result<bool, Error<DeleteOneMessageError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_id = id;

    let uri_str = format!(
        "{}/api/messages/{id}",
        configuration.base_path,
        id = crate::apis::urlencode(p_id)
    );
    configuration
        .execute(
            reqwest::Method::DELETE,
            &uri_str,
            None::<Value>,
            None::<Value>,
        )
        .await
}

pub async fn get_all_messages(
    configuration: &configuration::Configuration,
) -> Result<Vec<models::MessageItemDto>, Error<GetAllMessagesError>> {
    let uri_str = format!("{}/api/messages", configuration.base_path);
    configuration
        .execute(reqwest::Method::GET, &uri_str, None::<Value>, None::<Value>)
        .await
}

pub async fn get_one_message(
    configuration: &configuration::Configuration,
    id: &str,
) -> Result<models::MessageItemDto, Error<GetOneMessageError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_id = id;

    let uri_str = format!(
        "{}/api/messages/{id}",
        configuration.base_path,
        id = crate::apis::urlencode(p_id)
    );
    configuration
        .execute(reqwest::Method::GET, &uri_str, None::<Value>, None::<Value>)
        .await
}

pub async fn post_one_message(
    configuration: &configuration::Configuration,
    message_post_dto: Option<models::MessagePostDto>,
) -> Result<bool, Error<PostOneMessageError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_message_post_dto = message_post_dto;

    let uri_str = format!("{}/api/messages", configuration.base_path);
    configuration
        .execute(
            reqwest::Method::POST,
            &uri_str,
            Some(p_message_post_dto),
            None::<Value>,
        )
        .await
}
