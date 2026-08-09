use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

pub const BACKEND_COMMANDS: &[&str] = &[
    "copy_group",
    "delete_group",
    "discover_open_code_agents",
    "load_app_state",
    "save_group",
    "switch_group",
];

pub const PERMISSION_COMMANDS: &[&str] = BACKEND_COMMANDS;

pub const QUICK_SWITCH_COMMANDS: &[&str] = &["load_app_state", "switch_group"];

pub const SETTINGS_COMMANDS: &[&str] = BACKEND_COMMANDS;
pub const QUICK_SWITCH_PERMISSION_IDENTIFIERS: &[&str] = &["quick-switch-commands"];
pub const SETTINGS_PERMISSION_IDENTIFIERS: &[&str] = &["group-commands"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionManifest {
    permissions: BTreeMap<String, PermissionDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PermissionDefinition {
    allow: BTreeSet<String>,
    deny: BTreeSet<String>,
}

impl PermissionManifest {
    pub fn allowed_commands_for(
        &self,
        permission_identifiers: &[String],
    ) -> Result<BTreeSet<String>, String> {
        let mut seen_permissions = BTreeSet::new();
        let mut allowed = BTreeSet::new();
        let mut denied = BTreeSet::new();

        for identifier in permission_identifiers {
            if !seen_permissions.insert(identifier) {
                return Err(format!(
                    "permission reference {identifier:?} appears more than once"
                ));
            }
            let permission = self.permissions.get(identifier).ok_or_else(|| {
                format!(
                    "permission {identifier:?} is referenced by a capability but is not defined"
                )
            })?;
            allowed.extend(permission.allow.iter().cloned());
            denied.extend(permission.deny.iter().cloned());
        }

        allowed.retain(|command| !denied.contains(command));
        Ok(allowed)
    }

    fn all_allowed_commands(&self) -> Vec<String> {
        self.permissions
            .values()
            .flat_map(|permission| permission.allow.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityConfig {
    pub identifier: String,
    #[serde(default)]
    pub local: bool,
    #[serde(default)]
    pub windows: Vec<String>,
    pub permissions: Vec<String>,
}

impl CapabilityConfig {
    pub fn allowed_commands(
        &self,
        manifest: &PermissionManifest,
    ) -> Result<BTreeSet<String>, String> {
        manifest.allowed_commands_for(&self.permissions)
    }

    pub fn authorize_command(
        &self,
        manifest: &PermissionManifest,
        command: &str,
    ) -> Result<(), String> {
        let allowed = self.allowed_commands(manifest)?;
        if allowed.contains(command) {
            Ok(())
        } else {
            Err(format!(
                "capability {:?} does not authorize command {command:?}; permissions={:?}; allowed_commands={allowed:?}",
                self.identifier, self.permissions
            ))
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct TauriConfig {
    pub app: TauriAppConfig,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct TauriAppConfig {
    pub windows: Vec<TauriWindowConfig>,
    pub security: TauriSecurityConfig,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct TauriWindowConfig {
    pub label: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct TauriSecurityConfig {
    pub capabilities: Vec<String>,
}

pub fn load_capability_config(path: &Path) -> CapabilityConfig {
    let source = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("Given: capability config {} reads: {error}", path.display())
    });
    serde_json::from_str(&source).unwrap_or_else(|error| {
        panic!(
            "Given: capability config {} is valid JSON: {error}",
            path.display()
        )
    })
}

pub fn load_tauri_config(path: &Path) -> TauriConfig {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("Given: Tauri config {} reads: {error}", path.display()));
    serde_json::from_str(&source).unwrap_or_else(|error| {
        panic!(
            "Given: Tauri config {} is valid JSON: {error}",
            path.display()
        )
    })
}

pub fn parse_permission_manifest(source: &str) -> Result<PermissionManifest, String> {
    PermissionTomlParser::new(source)?.parse()
}

pub fn load_permission_manifest(path: &Path) -> PermissionManifest {
    let source = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("Given: permission config {} reads: {error}", path.display())
    });
    parse_permission_manifest(&source).unwrap_or_else(|error| {
        panic!(
            "Given: permission config {} parses: {error}",
            path.display()
        )
    })
}

struct PermissionTomlParser<'a> {
    source: &'a str,
    cursor: usize,
}

impl<'a> PermissionTomlParser<'a> {
    fn new(source: &'a str) -> Result<Self, String> {
        Ok(Self { source, cursor: 0 })
    }

    fn parse(mut self) -> Result<PermissionManifest, String> {
        let mut permissions = BTreeMap::new();

        while !self.is_finished() {
            self.expect("[[", "'[[' starting a permission table")?;
            if self.bare("permission table name")? != "permission" {
                return Err(format!(
                    "expected [[permission]] table at byte {}",
                    self.cursor
                ));
            }
            self.expect("]]", "closing ']]' for permission table")?;

            let mut identifier = None;
            let mut allow = None;
            let mut deny = None;
            while !self.is_finished() && !self.starts_with("[[") {
                let key = self.bare("permission property")?;
                self.expect("=", "'=' after permission property")?;
                match key.as_str() {
                    "identifier" => replace_once(&mut identifier, self.string()?, "identifier")?,
                    "description" => {
                        self.string()?;
                    }
                    "commands.allow" => replace_once(
                        &mut allow,
                        self.command_set("commands.allow")?,
                        "commands.allow",
                    )?,
                    "commands.deny" => replace_once(
                        &mut deny,
                        self.command_set("commands.deny")?,
                        "commands.deny",
                    )?,
                    _ => {
                        return Err(format!(
                            "unsupported permission property {key:?}; expected identifier, description, commands.allow, or commands.deny"
                        ));
                    }
                }
            }

            let identifier = identifier
                .ok_or_else(|| "[[permission]] table is missing its identifier".to_owned())?;
            let definition = PermissionDefinition {
                allow: allow.unwrap_or_default(),
                deny: deny.unwrap_or_default(),
            };
            if permissions.insert(identifier.clone(), definition).is_some() {
                return Err(format!(
                    "permission identifier {identifier:?} appears more than once"
                ));
            }
        }

        if permissions.is_empty() {
            Err("permission TOML defines no [[permission]] tables".to_owned())
        } else {
            Ok(PermissionManifest { permissions })
        }
    }

    fn command_set(&mut self, property: &str) -> Result<BTreeSet<String>, String> {
        self.expect("[", "'[' starting a command array")?;
        let mut commands = BTreeSet::new();
        if self.take_if("]") {
            return Ok(commands);
        }

        loop {
            let command = self.string()?;
            if !commands.insert(command.clone()) {
                return Err(format!(
                    "permission property {property:?} repeats command {command:?}"
                ));
            }
            if self.take_if("]") {
                return Ok(commands);
            }
            self.expect(",", "',' between command array values")?;
            if self.take_if("]") {
                return Ok(commands);
            }
        }
    }

    fn bare(&mut self, description: &str) -> Result<String, String> {
        self.skip_trivia();
        let start = self.cursor;
        while let Some(character) = self.current() {
            if character.is_whitespace() || matches!(character, '#' | '[' | ']' | '=' | ',' | '"') {
                break;
            }
            self.cursor += character.len_utf8();
        }
        if start == self.cursor {
            return Err(format!("expected {description} at byte {start}"));
        }
        Ok(self.source[start..self.cursor].to_owned())
    }

    fn string(&mut self) -> Result<String, String> {
        self.skip_trivia();
        let start = self.cursor;
        self.expect("\"", "a quoted TOML string")?;
        let mut value = String::new();
        while let Some(character) = self.current() {
            match character {
                '"' => {
                    self.cursor += 1;
                    return Ok(value);
                }
                '\\' => {
                    self.cursor += 1;
                    let escaped = self.current().ok_or_else(|| {
                        format!("unterminated TOML string escape starting at byte {start}")
                    })?;
                    value.push(match escaped {
                        '"' => '"',
                        '\\' => '\\',
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        _ => {
                            return Err(format!(
                                "unsupported TOML string escape \\{escaped} at byte {}",
                                self.cursor
                            ));
                        }
                    });
                    self.cursor += escaped.len_utf8();
                }
                '\n' | '\r' => {
                    return Err(format!(
                        "TOML basic string at byte {start} must not contain an unescaped newline"
                    ));
                }
                _ => {
                    value.push(character);
                    self.cursor += character.len_utf8();
                }
            }
        }
        Err(format!("unterminated TOML string starting at byte {start}"))
    }

    fn expect(&mut self, expected: &str, description: &str) -> Result<(), String> {
        self.skip_trivia();
        if self.starts_with(expected) {
            self.cursor += expected.len();
            Ok(())
        } else {
            Err(format!("expected {description} at byte {}", self.cursor))
        }
    }

    fn take_if(&mut self, expected: &str) -> bool {
        self.skip_trivia();
        if self.starts_with(expected) {
            self.cursor += expected.len();
            true
        } else {
            false
        }
    }

    fn skip_trivia(&mut self) {
        loop {
            while self.current().is_some_and(char::is_whitespace) {
                self.cursor += self.current().expect("character exists").len_utf8();
            }
            if self.starts_with("#") {
                while let Some(character) = self.current() {
                    self.cursor += character.len_utf8();
                    if character == '\n' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    fn starts_with(&self, expected: &str) -> bool {
        self.source[self.cursor..].starts_with(expected)
    }

    fn current(&self) -> Option<char> {
        self.source[self.cursor..].chars().next()
    }

    fn is_finished(&mut self) -> bool {
        self.skip_trivia();
        self.cursor == self.source.len()
    }
}

fn replace_once<T>(slot: &mut Option<T>, value: T, property: &str) -> Result<(), String> {
    if slot.replace(value).is_some() {
        Err(format!(
            "permission property {property:?} appears more than once"
        ))
    } else {
        Ok(())
    }
}

pub fn runtime_commands(source: &str) -> Vec<String> {
    let section = delimited(source, "tauri::generate_handler![", "])");
    section
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.trim_end_matches(','))
        .filter_map(|line| line.rsplit("::").next())
        .map(str::to_owned)
        .collect()
}

pub fn build_manifest_commands(source: &str) -> Vec<String> {
    quoted_strings(delimited(source, ".commands(&[", "])"))
}

pub fn permission_commands(source: &str) -> Vec<String> {
    parse_permission_manifest(source)
        .unwrap_or_else(|error| panic!("Given: permission TOML parses: {error}"))
        .all_allowed_commands()
}

pub fn autogenerated_permission_commands(directory: &Path) -> Vec<String> {
    let mut commands = BTreeSet::new();
    for entry in fs::read_dir(directory).expect("Given: autogenerated permission directory reads") {
        let entry = entry.expect("Given: autogenerated permission entry reads");
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("toml") {
            let source = fs::read_to_string(&path).unwrap_or_else(|error| {
                panic!(
                    "Given: autogenerated permission file {} reads: {error}",
                    path.display()
                )
            });
            commands.extend(
                parse_permission_manifest(&source)
                    .unwrap_or_else(|error| {
                        panic!(
                            "Given: autogenerated permission file {} parses: {error}",
                            path.display()
                        )
                    })
                    .all_allowed_commands(),
            );
        }
    }
    commands.into_iter().collect()
}

pub fn acl_manifest_commands(source: &str) -> Vec<String> {
    let manifest: Value =
        serde_json::from_str(source).expect("Given: generated ACL manifest is valid JSON");
    let permissions = manifest
        .get("__app-acl__")
        .and_then(|app_acl| app_acl.get("permissions"))
        .and_then(Value::as_object)
        .expect("Given: generated ACL manifest contains app permissions");

    permissions
        .values()
        .flat_map(|permission| {
            permission
                .get("commands")
                .and_then(|commands| commands.get("allow"))
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
        })
        .map(str::to_owned)
        .collect()
}

pub fn schema_commands(source: &str) -> Vec<String> {
    let schema: Value =
        serde_json::from_str(source).expect("Given: generated schema is valid JSON");
    let mut commands = Vec::new();
    collect_schema_commands(&schema, &mut commands);
    commands
}

pub fn exact_set_mismatch(surface: &str, actual: &[String], expected: &[&str]) -> Option<String> {
    let actual_set: BTreeSet<&str> = actual.iter().map(String::as_str).collect();
    let expected_set: BTreeSet<&str> = expected.iter().copied().collect();
    let missing: Vec<&str> = expected_set.difference(&actual_set).copied().collect();
    let unexpected: Vec<&str> = actual_set.difference(&expected_set).copied().collect();
    if missing.is_empty() && unexpected.is_empty() {
        None
    } else {
        Some(format!(
            "{surface} command contract mismatch; missing={missing:?}; unexpected={unexpected:?}"
        ))
    }
}

pub fn exact_string_set_mismatch(
    subject: &str,
    actual: &BTreeSet<String>,
    expected: &BTreeSet<String>,
) -> Option<String> {
    let missing: Vec<&String> = expected.difference(actual).collect();
    let unexpected: Vec<&String> = actual.difference(expected).collect();
    if missing.is_empty() && unexpected.is_empty() {
        None
    } else {
        Some(format!(
            "{subject} mismatch; missing={missing:?}; unexpected={unexpected:?}; expected={expected:?}; actual={actual:?}"
        ))
    }
}

fn delimited<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    source
        .split_once(start)
        .and_then(|(_, suffix)| suffix.split_once(end))
        .map(|(section, _)| section)
        .unwrap_or_else(|| {
            panic!("Given: command registration delimiters {start:?} and {end:?} exist")
        })
}

fn quoted_strings(source: &str) -> Vec<String> {
    let mut parts = source.split('"');
    let mut values = Vec::new();
    while let Some(_) = parts.next() {
        let Some(value) = parts.next() else {
            break;
        };
        values.push(value.to_owned());
    }
    values
}

fn collect_schema_commands(value: &Value, commands: &mut Vec<String>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_schema_commands(item, commands);
            }
        }
        Value::Object(object) => {
            let permission = object.get("const").and_then(Value::as_str);
            let description = object.get("description").and_then(Value::as_str);
            if permission.is_some_and(|name| name.starts_with("allow-") && !name.contains(':')) {
                if let Some(command) = description.and_then(command_from_description) {
                    commands.push(command.to_owned());
                }
            }
            for child in object.values() {
                collect_schema_commands(child, commands);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn command_from_description(description: &str) -> Option<&str> {
    description
        .strip_prefix("Enables the ")
        .and_then(|value| value.strip_suffix(" command without any pre-configured scope."))
}
