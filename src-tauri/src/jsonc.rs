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
    if path.is_empty() || path.iter().any(|segment| segment.is_empty()) {
        return Err(JsoncError::MalformedJson);
    }
    let root = CstRootNode::parse(input, &options()).map_err(|_| JsoncError::MalformedJson)?;
    let object = root.object_value().ok_or(JsoncError::MalformedJson)?;
    patch_object(&object, path, value);
    let output = root.to_string();
    parse_jsonc_object(&output)?;
    Ok(output)
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
