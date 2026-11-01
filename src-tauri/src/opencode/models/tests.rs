use super::*;
use crate::opencode::token_usage::tests::{connection, healthy_response, respond, FakeApi};

#[test]
fn model_route_uses_provider_model_identity_and_preserves_disabled_config_metadata() {
    let home = std::env::temp_dir().join(format!("mom-models-{}", uuid::Uuid::new_v4()));
    let paths = ConfigPaths::for_home(&home);
    std::fs::create_dir_all(paths.opencode_file().parent().unwrap()).unwrap();
    std::fs::write(paths.opencode_file(), br#"{"providers":{"custom":{"models":{"disabled":{"id":"disabled","name":"Configured disabled","disabled":true,"limit":{"context":123}}}}}}"#).unwrap();
    let api = FakeApi::new(|target, stream| {
        if target == "/api/info" {
            healthy_response(stream);
        } else {
            assert!(target.starts_with("/api/model"));
            respond(stream, 200, "", br#"{"location":{"directory":"/fixture"},"data":[{"id":"unrelated","providerID":"api","modelID":"enabled","enabled":true},{"id":"other","providerID":"api","modelID":"disabled","enabled":false}]}"#);
        }
    });
    let connection = connection(&api);
    let catalog = list_models(
        &paths,
        None,
        &connection,
        Instant::now() + Duration::from_secs(1),
    )
    .unwrap();
    assert_eq!(
        catalog
            .iter()
            .map(|model| model.model_ref.as_str())
            .collect::<Vec<_>>(),
        ["api/enabled", "custom/disabled"]
    );
    assert_eq!(catalog[1].name, "Configured disabled");
    assert_eq!(catalog[1].limit.as_ref().unwrap()["context"], 123);
    assert_eq!(
        list_models(
            &paths,
            Some("custom"),
            &connection,
            Instant::now() + Duration::from_secs(1)
        )
        .unwrap()
        .len(),
        1
    );
    std::fs::remove_dir_all(home).unwrap();
}
