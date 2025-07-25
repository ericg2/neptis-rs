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
pub struct WsConfigPutDto {
    #[serde(rename = "uri")]
    pub uri: String,
    #[serde(
        rename = "method",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub method: Option<Option<String>>,
    #[serde(
        rename = "subscriptions",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub subscriptions: Option<Option<Vec<String>>>,
}

impl WsConfigPutDto {
    pub fn new(uri: String) -> WsConfigPutDto {
        WsConfigPutDto {
            uri,
            method: None,
            subscriptions: None,
        }
    }
}
