use anyhow::{Context, Result};
use dirs::data_local_dir;
use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePoolOptions, Sqlite, SqlitePool};
use std::path::PathBuf;

pub async fn init_database() -> Result<SqlitePool> {
    let app_data_dir = get_app_data_dir()?;
    std::fs::create_dir_all(&app_data_dir).context("Failed to create app data directory")?;

    let db_path = app_data_dir.join("quickdev.db");
    let db_url = format!("sqlite:{}", db_path.to_string_lossy());

    if !Sqlite::database_exists(&db_url).await.unwrap_or(false) {
        Sqlite::create_database(&db_url)
            .await
            .context("Failed to create database")?;
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .context("Failed to connect to database")?;

    sqlx::query("PRAGMA foreign_keys = ON;")
        .execute(&pool)
        .await
        .context("Failed to enable SQLite foreign keys")?;

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("Failed to run database migrations")?;

    Ok(pool)
}

fn get_app_data_dir() -> Result<PathBuf> {
    let local_data_dir = data_local_dir().context("Failed to get local data directory")?;
    Ok(local_data_dir.join("QuickDev"))
}

#[cfg(test)]
mod tests {
    use sqlx::migrate::Migrator;
    use sqlx::sqlite::SqlitePoolOptions;

    static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

    #[tokio::test]
    async fn migrations_bootstrap_required_tables() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory db");

        MIGRATOR.run(&pool).await.expect("run migrations");

        for table in [
            "projects",
            "applications",
            "folders",
            "terminals",
            "integrations",
        ] {
            let exists = sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(1)
                FROM sqlite_master
                WHERE type = 'table' AND name = $1
                "#,
            )
            .bind(table)
            .fetch_one(&pool)
            .await
            .expect("check table exists");

            assert_eq!(exists, 1, "expected table `{table}` to exist");
        }
    }
}
