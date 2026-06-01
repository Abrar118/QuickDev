use quickdev::adapters::{
    infer_tool_id, infer_tool_id_from_path, is_editor_tool, launch_command_for_tool,
};

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
fn infer_tool_id_does_not_match_codex_as_vscode() {
    assert_eq!(infer_tool_id("Codex", "/Applications/Codex.app"), None);
}

#[test]
fn infer_tool_id_detects_vscode_by_path() {
    assert_eq!(
        infer_tool_id("Code", "/Applications/Visual Studio Code.app"),
        Some("vscode".to_string())
    );
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

#[test]
fn infer_tool_id_from_path_matches_real_tool_paths() {
    assert_eq!(
        infer_tool_id_from_path("/Applications/Visual Studio Code.app"),
        Some("vscode".to_string())
    );
    assert_eq!(
        infer_tool_id_from_path("/Applications/Cursor.app"),
        Some("cursor".to_string())
    );
    assert_eq!(
        infer_tool_id_from_path("/usr/local/bin/zed"),
        Some("zed".to_string())
    );
}

#[test]
fn infer_tool_id_from_path_ignores_wrapper_paths() {
    // A Flatpak/Snap/Squirrel wrapper path must NOT be identified as the tool,
    // even though the app's display name would match — otherwise the tool CLI
    // gets launched with the wrapper's args (e.g. `code run com.example.App`).
    assert_eq!(infer_tool_id_from_path("flatpak"), None);
    assert_eq!(infer_tool_id_from_path("/snap/bin/code-wrapper"), None);
    assert_eq!(
        infer_tool_id_from_path("C:\\Users\\me\\AppData\\Local\\App\\Update.exe"),
        None
    );
}

#[test]
fn infer_tool_id_still_matches_by_name_for_arg_defaults() {
    // The name+path variant (used only for arg defaulting, not CLI substitution)
    // continues to match on name.
    assert_eq!(
        infer_tool_id("Visual Studio Code", "flatpak"),
        Some("vscode".to_string())
    );
}
