use quickdev::ui;

#[test]
fn pad_ignores_ansi_codes_when_measuring_width() {
    let padded = anstream::adapter::strip_str(&ui::pad(ui::BOLD, "api", 6)).to_string();
    assert_eq!(padded, "api   ");
}

#[test]
fn tilde_abbreviates_only_paths_under_home() {
    let home = dirs::home_dir().unwrap();
    let sep = std::path::MAIN_SEPARATOR;
    assert_eq!(ui::tilde(&home.to_string_lossy()), "~");
    assert_eq!(
        ui::tilde(&home.join("code").to_string_lossy()),
        format!("~{sep}code")
    );
    assert_eq!(ui::tilde("/elsewhere/code"), "/elsewhere/code");
}

#[test]
fn count_pluralizes() {
    assert_eq!(ui::count(1, "item"), "1 item");
    assert_eq!(ui::count(0, "item"), "0 items");
}
