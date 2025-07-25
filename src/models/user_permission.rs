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
pub struct UserPermission {
    #[serde(
        rename = "userName",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub user_name: Option<Option<String>>,
    #[serde(
        rename = "permission",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub permission: Option<Option<String>>,
}

impl UserPermission {
    pub fn new() -> UserPermission {
        UserPermission {
            user_name: None,
            permission: None,
        }
    }
}
