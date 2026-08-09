use std::fs;
use std::path::Path;

use omo_switch_tauri::core::document::{OhMyOpenAgentDocument, OpenCodeDocument};
use omo_switch_tauri::core::jsonc::{parse_jsonc_object, JsoncError};

fn fixture(relative_path: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Given: src-tauri has workspace parent")
        .join("fixtures")
        .join("compatibility")
        .join(relative_path);
    fs::read_to_string(path).expect("Given: fixture reads")
}

#[test]
fn jsonc_when_comments_and_block_comments_are_present_parses_object() {
    let jsonc = fixture("json/with-jsonc-markers.jsonc");

    let value = parse_jsonc_object(&jsonc).expect("When: JSONC object parses");

    assert!(value.get("provider").is_some());
    assert!(value.get("agents").is_some());
    assert!(value.get("categories").is_some());
}

#[test]
fn jsonc_when_comment_like_content_is_inside_strings_preserves_string_values() {
    let jsonc = r#"{
  "url": "https://example.invalid/path",
  "line": "a // b",
  "block": "/* not a comment */"
}"#;

    let value = parse_jsonc_object(jsonc).expect("When: JSONC object parses");

    assert_eq!(value["url"], "https://example.invalid/path");
    assert_eq!(value["line"], "a // b");
    assert_eq!(value["block"], "/* not a comment */");
}

#[test]
fn jsonc_when_input_is_malformed_returns_malformed_error() {
    let jsonc = "{ // comment without closing brace";

    let error = parse_jsonc_object(jsonc).expect_err("When: malformed JSONC fails");

    assert_eq!(error, JsoncError::MalformedJson);
}

#[test]
fn jsonc_when_top_level_is_not_object_returns_malformed_error() {
    let jsonc = "[1, 2, 3]";

    let error = parse_jsonc_object(jsonc).expect_err("When: non-object JSONC fails");

    assert_eq!(error, JsoncError::MalformedJson);
}

#[test]
fn jsonc_document_when_unknown_fields_round_trip_preserves_unrelated_top_level_data() {
    let json = fixture("json/with-unknown-fields.json");

    let document = OhMyOpenAgentDocument::parse_jsonc(&json).expect("When: document parses");
    let serialized = document.serialize().expect("When: document serializes");
    let reparsed = OhMyOpenAgentDocument::parse_jsonc(&serialized).expect("When: output reparses");

    assert_eq!(reparsed.raw()["knownField"], "value");
    assert_eq!(reparsed.raw()["unknownField"], "should be preserved");
    assert_eq!(reparsed.raw()["_customMarker"], "保留的未知字段");
    assert_eq!(reparsed.raw()["someExtraData"]["numbers"][2], 3);
}

#[test]
fn jsonc_document_when_canonical_save_output_is_written_reparses_for_both_targets() {
    let oh_my = OhMyOpenAgentDocument::parse_jsonc(&fixture("ohmy/current-oh-my-openagent.json"))
        .expect("Given: Oh My fixture parses");
    let opencode = OpenCodeDocument::parse_jsonc(&fixture("opencode/current-opencode.json"))
        .expect("Given: OpenCode fixture parses");

    let oh_my_serialized = oh_my.serialize().expect("When: Oh My serializes");
    let opencode_serialized = opencode.serialize().expect("When: OpenCode serializes");

    let oh_my_round_trip =
        OhMyOpenAgentDocument::parse_jsonc(&oh_my_serialized).expect("Then: Oh My output reparses");
    let opencode_round_trip = OpenCodeDocument::parse_jsonc(&opencode_serialized)
        .expect("Then: OpenCode output reparses");
    assert!(oh_my_round_trip.agents().contains_key("oracle"));
    assert!(oh_my_round_trip.categories().contains_key("quick"));
    assert!(opencode_round_trip.agents().contains_key("karen"));
    assert!(opencode_round_trip.raw().get("provider").is_some());
    assert!(!opencode_serialized.contains("\\/"));
}
