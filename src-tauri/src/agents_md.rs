use std::collections::BTreeSet;

use serde_json::{Map, Value};

/// A parsed Markdown agent document. Serialization preserves untouched frontmatter
/// sections verbatim so user formatting survives edits.
#[derive(Debug, Clone, PartialEq)]
pub struct MarkdownAgentDocument {
    pub frontmatter: Map<String, Value>,
    pub prompt: String,
    original: String,
    original_frontmatter: Option<String>,
    original_prompt: String,
    key_order: Vec<String>,
    changed_fields: BTreeSet<String>,
}

impl MarkdownAgentDocument {
    pub fn empty() -> Self {
        Self {
            frontmatter: Map::new(),
            prompt: String::new(),
            original: "---\n---\n".to_owned(),
            original_frontmatter: Some(String::new()),
            original_prompt: String::new(),
            key_order: Vec::new(),
            changed_fields: BTreeSet::new(),
        }
    }

    pub fn parse(input: &str) -> Result<Self, crate::error::AppError> {
        let input = input.strip_prefix('\u{feff}').unwrap_or(input);
        let Some((frontmatter, prompt)) = split_frontmatter(input) else {
            return Ok(Self {
                frontmatter: Map::new(),
                prompt: input.to_owned(),
                original: input.to_owned(),
                original_frontmatter: None,
                original_prompt: input.to_owned(),
                key_order: Vec::new(),
                changed_fields: BTreeSet::new(),
            });
        };
        let original_frontmatter = frontmatter.to_owned();
        let frontmatter_value = if frontmatter.trim().is_empty() {
            serde_yaml::Value::Mapping(Default::default())
        } else {
            serde_yaml::from_str(frontmatter).map_err(|error| {
                crate::error::AppError::validation(format!("invalid YAML frontmatter: {error}"))
            })?
        };
        let Value::Object(frontmatter) = yaml_to_json(frontmatter_value)? else {
            return Err(crate::error::AppError::validation(
                "Markdown frontmatter must be an object",
            ));
        };

        Ok(Self {
            key_order: frontmatter_key_order(&original_frontmatter, frontmatter.keys()),
            frontmatter,
            prompt: prompt.to_owned(),
            original: input.to_owned(),
            original_frontmatter: Some(original_frontmatter),
            original_prompt: prompt.to_owned(),
            changed_fields: BTreeSet::new(),
        })
    }

    pub fn apply_fields(
        &mut self,
        fields: &Map<String, Value>,
        prompt: Option<String>,
        allow_existing_tools: bool,
    ) -> Result<(), crate::error::AppError> {
        for (key, value) in fields {
            if key == "prompt" {
                continue;
            }
            if key == "tools" && !allow_existing_tools && !self.frontmatter.contains_key(key) {
                continue;
            }
            if !self.frontmatter.contains_key(key) {
                self.key_order.push(key.clone());
            }
            self.frontmatter.insert(key.clone(), value.clone());
            self.changed_fields.insert(key.clone());
        }
        if let Some(prompt) = prompt {
            self.prompt = prompt;
        }
        Ok(())
    }

    pub fn remove_fields(&mut self, fields: &[String]) {
        for field in fields {
            if field != "prompt" && self.frontmatter.remove(field).is_some() {
                self.changed_fields.insert(field.clone());
            }
        }
    }

    /// Renders the document. Unchanged documents round-trip to their original text.
    pub fn serialize(&self) -> String {
        if self.prompt == self.original_prompt
            && self.original_frontmatter.as_ref().is_some_and(|original| {
                parse_frontmatter_object(original).is_ok_and(|fields| fields == self.frontmatter)
            })
        {
            return self.original.clone();
        }

        let mut output = String::from("---\n");
        output.push_str(&render_frontmatter(
            &self.frontmatter,
            &self.key_order,
            self.original_frontmatter.as_deref(),
            &self.changed_fields,
        ));
        output.push_str("---\n");
        output.push_str(&self.prompt);
        output
    }
}

fn split_frontmatter(input: &str) -> Option<(&str, &str)> {
    let Some(after_open) = input
        .strip_prefix("---\n")
        .or_else(|| input.strip_prefix("---\r\n"))
    else {
        return None;
    };
    let start_offset = input.len() - after_open.len();
    let mut offset = start_offset;
    for line in after_open.split_inclusive(['\n']) {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if matches!(trimmed, "---" | "...") {
            return Some((&input[start_offset..offset], &input[offset + line.len()..]));
        }
        offset += line.len();
    }
    None
}

fn parse_frontmatter_object(source: &str) -> Result<Map<String, Value>, crate::error::AppError> {
    let yaml: serde_yaml::Value = if source.trim().is_empty() {
        serde_yaml::Value::Mapping(Default::default())
    } else {
        serde_yaml::from_str(source).map_err(|error| {
            crate::error::AppError::validation(format!("invalid YAML frontmatter: {error}"))
        })?
    };
    let Value::Object(fields) = yaml_to_json(yaml)? else {
        return Err(crate::error::AppError::validation(
            "Markdown frontmatter must be an object",
        ));
    };
    Ok(fields)
}

