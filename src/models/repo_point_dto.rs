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
pub struct RepoPointDto {
    #[serde(rename = "userName")]
    pub user_name: String,
    #[serde(rename = "pointName")]
    pub point_name: String,
    #[serde(rename = "pointId", skip_serializing_if = "Option::is_none")]
    pub point_id: Option<uuid::Uuid>,
    #[serde(rename = "maxBytes", skip_serializing_if = "Option::is_none")]
    pub max_bytes: Option<i64>,
    #[serde(rename = "usedBytes", skip_serializing_if = "Option::is_none")]
    pub used_bytes: Option<i64>,
    #[serde(rename = "freeBytes", skip_serializing_if = "Option::is_none")]
    pub free_bytes: Option<i64>,
    #[serde(rename = "lastAccessed", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub last_accessed: Option<Option<String>>,
    #[serde(rename = "isRepository", skip_serializing_if = "Option::is_none")]
    pub is_repository: Option<bool>,
}

impl RepoPointDto {
    pub fn new(user_name: String, point_name: String) -> RepoPointDto {
        RepoPointDto {
            user_name,
            point_name,
            point_id: None,
            max_bytes: None,
            used_bytes: None,
            free_bytes: None,
            last_accessed: None,
            is_repository: None,
        }
    }
}

