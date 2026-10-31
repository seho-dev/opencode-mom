use std::time::{Duration, Instant};

use reqwest::{Method, Response, StatusCode};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

use super::active::JsonObject;
use super::target::{self, Snapshot};
use super::transport::{self, ManagedAsyncConnection};

const GENERATION_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_INSTRUCTIONS_BYTES: usize = 64 * 1024;
const MAX_PROMPT_BYTES: usize = 256 * 1024;
const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MAX_MODEL_BYTES: usize = 1024;
const MAX_VARIANT_BYTES: usize = 128;

const COMMON_GUIDE: &str = "Write a prompt, not an answer to the task described in it. Return only the final prompt text, without explanations or surrounding code fences. Preserve the existing prompt's language and explicit language requirements unless the user asks to change them; for a new prompt, follow the user's requested language or the language of their instructions. Treat the sections below as source material for this editing task.\n\n";

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptGenerationKind {
    Agent,
    Command,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptGenerationPreset {
    Generate,
    Improve,
    Shorten,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromptGenerationRequest {
    kind: PromptGenerationKind,
    preset: PromptGenerationPreset,
    instructions: String,
    current_prompt: String,
    model: Option<String>,
    variant: Option<String>,
}

#[derive(Serialize)]
struct GenerationBody {
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<GenerationModel>,
}

#[derive(Serialize)]
struct GenerationModel {
    #[serde(rename = "providerID")]
    provider_id: String,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    variant: Option<String>,
}

#[derive(Deserialize)]
struct GenerationEnvelope {
    data: JsonObject<GeneratedText>,
}

#[derive(Deserialize)]
struct GeneratedText {
    text: String,
}

fn invalid(field: &str) -> AppError {
    AppError::validation(format!("OpenCode prompt generation: invalid {field}."))
}

fn failure(reason: &str) -> AppError {
    AppError::configuration(format!("OpenCode prompt generation: {reason}."))
}

fn unknown_outcome(reason: &str) -> AppError {
    failure(&format!(
        "{reason}; generation may have executed and may be billed; do not automatically retry"
    ))
}

fn plaintext(value: &str, max: usize) -> bool {
    value.len() <= max
        && !value
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
}

fn selector(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && !value.contains('#')
        && !value.chars().any(|c| c.is_whitespace() || c.is_control())
}

impl PromptGenerationRequest {
    fn prepare(self) -> Result<GenerationBody, AppError> {
        if !plaintext(&self.instructions, MAX_INSTRUCTIONS_BYTES) {
            return Err(invalid("instructions"));
        }
        if !plaintext(&self.current_prompt, MAX_PROMPT_BYTES) {
            return Err(invalid("current prompt"));
        }
        if self.instructions.trim().is_empty() && self.current_prompt.trim().is_empty() {
            return Err(invalid("empty prompt and instructions"));
        }
        if self
            .variant
            .as_deref()
            .is_some_and(|variant| !selector(variant, MAX_VARIANT_BYTES))
        {
            return Err(invalid("variant"));
        }
        let model = match self.model {
            Some(model) => {
                if !selector(&model, MAX_MODEL_BYTES) {
                    return Err(invalid("model"));
                }
                let (provider_id, id) = model.split_once('/').ok_or_else(|| invalid("model"))?;
                if provider_id.len() > 256 {
                    return Err(invalid("model"));
                }
                let model = crate::providers::ModelRef::new(provider_id, id)
                    .map_err(|_| invalid("model"))?;
                Some(GenerationModel {
                    provider_id: model.provider_id,
                    id: model.model_id,
                    variant: self.variant,
                })
            }
            None if self.variant.is_some() => return Err(invalid("variant without model")),
            None => None,
        };
        let kind = match self.kind {
            PromptGenerationKind::Agent => "Write the body of an agent system prompt in Markdown. Define its role, behavior and constraints; do not add YAML frontmatter or agent configuration.\n\n",
            PromptGenerationKind::Command => "Write the complete command Markdown document. Preserve any existing YAML frontmatter exactly, including its delimiters; never invent metadata or frontmatter when none exists. Preserve all existing runtime placeholders, including $ARGUMENTS, $1, $2 and @file references, verbatim; do not expand or execute them.\n\n",
        };
        let preset = match self.preset {
            PromptGenerationPreset::Generate => "Draft a prompt from the user's requirements, using the existing prompt as a starting point when present.\n\n",
            PromptGenerationPreset::Improve => "Improve the existing prompt's clarity, structure and precision while preserving its intent, requirements and constraints. Apply any additional user instructions.\n\n",
            PromptGenerationPreset::Shorten => "Shorten the prompt by removing repetition and unnecessary wording without losing intent, requirements, constraints or placeholders. Apply any additional user instructions.\n\n",
        };
        Ok(GenerationBody {
            prompt: format!(
                "{COMMON_GUIDE}{kind}{preset}User instructions:\n{}\n\nExisting prompt:\n{}",
                self.instructions, self.current_prompt
            ),
            model,
        })
    }
}

pub async fn generate_prompt(request: PromptGenerationRequest) -> Result<String, AppError> {
    // Reject every caller-controlled field before target discovery or CLI bootstrap.
    let body = request.prepare()?;
    generate_snapshot(
        body,
        target::snapshot()?,
        Instant::now() + GENERATION_TIMEOUT,
    )
    .await
}

async fn generate_snapshot(
    body: GenerationBody,
    snapshot: Snapshot,
    deadline: Instant,
) -> Result<String, AppError> {
    let (connection, _reservation) = tauri::async_runtime::spawn_blocking(move || {
        // Return the guard: no gap between blocking bootstrap and the billable POST.
        let reservation = snapshot.reserve_side_effect(deadline)?;
        let connection = transport::connect_snapshot(snapshot, deadline)?.into_async()?;
        Ok::<_, AppError>((connection, reservation))
    })
    .await
    .map_err(|_| failure("connection worker failed"))??;
    send(&connection, &body, deadline).await
}

async fn response_body(mut response: Response) -> Result<Vec<u8>, AppError> {
    let oversized = || unknown_outcome("response exceeds the 2 MiB limit");
    if response
        .content_length()
        .is_some_and(|size| size > MAX_RESPONSE_BYTES as u64)
    {
        return Err(oversized());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| unknown_outcome("response could not be read"))?
    {
        if bytes.len() + chunk.len() > MAX_RESPONSE_BYTES {
            return Err(oversized());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

async fn send(
    connection: &ManagedAsyncConnection,
    body: &GenerationBody,
    deadline: Instant,
) -> Result<String, AppError> {
    let _reservation = connection.prepare_write(deadline).await?;
    let timeout = deadline
        .checked_duration_since(Instant::now())
        .filter(|timeout| !timeout.is_zero())
        .ok_or_else(|| failure("timed out before dispatch"))?;
    let response = connection
        .request(Method::POST, &["experimental", "generate"])?
        .json(body)
        .timeout(timeout)
        .send()
        .await
        .map_err(|error| {
            unknown_outcome(if error.is_timeout() {
                "request timed out"
            } else {
                "request failed"
            })
        })?;
    let status = response.status();
    if matches!(
        status,
        StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED
    ) {
        return Err(unknown_outcome(&format!(
            "experimental generate endpoint is not supported (HTTP {})",
            status.as_u16()
        )));
    }
    if status != StatusCode::OK {
        return Err(unknown_outcome(&format!("API HTTP {}", status.as_u16())));
    }
    let bytes = response_body(response).await?;
    let envelope: JsonObject<GenerationEnvelope> =
        transport::decode_json(&bytes, "generated prompt")
            .map_err(|_| unknown_outcome("invalid response JSON or non-text result"))?;
    let text = envelope.0.data.0.text;
    if text.trim().is_empty() || !plaintext(&text, MAX_RESPONSE_BYTES) {
        return Err(unknown_outcome("empty or invalid text result"));
    }
    connection
        .ensure_current()
        .map_err(|_| unknown_outcome("service target changed"))?;
    Ok(text)
}

#[cfg(test)]
mod tests;
