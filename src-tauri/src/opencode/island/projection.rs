use super::*;

#[derive(Deserialize)]
struct Active {
    #[serde(deserialize_with = "super::super::active::running_sessions")]
    data: HashSet<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Session {
    id: String,
    title: Option<String>,
    location: Location,
    outcome: Option<String>,
    #[serde(rename = "parentID")]
    parent_id: Option<String>,
    time: SessionTime,
}

#[derive(Deserialize)]
struct Location {
    directory: String,
}

#[derive(Deserialize)]
struct SessionTime {
    updated: u64,
    archived: Option<u64>,
}

impl Session {
    fn public(self, running: bool) -> Result<IslandSession, AppError> {
        identifier(&self.id, "ses_")?;
        if let Some(parent) = &self.parent_id {
            identifier(parent, "ses_")?;
        }
        let title = self.title.unwrap_or_default();
        plain(&title, 4096, false)?;
        plain(&self.location.directory, 4096, false)?;
        if self
            .outcome
            .as_deref()
            .is_some_and(|outcome| !matches!(outcome, "succeeded" | "failed" | "interrupted"))
        {
            return Err(invalid("session outcome"));
        }
        if let Some(time) = self.time.archived {
            safe_time(time)?;
        }
        Ok(IslandSession {
            id: self.id,
            title,
            directory: self.location.directory,
            updated_at: safe_time(self.time.updated)?,
            parent_id: self.parent_id,
            status: if running {
                "running"
            } else if self.outcome.as_deref() == Some("failed") {
                "error"
            } else {
                "idle"
            }
            .into(),
        })
    }
}

async fn bounded<T, F: std::future::Future<Output = Result<T, AppError>>>(
    futures: impl IntoIterator<Item = F>,
    mut retain: impl FnMut(T) -> Result<(), AppError>,
) -> Result<(), AppError> {
    let mut input = futures.into_iter();
    let mut pending: Vec<_> = input.by_ref().take(4).map(Box::pin).collect();
    while !pending.is_empty() {
        let (index, result) = std::future::poll_fn(|context| {
            for (index, future) in pending.iter_mut().enumerate() {
                if let std::task::Poll::Ready(result) = future.as_mut().poll(context) {
                    return std::task::Poll::Ready((index, result));
                }
            }
            std::task::Poll::Pending
        })
        .await;
        retain(result?)?;
        if let Some(future) = input.next() {
            pending[index] = Box::pin(future);
        } else {
            drop(pending.swap_remove(index));
        }
    }
    Ok(())
}

impl IslandApi {
    pub(super) async fn permissions(
        &self,
        session_id: &str,
    ) -> Result<Vec<IslandPermission>, AppError> {
        let pending: Data<Vec<Value>> = self
            .read(&["session", session_id, "permission"], &[])
            .await?;
        if pending.data.len() > MAX_PENDING {
            return Err(failure("permission display limit exceeded"));
        }
        let mut ids = HashSet::new();
        pending
            .data
            .into_iter()
            .map(|request| {
                let id = string(&request, "id")?;
                identifier(id, "per")?;
                if !ids.insert(id.to_owned()) || string(&request, "sessionID")? != session_id {
                    return Err(invalid("permission identity"));
                }
                let action = string(&request, "action")?;
                plain(action, 1024, false)?;
                if action.is_empty() {
                    return Err(invalid("permission action"));
                }
                Ok(IslandPermission {
                    id: id.to_owned(),
                    session_id: session_id.to_owned(),
                    action: action.to_owned(),
                    resources: bounded_strings(
                        request
                            .get("resources")
                            .ok_or_else(|| invalid("permission resources"))?,
                        MAX_OPTIONS,
                    )?,
                    save: request
                        .get("save")
                        .map(|value| bounded_strings(value, MAX_OPTIONS))
                        .transpose()?
                        .unwrap_or_default(),
                    message: optional_string(&request, "message", 4096)?,
                })
            })
            .collect()
    }

    async fn questions(&self, session_id: &str) -> Result<Vec<IslandQuestion>, AppError> {
        let pending: Data<Vec<Value>> = self.read(&["session", session_id, "form"], &[]).await?;
        if pending.data.len() > MAX_PENDING {
            return Err(failure("form display limit exceeded"));
        }
        let mut ids = HashSet::new();
        pending
            .data
            .into_iter()
            .map(|form| {
                let question = public_question(&form, session_id)?;
                if !ids.insert(question.id.clone()) {
                    return Err(invalid("duplicate form"));
                }
                Ok(question)
            })
            .collect()
    }

