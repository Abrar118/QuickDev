use crate::models::TimeLog;
use crate::services::time_log_service;
use crate::AppState;
use tauri::{command, State};

#[command]
pub async fn get_time_logs(state: State<'_, AppState>) -> Result<Vec<TimeLog>, String> {
    let pool = state
        .db_pool
        .as_ref()
        .ok_or_else(|| "Failed to get database pool".to_string())?;

    time_log_service::get_time_logs(pool).await
}

#[command]
pub async fn create_time_log(
    time_log: TimeLog,
    state: State<'_, AppState>,
) -> Result<TimeLog, String> {
    let pool = state
        .db_pool
        .as_ref()
        .ok_or_else(|| "Failed to get database pool".to_string())?;

    time_log_service::create_time_log(pool, time_log).await
}
