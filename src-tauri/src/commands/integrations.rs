use crate::models::Integration;
use crate::services::integration_service;
use crate::AppState;
use tauri::{command, State};

#[command]
pub async fn detect_integrations(state: State<'_, AppState>) -> Result<Vec<Integration>, String> {
    let pool = state
        .db_pool
        .as_ref()
        .ok_or_else(|| "Failed to get database pool".to_string())?;

    integration_service::detect_integrations(pool).await
}

#[command]
pub async fn list_integrations(state: State<'_, AppState>) -> Result<Vec<Integration>, String> {
    let pool = state
        .db_pool
        .as_ref()
        .ok_or_else(|| "Failed to get database pool".to_string())?;

    integration_service::list_integrations(pool).await
}

#[command]
pub async fn get_available_integrations(
    state: State<'_, AppState>,
) -> Result<Vec<Integration>, String> {
    let pool = state
        .db_pool
        .as_ref()
        .ok_or_else(|| "Failed to get database pool".to_string())?;

    integration_service::get_available_integrations(pool).await
}
