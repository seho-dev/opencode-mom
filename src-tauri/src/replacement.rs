use serde_json::Value;

use crate::document::JsoncDoc;
use crate::error::AppError;
use crate::paths::ConfigPaths;
use crate::providers::ModelRef;

// OMO stores OpenCode-specific mappings under the literal `opencode` object.
const OMO_OPENCODE_BLOCK: &str = "opencode";

/// Scans Slim and OMO files for model references used by deletion protection.
pub fn scan_plugin_references(
    paths: &ConfigPaths,
) -> Result<Vec<crate::refs::ModelReference>, AppError> {
    let mut references = Vec::new();
    if let Some(content) = crate::document::read_text(&paths.slim_file())? {
        let document = JsoncDoc::parse(&content)?;
        scan_slim(&document, &mut references)?;
    }
    if let Some(content) = crate::document::read_text(&paths.omo_file())? {
        let document = JsoncDoc::parse(&content)?;
        scan_omo(&document, &mut references)?;
    }
    references.sort();
    Ok(references)
}

fn scan_slim(
    document: &JsoncDoc,
    references: &mut Vec<crate::refs::ModelReference>,
) -> Result<(), AppError> {
    let Some(presets) = document.raw().get("presets") else {
        return Ok(());
    };
    let presets = presets
        .as_object()
        .ok_or_else(|| AppError::validation("Slim presets must be an object"))?;
    for (preset, entries) in presets {
        let entries = entries.as_object().ok_or_else(|| {
            AppError::validation(format!("Slim presets.{preset} must be an object"))
        })?;
        for (agent, entry) in entries {
            let entry = entry.as_object().ok_or_else(|| {
                AppError::validation(format!("Slim presets.{preset}.{agent} must be an object"))
            })?;
            if let Some(model) = entry.get("model").and_then(Value::as_str) {
                if !model.is_empty() {
                    references.push(crate::refs::ModelReference {
                        source: crate::refs::ReferenceSource::SlimPreset,
                        location: format!("presets.{preset}.{agent}.model"),
                        model_ref: ModelRef::parse(model)?,
                    });
                }
            }
        }
    }
    Ok(())
}

fn scan_omo(
    document: &JsoncDoc,
    references: &mut Vec<crate::refs::ModelReference>,
) -> Result<(), AppError> {
    let Some(opencode) = document.raw().get(OMO_OPENCODE_BLOCK) else {
        return Ok(());
    };
    let opencode = opencode
        .as_object()
        .ok_or_else(|| AppError::validation("OMO opencode must be an object"))?;
    for (key, source) in [
        ("agents", crate::refs::ReferenceSource::OmoAgent),
        ("categories", crate::refs::ReferenceSource::OmoCategory),
    ] {
        let Some(entries) = opencode.get(key) else {
            continue;
        };
        let entries = entries
            .as_object()
            .ok_or_else(|| AppError::validation(format!("OMO opencode.{key} must be an object")))?;
        for (name, entry) in entries {
            let entry = entry.as_object().ok_or_else(|| {
                AppError::validation(format!("OMO opencode.{key}.{name} must be an object"))
            })?;
            if let Some(model) = entry.get("model").and_then(Value::as_str) {
                if !model.is_empty() {
                    references.push(crate::refs::ModelReference {
                        source,
                        location: format!("{OMO_OPENCODE_BLOCK}.{key}.{name}.model"),
                        model_ref: ModelRef::parse(model)?,
                    });
                }
            }
        }
    }
    Ok(())
}
