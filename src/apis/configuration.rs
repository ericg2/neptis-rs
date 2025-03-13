/*
 * Neptis
 *
 * No description provided (generated by Openapi Generator https://github.com/openapitools/openapi-generator)
 *
 * The version of the OpenAPI document: v1
 *
 * Generated by: https://openapi-generator.tech
 */
use crate::apis::config_api::UpdateGlobalConfigError;
use crate::apis::notification_api::GetAllNotificationConfigsError;
use crate::apis::{Error, ResponseContent};
use crate::rolling_secret::RollingSecret;
use base64::engine::Config;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::{Body, Client, IntoUrl, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Configuration {
    pub base_path: String,
    pub user_agent: Option<String>,
    pub client: Client,
    pub secret: Option<RollingSecret>,
    pub basic_auth: Option<BasicAuth>,
    pub oauth_access_token: Option<String>,
    pub bearer_access_token: Option<String>,
    pub api_key: Option<ApiKey>,
}

pub type BasicAuth = (String, Option<String>);

#[derive(Debug, Clone)]
pub struct ApiKey {
    pub prefix: Option<String>,
    pub key: String,
}

pub struct ApiBuilder<'a, U: IntoUrl> {
    config: &'a Configuration,
    method: reqwest::Method,
    full_uri: U,
    body: Option<serde_json::Value>,
    queries: Vec<(String, String)>,
}

impl<'a, U: IntoUrl> ApiBuilder<'a, U> {
    pub fn new(config: &'a Configuration, method: reqwest::Method, full_uri: U) -> Self {
        ApiBuilder::<'a, U> {
            config,
            method,
            full_uri,
            body: None,
            queries: vec![],
        }
    }

    pub fn with_body<T: Serialize>(mut self, body: T) -> Self {
        self.body = Some(serde_json::to_value(body).expect("Failed to serialize body"));
        self
    }
    pub fn with_query<T: Serialize, E>(self, key: &str, val: T) -> Result<Self, Error<E>> {
        Self::with_opt_query(self, key, Some(val))
    }

    pub fn with_opt_query<T: Serialize, E>(
        mut self,
        key: &str,
        val: Option<T>,
    ) -> Result<Self, Error<E>> {
        if let Some(v) = val {
            self.queries
                .push((key.to_string(), serde_json::to_string(&v)?));
        }
        Ok(self)
    }

    pub async fn execute<JsonOut, E>(&self) -> Result<JsonOut, Error<E>>
    where
        JsonOut: DeserializeOwned,
        E: DeserializeOwned,
    {
        // First, we need to create the request.
        let mut final_url: String = self.full_uri.as_str().to_string();
        let mut final_body = self
            .body
            .as_ref()
            .map(|x| serde_json::to_vec(&x))
            .transpose()?;

        if let Some(ref secret) = self.config.secret {
            let mut full_query = final_url.replace(self.config.base_path.as_str(), "".into());
            full_query = full_query
                .strip_prefix("/")
                .unwrap_or(full_query.as_str())
                .to_string();
            full_query = full_query
                .strip_prefix("/api")
                .unwrap_or(full_query.as_str())
                .to_string();
            full_query = full_query
                .strip_prefix("api/")
                .unwrap_or(full_query.as_str())
                .to_string();

            if !full_query.starts_with("/api/") {
                full_query = "/api/".to_string() + full_query.as_str();
            }

            // Finally, encrypt the data into the "secure api"
            let enc_query = secret
                .encrypt(full_query.as_bytes())
                .map(|x| STANDARD.encode(x))
                .ok_or(Error::Str("Failed to encrypt query".into()))?;

            let mut enc_url = self.config.base_path.replace("/api", "");
            enc_url = enc_url
                .strip_suffix("/")
                .unwrap_or(enc_url.as_str())
                .to_string();
            enc_url += format!("/secure/{}", enc_query).as_str();

            if let Some(body) = final_body {
                // There is something in the body - we need to encrypt it as well.
                final_body = Some(
                    secret
                        .encrypt(body.as_slice())
                        .map(|x| STANDARD.encode(x).as_bytes().to_vec())
                        .ok_or(Error::Str("Failed to encrypt body!".into()))?,
                );
            }
            final_url = enc_url
        }

        // Finally, build the request and process.
        let mut req_builder = self.config.client.request(self.method.clone(), final_url);
        // if let Some(u_queries) = self.queries {
        //     req_builder = req_builder.query(u_queries);
        // }
        for (k, v) in self.queries.iter() {
            req_builder = req_builder.query(&[(k.to_owned(), v.to_owned())]);
        }

        if let Some(ref user_agent) = self.config.user_agent {
            req_builder = req_builder.header(reqwest::header::USER_AGENT, user_agent.clone());
        }
        if let Some(ref token) = self.config.bearer_access_token {
            req_builder = req_builder.bearer_auth(token.to_owned());
        };
        if let Some(body) = final_body {
            req_builder = req_builder.body(body);
            req_builder = req_builder.header("Content-Type", "application/json");
        }

        let req = req_builder.build()?;
        let res = self.config.client.execute(req).await?;
        let status = res.status().clone();

        let mut res_body = res
            .bytes()
            .await
            .ok()
            .map(|x| x.to_vec())
            .ok_or(Error::Str("Failed to pull body".into()))?;

        if let Some(ref secret) = self.config.secret {
            // We need to decode the body from base64.
            let p_body = STANDARD
                .decode(res_body.as_slice())
                .map_err(|_| Error::Str("Failed to decode!".into()))?;
            res_body = secret
                .decrypt(p_body.as_slice())
                .ok_or(Error::Str("Failed to decrypt body!".into()))?;
        }

        if !status.is_client_error() && !status.is_server_error() {
            // We need to convert the output to JSON and return.
            let json_res: JsonOut = serde_json::from_slice(res_body.as_slice())?;
            Ok(json_res)
        } else {
            let entity: Option<E> = serde_json::from_slice(res_body.as_slice()).ok();
            Err(Error::ResponseError(ResponseContent {
                status,
                content: String::from_utf8(res_body).unwrap_or(String::new()),
                entity,
            }))
        }
    }
}

impl Configuration {
    pub fn new() -> Configuration {
        Configuration::default()
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration {
            base_path: "http://localhost".to_owned(),
            user_agent: Some("OpenAPI-Generator/v1/rust".to_owned()),
            client: Client::new(),
            secret: None,
            basic_auth: None,
            oauth_access_token: None,
            bearer_access_token: None,
            api_key: None,
        }
    }
}
