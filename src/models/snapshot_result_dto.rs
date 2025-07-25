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
pub struct SnapshotResultDto {
    #[serde(rename = "backupStart", skip_serializing_if = "Option::is_none")]
    pub backup_start: Option<String>,
    #[serde(rename = "backupEnd", skip_serializing_if = "Option::is_none")]
    pub backup_end: Option<String>,
    #[serde(rename = "filesNew", skip_serializing_if = "Option::is_none")]
    pub files_new: Option<i32>,
    #[serde(rename = "filesChanged", skip_serializing_if = "Option::is_none")]
    pub files_changed: Option<i32>,
    #[serde(rename = "filesUnmodified", skip_serializing_if = "Option::is_none")]
    pub files_unmodified: Option<i32>,
    #[serde(rename = "dirsNew", skip_serializing_if = "Option::is_none")]
    pub dirs_new: Option<i32>,
    #[serde(rename = "dirsChanged", skip_serializing_if = "Option::is_none")]
    pub dirs_changed: Option<i32>,
    #[serde(rename = "dirsUnmodified", skip_serializing_if = "Option::is_none")]
    pub dirs_unmodified: Option<i32>,
    #[serde(rename = "dataBlobs", skip_serializing_if = "Option::is_none")]
    pub data_blobs: Option<i32>,
    #[serde(rename = "treeBlobs", skip_serializing_if = "Option::is_none")]
    pub tree_blobs: Option<i32>,
    #[serde(rename = "dataAdded", skip_serializing_if = "Option::is_none")]
    pub data_added: Option<i64>,
    #[serde(rename = "dataAddedPacked", skip_serializing_if = "Option::is_none")]
    pub data_added_packed: Option<i64>,
    #[serde(
        rename = "totalFilesProcessed",
        skip_serializing_if = "Option::is_none"
    )]
    pub total_files_processed: Option<i32>,
    #[serde(
        rename = "totalBytesProcessed",
        skip_serializing_if = "Option::is_none"
    )]
    pub total_bytes_processed: Option<i64>,
}

impl SnapshotResultDto {
    pub fn new() -> SnapshotResultDto {
        SnapshotResultDto {
            backup_start: None,
            backup_end: None,
            files_new: None,
            files_changed: None,
            files_unmodified: None,
            dirs_new: None,
            dirs_changed: None,
            dirs_unmodified: None,
            data_blobs: None,
            tree_blobs: None,
            data_added: None,
            data_added_packed: None,
            total_files_processed: None,
            total_bytes_processed: None,
        }
    }
}
