use super::*;

#[test]
fn default_and_malformed_preferences_never_enable() {
    let directory =
        std::env::temp_dir().join(format!("mom-power-settings-{}", uuid::Uuid::new_v4()));
    assert!(!load(&directory).unwrap().enabled);
    save(&directory, true).unwrap();
    assert!(load(&directory).unwrap().enabled);
    save(&directory, false).unwrap();
    assert!(!load(&directory).unwrap().enabled);
    for contents in [
        "",
        "{",
        "{}",
        "{\"enabled\":\"true\"}",
        "{\"enabled\":true,\"extra\":1}",
    ] {
        fs::write(directory.join("power.json"), contents).unwrap();
        assert!(load(&directory).is_err());
    }
    assert!(!directory.join("config.json").exists());
    fs::remove_dir_all(directory).unwrap();
}
