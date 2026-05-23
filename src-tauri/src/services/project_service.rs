use crate::models::{Application, Folder, LaunchItem, Project, Terminal};
use crate::services::launch_service;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::path::Path;

pub async fn get_projects(pool: &SqlitePool) -> Result<Vec<Project>, String> {
    let projects = sqlx::query_as::<_, Project>(
        "SELECT *
                FROM projects ORDER BY last_opened DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut result = Vec::new();

    for project in projects {
        let project_id = project.id;

        let application = sqlx::query_as::<_, Application>(
            "SELECT *
            FROM applications WHERE project_id = $1",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

        let folders = sqlx::query_as::<_, Folder>("SELECT * FROM folders WHERE project_id = $1")
            .bind(project_id)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;

        let terminals =
            sqlx::query_as::<_, Terminal>("SELECT * FROM terminals WHERE project_id = $1")
                .bind(project_id)
                .fetch_all(pool)
                .await
                .map_err(|e| e.to_string())?;

        let project_json = json!({
            "id": project.id,
            "name": project.name,
            "description": project.description,
            "color": project.color,
            "icon": project.icon,
            "last_opened": project.last_opened,
            "total_time": project.total_time,
            "is_active": project.is_active,
            "applications": application.iter().map(|app| {
                let args: Vec<String> = serde_json::from_str(&app.args).unwrap_or_default();
                json!({
                    "id": app.id,
                    "name": app.name,
                    "path": app.path,
                    "args": args
                })
            }).collect::<Vec<Value>>(),
            "folders": folders.iter().map(|folder| {
                json!({
                    "id": folder.id,
                    "name": folder.name,
                    "path": folder.path
                })
            }).collect::<Vec<Value>>(),
            "terminals": terminals.iter().map(|terminal| {
                json!({
                    "id": terminal.id,
                    "name": terminal.name,
                    "path": terminal.path,
                    "command": terminal.command
                })
            }).collect::<Vec<Value>>()
        });

        let project_with_details =
            serde_json::from_value(project_json).map_err(|e| e.to_string())?;

        result.push(project_with_details);
    }

    Ok(result)
}

pub async fn create_project(
    pool: &SqlitePool,
    project: Project,
    applications: Vec<Application>,
    folders: Vec<Folder>,
    terminals: Vec<Terminal>,
) -> Result<bool, String> {
    validate_project_payload(&project, &applications, &folders, &terminals)?;
    let mut txn = pool.begin().await.map_err(|e| e.to_string())?;

    let insert_project_query = r"
        INSERT INTO projects (name, description, color, icon, last_opened, total_time, is_active)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id
    ";

    let project_response = sqlx::query(insert_project_query)
        .bind(project.name)
        .bind(project.description)
        .bind(project.color)
        .bind(project.icon)
        .bind(project.last_opened)
        .bind(project.total_time)
        .bind(project.is_active)
        .execute(&mut *txn)
        .await
        .map_err(|e| e.to_string())?;

    let project_id = project_response.last_insert_rowid();

    for app in applications {
        let insert_application_query = r"
            INSERT INTO applications (project_id, name, path, args)
            VALUES ($1, $2, $3, $4)
        ";

        let args_json = serde_json::to_string(&app.args).map_err(|e| e.to_string())?;
        sqlx::query(insert_application_query)
            .bind(project_id)
            .bind(app.name)
            .bind(app.path)
            .bind(args_json)
            .execute(&mut *txn)
            .await
            .map_err(|e| e.to_string())?;
    }

    for folder in folders {
        let insert_folder_query = r"
            INSERT INTO folders (project_id, name, path)
            VALUES ($1, $2, $3)
        ";

        sqlx::query(insert_folder_query)
            .bind(project_id)
            .bind(folder.name)
            .bind(folder.path)
            .execute(&mut *txn)
            .await
            .map_err(|e| e.to_string())?;
    }

    for terminal in terminals {
        let insert_terminal_query = r"
            INSERT INTO terminals (project_id, name, path, command)
            VALUES ($1, $2, $3, $4)
        ";

        sqlx::query(insert_terminal_query)
            .bind(project_id)
            .bind(terminal.name)
            .bind(terminal.path)
            .bind(terminal.command)
            .execute(&mut *txn)
            .await
            .map_err(|e| e.to_string())?;
    }

    txn.commit().await.map_err(|e| e.to_string())?;

    Ok(true)
}

pub async fn get_project(pool: &SqlitePool, project_id: i32) -> Result<Project, String> {
    sqlx::query_as::<_, Project>(
        r#"
        SELECT id, name, description, color, icon, last_opened, total_time, is_active
        FROM projects
        WHERE id = $1
        "#,
    )
    .bind(project_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())
}

pub async fn update_project(pool: &SqlitePool, project: Project) -> Result<Project, String> {
    validate_project_basics(&project)?;
    sqlx::query_as::<_, Project>(
        r#"
        UPDATE projects
        SET name = $2,
            description = $3,
            color = $4,
            icon = $5,
            last_opened = $6,
            total_time = $7,
            is_active = $8
        WHERE id = $1
        RETURNING id, name, description, color, icon, last_opened, total_time, is_active
        "#,
    )
    .bind(project.id)
    .bind(project.name)
    .bind(project.description)
    .bind(project.color)
    .bind(project.icon)
    .bind(project.last_opened)
    .bind(project.total_time)
    .bind(project.is_active)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())
}

pub async fn delete_project(pool: &SqlitePool, project_id: i32) -> Result<bool, String> {
    let affected = sqlx::query(
        r#"
        DELETE FROM projects
        WHERE id = $1
        "#,
    )
    .bind(project_id)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?
    .rows_affected();

    Ok(affected > 0)
}

pub async fn launch_project(pool: &SqlitePool, project_id: i32) -> Result<bool, String> {
    let project = get_project(pool, project_id).await?;

    let applications = sqlx::query_as::<_, Application>(
        r#"
        SELECT id, project_id, name, path, args
        FROM applications
        WHERE project_id = $1
        ORDER BY id ASC
        "#,
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let folders = sqlx::query_as::<_, Folder>(
        r#"
        SELECT id, project_id, name, path
        FROM folders
        WHERE project_id = $1
        ORDER BY id ASC
        "#,
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let terminals = sqlx::query_as::<_, Terminal>(
        r#"
        SELECT id, project_id, name, path, command
        FROM terminals
        WHERE project_id = $1
        ORDER BY id ASC
        "#,
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let folder_paths: Vec<String> = folders.iter().map(|f| f.path.clone()).collect();

    let folder_items: Vec<LaunchItem> = folders
        .into_iter()
        .map(|folder| LaunchItem {
            item_type: "folder".to_string(),
            label: folder.name,
            path: Some(folder.path),
            command: None,
            args: None,
            tool_id: None,
        })
        .collect();

    let app_items: Vec<LaunchItem> = applications
        .into_iter()
        .map(|app| {
            let tool_id = launch_service::infer_tool_id(&app.name, &app.path);
            let args = if let Some(ref tid) = tool_id {
                if launch_service::is_editor_tool(tid) {
                    serde_json::to_string(&folder_paths).ok()
                } else {
                    parse_stored_args(&app.args)
                }
            } else {
                parse_stored_args(&app.args)
            };
            LaunchItem {
                item_type: "application".to_string(),
                label: app.name,
                path: Some(app.path),
                command: None,
                args,
                tool_id,
            }
        })
        .collect();

    let terminal_items: Vec<LaunchItem> = terminals
        .into_iter()
        .map(|terminal| LaunchItem {
            item_type: "command".to_string(),
            label: terminal.name,
            path: Some(terminal.path),
            command: Some(terminal.command),
            args: None,
            tool_id: None,
        })
        .collect();

    let all_items_empty = folder_items.is_empty() && app_items.is_empty() && terminal_items.is_empty();
    if all_items_empty {
        return Err("No launch items configured for this project".to_string());
    }

    let mut failed = Vec::new();

    for item in &folder_items {
        if let Err(reason) = launch_service::launch_item(item) {
            failed.push(format!("{} ({})", item.label, reason));
        }
    }

    for item in &app_items {
        if let Err(reason) = launch_service::launch_item(item) {
            failed.push(format!("{} ({})", item.label, reason));
        }
    }

    for item in &terminal_items {
        if let Err(reason) = launch_service::launch_item(item) {
            failed.push(format!("{} ({})", item.label, reason));
        }
    }

    if failed.is_empty() {
        return Ok(true);
    }

    if failed.len() == 1 {
        return Err(format!("Failed to launch {}", failed[0]));
    }

    Err(format!(
        "Project '{}' launched with {} failures: {}",
        project.name,
        failed.len(),
        failed.join("; ")
    ))
}

fn parse_stored_args(args_str: &str) -> Option<String> {
    let trimmed = args_str.trim();
    if trimmed.is_empty() || trimmed == "\"\"" || trimmed == "[]" {
        return None;
    }
    if let Ok(arr) = serde_json::from_str::<Vec<String>>(trimmed) {
        if arr.is_empty() {
            return None;
        }
        return Some(serde_json::to_string(&arr).unwrap_or_default());
    }
    None
}

fn validate_project_payload(
    project: &Project,
    applications: &[Application],
    folders: &[Folder],
    terminals: &[Terminal],
) -> Result<(), String> {
    validate_project_basics(project)?;

    if folders.is_empty() {
        return Err("At least one project folder is required".to_string());
    }

    for app in applications {
        if app.name.trim().is_empty() {
            return Err("Application name cannot be empty".to_string());
        }
        if app.path.trim().is_empty() {
            return Err("Application path cannot be empty".to_string());
        }
    }

    for folder in folders {
        if folder.name.trim().is_empty() {
            return Err("Folder name cannot be empty".to_string());
        }
        if folder.path.trim().is_empty() {
            return Err("Folder path cannot be empty".to_string());
        }
        if !Path::new(&folder.path).exists() {
            return Err(format!("Folder path does not exist: {}", folder.path));
        }
    }

    for terminal in terminals {
        if terminal.name.trim().is_empty() {
            return Err("Terminal name cannot be empty".to_string());
        }
        if terminal.path.trim().is_empty() {
            return Err("Terminal path cannot be empty".to_string());
        }
    }

    Ok(())
}

fn validate_project_basics(project: &Project) -> Result<(), String> {
    if project.name.trim().is_empty() {
        return Err("Project name cannot be empty".to_string());
    }
    if project.color.trim().is_empty() {
        return Err("Project color cannot be empty".to_string());
    }
    if project.icon.trim().is_empty() {
        return Err("Project icon cannot be empty".to_string());
    }
    Ok(())
}
