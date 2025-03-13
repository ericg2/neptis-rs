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
pub struct FileDto {
    #[serde(rename = "accessDate", skip_serializing_if = "Option::is_none")]
    pub access_date: Option<String>,
    #[serde(rename = "modifyDate", skip_serializing_if = "Option::is_none")]
    pub modify_date: Option<String>,
    #[serde(rename = "createDate", skip_serializing_if = "Option::is_none")]
    pub create_date: Option<String>,
    #[serde(rename = "level", skip_serializing_if = "Option::is_none")]
    pub level: Option<i64>,

    #[serde(rename = "sizeBytes", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<Option<i64>>,
    #[serde(rename = "name", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub name: Option<Option<String>>,
    #[serde(rename = "path", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub path: Option<Option<String>>,

    #[serde(rename = "isDirectory", skip_serializing_if = "Option::is_none")]
    pub is_directory: Option<bool>,
}

impl FileDto {
    pub fn new() -> FileDto {
        FileDto {
            access_date: None,
            modify_date: None,
            create_date: None,
            size_bytes: None,
            name: None,
            path: None,
            is_directory: None,
            level: None
        }
    }
}

