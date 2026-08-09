fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "load_app_state",
            "save_group",
            "copy_group",
            "delete_group",
            "switch_group",
            "discover_open_code_agents",
        ]),
    ))
    .expect("failed to build Tauri command manifest")
}
