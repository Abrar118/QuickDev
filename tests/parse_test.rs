use quickdev::parse::parse_shell_args;

#[test]
fn parse_shell_args_splits_quoted_value() {
    let result = parse_shell_args(r#"--profile "Dev User""#).unwrap();
    assert_eq!(result, vec!["--profile", "Dev User"]);
}

#[test]
fn parse_shell_args_plain_words() {
    let result = parse_shell_args("--flag value").unwrap();
    assert_eq!(result, vec!["--flag", "value"]);
}

#[test]
fn parse_shell_args_errors_on_unbalanced_quote() {
    let result = parse_shell_args(r#"--name "unterminated"#);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("invalid"));
}
