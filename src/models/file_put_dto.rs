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
pub struct FilePutDto {
    #[serde(
        rename = "path",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub path: Option<Option<String>>,
    #[serde(
        rename = "newPath",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub new_path: Option<Option<String>>,
    #[serde(
        rename = "base64Content",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub base64_content: Option<Option<String>>,
    #[serde(rename = "isDirectory", skip_serializing_if = "Option::is_none")]
    pub is_directory: Option<bool>,
    #[serde(
        rename = "copy",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub copy: Option<Option<bool>>,
}

impl FilePutDto {
    pub fn new() -> FilePutDto {
        FilePutDto {
            path: None,
            new_path: None,
            base64_content: None,
            is_directory: None,
            copy: None,
        }
    }
}
