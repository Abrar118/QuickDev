use crate::models::TimeLog;
use sqlx::SqlitePool;

pub async fn get_time_logs(pool: &SqlitePool) -> Result<Vec<TimeLog>, String> {
    sqlx::query_as::<_, TimeLog>(
        r#"
        SELECT id, project_id, task_id, start_time, end_time, duration, notes
        FROM time_logs
        ORDER BY start_time DESC, id DESC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

pub async fn create_time_log(pool: &SqlitePool, time_log: TimeLog) -> Result<TimeLog, String> {
    sqlx::query_as::<_, TimeLog>(
        r#"
        INSERT INTO time_logs (project_id, task_id, start_time, end_time, duration, notes)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, project_id, task_id, start_time, end_time, duration, notes
        "#,
    )
    .bind(time_log.project_id)
    .bind(time_log.task_id)
    .bind(time_log.start_time)
    .bind(time_log.end_time)
    .bind(time_log.duration)
    .bind(time_log.notes)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())
}
