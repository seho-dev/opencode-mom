use serde::{Deserialize, Deserializer, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

use crate::error::{AppError, ErrorCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LogType {
    App,
    Config,
    Group,
    Resource,
    Opencode,
    Http,
    McpProxy,
    Power,
    Island,
    Logging,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LogStatus {
    Success,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoggingState {
    pub enabled: bool,
    pub recording: bool,
    pub directory: String,
    pub error: Option<String>,
    pub dropped_records: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LogContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_written: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LogRecord {
    pub schema_version: u32,
    pub timestamp: String,
    pub session_id: Uuid,
    pub sequence: u64,
    pub operation_id: Uuid,
    #[serde(rename = "type")]
    pub kind: LogType,
    pub status: LogStatus,
    pub event: String,
    pub message: String,
    pub duration_ms: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_error_code")]
    pub error_code: Option<ErrorCode>,
    pub context: Option<LogContext>,
}

impl LogRecord {
    pub(super) fn is_safe(&self) -> bool {
        self.schema_version == 1
            && safe_event(self.kind, &self.event)
            && self.timestamp.len() <= 40
            && OffsetDateTime::parse(&self.timestamp, &Rfc3339)
                .is_ok_and(|time| time.offset().is_utc())
            && match self.status {
                LogStatus::Success => {
                    matches!(
                        self.message.as_str(),
                        "Operation completed" | "Checkpoint reached" | "Event completed"
                    ) && self.error_code.is_none()
                }
                LogStatus::Error => {
                    matches!(self.message.as_str(), "Operation failed" | "Event failed")
                }
            }
            && match (&self.context, self.message.as_str()) {
                (Some(context), "Checkpoint reached") => {
                    checkpoint_event(self.kind, &self.event)
                        && context.step.as_deref() == Some(self.event.as_str())
                }
                (None, "Checkpoint reached") => false,
                (None, _) => true,
                (Some(_), _) => false,
            }
    }
}

pub(super) fn safe_event(kind: LogType, event: &str) -> bool {
    match kind {
        LogType::App => matches!(
            event,
            "app.started"
                | "app.stopped"
                | "app_update_check"
                | "autostart_set"
                | "project_page_open"
        ),
        LogType::Config => matches!(
            event,
            "app_state_load"
                | "global_instructions_save"
                | "global_instructions_read"
                | "project_add"
                | "project_remove"
                | "project_select"
                | "preferences_save"
                | "preferences_committed_tray_notify_failed"
                | "preferences_committed_island_notify_failed"
        ),
        LogType::Group => matches!(
            event,
            "group_save"
                | "group_copy"
                | "group_delete"
                | "group_switch"
                | "group_switch_projection_warnings"
                | "group_switch_worker_failed"
                | "group_switch_committed_notify_failed"
                | "group_switch_validated"
                | "group_switch_projection_complete"
        ),
        LogType::Resource => matches!(
            event,
            "provider_create"
                | "provider_update"
                | "provider_delete"
                | "provider_list"
                | "model_create"
                | "model_update"
                | "model_delete"
                | "model_list"
                | "agent_create"
                | "agent_update"
                | "agent_delete"
                | "agent_list"
                | "agent_read"
                | "mcp_create"
                | "mcp_update"
                | "mcp_delete"
                | "mcp_list"
                | "mcp_read"
                | "skill_create"
                | "skill_update"
                | "skill_list"
                | "skill_read"
                | "slash_command_create"
                | "slash_command_update"
                | "slash_command_delete"
                | "slash_command_list"
                | "slash_command_read"
                | "plugin_package_update"
                | "plugin_declaration_remove"
                | "plugin_runtime_list"
                | "plugin_removal_preview"
        ),
        LogType::Opencode => matches!(
            event,
            "opencode_target_set"
                | "opencode_update_check"
                | "opencode_upgrade"
                | "opencode_reload"
                | "opencode_model_catalog"
                | "opencode_installation_read"
                | "opencode_prompt_generate"
                | "opencode_target_committed_island_unavailable"
        ),
        LogType::Http => matches!(
            event,
            "http_key_rotate"
                | "http_key_rotate_worker_failed"
                | "http_key_committed_cleanup_failed"
                | "http_start"
                | "http_listener_failed"
                | "http_settings_save"
                | "http_settings_worker_failed"
                | "http_stopped_cleanup_failed"
                | "http_settings_committed"
                | "http_admission_closed_cleanup_failed"
                | "http_listener_task_failed"
                | "http_shutdown"
                | "http_settings_unavailable"
                | "http_key_rotate_unavailable"
        ),
        LogType::McpProxy => matches!(
            event,
            "mcp_proxy_import_prepare"
                | "mcp_proxy_settings_save"
                | "mcp_proxy_server_create"
                | "mcp_proxy_server_update"
                | "mcp_proxy_server_delete"
                | "mcp_proxy_unit_close"
                | "mcp_proxy_config_save_failed"
                | "mcp_proxy_config_worker_failed"
                | "mcp_proxy_config_committed"
                | "mcp_proxy_config_committed_cleanup_failed"
                | "mcp_proxy_shutdown"
                | "mcp_proxy_shutdown_worker_failed"
        ),
        LogType::Power => matches!(
            event,
            "lid_protection_set"
                | "power_checking"
                | "power_protecting"
                | "power_idle"
                | "power_unknown"
                | "power_error"
                | "power_disabled"
        ),
        LogType::Island => matches!(
            event,
            "island_enabled_set"
                | "island_refresh"
                | "island_view_set"
                | "island_send"
                | "island_permission_reply"
                | "island_question_reply"
                | "island_question_cancel"
                | "island_interrupt"
                | "island_connected"
                | "island_connection_failed"
                | "island_window_failed"
        ),
        LogType::Logging => matches!(
            event,
            "logging_enabled" | "logging_disabled" | "logging_cleared"
        ),
    }
}

pub(super) fn checkpoint_event(kind: LogType, event: &str) -> bool {
    kind == LogType::Group
        && matches!(
            event,
            "group_switch_validated" | "group_switch_projection_complete"
        )
}

fn deserialize_error_code<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<ErrorCode>, D::Error> {
    #[derive(Deserialize)]
    #[serde(rename_all = "snake_case")]
    enum Code {
        NotFound,
        ReferencesBlocked,
        ValidationFailed,
        ConfigurationFailed,
        Busy,
    }
    Ok(
        Option::<Code>::deserialize(deserializer)?.map(|code| match code {
            Code::NotFound => ErrorCode::NotFound,
            Code::ReferencesBlocked => ErrorCode::ReferencesBlocked,
            Code::ValidationFailed => ErrorCode::ValidationFailed,
            Code::ConfigurationFailed => ErrorCode::ConfigurationFailed,
            Code::Busy => ErrorCode::Busy,
        }),
    )
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LogQuery {
    #[serde(rename = "type")]
    pub kind: Option<LogType>,
    pub status: Option<LogStatus>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub operation_id: Option<Uuid>,
    pub cursor: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LogPage {
    pub records: Vec<LogRecord>,
    pub next_cursor: Option<String>,
    pub invalid_lines: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Filter {
    kind: Option<LogType>,
    status: Option<LogStatus>,
    from: Option<i128>,
    to: Option<i128>,
    operation_id: Option<Uuid>,
}

impl Filter {
    pub fn new(query: &LogQuery) -> Result<Self, AppError> {
        fn timestamp(value: &Option<String>) -> Result<Option<i128>, AppError> {
            value
                .as_ref()
                .map(|value| {
                    if value.len() > 64 {
                        return Err(AppError::validation("Invalid log timestamp"));
                    }
                    OffsetDateTime::parse(value, &Rfc3339)
                        .map(|time| time.unix_timestamp_nanos())
                        .map_err(|_| AppError::validation("Invalid log timestamp"))
                })
                .transpose()
        }
        let filter = Self {
            kind: query.kind,
            status: query.status,
            from: timestamp(&query.from)?,
            to: timestamp(&query.to)?,
            operation_id: query.operation_id,
        };
        if filter
            .from
            .zip(filter.to)
            .is_some_and(|(from, to)| from > to)
        {
            return Err(AppError::validation("Invalid log time range"));
        }
        Ok(filter)
    }

    pub fn matches(&self, record: &LogRecord) -> bool {
        let Ok(time) = OffsetDateTime::parse(&record.timestamp, &Rfc3339) else {
            return false;
        };
        let time = time.unix_timestamp_nanos();
        self.kind.is_none_or(|kind| kind == record.kind)
            && self.status.is_none_or(|status| status == record.status)
            && self.operation_id.is_none_or(|id| id == record.operation_id)
            && self.from.is_none_or(|from| time >= from)
            && self.to.is_none_or(|to| time <= to)
    }
}
