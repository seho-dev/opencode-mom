use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsoncError {
    MalformedJson,
}

impl std::fmt::Display for JsoncError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedJson => formatter.write_str("malformed JSONC"),
        }
    }
}

impl std::error::Error for JsoncError {}

pub fn parse_jsonc_object(input: &str) -> Result<Map<String, Value>, JsoncError> {
    let stripped = strip_comments(input);
    let value: Value = serde_json::from_str(&stripped).map_err(|_| JsoncError::MalformedJson)?;
    match value {
        Value::Object(object) => Ok(object),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Array(_) => {
            Err(JsoncError::MalformedJson)
        }
    }
}

fn strip_comments(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(character) = chars.next() {
        if in_string {
            output.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        if character == '"' {
            in_string = true;
            output.push(character);
            continue;
        }

        if character == '/' {
            match chars.peek().copied() {
                Some('/') => {
                    chars.next();
                    for next in chars.by_ref() {
                        if next == '\n' {
                            output.push('\n');
                            break;
                        }
                    }
                }
                Some('*') => {
                    chars.next();
                    let mut previous = '\0';
                    for next in chars.by_ref() {
                        if previous == '*' && next == '/' {
                            break;
                        }
                        previous = next;
                    }
                }
                Some(_) | None => output.push(character),
            }
            continue;
        }

        output.push(character);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::strip_comments;

    #[test]
    fn strip_comments_when_markers_are_inside_strings_preserves_them() {
        let input = r#"{"url":"https://example.invalid","code":"a // b","block":"/* c */"}"#;

        let stripped = strip_comments(input);

        assert!(stripped.contains("https://example.invalid"));
        assert!(stripped.contains("a // b"));
        assert!(stripped.contains("/* c */"));
    }
}