    pub(crate) async fn reply_question(
        &self,
        session_id: &str,
        form_id: &str,
        mut answer: Map<String, Value>,
    ) -> Result<(), AppError> {
        identifier(session_id, "ses_")?;
        identifier(form_id, "frm_")?;
        if answer.len() > MAX_FIELDS {
            return Err(invalid("question answer limit"));
        }
        let current: Data<Value> = self
            .read(&["session", session_id, "form", form_id], &[])
            .await?;
        if current
            .data
            .get("state")
            .and_then(|state| state.get("status"))
            .and_then(Value::as_str)
            != Some("pending")
        {
            return Err(invalid("stale question"));
        }
        let question = public_question(&current.data, session_id)?;
        if question.id != form_id {
            return Err(invalid("form identity"));
        }
        answer_valid(&question, &mut answer)?;
        self.write(
            self.request(
                Method::POST,
                &["session", session_id, "form", form_id, "reply"],
            )?
            .json(&serde_json::json!({"answer":answer})),
        )
        .await
    }

    pub(crate) async fn snapshot(&self) -> Result<IslandSnapshot, AppError> {
        let snapshot = tokio::time::timeout(Duration::from_secs(15), self.snapshot_inner())
            .await
            .map_err(|_| failure("snapshot timed out"))??;
        self.connection.ensure_current()?;
        Ok(snapshot)
    }

