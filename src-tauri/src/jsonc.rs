use jsonc_parser::cst::{CstInputValue, CstObject, CstRootNode};
use jsonc_parser::ParseOptions;
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsoncError {
    MalformedJson,
}
impl std::fmt::Display for JsoncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("malformed JSONC")
    }
}
impl std::error::Error for JsoncError {}

pub fn parse_jsonc_object(input: &str) -> Result<Map<String, Value>, JsoncError> {
    let root = CstRootNode::parse(input, &options()).map_err(|_| JsoncError::MalformedJson)?;
    match root.object_value().and_then(|value| value.to_serde_value()) {
        Some(Value::Object(object)) => Ok(object),
        _ => Err(JsoncError::MalformedJson),
    }
}

pub fn patch(input: &str, path: &[&str], value: Option<Value>) -> Result<String, JsoncError> {
    patch_with_effective(input, path, value).map(|(output, _)| output)
}

pub fn patch_with_effective(
    input: &str,
    path: &[&str],
    value: Option<Value>,
) -> Result<(String, Map<String, Value>), JsoncError> {
    if path.is_empty() || path.iter().any(|segment| segment.is_empty()) {
        return Err(JsoncError::MalformedJson);
    }
    let root = CstRootNode::parse(input, &options()).map_err(|_| JsoncError::MalformedJson)?;
    let object = root.object_value().ok_or(JsoncError::MalformedJson)?;
    patch_object(&object, path, value.clone());
    let output = root.to_string();
    let effective = parse_jsonc_object(&output)?;

    match &value {
        Some(expected) => {
            if get_path(&effective, path) != Some(expected) {
                return Err(JsoncError::MalformedJson);
            }
        }
        None => {
            if get_path(&effective, path).is_some() {
                return Err(JsoncError::MalformedJson);
            }
        }
    }

    Ok((output, effective))
}

fn get_path<'a>(mut current: &'a Map<String, Value>, path: &[&str]) -> Option<&'a Value> {
    for (i, segment) in path.iter().enumerate() {
        if i == path.len() - 1 {
            return current.get(*segment);
        }
        match current.get(*segment) {
            Some(Value::Object(map)) => current = map,
            _ => return None,
        }
    }
    None
}

fn patch_object(object: &CstObject, path: &[&str], value: Option<Value>) {
    if path.len() == 1 {
        match (object.get(path[0]), value) {
            (Some(prop), Some(value)) => prop.set_value(to_cst(value)),
            (Some(prop), None) => prop.remove(),
            (None, Some(value)) => {
                object.append(path[0], to_cst(value));
            }
            (None, None) => {}
        }
        return;
    }
    match value {
        Some(value) => {
            let next = object.object_value_or_set(path[0]);
            patch_object(&next, &path[1..], Some(value));
        }
        None => {
            if let Some(next) = object.object_value(path[0]) {
                patch_object(&next, &path[1..], None);
            }
        }
    }
}

fn to_cst(value: Value) -> CstInputValue {
    match value {
        Value::Null => CstInputValue::Null,
        Value::Bool(value) => CstInputValue::Bool(value),
        Value::Number(value) => CstInputValue::Number(value.to_string()),
        Value::String(value) => CstInputValue::String(value),
        Value::Array(values) => CstInputValue::Array(values.into_iter().map(to_cst).collect()),
        Value::Object(values) => CstInputValue::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, to_cst(value)))
                .collect(),
        ),
    }
}

fn options() -> ParseOptions {
    ParseOptions {
        allow_comments: true,
        allow_loose_object_property_names: false,
        allow_trailing_commas: true,
        allow_missing_commas: false,
        allow_single_quoted_strings: false,
        allow_hexadecimal_numbers: false,
        allow_unary_plus_numbers: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn duplicate_key_rejects_shadowed_modification() {
        let input = r#"{"x": 1, "x": 2}"#;
        let result = patch(input, &["x"], Some(json!(3)));
        assert_eq!(result, Err(JsoncError::MalformedJson));
    }

    #[test]
    fn duplicate_key_rejects_shadowed_deletion() {
        let input = r#"{"x": 1, "x": 2}"#;
        let result = patch(input, &["x"], None);
        assert_eq!(result, Err(JsoncError::MalformedJson));
    }

    #[test]
    fn normal_patch_and_patch_with_effective() {
        let input = "{\n  // keep comment\n  \"x\": 1\n}";
        let (output, effective) = patch_with_effective(input, &["x"], Some(json!(3))).unwrap();
        assert_eq!(output, "{\n  // keep comment\n  \"x\": 3\n}");
        assert_eq!(effective.get("x"), Some(&json!(3)));

        let (output_deleted, effective_deleted) =
            patch_with_effective(&output, &["x"], None).unwrap();
        assert_eq!(effective_deleted.get("x"), None);
        assert!(!output_deleted.contains("\"x\""));
    }

    #[test]
    fn nested_duplicate_key_rejects_shadowed_modification_and_deletion() {
        let input = r#"{"a": {"x": 1, "x": 2}}"#;
        let modify_result = patch(input, &["a", "x"], Some(json!(3)));
        assert_eq!(modify_result, Err(JsoncError::MalformedJson));

        let delete_result = patch(input, &["a", "x"], None);
        assert_eq!(delete_result, Err(JsoncError::MalformedJson));
    }
}
