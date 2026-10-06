use super::*;
use serde_json::json;

#[test]
fn legacy_config_without_preferences_yields_defaults() {
    let config: AppConfig = serde_json::from_value(json!({
        "groups": [],
        "state": { "selectedGroupID": null, "selectedGroupName": null }
    }))
    .expect("legacy config should deserialize");
    assert_eq!(config.state.preferences, AppPreferences::default());
    assert_eq!(config.state.preferences.theme, ThemePreference::Dark);
    assert_eq!(config.state.preferences.locale, LocalePreference::En);
}

#[test]
fn explicit_preferences_round_trip_unchanged() {
    let config: AppConfig = serde_json::from_value(json!({
        "groups": [],
        "state": {
            "selectedGroupID": null,
            "selectedGroupName": null,
            "preferences": { "theme": "light", "locale": "zh" }
        }
    }))
    .expect("config with preferences should deserialize");
    let encoded = serde_json::to_string(&config).expect("serialize config");
    let decoded: AppConfig = serde_json::from_str(&encoded).expect("deserialize config");
    assert_eq!(decoded, config);
    assert_eq!(decoded.state.preferences.theme, ThemePreference::Light);
    assert_eq!(decoded.state.preferences.locale, LocalePreference::Zh);
}

#[test]
fn default_config_serializes_preferences() {
    let value = serde_json::to_value(AppConfig::default()).expect("serialize default config");
    assert_eq!(
        value["state"]["preferences"],
        json!({ "theme": "dark", "locale": "en" })
    );
}

#[test]
fn empty_object_deserializes_to_defaults() {
    let config: AppConfig = serde_json::from_value(json!({})).expect("empty object");
    assert_eq!(config, AppConfig::default());
}
