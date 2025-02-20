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
pub struct LogItemDto {
    #[serde(rename = "id", skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[serde(rename = "logDate", skip_serializing_if = "Option::is_none")]
    pub log_date: Option<String>,
    #[serde(rename = "category", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub category: Option<Option<String>>,
    #[serde(rename = "message", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub message: Option<Option<String>>,
    #[serde(rename = "className", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub class_name: Option<Option<String>>,
    #[serde(rename = "userName", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub user_name: Option<Option<String>>,
}

impl LogItemDto {
    pub fn new() -> LogItemDto {
        LogItemDto {
            id: None,
            log_date: None,
            category: None,
            message: None,
            class_name: None,
            user_name: None,
        }
    }
}

