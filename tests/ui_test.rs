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

#[test]
fn closest_suggests_only_similar_names() {
    let names = ["my-api", "web", "inventory"];
    assert_eq!(ui::closest("my-ap", names), Some("my-api"));
    assert_eq!(ui::closest("invetory", names), Some("inventory"));
    assert_eq!(ui::closest("zzz", names), None);
}

#[test]
fn ago_rounds_down_to_the_largest_unit() {
    assert_eq!(ui::ago(1_000, 1_030), "just now");
    assert_eq!(ui::ago(1_000, 1_000 + 5 * 60 + 59), "5m ago");
    assert_eq!(ui::ago(1_000, 1_000 + 3 * 3_600), "3h ago");
    assert_eq!(ui::ago(1_000, 1_000 + 2 * 86_400), "2d ago");
    assert_eq!(ui::ago(2_000, 1_000), "just now", "future timestamps clamp");
}
