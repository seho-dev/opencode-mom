use std::collections::BTreeSet;

pub const BACKEND_COMMANDS: &[&str] = &[
    "copy_group",
    "delete_group",
    "discover_open_code_agents",
    "load_app_state",
    "save_group",
    "switch_group",
];

pub fn runtime_commands(source: &str) -> Vec<String> {
    let section = source
        .split_once("tauri::generate_handler![")
        .and_then(|(_, suffix)| suffix.split_once("])"))
        .map(|(section, _)| section)
        .expect("Given: Tauri handler registration delimiters exist");

    section
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.trim_end_matches(','))
        .filter_map(|line| line.rsplit("::").next())
        .map(str::to_owned)
        .collect()
}

pub fn exact_set_mismatch(surface: &str, actual: &[String], expected: &[&str]) -> Option<String> {
    let actual_set: BTreeSet<&str> = actual.iter().map(String::as_str).collect();
    let expected_set: BTreeSet<&str> = expected.iter().copied().collect();
    let missing: Vec<&str> = expected_set.difference(&actual_set).copied().collect();
    let unexpected: Vec<&str> = actual_set.difference(&expected_set).copied().collect();
    if missing.is_empty() && unexpected.is_empty() {
        None
    } else {
        Some(format!(
            "{surface} command contract mismatch; missing={missing:?}; unexpected={unexpected:?}"
        ))
    }
}
