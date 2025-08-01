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
pub struct DynamicConfigDto {
    #[serde(
        rename = "repoBaseDirectory",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub repo_base_directory: Option<Option<String>>,
    #[serde(
        rename = "dataBaseDirectory",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub data_base_directory: Option<Option<String>>,
    #[serde(
        rename = "logBaseDirectory",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub log_base_directory: Option<Option<String>>,
    #[serde(
        rename = "mountBaseDirectory",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub mount_base_directory: Option<Option<String>>,
    #[serde(
        rename = "serverSmtpUrl",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub server_smtp_url: Option<Option<String>>,
    #[serde(
        rename = "serverEmailAddress",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub server_email_address: Option<Option<String>>,
    #[serde(
        rename = "serverEmailPassword",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub server_email_password: Option<Option<String>>,
    #[serde(
        rename = "authMins",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub auth_mins: Option<Option<i32>>,
    #[serde(
        rename = "maxBytesPerUser",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_bytes_per_user: Option<Option<i64>>,
    #[serde(
        rename = "maxRequestsPerUser",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_requests_per_user: Option<Option<i64>>,
    #[serde(
        rename = "rateLimitResetMins",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub rate_limit_reset_mins: Option<Option<i64>>,
}

impl DynamicConfigDto {
    pub fn new() -> DynamicConfigDto {
        DynamicConfigDto {
            repo_base_directory: None,
            data_base_directory: None,
            log_base_directory: None,
            mount_base_directory: None,
            server_smtp_url: None,
            server_email_address: None,
            server_email_password: None,
            auth_mins: None,
            max_bytes_per_user: None,
            max_requests_per_user: None,
            rate_limit_reset_mins: None,
        }
    }
}
