/*
 * Neptis
 *
 * No description provided (generated by Openapi Generator https://github.com/openapitools/openapi-generator)
 *
 * The version of the OpenAPI document: v1
 *
 * Generated by: https://openapi-generator.tech
 */

use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct MessageReadItem {
    #[serde(rename = "messageId", skip_serializing_if = "Option::is_none")]
    pub message_id: Option<uuid::Uuid>,
    #[serde(
        rename = "readBy",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub read_by: Option<Option<String>>,
    #[serde(rename = "readDate", skip_serializing_if = "Option::is_none")]
    pub read_date: Option<String>,
}

impl MessageReadItem {
    pub fn new() -> MessageReadItem {
        MessageReadItem {
            message_id: None,
            read_by: None,
            read_date: None,
        }
    }
}
