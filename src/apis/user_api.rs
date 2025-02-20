/*
 * Neptis
 *
 * No description provided (generated by Openapi Generator https://github.com/openapitools/openapi-generator)
 *
 * The version of the OpenAPI document: v1
 * 
 * Generated by: https://openapi-generator.tech
 */


use reqwest;
use serde::{Deserialize, Serialize};
use crate::{apis::ResponseContent, models};
use super::{Error, configuration};


/// struct for typed errors of method [`create_one_user`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CreateOneUserError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`delete_one_user`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeleteOneUserError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`disable_one_permission_for_user`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DisableOnePermissionForUserError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`enable_one_permission_for_user`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EnableOnePermissionForUserError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`get_all_permissions_for_user`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetAllPermissionsForUserError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`get_all_users`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetAllUsersError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`get_one_permission_for_user`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetOnePermissionForUserError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`get_one_user`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetOneUserError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`update_all_permissions_for_user`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UpdateAllPermissionsForUserError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`update_one_user`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UpdateOneUserError {
    UnknownValue(serde_json::Value),
}


pub async fn create_one_user(configuration: &configuration::Configuration, user_create_dto: Option<models::UserCreateDto>) -> Result<models::UserSummaryDto, Error<CreateOneUserError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_user_create_dto = user_create_dto;

    let uri_str = format!("{}/api/users", configuration.base_path);
    let mut req_builder = configuration.client.request(reqwest::Method::POST, &uri_str);

    if let Some(ref user_agent) = configuration.user_agent {
        req_builder = req_builder.header(reqwest::header::USER_AGENT, user_agent.clone());
    }
    if let Some(ref token) = configuration.bearer_access_token {
        req_builder = req_builder.bearer_auth(token.to_owned());
    };
    req_builder = req_builder.json(&p_user_create_dto);

    let req = req_builder.build()?;
    let resp = configuration.client.execute(req).await?;

    let status = resp.status();

    if !status.is_client_error() && !status.is_server_error() {
        let content = resp.text().await?;
        serde_json::from_str(&content).map_err(Error::from)
    } else {
        let content = resp.text().await?;
        let entity: Option<CreateOneUserError> = serde_json::from_str(&content).ok();
        Err(Error::ResponseError(ResponseContent { status, content, entity }))
    }
}

pub async fn delete_one_user(configuration: &configuration::Configuration, user_name: Option<&str>) -> Result<bool, Error<DeleteOneUserError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_user_name = user_name;

    let uri_str = format!("{}/api/users", configuration.base_path);
    let mut req_builder = configuration.client.request(reqwest::Method::DELETE, &uri_str);

    if let Some(ref param_value) = p_user_name {
        req_builder = req_builder.query(&[("userName", &param_value.to_string())]);
    }
    if let Some(ref user_agent) = configuration.user_agent {
        req_builder = req_builder.header(reqwest::header::USER_AGENT, user_agent.clone());
    }
    if let Some(ref token) = configuration.bearer_access_token {
        req_builder = req_builder.bearer_auth(token.to_owned());
    };

    let req = req_builder.build()?;
    let resp = configuration.client.execute(req).await?;

    let status = resp.status();

    if !status.is_client_error() && !status.is_server_error() {
        let content = resp.text().await?;
        serde_json::from_str(&content).map_err(Error::from)
    } else {
        let content = resp.text().await?;
        let entity: Option<DeleteOneUserError> = serde_json::from_str(&content).ok();
        Err(Error::ResponseError(ResponseContent { status, content, entity }))
    }
}

pub async fn disable_one_permission_for_user(configuration: &configuration::Configuration, user_name: &str, permission_name: &str) -> Result<bool, Error<DisableOnePermissionForUserError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_user_name = user_name;
    let p_permission_name = permission_name;

    let uri_str = format!("{}/api/users/{userName}/perms/{permissionName}", configuration.base_path, userName=crate::apis::urlencode(p_user_name), permissionName=crate::apis::urlencode(p_permission_name));
    let mut req_builder = configuration.client.request(reqwest::Method::DELETE, &uri_str);

    if let Some(ref user_agent) = configuration.user_agent {
        req_builder = req_builder.header(reqwest::header::USER_AGENT, user_agent.clone());
    }
    if let Some(ref token) = configuration.bearer_access_token {
        req_builder = req_builder.bearer_auth(token.to_owned());
    };

    let req = req_builder.build()?;
    let resp = configuration.client.execute(req).await?;

    let status = resp.status();

    if !status.is_client_error() && !status.is_server_error() {
        let content = resp.text().await?;
        serde_json::from_str(&content).map_err(Error::from)
    } else {
        let content = resp.text().await?;
        let entity: Option<DisableOnePermissionForUserError> = serde_json::from_str(&content).ok();
        Err(Error::ResponseError(ResponseContent { status, content, entity }))
    }
}

