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
pub struct SnapshotDto {
    #[serde(
        rename = "snapshotId",
        default,
        with = "::serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_id: Option<Option<String>>,
    #[serde(rename = "summary", skip_serializing_if = "Option::is_none")]
    pub summary: Option<Box<models::SnapshotResultDto>>,
}

impl SnapshotDto {
    pub fn new() -> SnapshotDto {
        SnapshotDto {
            snapshot_id: None,
            summary: None,
        }
    }
}
