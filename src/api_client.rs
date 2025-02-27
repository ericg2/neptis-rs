use crate::apis::Error::Reqwest;
use crate::apis::ResponseContent;
use crate::rolling_secret::RollingSecret;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use rand::distr::uniform::Error;
use reqwest::{Client, ClientBuilder, IntoUrl, Method, Request, RequestBuilder, Response, StatusCode};
use crate::apis::configuration::Configuration;
// Modify the request builder to modify

#[derive(Clone, Debug)]
pub struct EncryptedClient {
    client: Client,
    base_path: String,
    secret: Option<RollingSecret>,
}

impl EncryptedClient {
    pub fn new(base_path: &str, secret: Option<RollingSecret>) -> EncryptedClient {
        EncryptedClient {
            base_path: base_path.to_string(),
            secret,
            client: Client::builder().build().unwrap()
        }
    }

    pub fn request<U: IntoUrl>(&self, method: Method, url: U) -> RequestBuilder {
        self.client.request(method, url)
    }

    pub async fn execute<U>(&self, config: &Configuration) -> Result<Response, crate::apis::Error<U>> {
        fn to_response_err(message: &str) -> ResponseContent<U> {
            ResponseContent {
                status: StatusCode::BAD_REQUEST,
                content: message.to_string(),
                entity: None,
            }
        }
        match &self.secret {
            None => self.client.execute(request.build()?).await.map_err(|x| Reqwest(x)),
            Some(secret) => {
                // Otherwise, we need to convert to a "secure system"
                let mut full_query = request
                    .as_str()
                    .to_string()
                    .replace(self.base_path.as_str(), "".into());

                full_query = full_query
                    .strip_prefix("/")
                    .unwrap_or(full_query.as_str())
                    .to_string();

                if !full_query.starts_with("/api/") {
                    full_query = "/api/".to_string() + full_query.as_str();
                }

                // Finally, encrypt the data into the "secure api"
                let enc_query = secret
                    .encrypt(full_query.as_bytes())
                    .map(|x| STANDARD.encode(x))
                    .ok_or("Failed to encrypt query".into())?;

                let mut enc_url = self.base_path.replace("/api", "");
                enc_url = enc_url
                    .strip_suffix("/")
                    .unwrap_or(enc_url.as_str())
                    .to_string();
                enc_url += concat!("/secure/", enc_query); // append the secure ending

                let mut enc_body = None;
                match request.body().map(|x| x.as_bytes()) {
                    None => enc_body = None,
                    Some(bytes) => {
                        // A second statement is required, since `bytes` may be a stream.
                        if let Some(body_bytes) = bytes {
                            enc_body = Some(
                                secret
                                    .encrypt(body_bytes)
                                    .ok_or("Failed to encrypt body!".into())?,
                            );
                        }
                    }
                }

                // Finally, create the build and send the request.
                let mut enc_builder = self
                    .client
                    .request(request.method().clone(), enc_url)
                    .headers(request.headers().clone());
                if let Some(b) = enc_body {
                    enc_builder = enc_builder.body(b);
                }

                let enc_req = enc_builder.build()?;
                let enc_res = self.client.execute(enc_req).await?;
                let mut res_bytes = None;

                if let Some(enc_bytes) = enc_res.bytes().await.ok() {
                    // Attempt to decrypt the response bytes.
                    res_bytes = Some(
                        secret
                            .decrypt(&enc_bytes.to_vec()[..])
                            .ok_or("Failed to decrypt body!".into())?,
                    );
                }
            }
        }
    }
}
