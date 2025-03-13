#[derive(Debug, thiserror::Error)]
pub enum NeptisError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    AuthenticateError(#[from] crate::apis::Error<crate::apis::auth_api::AuthenticateError>),
    #[error(transparent)]
    DeleteOneAutoJobError(
        #[from] crate::apis::Error<crate::apis::auto_job_api::DeleteOneAutoJobError>,
    ),
    #[error(transparent)]
    GetAllAutoJobsError(
        #[from] crate::apis::Error<crate::apis::auto_job_api::GetAllAutoJobsError>,
    ),
    #[error(transparent)]
    GetGlobalConfigError(
        #[from] crate::apis::Error<crate::apis::config_api::GetGlobalConfigError>,
    ),
    #[error(transparent)]
    UpdateGlobalConfigError(
        #[from] crate::apis::Error<crate::apis::config_api::UpdateGlobalConfigError>,
    ),
    #[error(transparent)]
    ApiDatasGetError(#[from] crate::apis::Error<crate::apis::data_api::ApiDatasGetError>),
    #[error(transparent)]
    ApiDatasPointUserPointNameDeleteError(
        #[from]
        crate::apis::Error<crate::apis::data_api::ApiDatasPointUserPointNameDeleteError>,
    ),
    #[error(transparent)]
    ApiDatasPointUserPointNameGetError(
        #[from]
        crate::apis::Error<crate::apis::data_api::ApiDatasPointUserPointNameGetError>,
    ),
    #[error(transparent)]
    ApiDatasPointUserPointNamePutError(
        #[from]
        crate::apis::Error<crate::apis::data_api::ApiDatasPointUserPointNamePutError>,
    ),
    #[error(transparent)]
    ApiDatasPointUserPointNameSharesDeleteError(
        #[from]
        crate::apis::Error<
            crate::apis::data_api::ApiDatasPointUserPointNameSharesDeleteError,
        >,
    ),
    #[error(transparent)]
    ApiDatasPointUserPointNameSharesGetError(
        #[from]
        crate::apis::Error<
            crate::apis::data_api::ApiDatasPointUserPointNameSharesGetError,
        >,
    ),
    #[error(transparent)]
    ApiDatasPointUserPointNameSharesPutError(
        #[from]
        crate::apis::Error<
            crate::apis::data_api::ApiDatasPointUserPointNameSharesPutError,
        >,
    ),
    #[error(transparent)]
    ApiDatasPostError(#[from] crate::apis::Error<crate::apis::data_api::ApiDatasPostError>),
    #[error(transparent)]
    BrowseFilesForDataError(
        #[from] crate::apis::Error<crate::apis::data_api::BrowseFilesForDataError>,
    ),
    #[error(transparent)]
    DeleteOneFileForDataError(
        #[from] crate::apis::Error<crate::apis::data_api::DeleteOneFileForDataError>,
    ),
    #[error(transparent)]
    DumpOneFileForDataError(
        #[from] crate::apis::Error<crate::apis::data_api::DumpOneFileForDataError>,
    ),
    #[error(transparent)]
    GetAllJobsForDataError(
        #[from] crate::apis::Error<crate::apis::data_api::GetAllJobsForDataError>,
    ),
    #[error(transparent)]
    UpdateOneFileForDataError(
        #[from] crate::apis::Error<crate::apis::data_api::UpdateOneFileForDataError>,
    ),
    #[error(transparent)]
    GetSystemSummaryError(
        #[from] crate::apis::Error<crate::apis::info_api::GetSystemSummaryError>,
    ),
    #[error(transparent)]
    GetValidNotifyMethodsError(
        #[from] crate::apis::Error<crate::apis::info_api::GetValidNotifyMethodsError>,
    ),
    #[error(transparent)]
    GetValidNotifySubscriptionsError(
        #[from] crate::apis::Error<crate::apis::info_api::GetValidNotifySubscriptionsError>,
    ),
    #[error(transparent)]
    GetValidPermissionsError(
        #[from] crate::apis::Error<crate::apis::info_api::GetValidPermissionsError>,
    ),
    #[error(transparent)]
    CancelOneJobError(#[from] crate::apis::Error<crate::apis::job_api::CancelOneJobError>),
    #[error(transparent)]
    GetAllJobsError(#[from] crate::apis::Error<crate::apis::job_api::GetAllJobsError>),
    #[error(transparent)]
    GetOneJobError(#[from] crate::apis::Error<crate::apis::job_api::GetOneJobError>),
    #[error(transparent)]
    StartOneBackupError(
        #[from] crate::apis::Error<crate::apis::job_api::StartOneBackupError>,
    ),
    #[error(transparent)]
    StartOneRestoreError(
        #[from] crate::apis::Error<crate::apis::job_api::StartOneRestoreError>,
    ),
    #[error(transparent)]
    DeleteOneLogError(#[from] crate::apis::Error<crate::apis::log_api::DeleteOneLogError>),
    #[error(transparent)]
    GetAllLogsError(#[from] crate::apis::Error<crate::apis::log_api::GetAllLogsError>),
    #[error(transparent)]
    GetOneLogError(#[from] crate::apis::Error<crate::apis::log_api::GetOneLogError>),
    #[error(transparent)]
    DeleteOneMessageError(
        #[from] crate::apis::Error<crate::apis::message_api::DeleteOneMessageError>,
    ),
    #[error(transparent)]
    GetAllMessagesError(
        #[from] crate::apis::Error<crate::apis::message_api::GetAllMessagesError>,
    ),
    #[error(transparent)]
    GetOneMessageError(
        #[from] crate::apis::Error<crate::apis::message_api::GetOneMessageError>,
    ),
    #[error(transparent)]
    GetAllNotificationsError(
        #[from] crate::apis::Error<crate::apis::notification_api::GetAllNotificationsError>,
    ),
    #[error(transparent)]
    GetAllNotificationConfigsError(
        #[from]
        crate::apis::Error<crate::apis::notification_api::GetAllNotificationConfigsError>,
    ),
    #[error(transparent)]
    Parse(#[from] std::string::ParseError),
    #[error(transparent)]
    ChronoParse(#[from] chrono::ParseError),
    #[error("{0}")]
    Str(String),
}

// we must manually implement serde::Serialize
impl serde::Serialize for NeptisError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}