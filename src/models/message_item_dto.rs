/*
 * Neptis
 *
 * No description provided (generated by Openapi Generator https://github.com/openapitools/openapi-generator)
 *
 * The version of the OpenAPI document: v1
 *
 * Generated by: https://openapi-generator.tech
 */

use crate::models;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct MessageItemDto {
    #[serde(rename = "id", skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(
        rename = "fromUser",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub from_user: Option<Option<String>>,
    #[serde(
        rename = "toUsers",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub to_users: Option<Option<Vec<String>>>,
    #[serde(
        rename = "subject",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub subject: Option<Option<String>>,
    #[serde(
        rename = "text",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub text: Option<Option<String>>,
    #[serde(rename = "highPriority", skip_serializing_if = "Option::is_none")]
    pub high_priority: Option<bool>,
    #[serde(rename = "dateSent", skip_serializing_if = "Option::is_none")]
    pub date_sent: Option<String>,
    #[serde(
        rename = "readBy",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub read_by: Option<Option<Vec<models::MessageReadItem>>>,
}

impl MessageItemDto {
    pub fn new() -> MessageItemDto {
        MessageItemDto {
            id: None,
            from_user: None,
            to_users: None,
            subject: None,
            text: None,
            high_priority: None,
            date_sent: None,
            read_by: None,
        }
    }
}