    async fn snapshot_inner(&self) -> Result<IslandSnapshot, AppError> {
        let recent: Data<Vec<Session>> = self
            .read(
                &["session"],
                &[("limit", "20"), ("order", "desc"), ("parentID", "null")],
            )
            .await?;
        if recent.data.len() > 20 {
            return Err(failure("recent session limit exceeded"));
        }
        let active: Active = self.read(&["session", "active"], &[]).await?;
        if active.data.len() > MAX_SESSIONS {
            return Err(failure("active session display limit exceeded"));
        }
        for id in &active.data {
            identifier(id, "ses_")?;
        }
        let mut seen = HashSet::new();
        let mut sessions = Vec::new();
        for session in recent.data {
            if session.parent_id.is_some() || !seen.insert(session.id.clone()) {
                return Err(invalid("recent root session"));
            }
            let archived = session.time.archived.is_some();
            let running = active.data.contains(&session.id);
            let session = session.public(running)?;
            if !archived || running {
                sessions.push(session);
            }
        }
        let missing: Vec<_> = active
            .data
            .iter()
            .filter(|id| !sessions.iter().any(|session| &session.id == *id))
            .cloned()
            .collect();
        if sessions.len() + missing.len() > MAX_SESSIONS {
            return Err(failure("session display limit exceeded"));
        }
        let details: Vec<_> = missing
            .into_iter()
            .map(|id| async move {
                let session: Data<Session> = self.read(&["session", &id], &[]).await?;
                if session.data.id != id {
                    return Err(invalid("session identity"));
                }
                session.data.public(true)
            })
            .collect();
        bounded(details, |session| {
            sessions.push(session);
            Ok(())
        })
        .await?;
        let mut snapshot = IslandSnapshot {
            sessions,
            ..IslandSnapshot::default()
        };
        let mut retained_bytes = serde_json::to_vec(&snapshot)
            .map_err(|_| failure("snapshot encoding failed"))?
            .len();
        if retained_bytes > MAX_BYTES {
            return Err(failure("snapshot retained byte limit exceeded"));
        }
        // Recent roots are checked once too: pending work is not inferred from connection state.
        let ids: Vec<_> = snapshot
            .sessions
            .iter()
            .map(|session| session.id.clone())
            .collect();
        let pending: Vec<_> = ids
            .into_iter()
            .map(|id| async move {
                let permissions = self.permissions(&id).await?;
                let questions = self.questions(&id).await?;
                Ok((permissions, questions))
            })
            .collect();
        bounded(pending, |(permissions, questions)| {
            if snapshot.permissions.len()
                + snapshot.questions.len()
                + permissions.len()
                + questions.len()
                > MAX_PENDING
            {
                return Err(failure("pending request display limit exceeded"));
            }
            retained_bytes += serde_json::to_vec(&(&permissions, &questions))
                .map_err(|_| failure("snapshot encoding failed"))?
                .len();
            if retained_bytes > MAX_BYTES {
                return Err(failure("snapshot retained byte limit exceeded"));
            }
            snapshot.permissions.extend(permissions);
            snapshot.questions.extend(questions);
            Ok(())
        })
        .await?;
        let pending_ids: HashSet<_> = snapshot
            .permissions
            .iter()
            .map(|permission| permission.session_id.as_str())
            .chain(
                snapshot
                    .questions
                    .iter()
                    .map(|question| question.session_id.as_str()),
            )
            .collect();
        snapshot.sessions.sort_by(|left, right| {
            let priority = |session: &IslandSession| {
                if pending_ids.contains(session.id.as_str()) {
                    0
                } else if session.status == "running" {
                    1
                } else {
                    2
                }
            };
            priority(left)
                .cmp(&priority(right))
                .then_with(|| right.updated_at.cmp(&left.updated_at))
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(snapshot)
    }

    pub(crate) async fn messages(&self, session_id: &str) -> Result<Vec<IslandMessage>, AppError> {
        identifier(session_id, "ses_")?;
        let messages: Data<Vec<Value>> = self
            .read(
                &["session", session_id, "message"],
                &[("limit", "20"), ("order", "desc")],
            )
            .await?;
        public_messages(messages.data)
    }
}

pub(super) fn public_question(form: &Value, session_id: &str) -> Result<IslandQuestion, AppError> {
    let id = string(form, "id")?;
    identifier(id, "frm_")?;
    if string(form, "sessionID")? != session_id {
        return Err(invalid("form owner"));
    }
    let title = string(form, "title")?;
    plain(title, 4096, true)?;
    let fields = form
        .get("fields")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("form fields"))?;
    if fields.len() > MAX_FIELDS {
        return Err(failure("form field display limit exceeded"));
    }
    let mut question = IslandQuestion {
        id: id.to_owned(),
        session_id: session_id.to_owned(),
        title: title.to_owned(),
        answerable: !fields.is_empty()
            && form
                .get("metadata")
                .and_then(|metadata| metadata.get("kind"))
                .and_then(Value::as_str)
                == Some("question"),
        fields: Vec::new(),
    };
    let mut keys = HashSet::new();
    for field in fields {
        let key = string(field, "key")?;
        plain(key, 256, false)?;
        if key.is_empty() || !keys.insert(key.to_owned()) {
            return Err(invalid("form field key"));
        }
        let kind = string(field, "type")?;
        plain(kind, 64, false)?;
        let mut required = public_flag(field, "required")?;
        let conditional = match field.get("when") {
            None | Some(Value::Null) => false,
            Some(Value::Array(conditions)) => !conditions.is_empty(),
            _ => return Err(invalid("form conditions")),
        };
        let selection_minimum = match field.get("minItems") {
            None | Some(Value::Null) => false,
            Some(value) if kind == "multiselect" && value.as_u64() == Some(0) => false,
            Some(value) if kind == "multiselect" && value.as_u64() == Some(1) => {
                required = true;
                false
            }
            Some(_) => true,
        };
        // Constraints/conditions and non-question forms require the full OpenCode client.
        if !matches!(kind, "string" | "multiselect")
            || public_flag(field, "hidden")?
            || conditional
            || selection_minimum
            || ["format", "minLength", "maxLength", "pattern", "maxItems"]
                .iter()
                .any(|key| field.get(*key).is_some_and(|value| !value.is_null()))
        {
            question.answerable = false;
        }
        let mut options = Vec::new();
        let mut values = HashSet::new();
        if let Some(list) = field.get("options").filter(|value| !value.is_null()) {
            let list = list.as_array().ok_or_else(|| invalid("form options"))?;
            if list.len() > MAX_OPTIONS {
                return Err(failure("form option display limit exceeded"));
            }
            for option in list {
                let value = string(option, "value")?;
                let label = string(option, "label")?;
                plain(value, 4096, true)?;
                plain(label, 4096, true)?;
                if !values.insert(value.to_owned()) {
                    return Err(invalid("duplicate form option"));
                }
                options.push(IslandQuestionOption {
                    value: value.to_owned(),
                    label: label.to_owned(),
                    description: optional_string(option, "description", 4096)?,
                });
            }
        } else if kind == "multiselect" {
            question.answerable = false;
        }
        let custom = match field.get("custom") {
            None | Some(Value::Null) => options.is_empty(),
            Some(Value::Bool(custom)) => *custom,
            _ => return Err(invalid("form custom flag")),
        };
        question.fields.push(IslandQuestionField {
            key: key.to_owned(),
            kind: kind.to_owned(),
            title: optional_string(field, "title", 4096)?,
            description: optional_string(field, "description", 4096)?,
            options,
            custom,
            required,
        });
    }
    if !question.answerable {
        question.fields.clear();
    }
    Ok(question)
}

pub(super) fn public_messages(messages: Vec<Value>) -> Result<Vec<IslandMessage>, AppError> {
    if messages.len() > 20 {
        return Err(failure("message display limit exceeded"));
    }
    let mut result = Vec::new();
    let mut ids = HashSet::new();
    for message in messages {
        let id = string(&message, "id")?;
        identifier(id, "msg_")?;
        if !ids.insert(id.to_owned()) {
            return Err(invalid("duplicate message"));
        }
        let role = string(&message, "type")?;
        plain(role, 64, false)?;
        if !matches!(role, "user" | "assistant") {
            continue;
        }
        if private_content(&message)? {
            continue;
        }
        let time = safe_time(
            message
                .get("time")
                .and_then(|time| time.get("created"))
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid("message time"))?,
        )?;
        let mut text = String::new();
        let mut truncated = false;
        let mut append = |value: &str| -> Result<(), AppError> {
            plain(value, MAX_BYTES, true)?;
            let (piece, cut) = trim_text(value, MAX_TEXT - text.len());
            text.push_str(piece);
            truncated |= cut;
            Ok(())
        };
        if role == "user" {
            append(string(&message, "text")?)?;
        } else {
            let content = message
                .get("content")
                .and_then(Value::as_array)
                .ok_or_else(|| invalid("assistant content"))?;
            if content.len() > 256 {
                return Err(failure("message content item limit exceeded"));
            }
            for part in content {
                if part.get("type").and_then(Value::as_str) == Some("text")
                    && !private_content(part)?
                {
                    append(string(part, "text")?)?;
                }
            }
        }
        if !text.is_empty() {
            result.push(IslandMessage {
                id: id.to_owned(),
                role: role.to_owned(),
                text,
                time,
                truncated,
            });
        }
    }
    result.sort_by(|left, right| {
        left.time
            .cmp(&right.time)
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(result)
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, AppError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("response field"))
}

fn optional_string(value: &Value, key: &str, max: usize) -> Result<Option<String>, AppError> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => {
            plain(text, max, true)?;
            Ok(Some(text.clone()))
        }
        _ => Err(invalid("response field")),
    }
}