pub async fn enable_one_permission_for_user(configuration: &configuration::Configuration, user_name: &str, permission_name: &str) -> Result<bool, Error<EnableOnePermissionForUserError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_user_name = user_name;
    let p_permission_name = permission_name;

    let uri_str = format!("{}/api/users/{userName}/perms/{permissionName}", configuration.base_path, userName=crate::apis::urlencode(p_user_name), permissionName=crate::apis::urlencode(p_permission_name));
    let mut req_builder = configuration.client.request(reqwest::Method::POST, &uri_str);

    if let Some(ref user_agent) = configuration.user_agent {
        req_builder = req_builder.header(reqwest::header::USER_AGENT, user_agent.clone());
    }
    if let Some(ref token) = configuration.bearer_access_token {
        req_builder = req_builder.bearer_auth(token.to_owned());
    };

    let req = req_builder.build()?;
    let resp = configuration.client.execute(req).await?;

    let status = resp.status();

    if !status.is_client_error() && !status.is_server_error() {
        let content = resp.text().await?;
        serde_json::from_str(&content).map_err(Error::from)
    } else {
        let content = resp.text().await?;
        let entity: Option<EnableOnePermissionForUserError> = serde_json::from_str(&content).ok();
        Err(Error::ResponseError(ResponseContent { status, content, entity }))
    }
}

pub async fn get_all_permissions_for_user(configuration: &configuration::Configuration, user_name: &str) -> Result<Vec<models::UserPermission>, Error<GetAllPermissionsForUserError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_user_name = user_name;

    let uri_str = format!("{}/api/users/{userName}/perms", configuration.base_path, userName=crate::apis::urlencode(p_user_name));
    let mut req_builder = configuration.client.request(reqwest::Method::GET, &uri_str);

    if let Some(ref user_agent) = configuration.user_agent {
        req_builder = req_builder.header(reqwest::header::USER_AGENT, user_agent.clone());
    }
    if let Some(ref token) = configuration.bearer_access_token {
        req_builder = req_builder.bearer_auth(token.to_owned());
    };

    let req = req_builder.build()?;
    let resp = configuration.client.execute(req).await?;

    let status = resp.status();

    if !status.is_client_error() && !status.is_server_error() {
        let content = resp.text().await?;
        serde_json::from_str(&content).map_err(Error::from)
    } else {
        let content = resp.text().await?;
        let entity: Option<GetAllPermissionsForUserError> = serde_json::from_str(&content).ok();
        Err(Error::ResponseError(ResponseContent { status, content, entity }))
    }
}

pub async fn get_all_users(configuration: &configuration::Configuration, ) -> Result<Vec<models::UserSummaryDto>, Error<GetAllUsersError>> {

    let uri_str = format!("{}/api/users", configuration.base_path);
    let mut req_builder = configuration.client.request(reqwest::Method::GET, &uri_str);

    if let Some(ref user_agent) = configuration.user_agent {
        req_builder = req_builder.header(reqwest::header::USER_AGENT, user_agent.clone());
    }
    if let Some(ref token) = configuration.bearer_access_token {
        req_builder = req_builder.bearer_auth(token.to_owned());
    };

    let req = req_builder.build()?;
    let resp = configuration.client.execute(req).await?;

    let status = resp.status();

    if !status.is_client_error() && !status.is_server_error() {
        let content = resp.text().await?;
        serde_json::from_str(&content).map_err(Error::from)
    } else {
        let content = resp.text().await?;
        let entity: Option<GetAllUsersError> = serde_json::from_str(&content).ok();
        Err(Error::ResponseError(ResponseContent { status, content, entity }))
    }
}

pub async fn get_one_permission_for_user(configuration: &configuration::Configuration, user_name: &str, permission_name: &str) -> Result<models::UserPermission, Error<GetOnePermissionForUserError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_user_name = user_name;
    let p_permission_name = permission_name;

    let uri_str = format!("{}/api/users/{userName}/perms/{permissionName}", configuration.base_path, userName=crate::apis::urlencode(p_user_name), permissionName=crate::apis::urlencode(p_permission_name));
    let mut req_builder = configuration.client.request(reqwest::Method::GET, &uri_str);

    if let Some(ref user_agent) = configuration.user_agent {
        req_builder = req_builder.header(reqwest::header::USER_AGENT, user_agent.clone());
    }
    if let Some(ref token) = configuration.bearer_access_token {
        req_builder = req_builder.bearer_auth(token.to_owned());
    };

    let req = req_builder.build()?;
    let resp = configuration.client.execute(req).await?;

    let status = resp.status();

    if !status.is_client_error() && !status.is_server_error() {
        let content = resp.text().await?;
        serde_json::from_str(&content).map_err(Error::from)
    } else {
        let content = resp.text().await?;
        let entity: Option<GetOnePermissionForUserError> = serde_json::from_str(&content).ok();
        Err(Error::ResponseError(ResponseContent { status, content, entity }))
    }
}

