use quickdev::fzf::{check_fzf, fzf_install_hint, is_cancellation, CANCELLED};

#[test]
fn check_fzf_returns_bool() {
    let _available = check_fzf();
}

#[test]
fn install_hint_contains_instructions() {
    let hint = fzf_install_hint();
    assert!(
        hint.contains("brew") || hint.contains("apt") || hint.contains("choco"),
        "install hint should mention a package manager, got: {hint}"
    );
}

#[test]
fn install_hint_mentions_fzf() {
    let hint = fzf_install_hint();
    assert!(hint.contains("fzf"), "install hint should mention fzf");
}

#[test]
fn is_cancellation_matches_only_sentinel() {
    assert!(is_cancellation(CANCELLED));
    assert!(!is_cancellation("some other error"));
    assert!(!is_cancellation("selection cancelled"));
}

#[test]
fn indexed_selection_recovers_positions_not_display_text() {
    use quickdev::fzf::parse_indexed_selection;

    // The display half may contain anything — including the " — " separator the
    // old parser split on, and a tab of its own.
    let selected = vec![
        "0\t[terminal] a — b — c".to_string(),
        "2\t[app] name\twith tab".to_string(),
    ];
    assert_eq!(parse_indexed_selection(&selected, 3).unwrap(), vec![0, 2]);
}

#[test]
fn indexed_selection_rejects_rows_it_cannot_map() {
    use quickdev::fzf::parse_indexed_selection;

    // No index column at all.
    assert!(parse_indexed_selection(&["[terminal] a".to_string()], 3).is_err());
    // Index outside the presented list.
    assert!(parse_indexed_selection(&["9\t[terminal] a".to_string()], 3).is_err());
    // Non-numeric index.
    assert!(parse_indexed_selection(&["x\t[terminal] a".to_string()], 3).is_err());
}

#[test]
fn sanitize_row_removes_characters_that_would_forge_picker_rows() {
    use quickdev::fzf::sanitize_row;

    // A newline would become an extra row; a tab would shift the index column.
    assert_eq!(sanitize_row("api\nlaunch evil"), "apilaunch evil");
    assert_eq!(sanitize_row("a\tb"), "ab");
    assert_eq!(sanitize_row("normal — name"), "normal — name");
}
