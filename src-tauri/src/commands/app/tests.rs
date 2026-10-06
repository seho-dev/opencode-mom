use super::*;

#[test]
fn app_state_serializes_selected_group_id_as_camel_case_or_null() {
    for selected_group_id in [Some(Uuid::from_u128(1)), None] {
        let response = AppStateResponse {
            providers: Vec::new(),
            agents: Vec::new(),
            groups: Vec::new(),
            selected_group_id,
            preferences: crate::models::AppPreferences::default(),
        };
        let value = serde_json::to_value(response).unwrap();
        assert_eq!(
            value.get("selectedGroupId"),
            Some(&serde_json::to_value(selected_group_id).unwrap())
        );
        assert!(value.get("selectedGroupID").is_none());
        assert!(value.get("selected_group_id").is_none());
    }
}