pub async fn get_one_user(configuration: &configuration::Configuration, user_name: &str) -> Result<models::UserSummaryDto, Error<GetOneUserError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_user_name = user_name;

    let uri_str = format!("{}/api/users/{userName}", configuration.base_path, userName=crate::apis::urlencode(p_user_name));
    let mut req_builder = configuration.client.request(reqwest::Method::GET, &uri_str);

    if let Some(ref user_agent) = configuration.user_agent {
        req_builder = req_builder.header(reqwest::header::USER_AGENT, user_agent.clone());
    }
    if let Some(ref token) = configuration.bearer_access_token {
        req_builder = req_builder.bearer_auth(token.to_owned());
    };

    let req = req_builder.build()?;
    let resp = configuration.client.execute(req).await?;

    let status = resp.status();

    if !status.is_client_error() && !status.is_server_error() {
        let content = resp.text().await?;
        serde_json::from_str(&content).map_err(Error::from)
    } else {
        let content = resp.text().await?;
        let entity: Option<GetOneUserError> = serde_json::from_str(&content).ok();
        Err(Error::ResponseError(ResponseContent { status, content, entity }))
    }
}

pub async fn update_all_permissions_for_user(configuration: &configuration::Configuration, user_name: &str, user_permission_dto: Option<Vec<models::UserPermissionDto>>) -> Result<Vec<models::UserPermission>, Error<UpdateAllPermissionsForUserError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_user_name = user_name;
    let p_user_permission_dto = user_permission_dto;

    let uri_str = format!("{}/api/users/{userName}/perms", configuration.base_path, userName=crate::apis::urlencode(p_user_name));
    let mut req_builder = configuration.client.request(reqwest::Method::PUT, &uri_str);

    if let Some(ref user_agent) = configuration.user_agent {
        req_builder = req_builder.header(reqwest::header::USER_AGENT, user_agent.clone());
    }
    if let Some(ref token) = configuration.bearer_access_token {
        req_builder = req_builder.bearer_auth(token.to_owned());
    };
    req_builder = req_builder.json(&p_user_permission_dto);

    let req = req_builder.build()?;
    let resp = configuration.client.execute(req).await?;

    let status = resp.status();

    if !status.is_client_error() && !status.is_server_error() {
        let content = resp.text().await?;
        serde_json::from_str(&content).map_err(Error::from)
    } else {
        let content = resp.text().await?;
        let entity: Option<UpdateAllPermissionsForUserError> = serde_json::from_str(&content).ok();
        Err(Error::ResponseError(ResponseContent { status, content, entity }))
    }
}

pub async fn update_one_user(configuration: &configuration::Configuration, user_name: &str, user_put_dto: Option<models::UserPutDto>) -> Result<models::UserSummaryDto, Error<UpdateOneUserError>> {
    // add a prefix to parameters to efficiently prevent name collisions
    let p_user_name = user_name;
    let p_user_put_dto = user_put_dto;

    let uri_str = format!("{}/api/users/{userName}", configuration.base_path, userName=crate::apis::urlencode(p_user_name));
    let mut req_builder = configuration.client.request(reqwest::Method::PUT, &uri_str);

    if let Some(ref user_agent) = configuration.user_agent {
        req_builder = req_builder.header(reqwest::header::USER_AGENT, user_agent.clone());
    }
    if let Some(ref token) = configuration.bearer_access_token {
        req_builder = req_builder.bearer_auth(token.to_owned());
    };
    req_builder = req_builder.json(&p_user_put_dto);

    let req = req_builder.build()?;
    let resp = configuration.client.execute(req).await?;

    let status = resp.status();

    if !status.is_client_error() && !status.is_server_error() {
        let content = resp.text().await?;
        serde_json::from_str(&content).map_err(Error::from)
    } else {
        let content = resp.text().await?;
        let entity: Option<UpdateOneUserError> = serde_json::from_str(&content).ok();
        Err(Error::ResponseError(ResponseContent { status, content, entity }))
    }
}

