use std::fs;
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
use crate::prelude::{TransferAutoJob, TransferAutoSchedule};

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

    pub async fn save_transfer_auto_schedule(&self, schedule: &TransferAutoSchedule) -> Result<(), sqlx::Error> {
        if sqlx::query!(
            r#"
            UPDATE transfer_auto_schedules
            SET
                server_name = ?,
                cron_schedule = ?
            WHERE
                schedule_name = ?
            "#,
            schedule.server_name,
            schedule.cron_schedule,
            schedule.schedule_name,
        )
            .execute(&self.pool)
            .await
            .map(|x| x.rows_affected())?
            <= 0
        {
            sqlx::query!(
                r#"
                INSERT INTO transfer_auto_schedules (
                    schedule_name,
                    server_name,
                    cron_schedule
                ) VALUES (?, ?, ?)
                "#,
                schedule.schedule_name,
                schedule.server_name,
                schedule.cron_schedule,
            )
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    pub fn save_transfer_auto_schedule_sync(&self, schedule: &TransferAutoSchedule) -> Result<(), sqlx::Error> {
        self.rt.block_on(async move { self.save_transfer_auto_schedule(schedule).await })
    }

    pub async fn get_all_transfer_auto_schedules(&self) -> Result<Vec<TransferAutoSchedule>, sqlx::Error> {
        let results = sqlx::query_as::<_, TransferAutoSchedule>(
            r#"
            SELECT * FROM transfer_auto_schedules
            "#
        )
            .fetch_all(&self.pool)
            .await?;
        Ok(results)
    }

    pub fn get_all_transfer_auto_schedules_sync(&self) -> Result<Vec<TransferAutoSchedule>, sqlx::Error> {
        self.rt.block_on(async move { self.get_all_transfer_auto_schedules().await })
    }

    pub async fn delete_transfer_auto_schedule(&self, schedule_name: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM transfer_auto_schedules WHERE schedule_name = ?",
            batch_id
        )
            .execute(&self.pool)
            .await
            .map(|_| ())
    }

    pub fn delete_transfer_auto_schedule_sync(&self, batch_id: &str) -> Result<(), sqlx::Error> {
        self.rt.block_on(async move { self.delete_transfer_auto_schedule(batch_id).await })
    }

    pub async fn save_transfer_auto_job(&self, job: &TransferAutoJob) -> Result<(), sqlx::Error> {
        if sqlx::query!(
            r#"
            UPDATE transfer_auto_jobs
            SET
                batch_id = ?,
                schedule_name = ?,
                smb_user_name = ?,
                smb_password = ?,
                smb_folder = ?,
                local_folder = ?
            WHERE
                id = ?
            "#,
            job.batch_id,
            job.schedule_name,
            job.smb_user_name,
            job.smb_password,
            job.smb_folder,
            job.local_folder,
            job.id,
        )
            .execute(&self.pool)
            .await
            .map(|x| x.rows_affected())?
            <= 0
        {
            sqlx::query!(
                r#"
                INSERT INTO transfer_auto_jobs (
                    id,
                    batch_id,
                    schedule_name,
                    smb_user_name,
                    smb_password,
                    smb_folder,
                    local_folder
                ) VALUES (?, ?, ?, ?, ?, ?)
                "#,
                job.id,
                job.batch_id,
                job.schedule_name,
                job.smb_user_name,
                job.smb_password,
                job.smb_folder,
                job.local_folder,
            )
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    pub fn save_transfer_auto_job_sync(&self, job: &TransferAutoJob) -> Result<(), sqlx::Error> {
        self.rt.block_on(async move { self.save_transfer_auto_job(job).await })
    }

    pub async fn get_all_transfer_auto_jobs(&self) -> Result<Vec<TransferAutoJob>, sqlx::Error> {
        let results = sqlx::query_as::<_, TransferAutoJob>(
            r#"
            SELECT * FROM transfer_auto_jobs
            "#
        )
            .fetch_all(&self.pool)
            .await?;
        Ok(results)
    }

    pub fn get_all_transfer_auto_jobs_sync(&self) -> Result<Vec<TransferAutoJob>, sqlx::Error> {
        self.rt.block_on(async move { self.get_all_transfer_auto_jobs().await })
    }

    pub async fn delete_transfer_auto_job(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM transfer_auto_jobs WHERE id = ?",
            id
        )
            .execute(&self.pool)
            .await
            .map(|_| ())
    }

    pub fn delete_transfer_auto_job_sync(&self, id: &str) -> Result<(), sqlx::Error> {
        self.rt.block_on(async move { self.delete_transfer_auto_job(id).await })
    }

    fn get_db(db_path: Option<String>) -> String {
        if let Some(b_dir) = dirs_next::home_dir().map(|x| x.join(".neptis")) {
            if !b_dir.exists() {
                fs::create_dir_all(b_dir).expect("Failed to create Neptis directory!");
            }
        }
        db_path.clone()
            .or(
                dirs_next::home_dir()
                    .map(|x|x.join(".neptis/neptis.db").to_str().unwrap().to_string()))
            .expect("Failed to find database location! Please set 'NEPTIS_DB' to a path, or use a user account with a home directory.")
    }

    pub fn new_default(rt: Arc<Runtime>, db_path: Option<String>) -> Self {
        Self::new(rt, &Self::get_db(db_path))
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
