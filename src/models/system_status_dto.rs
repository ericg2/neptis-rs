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
pub struct SystemStatusDto {
    #[serde(rename = "cpus", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub cpus: Option<Option<Vec<models::CpuItemDto>>>,
    #[serde(rename = "errors", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub errors: Option<Option<Vec<models::Error>>>,
    #[serde(rename = "apiUptime", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub api_uptime: Option<Option<String>>,
    #[serde(rename = "systemUptime", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub system_uptime: Option<Option<String>>,
    #[serde(rename = "apiVersion", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub api_version: Option<Option<String>>,
    #[serde(rename = "kernelVersion", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub kernel_version: Option<Option<String>>,
    #[serde(rename = "totalUsersRegistered", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub total_users_registered: Option<Option<i64>>,
    #[serde(rename = "totalDataPoints", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub total_data_points: Option<Option<i64>>,
    #[serde(rename = "totalRepoPoints", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub total_repo_points: Option<Option<i64>>,
    #[serde(rename = "maxDataBytes", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub max_data_bytes: Option<Option<i64>>,
    #[serde(rename = "maxRepoBytes", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub max_repo_bytes: Option<Option<i64>>,
    #[serde(rename = "allocatedDataBytes", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub allocated_data_bytes: Option<Option<i64>>,
    #[serde(rename = "allocatedRepoBytes", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub allocated_repo_bytes: Option<Option<i64>>,
    #[serde(rename = "totalJobs", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub total_jobs: Option<Option<i64>>,
    #[serde(rename = "totalRunningJobs", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub total_running_jobs: Option<Option<i64>>,
    #[serde(rename = "freeDataBytes", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub free_data_bytes: Option<Option<i64>>,
    #[serde(rename = "freeRepoBytes", default, with = "::serde_with::rust::double_option", skip_serializing_if = "Option::is_none")]
    pub free_repo_bytes: Option<Option<i64>>,
    #[serde(rename = "averageCpu", skip_serializing_if = "Option::is_none")]
    pub average_cpu: Option<Box<models::CpuItemDto>>,
}

impl SystemStatusDto {
    pub fn new() -> SystemStatusDto {
        SystemStatusDto {
            cpus: None,
            errors: None,
            api_uptime: None,
            system_uptime: None,
            api_version: None,
            kernel_version: None,
            total_users_registered: None,
            total_data_points: None,
            total_repo_points: None,
            max_data_bytes: None,
            max_repo_bytes: None,
            allocated_data_bytes: None,
            allocated_repo_bytes: None,
            total_jobs: None,
            total_running_jobs: None,
            free_data_bytes: None,
            free_repo_bytes: None,
            average_cpu: None,
        }
    }
}

