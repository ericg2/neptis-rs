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
pub struct JobRestoreDto {
    #[serde(rename = "fromRepo", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub from_repo: Option<Option<String>>,
    #[serde(rename = "toData", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub to_data: Option<Option<String>>,
}

impl JobRestoreDto {
    pub fn new() -> JobRestoreDto {
        JobRestoreDto {
            from_repo: None,
            to_data: None,
        }
    }
}

