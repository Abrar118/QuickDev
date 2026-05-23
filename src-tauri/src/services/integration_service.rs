use crate::adapters::tool_adapter::registry_for_platform;
use crate::models::Integration;
use sqlx::SqlitePool;

fn current_platform() -> String {
    std::env::consts::OS.to_string()
}

pub async fn detect_integrations(pool: &SqlitePool) -> Result<Vec<Integration>, String> {
    let platform = current_platform();
    let adapters = registry_for_platform(&platform);

    for adapter in adapters {
        let is_available = adapter.is_available();
        let metadata_json = adapter.capabilities_json().to_string();

        sqlx::query(
            r#"
            INSERT INTO integrations (
                tool_id, display_name, platform, detection_method, launch_command,
                is_available, last_checked_at, metadata_json
            )
            VALUES ($1, $2, $3, $4, $5, $6, datetime('now'), $7)
            ON CONFLICT(tool_id, platform) DO UPDATE SET
                display_name = excluded.display_name,
                detection_method = excluded.detection_method,
                launch_command = excluded.launch_command,
                is_available = excluded.is_available,
                metadata_json = excluded.metadata_json,
                last_checked_at = datetime('now')
            "#,
        )
        .bind(adapter.tool_id)
        .bind(adapter.display_name)
        .bind(&platform)
        .bind(adapter.detection_method)
        .bind(adapter.launch_command)
        .bind(is_available)
        .bind(metadata_json)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    }

    list_integrations(pool).await
}

pub async fn list_integrations(pool: &SqlitePool) -> Result<Vec<Integration>, String> {
    let platform = current_platform();

    sqlx::query_as::<_, Integration>(
        r#"
        SELECT id, tool_id, display_name, platform, detection_method, launch_command,
               is_available, last_checked_at, metadata_json
        FROM integrations
        WHERE platform = $1
        ORDER BY display_name ASC
        "#,
    )
    .bind(platform)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

pub async fn get_available_integrations(pool: &SqlitePool) -> Result<Vec<Integration>, String> {
    let platform = current_platform();

    sqlx::query_as::<_, Integration>(
        r#"
        SELECT id, tool_id, display_name, platform, detection_method, launch_command,
               is_available, last_checked_at, metadata_json
        FROM integrations
        WHERE platform = $1 AND is_available = 1
        ORDER BY display_name ASC
        "#,
    )
    .bind(platform)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::{get_available_integrations, list_integrations};
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn returns_empty_when_no_integrations_exist() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory db");

        sqlx::query(
            r#"
            CREATE TABLE integrations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tool_id TEXT NOT NULL,
                display_name TEXT NOT NULL,
                platform TEXT NOT NULL,
                detection_method TEXT,
                launch_command TEXT,
                is_available BOOLEAN NOT NULL DEFAULT 0,
                last_checked_at TEXT,
                metadata_json TEXT,
                UNIQUE(tool_id, platform)
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("create integrations table");

        let all = list_integrations(&pool).await.expect("list integrations");
        let available = get_available_integrations(&pool)
            .await
            .expect("list available integrations");

        assert!(all.is_empty());
        assert!(available.is_empty());
    }
}
