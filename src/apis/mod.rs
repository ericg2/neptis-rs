use std::error;
use std::fmt;

#[derive(Debug)]
pub enum NeptisError {
    ApiRegular(reqwest::Error),
    Api(reqwest_middleware::Error),
    Serde(serde_json::Error),
    Io(std::io::Error),
    Sql(sqlx::Error),
    Str(String),
    Zip(zip::result::ZipError),
}

impl fmt::Display for NeptisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (module, e) = match self {
            NeptisError::ApiRegular(e) => ("reqwest", e.to_string()),
            NeptisError::Api(e) => ("reqwest-middle", e.to_string()),
            NeptisError::Serde(e) => ("serde", e.to_string()),
            NeptisError::Io(e) => ("IO", e.to_string()),
            NeptisError::Str(e) => ("custom", e.to_string()),
            NeptisError::Sql(e) => ("SQL", e.to_string()),
            NeptisError::Zip(e) => ("Zip", e.to_string()),
        };
        write!(f, "error in {}: {}", module, e)
    }
}

impl error::Error for NeptisError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(match self {
            NeptisError::ApiRegular(e) => e,
            NeptisError::Api(e) => e,
            NeptisError::Serde(e) => e,
            NeptisError::Io(e) => e,
            NeptisError::Sql(e) => e,
            NeptisError::Zip(e) => e,
            NeptisError::Str(_) => return None,
        })
    }
}

impl From<reqwest::Error> for NeptisError {
    fn from(e: reqwest::Error) -> Self {
        NeptisError::ApiRegular(e)
    }
}

impl From<reqwest_middleware::Error> for NeptisError {
    fn from(e: reqwest_middleware::Error) -> Self {
        NeptisError::Api(e)
    }
}

impl From<serde_json::Error> for NeptisError {
    fn from(e: serde_json::Error) -> Self {
        NeptisError::Serde(e)
    }
}

impl From<std::io::Error> for NeptisError {
    fn from(e: std::io::Error) -> Self {
        NeptisError::Io(e)
    }
}

impl From<sqlx::Error> for NeptisError {
    fn from(e: sqlx::Error) -> Self {
        NeptisError::Sql(e)
    }
}

impl From<zip::result::ZipError> for NeptisError {
    fn from(e: zip::result::ZipError) -> Self {
        NeptisError::Zip(e)
    }
}

pub fn urlencode<T: AsRef<str>>(s: T) -> String {
    ::url::form_urlencoded::byte_serialize(s.as_ref().as_bytes()).collect()
}

pub fn parse_deep_object(prefix: &str, value: &serde_json::Value) -> Vec<(String, String)> {
    if let serde_json::Value::Object(object) = value {
        let mut params = vec![];

        for (key, value) in object {
            match value {
                serde_json::Value::Object(_) => params.append(&mut parse_deep_object(
                    &format!("{}[{}]", prefix, key),
                    value,
                )),
                serde_json::Value::Array(array) => {
                    for (i, value) in array.iter().enumerate() {
                        params.append(&mut parse_deep_object(
                            &format!("{}[{}][{}]", prefix, key, i),
                            value,
                        ));
                    }
                }
                serde_json::Value::String(s) => {
                    params.push((format!("{}[{}]", prefix, key), s.clone()))
                }
                _ => params.push((format!("{}[{}]", prefix, key), value.to_string())),
            }
        }

        return params;
    }

    unimplemented!("Only objects are supported with style=deepObject")
}

pub mod api;
pub mod dtos;
pub mod prelude;