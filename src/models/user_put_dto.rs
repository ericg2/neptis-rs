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
pub struct UserPutDto {
    #[serde(
        rename = "emailAddress",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub email_address: Option<Option<String>>,
    #[serde(
        rename = "isPrivate",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub is_private: Option<Option<bool>>,
    #[serde(
        rename = "maxDataBytes",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_data_bytes: Option<Option<i64>>,
    #[serde(
        rename = "maxRepoBytes",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_repo_bytes: Option<Option<i64>>,
    #[serde(
        rename = "password",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub password: Option<Option<String>>,
}

impl UserPutDto {
    pub fn new() -> UserPutDto {
        UserPutDto {
            email_address: None,
            is_private: None,
            max_data_bytes: None,
            max_repo_bytes: None,
            password: None,
        }
    }
}
