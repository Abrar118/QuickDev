use chrono::prelude::*;

pub fn get_current_datetime() -> String {
    let now: DateTime<Utc> = Utc::now();
    now.format("%d-%m-%Y %H:%M:%S").to_string()
}
