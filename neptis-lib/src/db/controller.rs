use std::sync::Arc;

use sqlx::{
    Sqlite, SqlitePool,
    migrate::MigrateDatabase,
    prelude::*,
    sqlite::{SqlitePoolOptions, SqliteQueryResult},
};
use tokio::runtime::Runtime;
use uuid::Uuid;

use super::server::ServerItem;
use crate::db::transfer::AutoTransfer;

pub struct DbController {
    rt: Arc<Runtime>,
    pool: SqlitePool,
}

impl DbController {
    pub async fn save_server(&self, server: &ServerItem) -> Result<(), sqlx::Error> {
        // Run an update if we need to first
        if sqlx::query!(
            r#"
            UPDATE server_items
            SET
                server_endpoint = ?,
                server_password = ?,
                user_name = ?,
                user_password = ?,
                arduino_endpoint = ?,
                auto_fuse = ?,
                is_default = ?
            WHERE
                server_name = ?
            "#,
            server.server_endpoint,
            server.server_password,
            server.user_name,
            server.user_password,
            server.arduino_endpoint,
            server.auto_fuse,
            server.is_default,
            server.server_name,
        )
        .execute(&self.pool)
        .await
        .map(|x| x.rows_affected())?
            <= 0
        {
            sqlx::query!(
                r#"
                INSERT INTO server_items (
                    server_name,
                    server_endpoint,
                    server_password,
                    user_name,
                    user_password,
                    arduino_endpoint,
                    arduino_password,
                    auto_fuse,
                    is_default
                ) VALUES (
                    ?,
                    ?,
                    ?,
                    ?,
                    ?,
                    ?,
                    ?,
                    ?,
                    ?
                )"#,
                server.server_name,
                server.server_endpoint,
                server.server_password,
                server.user_name,
                server.user_password,
                server.arduino_endpoint,
                server.arduino_password,
                server.auto_fuse,
                server.is_default
            )
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn save_servers(&self, servers: &[ServerItem]) -> Result<(), sqlx::Error> {
        let tx = self.pool.begin().await?;
        for server in servers {
            self.save_server(&server).await?;
        }
        tx.commit().await
    }

    pub async fn overwrite_servers(&self, servers: &[ServerItem]) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM server_items")
            .execute(&self.pool)
            .await?;
        self.save_servers(servers).await
    }

    pub fn overwrite_servers_sync(&self, servers: &[ServerItem]) -> Result<(), sqlx::Error> {
        self.rt
            .block_on(async move { self.overwrite_servers(servers).await })
    }

    pub async fn delete_server(&self, name: &str) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM server_items WHERE server_name = ?", name,)
            .execute(&self.pool)
            .await
            .map(|_| ())
    }

    pub async fn get_all_servers(&self) -> Result<Vec<ServerItem>, sqlx::Error> {
        let servers = sqlx::query_as::<_, ServerItem>(
            r#"
            SELECT *
            FROM server_items
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(servers)
    }

    pub fn get_all_servers_sync(&self) -> Result<Vec<ServerItem>, sqlx::Error> {
        self.rt
            .block_on(async move { self.get_all_servers().await })
    }

    pub fn save_server_sync(&self, server: &ServerItem) -> Result<(), sqlx::Error> {
        self.rt
            .block_on(async move { self.save_server(server).await })
    }

    pub fn delete_server_sync(&self, name: &str) -> Result<(), sqlx::Error> {
        self.rt
            .block_on(async move { self.delete_server(name).await })
    }

    pub async fn save_auto_transfer(&self, auto: &AutoTransfer) -> Result<(), sqlx::Error> {
        if sqlx::query!(
            r#"
        UPDATE auto_transfers
        SET
            server_name = ?,
            user_name = ?,
            user_password = ?,
            point_name = ?,
            cron_schedule = ?,
            last_ran = ?
        WHERE
            id = ?
        "#,
            auto.server_name,
            auto.user_name,
            auto.user_password,
            auto.point_name,
            auto.cron_schedule,
            auto.last_ran,
            auto.id,
        )
        .execute(&self.pool)
        .await
        .map(|x| x.rows_affected())?
            <= 0
        {
            sqlx::query!(
                r#"
            INSERT INTO auto_transfers (
                id,
                server_name,
                user_name,
                user_password,
                point_name,
                cron_schedule,
                last_ran
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
                auto.id,
                auto.server_name,
                auto.user_name,
                auto.user_password,
                auto.point_name,
                auto.cron_schedule,
                auto.last_ran,
            )
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn save_auto_transfers(&self, autos: &[AutoTransfer]) -> Result<(), sqlx::Error> {
        let tx = self.pool.begin().await?;
        for auto in autos {
            self.save_auto_transfer(&auto).await?;
        }
        tx.commit().await
    }

    pub async fn overwrite_auto_transfers(
        &self,
        autos: &[AutoTransfer],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM auto_transfers")
            .execute(&self.pool)
            .await?;
        self.save_auto_transfers(autos).await
    }

    pub fn overwrite_auto_transfers_sync(&self, autos: &[AutoTransfer]) -> Result<(), sqlx::Error> {
        self.rt
            .block_on(async move { self.overwrite_auto_transfers(autos).await })
    }

    pub async fn delete_auto_transfer(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM auto_transfers WHERE id = ?", id)
            .execute(&self.pool)
            .await
            .map(|_| ())
    }

    pub async fn get_all_auto_transfers(&self) -> Result<Vec<AutoTransfer>, sqlx::Error> {
        let autos = sqlx::query_as::<_, AutoTransfer>(
            r#"
        SELECT *
        FROM auto_transfer
        "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(autos)
    }

    pub fn get_all_auto_transfers_sync(&self) -> Result<Vec<AutoTransfer>, sqlx::Error> {
        self.rt
            .block_on(async move { self.get_all_auto_transfers().await })
    }

    pub fn save_auto_transfer_sync(&self, auto: &AutoTransfer) -> Result<(), sqlx::Error> {
        self.rt
            .block_on(async move { self.save_auto_transfer(auto).await })
    }

    pub fn delete_auto_transfer_sync(&self, id: Uuid) -> Result<(), sqlx::Error> {
        self.rt
            .block_on(async move { self.delete_auto_transfer(id).await })
    }

    pub fn new(rt: Arc<Runtime>, path: &str) -> Self {
        let url = format!("sqlite://{}", path);
        Self {
            rt: rt.clone(),
            pool: {
                rt.block_on(async move {
                    if !Sqlite::database_exists(&url).await.unwrap_or(false) {
                        Sqlite::create_database(&url)
                            .await
                            .expect("Failed to create Database!");
                    }
                    let pool = SqlitePoolOptions::new()
                        .max_connections(4)
                        .connect(&url)
                        .await
                        .expect("Expected pool to open!");
                    sqlx::migrate!()
                        .run(&pool)
                        .await
                        .expect("Failed to run migrations!");
                    pool
                })
            },
        }
    }
}
