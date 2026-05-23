use serde_json::{json, Value};

#[derive(Clone)]
pub struct ToolAdapter {
    pub tool_id: &'static str,
    pub display_name: &'static str,
    pub detection_method: &'static str,
    pub launch_command: Option<&'static str>,
    pub item_types: &'static [&'static str],
    pub restore_modes: &'static [&'static str],
}

impl ToolAdapter {
    pub fn capabilities_json(&self) -> Value {
        json!({
            "item_types": self.item_types,
            "restore_modes": self.restore_modes,
        })
    }

    pub fn is_available(&self) -> bool {
        match (self.detection_method, self.launch_command) {
            ("builtin", _) => true,
            ("command", Some(command)) => command_exists(command),
            _ => false,
        }
    }
}

fn command_exists(command: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("where")
            .arg(command)
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }

    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("which")
            .arg(command)
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }
}

pub fn registry_for_platform(platform: &str) -> Vec<ToolAdapter> {
    let mut adapters = vec![
        ToolAdapter {
            tool_id: "vscode",
            display_name: "VS Code",
            detection_method: "command",
            launch_command: Some("code"),
            item_types: &["folder", "file"],
            restore_modes: &["automatic", "tracked_only", "manual"],
        },
        ToolAdapter {
            tool_id: "cursor",
            display_name: "Cursor",
            detection_method: "command",
            launch_command: Some("cursor"),
            item_types: &["folder", "file"],
            restore_modes: &["automatic", "tracked_only", "manual"],
        },
        ToolAdapter {
            tool_id: "zed",
            display_name: "Zed",
            detection_method: "command",
            launch_command: Some("zed"),
            item_types: &["folder", "file"],
            restore_modes: &["automatic", "tracked_only", "manual"],
        },
        ToolAdapter {
            tool_id: "nano",
            display_name: "Nano",
            detection_method: "command",
            launch_command: Some("nano"),
            item_types: &["file"],
            restore_modes: &["manual"],
        },
        ToolAdapter {
            tool_id: "ghostty",
            display_name: "Ghostty",
            detection_method: "command",
            launch_command: Some("ghostty"),
            item_types: &["folder", "file", "command", "terminal"],
            restore_modes: &["automatic", "manual"],
        },
        ToolAdapter {
            tool_id: "terminal",
            display_name: "System Terminal",
            detection_method: "builtin",
            launch_command: None,
            item_types: &["folder", "command"],
            restore_modes: &["automatic", "manual"],
        },
        ToolAdapter {
            tool_id: "browser",
            display_name: "Default Browser",
            detection_method: "builtin",
            launch_command: None,
            item_types: &["url", "link"],
            restore_modes: &["automatic", "manual"],
        },
        ToolAdapter {
            tool_id: "file_explorer",
            display_name: "File Explorer",
            detection_method: "builtin",
            launch_command: None,
            item_types: &["folder", "file"],
            restore_modes: &["automatic", "manual"],
        },
    ];

    if platform == "windows" {
        adapters.push(ToolAdapter {
            tool_id: "notepad",
            display_name: "Notepad",
            detection_method: "command",
            launch_command: Some("notepad"),
            item_types: &["file"],
            restore_modes: &["automatic", "manual"],
        });
    }

    adapters
}

pub fn launch_command_for_tool(platform: &str, tool_id: &str) -> Option<&'static str> {
    registry_for_platform(platform)
        .into_iter()
        .find(|adapter| adapter.tool_id == tool_id)
        .and_then(|adapter| adapter.launch_command)
}
