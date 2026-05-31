/// Split a string into arguments, honoring shell-style quoting.
pub fn parse_shell_args(input: &str) -> Result<Vec<String>, String> {
    shell_words::split(input).map_err(|e| format!("invalid shell string: {e}"))
}
