#![allow(unused_imports)]
#![allow(clippy::too_many_arguments)]

extern crate serde_repr;
extern crate serde;
extern crate serde_json;
extern crate url;
extern crate reqwest;
extern crate core;

pub mod apis;
pub mod models;
mod rolling_secret;

#[cfg(test)]
mod tests {
    use crate::apis::configuration::Configuration;
    use crate::rolling_secret::RollingSecret;
    use crate::apis::auth_api;
    use crate::models::AuthInputDto;

    #[tokio::test]
    pub async fn test_auth() {
        let mut config = Configuration::new();
        let secret = RollingSecret::from_string("RmLqwCNncT5y/1ChQrg+AiKFDCzSsBhDkE4J55fLXignoriA3ey4epVeZYzHMakRukPthHsQuCWYM88smJ015g==§T3dTS0kRbjfV0XxhZYMQiHMHn0tpKb0uY0lc9PbVE+de1eX6wM+x8EVzahsZdWS0ze5a0APy6XD0ylkYnoF4jQ==§Lr2p9vU^NzIPIycW".into());
        let u_secret = secret.expect("The secret failed to load!");

        let mut u_dto = AuthInputDto::new();
        u_dto.user_name = Some(Some("admin".into()));
        u_dto.password = Some(Some("tmLBtjejlYTdA0mk".into()));

        config.base_path = "http://192.168.1.159".into();
        config.secret = Some(u_secret);

        let dto = auth_api::authenticate(&config, Some(u_dto)).await.expect("Failed to authenticate!");
        assert!(dto.token.is_some(), "Failed to pull token!");
    }
}