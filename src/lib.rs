#![allow(unused_imports)]
#![allow(clippy::too_many_arguments)]

extern crate core;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate serde_repr;
extern crate url;

pub mod apis;
pub mod models;
pub mod rolling_secret;

#[cfg(test)]
mod tests {
    use crate::apis::auth_api;
    use crate::apis::configuration::Configuration;
    use crate::models::AuthInputDto;
    use crate::rolling_secret::RollingSecret;
    use std::sync::Arc;

    static KEY: &'static str = "RmLqwCNncT5y/1ChQrg+AiKFDCzSsBhDkE4J55fLXignoriA3ey4epVeZYzHMakRukPthHsQuCWYM88smJ015g==§T3dTS0kRbjfV0XxhZYMQiHMHn0tpKb0uY0lc9PbVE+de1eX6wM+x8EVzahsZdWS0ze5a0APy6XD0ylkYnoF4jQ==§Lr2p9vU^NzIPIycW";

    #[tokio::test]
    pub async fn test_key() {
        let secret = RollingSecret::from_string(KEY).expect("Failed to generate secret!");
        let a_key = secret
            .rolling_key()
            .expect("Failed to generate the first key!");
        let b_key = secret
            .rolling_key()
            .expect("Failed to generate the second key!");
        assert_eq!(a_key, b_key);
    }

    #[tokio::test]
    pub async fn test_secret() {
        let secret = RollingSecret::from_string(KEY).expect("Failed to generate secret!");
        let b_hello = b"Hello World!".to_vec();

        let b_enc = secret
            .encrypt(b_hello.as_slice())
            .expect("Failed to encrypt!");
        let b_dec = secret
            .decrypt(b_enc.as_slice())
            .expect("Failed to decrypt!");

        assert_eq!(b_hello, b_dec, "Decryption does not match!");

        println!("{:?}", secret.rolling_key().expect("Should be good!"));
    }

    #[tokio::test]
    pub async fn try_auth() {
        let secret = RollingSecret::from_string(KEY).expect("Failed to generate secret!");
        let mut config = Configuration::default();
        let mut u_dto = AuthInputDto::new();
        u_dto.user_name = Some(Some("admin".into()));
        u_dto.password = Some(Some("tmLBtjejlYTdA0mk".into()));
        config.base_path = "http://127.0.0.1:5000".into();
        config.secret = Some(secret);
        let dto = auth_api::authenticate(&config, Some(u_dto))
            .await
            .expect("Failed to authenticate!");
        assert!(dto.token.is_some(), "Failed to pull token!");
    }
}
