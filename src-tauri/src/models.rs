use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Users {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Project {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub icon: String,
    pub last_opened: Option<String>,
    pub total_time: i64,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Application {
    pub id: i32,
    pub project_id: i32,
    pub name: String,
    pub path: String,
    pub args: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Folder {
    pub id: i32,
    pub project_id: i32,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Terminal {
    pub id: i32,
    pub project_id: i32,
    pub name: String,
    pub path: String,
    pub command: String,
}

#[derive(Debug)]
pub struct LaunchItem {
    pub item_type: String,
    pub label: String,
    pub path: Option<String>,
    pub command: Option<String>,
    pub args: Option<String>,
    pub tool_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Task {
    pub id: i32,
    pub title: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub project_id: i32,
    pub status: String,
    pub priority: String,
    pub due_date: String,
    pub created_at: String,
    pub order_index: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ChecklistItem {
    pub id: i32,
    pub task_id: i32,
    pub text: String,
    pub completed: bool,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct TimeLog {
    pub id: i32,
    pub project_id: i32,
    pub task_id: Option<i32>,
    pub start_time: String,
    pub end_time: String,
    pub duration: i64,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Settings {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneralSettings {
    pub default_application_paths: DefaultApplicationPaths,
    pub startup: StartupSettings,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DefaultApplicationPaths {
    pub editor: String,
    pub browser: String,
    pub terminal: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartupSettings {
    pub launch_on_startup: bool,
    pub minimize_to_tray: bool,
    pub reopen_last_project: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimerSettings {
    pub pomodoro_length: i32,
    pub short_break_length: i32,
    pub long_break_length: i32,
    pub auto_start_breaks: bool,
    pub auto_start_pomodoros: bool,
    pub show_notifications: bool,
    pub play_sound: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThemeSettings {
    pub theme: String,
    pub accent_color: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataSettings {
    pub data_directory: String,
    pub auto_backup: bool,
    pub backup_frequency: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Integration {
    pub id: i32,
    pub tool_id: String,
    pub display_name: String,
    pub platform: String,
    pub detection_method: Option<String>,
    pub launch_command: Option<String>,
    pub is_available: bool,
    pub last_checked_at: Option<String>,
    pub metadata_json: Option<String>,
}
