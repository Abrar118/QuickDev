use quickdev::adapters::{infer_tool_id, is_editor_tool, launch_command_for_tool};

#[test]
fn infer_tool_id_detects_cursor() {
    assert_eq!(
        infer_tool_id("Cursor", "/Applications/Cursor.app"),
        Some("cursor".to_string())
    );
}

#[test]
fn infer_tool_id_detects_vscode() {
    assert_eq!(
        infer_tool_id("Visual Studio Code", "/Applications/Visual Studio Code.app"),
        Some("vscode".to_string())
    );
}

#[test]
fn infer_tool_id_detects_zed() {
    assert_eq!(
        infer_tool_id("Zed", "/usr/local/bin/zed"),
        Some("zed".to_string())
    );
}

#[test]
fn infer_tool_id_returns_none_for_unknown() {
    assert_eq!(infer_tool_id("My Custom App", "/usr/local/bin/myapp"), None);
}

#[test]
fn is_editor_tool_recognizes_editors() {
    assert!(is_editor_tool("vscode"));
    assert!(is_editor_tool("cursor"));
    assert!(is_editor_tool("zed"));
    assert!(!is_editor_tool("ghostty"));
    assert!(!is_editor_tool("unknown"));
}

#[test]
fn launch_command_for_known_tools() {
    assert_eq!(launch_command_for_tool("macos", "vscode"), Some("code"));
    assert_eq!(launch_command_for_tool("linux", "cursor"), Some("cursor"));
    assert_eq!(launch_command_for_tool("macos", "ghostty"), Some("ghostty"));
}

#[test]
fn launch_command_for_unknown_tool() {
    assert_eq!(launch_command_for_tool("macos", "unknown_tool"), None);
}