fn bounded_strings(value: &Value, max_items: usize) -> Result<Vec<String>, AppError> {
    let values = value.as_array().ok_or_else(|| invalid("response list"))?;
    if values.len() > max_items {
        return Err(failure("response item limit exceeded"));
    }
    values
        .iter()
        .map(|value| {
            let text = value.as_str().ok_or_else(|| invalid("response field"))?;
            plain(text, 4096, true)?;
            Ok(text.to_owned())
        })
        .collect()
}

fn trim_text(text: &str, remaining: usize) -> (&str, bool) {
    let mut end = text.len().min(remaining);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (&text[..end], end < text.len())
}

fn public_flag(value: &Value, key: &str) -> Result<bool, AppError> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(flag)) => Ok(*flag),
        _ => Err(invalid("public content flag")),
    }
}

fn private_content(value: &Value) -> Result<bool, AppError> {
    for flags in
        std::iter::once(value).chain(value.get("metadata").filter(|metadata| !metadata.is_null()))
    {
        if !flags.is_object() {
            return Err(invalid("content metadata"));
        }
        for key in ["hidden", "synthetic", "internal"] {
            if public_flag(flags, key)? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn answer_valid(
    question: &IslandQuestion,
    answer: &mut Map<String, Value>,
) -> Result<(), AppError> {
    if !question.answerable || answer.len() > MAX_FIELDS {
        return Err(invalid("question answer"));
    }
    for key in answer.keys() {
        if !question.fields.iter().any(|field| field.key == *key) {
            return Err(invalid("question answer key"));
        }
    }
    for field in &question.fields {
        if !field.required
            && answer.get(&field.key).is_some_and(|value| {
                (field.kind == "string" && value.as_str() == Some(""))
                    || (field.kind == "multiselect" && value.as_array().is_some_and(Vec::is_empty))
            })
        {
            answer.remove(&field.key);
            continue;
        }
        let Some(value) = answer.get(&field.key) else {
            if field.required {
                return Err(invalid("required question answer"));
            }
            continue;
        };
        let values = match field.kind.as_str() {
            "string" => vec![value
                .as_str()
                .ok_or_else(|| invalid("question answer type"))?],
            "multiselect" => {
                let values = value
                    .as_array()
                    .ok_or_else(|| invalid("question answer type"))?;
                if values.len() > MAX_OPTIONS {
                    return Err(invalid("question answer limit"));
                }
                values
                    .iter()
                    .map(|value| {
                        value
                            .as_str()
                            .ok_or_else(|| invalid("question answer type"))
                    })
                    .collect::<Result<Vec<_>, _>>()?
            }
            _ => return Err(invalid("question answer type")),
        };
        if field.required && (values.is_empty() || values.iter().any(|v| v.trim().is_empty())) {
            return Err(invalid("required question answer"));
        }
        let mut unique = HashSet::new();
        for value in values {
            plain(value, 4096, true)?;
            if !unique.insert(value)
                || (!field.custom && !field.options.iter().any(|option| option.value == value))
            {
                return Err(invalid("question answer option"));
            }
        }
    }
    Ok(())
}
