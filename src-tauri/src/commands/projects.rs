use crate::models::{Application, Folder, Project, Terminal};
use crate::services::project_service;
use crate::AppState;
use tauri::{command, State};

#[command]
pub async fn get_projects(state: State<'_, AppState>) -> Result<Vec<Project>, String> {
    let pool = state
        .db_pool
        .as_ref()
        .ok_or_else(|| "Failed to get database pool".to_string())?;

    project_service::get_projects(pool).await
}

#[command]
pub async fn list_projects(state: State<'_, AppState>) -> Result<Vec<Project>, String> {
    get_projects(state).await
}

#[command]
pub async fn get_project(project_id: i32, state: State<'_, AppState>) -> Result<Project, String> {
    let pool = state
        .db_pool
        .as_ref()
        .ok_or_else(|| "Failed to get database pool".to_string())?;

    project_service::get_project(pool, project_id).await
}

#[command]
pub async fn create_project(
    project: Project,
    applications: Vec<Application>,
    folders: Vec<Folder>,
    terminals: Vec<Terminal>,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let pool = state
        .db_pool
        .as_ref()
        .ok_or_else(|| "Failed to get database pool".to_string())?;

    project_service::create_project(pool, project, applications, folders, terminals).await
}

#[command]
pub async fn update_project(
    project: Project,
    state: State<'_, AppState>,
) -> Result<Project, String> {
    let pool = state
        .db_pool
        .as_ref()
        .ok_or_else(|| "Failed to get database pool".to_string())?;

    project_service::update_project(pool, project).await
}

#[command]
pub async fn delete_project(project_id: i32, state: State<'_, AppState>) -> Result<bool, String> {
    let pool = state
        .db_pool
        .as_ref()
        .ok_or_else(|| "Failed to get database pool".to_string())?;

    project_service::delete_project(pool, project_id).await
}

#[command]
pub async fn launch_project(project_id: i32, state: State<'_, AppState>) -> Result<bool, String> {
    let pool = state
        .db_pool
        .as_ref()
        .ok_or_else(|| "Failed to get database pool".to_string())?;

    project_service::launch_project(pool, project_id).await
}