fn yaml_to_json(value: serde_yaml::Value) -> Result<Value, crate::error::AppError> {
    match value {
        serde_yaml::Value::Null => Ok(Value::Null),
        serde_yaml::Value::Bool(value) => Ok(Value::Bool(value)),
        serde_yaml::Value::Number(value) => serde_json::to_value(value)
            .map_err(|error| crate::error::AppError::validation(error.to_string())),
        serde_yaml::Value::String(value) => Ok(Value::String(value)),
        serde_yaml::Value::Sequence(values) => values
            .into_iter()
            .map(yaml_to_json)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        serde_yaml::Value::Mapping(values) => {
            let mut object = Map::new();
            for (key, value) in values {
                let serde_yaml::Value::String(key) = key else {
                    return Err(crate::error::AppError::validation(
                        "Markdown frontmatter must be an object",
                    ));
                };
                object.insert(key, yaml_to_json(value)?);
            }
            Ok(Value::Object(object))
        }
        serde_yaml::Value::Tagged(value) => yaml_to_json(value.value),
    }
}

fn frontmatter_key_order<'a>(source: &str, keys: impl Iterator<Item = &'a String>) -> Vec<String> {
    let available = keys.cloned().collect::<BTreeSet<_>>();
    let mut ordered = Vec::new();
    for line in source.lines() {
        if line.starts_with([' ', '\t', '-']) {
            continue;
        }
        let Some((key, _)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        if available.contains(key) && !ordered.iter().any(|known| known == key) {
            ordered.push(key.to_owned());
        }
    }
    for key in available {
        if !ordered.contains(&key) {
            ordered.push(key);
        }
    }
    ordered
}

fn render_frontmatter(
    fields: &Map<String, Value>,
    order: &[String],
    original: Option<&str>,
    changed_fields: &BTreeSet<String>,
) -> String {
    match original {
        Some(original) => render_frontmatter_preserving(original, fields, order, changed_fields),
        None => render_all_frontmatter(fields, order),
    }
}

fn render_frontmatter_preserving(
    original: &str,
    fields: &Map<String, Value>,
    order: &[String],
    changed_fields: &BTreeSet<String>,
) -> String {
    let mut output = String::new();
    let mut emitted = BTreeSet::new();
    for section in frontmatter_sections(original) {
        if section.key.is_empty() {
            output.push_str(&section.source);
            continue;
        }
        let Some(value) = fields.get(&section.key) else {
            continue;
        };
        emitted.insert(section.key.clone());
        if changed_fields.contains(&section.key) {
            render_key_value(&mut output, &section.key, value, 0);
        } else {
            output.push_str(&section.source);
        }
    }
    for key in order.iter().chain(fields.keys()) {
        if emitted.insert(key.clone()) {
            if let Some(value) = fields.get(key) {
                render_key_value(&mut output, key, value, 0);
            }
        }
    }
    output
}

fn render_all_frontmatter(fields: &Map<String, Value>, order: &[String]) -> String {
    let mut output = String::new();
    let mut emitted = BTreeSet::new();
    for key in order.iter().chain(fields.keys()) {
        if !emitted.insert(key.clone()) {
            continue;
        }
        if let Some(value) = fields.get(key) {
            render_key_value(&mut output, key, value, 0);
        }
    }
    output
}

struct FrontmatterSection {
    key: String,
    source: String,
}

fn frontmatter_sections(source: &str) -> Vec<FrontmatterSection> {
    let mut sections = Vec::new();
    let mut current_key = String::new();
    let mut current_source = String::new();
    for line in source.split_inclusive('\n') {
        if let Some(key) = top_level_key(line) {
            if !current_source.is_empty() {
                sections.push(FrontmatterSection {
                    key: current_key,
                    source: current_source,
                });
            }
            current_key = key;
            current_source = line.to_owned();
        } else {
            current_source.push_str(line);
        }
    }
    if !current_source.is_empty() {
        sections.push(FrontmatterSection {
            key: current_key,
            source: current_source,
        });
    }
    sections
}

fn top_level_key(line: &str) -> Option<String> {
    if line.starts_with([' ', '\t', '#', '-']) {
        return None;
    }
    let (key, _) = line.split_once(':')?;
    let key = key.trim();
    (!key.is_empty()
        && key
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_')))
    .then(|| key.to_owned())
}

fn render_key_value(output: &mut String, key: &str, value: &Value, indent: usize) {
    let prefix = " ".repeat(indent);
    match value {
        Value::Array(values) if values.is_empty() => {
            output.push_str(&format!("{prefix}{key}: []\n"))
        }
        Value::Object(values) if values.is_empty() => {
            output.push_str(&format!("{prefix}{key}: {{}}\n"))
        }
        Value::Array(_) | Value::Object(_) => {
            output.push_str(&format!("{prefix}{key}:\n"));
            render_value(output, value, indent + 2);
        }
        _ => output.push_str(&format!("{prefix}{key}: {}\n", render_scalar(value))),
    }
}

fn render_value(output: &mut String, value: &Value, indent: usize) {
    match value {
        Value::Array(values) => {
            for value in values {
                let prefix = " ".repeat(indent);
                match value {
                    Value::Array(_) | Value::Object(_) => {
                        output.push_str(&format!("{prefix}-\n"));
                        render_value(output, value, indent + 2);
                    }
                    _ => output.push_str(&format!("{prefix}- {}\n", render_scalar(value))),
                }
            }
        }
        Value::Object(values) => {
            for (key, value) in values {
                render_key_value(output, key, value, indent);
            }
        }
        _ => output.push_str(&format!("{}{}\n", " ".repeat(indent), render_scalar(value))),
    }
}

fn render_scalar(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => {
            serde_json::to_string(value).expect("string serialization is infallible")
        }
        Value::Array(_) | Value::Object(_) => {
            unreachable!("complex YAML values are rendered structurally")
        }
    }
}
