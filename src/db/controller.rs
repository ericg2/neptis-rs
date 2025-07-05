use std::sync::Arc;

use super::server::ServerItem;
use crate::get_working_dir;
use crate::prelude::{TransferAutoJob, TransferAutoSchedule, TransferJobInternalDto};
use sqlx::{
    Sqlite, SqlitePool,
    migrate::MigrateDatabase,
    sqlite::SqlitePoolOptions,
};
use tokio::runtime::Runtime;
use uuid::Uuid;

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

    pub async fn save_transfer_auto_schedule(
        &self,
        schedule: &TransferAutoSchedule,
    ) -> Result<(), sqlx::Error> {
        if sqlx::query!(
            r#"
            UPDATE transfer_auto_schedules
            SET
                cron_schedule = ?,
                smb_user_name = ?,
                smb_password = ?,
                last_updated = ?,
                backup_on_finish = ?,
                user_password = ?
            WHERE
                schedule_name = ?
            AND
                server_name = ?
            "#,
            schedule.cron_schedule,
            schedule.smb_user_name,
            schedule.smb_password,
            schedule.last_updated,
            schedule.backup_on_finish,
            schedule.user_password,
            schedule.schedule_name,
            schedule.server_name
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
                    cron_schedule,
                    smb_user_name,
                    smb_password,
                    last_updated,
                    backup_on_finish,
                    user_password
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                schedule.schedule_name,
                schedule.server_name,
                schedule.cron_schedule,
                schedule.smb_user_name,
                schedule.smb_password,
                schedule.last_updated,
                schedule.backup_on_finish,
                schedule.user_password
            )
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub fn save_transfer_auto_schedule_sync(
        &self,
        schedule: &TransferAutoSchedule,
    ) -> Result<(), sqlx::Error> {
        self.rt
            .block_on(async move { self.save_transfer_auto_schedule(schedule).await })
    }

    pub async fn get_all_transfer_auto_schedules(
        &self,
    ) -> Result<Vec<TransferAutoSchedule>, sqlx::Error> {
        let results = sqlx::query_as::<_, TransferAutoSchedule>(
            r#"
            SELECT * FROM transfer_auto_schedules
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(results)
    }

    pub fn get_all_transfer_auto_schedules_sync(
        &self,
    ) -> Result<Vec<TransferAutoSchedule>, sqlx::Error> {
        self.rt
            .block_on(async move { self.get_all_transfer_auto_schedules().await })
    }

    pub async fn delete_transfer_auto_schedule(
        &self,
        schedule_name: &str,
        server_name: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM transfer_auto_schedules WHERE schedule_name = ? AND server_name = ?",
            schedule_name,
            server_name
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    pub fn delete_transfer_auto_schedule_sync(
        &self,
        schedule_name: &str,
        server_name: &str,
    ) -> Result<(), sqlx::Error> {
        self.rt.block_on(async move {
            self.delete_transfer_auto_schedule(schedule_name, server_name)
                .await
        })
    }

    pub async fn save_transfer_auto_job(&self, job: &TransferAutoJob) -> Result<(), sqlx::Error> {
        if sqlx::query!(
            r#"
            UPDATE transfer_auto_jobs
            SET
                smb_folder = ?,
                local_folder = ?,
                enabled = ?
            WHERE
                schedule_name = ?
            AND
                action_name = ?
            AND
                server_name = ?
            "#,
            job.smb_folder,
            job.local_folder,
            job.enabled,
            job.schedule_name,
            job.action_name,
            job.server_name,
        )
        .execute(&self.pool)
        .await
        .map(|x| x.rows_affected())?
            <= 0
        {
            sqlx::query!(
                r#"
                INSERT INTO transfer_auto_jobs (
                    schedule_name,
                    action_name,
                    server_name,
                    smb_folder,
                    local_folder,
                    enabled
                ) VALUES (?, ?, ?, ?, ?, ?)
                "#,
                job.schedule_name,
                job.action_name,
                job.server_name,
                job.smb_folder,
                job.local_folder,
                job.enabled,
            )
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub fn save_transfer_auto_job_sync(&self, job: &TransferAutoJob) -> Result<(), sqlx::Error> {
        self.rt
            .block_on(async move { self.save_transfer_auto_job(job).await })
    }

    pub async fn get_all_transfer_auto_jobs(&self) -> Result<Vec<TransferAutoJob>, sqlx::Error> {
        let results = sqlx::query_as::<_, TransferAutoJob>(
            r#"
            SELECT * FROM transfer_auto_jobs
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(results)
    }

    pub fn get_all_transfer_auto_jobs_sync(&self) -> Result<Vec<TransferAutoJob>, sqlx::Error> {
        self.rt
            .block_on(async move { self.get_all_transfer_auto_jobs().await })
    }

    pub async fn delete_transfer_auto_job(
        &self,
        schedule_name: &str,
        server_name: &str,
        action_name: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM transfer_auto_jobs WHERE schedule_name = ? AND server_name = ? AND action_name = ?",
            schedule_name,
            server_name,
            action_name,
        )
            .execute(&self.pool)
            .await
            .map(|_| ())
    }

    pub fn delete_transfer_auto_job_sync(
        &self,
        schedule_name: &str,
        server_name: &str,
        action_name: &str,
    ) -> Result<(), sqlx::Error> {
        self.rt.block_on(async move {
            self.delete_transfer_auto_job(schedule_name, server_name, action_name)
                .await
        })
    }

    pub async fn save_transfer_job_internal(
        &self,
        job: &TransferJobInternalDto,
    ) -> Result<(), sqlx::Error> {
        let last_stats_json = match &job.last_stats {
            Some(s) => {
                Some(serde_json::to_string(s).map_err(|e| sqlx::Error::Decode(Box::new(e)))?)
            }
            None => None,
        };
        let fatal_errors_json = serde_json::to_string(&job.fatal_errors)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        let warnings_json =
            serde_json::to_string(&job.warnings).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        if sqlx::query!(
            r#"
        UPDATE transfer_jobs_internal
        SET
            auto_job_action_name = ?,
            auto_job_schedule_name = ?,
            server_name = ?,
            smb_user_name = ?,
            smb_password = ?,
            smb_folder = ?,
            local_folder = ?,
            last_stats = ?,
            start_date = ?,
            end_date = ?,
            fatal_errors = ?,
            warnings = ?,
            last_updated = ?,
            init_hash = ?
        WHERE
            job_id = ?
        "#,
            job.auto_job_action_name,
            job.auto_job_schedule_name,
            job.server_name,
            job.smb_user_name,
            job.smb_password,
            job.smb_folder,
            job.local_folder,
            last_stats_json,
            job.start_date,
            job.end_date,
            fatal_errors_json,
            warnings_json,
            job.last_updated,
            job.init_hash,
            job.job_id,
        )
        .execute(&self.pool)
        .await?
        .rows_affected()
            == 0
        {
            sqlx::query!(
                r#"
            INSERT INTO transfer_jobs_internal (
                job_id,
                auto_job_schedule_name,
                auto_job_action_name,
                server_name,
                smb_user_name,
                smb_password,
                smb_folder,
                local_folder,
                last_stats,
                start_date,
                end_date,
                fatal_errors,
                warnings,
                last_updated,
                init_hash
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
                job.job_id,
                job.auto_job_schedule_name,
                job.auto_job_action_name,
                job.server_name,
                job.smb_user_name,
                job.smb_password,
                job.smb_folder,
                job.local_folder,
                last_stats_json,
                job.start_date,
                job.end_date,
                fatal_errors_json,
                warnings_json,
                job.last_updated,
                job.init_hash
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    pub fn save_transfer_job_internal_sync(
        &self,
        job: &TransferJobInternalDto,
    ) -> Result<(), sqlx::Error> {
        self.rt
            .block_on(async move { self.save_transfer_job_internal(job).await })
    }

    pub async fn get_all_transfer_jobs_internal(
        &self,
    ) -> Result<Vec<TransferJobInternalDto>, sqlx::Error> {
        let results = sqlx::query_as::<_, TransferJobInternalDto>(
            r#"
            SELECT * FROM transfer_jobs_internal
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(results)
    }

    pub fn get_all_transfer_jobs_internal_sync(
        &self,
    ) -> Result<Vec<TransferJobInternalDto>, sqlx::Error> {
        self.rt
            .block_on(async move { self.get_all_transfer_jobs_internal().await })
    }

    pub async fn delete_transfer_job_internal(&self, job_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM transfer_jobs_internal WHERE job_id = ?",
            job_id
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    pub fn delete_transfer_job_internal_sync(&self, job_id: Uuid) -> Result<(), sqlx::Error> {
        self.rt
            .block_on(async move { self.delete_transfer_job_internal(job_id).await })
    }

    pub fn new(rt: Arc<Runtime>) -> Self {
        let url = format!(
            "sqlite://{}",
            get_working_dir().join("neptis.db").to_str().unwrap()
        );
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
