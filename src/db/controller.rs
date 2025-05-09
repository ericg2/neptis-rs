use sqlx::{
    Sqlite, SqlitePool,
    migrate::MigrateDatabase,
    prelude::*,
    sqlite::{SqlitePoolOptions, SqliteQueryResult},
};
use tokio::runtime::Runtime;

use super::server::ServerItem;

pub struct DbController<'a> {
    rt: &'a Runtime,
    pool: SqlitePool,
}

impl<'a> DbController<'a> {
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

    pub fn new(rt: &'a Runtime, path: &str) -> Self {
        let url = format!("sqlite://{}", path);
        Self {
            rt,
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
