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
