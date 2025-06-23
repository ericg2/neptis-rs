#[macro_export]
macro_rules! to_dto_time {
    ($system_time:expr) => {{
        let datetime: chrono::DateTime<chrono::Utc> = $system_time.into();
        datetime.naive_utc()
    }};
}

#[macro_export]
macro_rules! from_dto_time {
    ($naive_datetime:expr) => {{
        let naive_epoch = $naive_datetime.and_utc().timestamp();
        std::time::UNIX_EPOCH + std::time::Duration::from_secs(naive_epoch as u64)
    }};
}
