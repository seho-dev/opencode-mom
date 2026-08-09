use serde_json::{Map, Value};

pub use crate::core::error::DocumentError;
use crate::core::jsonc::{parse_jsonc_object, JsoncError};

const OH_MY_BOOTSTRAP_SCHEMA: &str =
    "https://raw.githubusercontent.com/code-yeongyu/oh-my-openagent/dev/assets/oh-my-opencode.schema.json";

impl From<JsoncError> for DocumentError {
    fn from(error: JsoncError) -> Self {
        match error {
            JsoncError::MalformedJson => Self::MalformedJson,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OhMyOpenAgentDocument {
    raw: Map<String, Value>,
}

impl OhMyOpenAgentDocument {
    pub fn from_raw(raw: Map<String, Value>) -> Self {
        Self { raw }
    }

    pub fn bootstrap() -> Self {
        let mut raw = Map::new();
        raw.insert(
            "$schema".to_owned(),
            Value::String(OH_MY_BOOTSTRAP_SCHEMA.to_owned()),
        );
        raw.insert("agents".to_owned(), Value::Object(Map::new()));
        raw.insert("categories".to_owned(), Value::Object(Map::new()));
        Self { raw }
    }

    pub fn parse_jsonc(input: &str) -> Result<Self, DocumentError> {
        Ok(Self {
            raw: parse_jsonc_object(input)?,
        })
    }

    pub fn serialize(&self) -> Result<String, DocumentError> {
        serde_json::to_string_pretty(&self.raw)
            .map(|json| json.replace("\\/", "/"))
            .map_err(|_| DocumentError::MalformedJson)
    }

    pub const fn raw(&self) -> &Map<String, Value> {
        &self.raw
    }

    pub fn agents(&self) -> &Map<String, Value> {
        object_field(&self.raw, "agents")
    }

    pub fn categories(&self) -> &Map<String, Value> {
        object_field(&self.raw, "categories")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpenCodeDocument {
    raw: Map<String, Value>,
}

impl OpenCodeDocument {
    pub fn from_raw(raw: Map<String, Value>) -> Self {
        Self { raw }
    }

    pub fn parse_jsonc(input: &str) -> Result<Self, DocumentError> {
        Ok(Self {
            raw: parse_jsonc_object(input)?,
        })
    }

    pub fn serialize(&self) -> Result<String, DocumentError> {
        serde_json::to_string_pretty(&self.raw)
            .map(|json| json.replace("\\/", "/"))
            .map_err(|_| DocumentError::MalformedJson)
    }

    pub const fn raw(&self) -> &Map<String, Value> {
        &self.raw
    }

    pub fn agents(&self) -> &Map<String, Value> {
        object_field(&self.raw, "agent")
    }
}

fn object_field<'a>(raw: &'a Map<String, Value>, key: &str) -> &'a Map<String, Value> {
    match raw.get(key) {
        Some(Value::Object(object)) => object,
        Some(
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Array(_),
        )
        | None => empty_object(),
    }
}

fn empty_object() -> &'static Map<String, Value> {
    static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
    EMPTY.get_or_init(Map::new)
}
