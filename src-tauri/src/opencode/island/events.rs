use super::*;

const MAX_FRAME: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IslandChange {
    Snapshot,
    Messages { session_id: String },
}

pub(crate) struct IslandEvents {
    response: Response,
    buffer: Vec<u8>,
    offset: usize,
    frame: Frame,
}

#[derive(Default)]
pub(super) struct Frame {
    line: Vec<u8>,
    data: Vec<u8>,
    size: usize,
    skip_lf: bool,
}

impl Frame {
    pub(super) fn byte(&mut self, byte: u8) -> Result<Option<&[u8]>, AppError> {
        if self.size == 0 {
            self.data.clear();
        }
        if self.skip_lf {
            self.skip_lf = false;
            if byte == b'\n' {
                return Ok(None);
            }
        }
        self.size += 1;
        if self.size > MAX_FRAME {
            return Err(failure("event frame byte limit exceeded"));
        }
        if !matches!(byte, b'\r' | b'\n') {
            self.line.push(byte);
            return Ok(None);
        }
        self.skip_lf = byte == b'\r';
        if self.line.is_empty() {
            self.size = 0;
            if self.data.last() == Some(&b'\n') {
                self.data.pop();
            }
            return Ok(Some(&self.data));
        }
        if let Some(data) = self.line.strip_prefix(b"data:") {
            self.data
                .extend_from_slice(data.strip_prefix(b" ").unwrap_or(data));
            self.data.push(b'\n');
        }
        self.line.clear();
        Ok(None)
    }
}

impl IslandApi {
    pub(crate) async fn events(&self) -> Result<super::IslandEvents, AppError> {
        // Bound the handshake only; a quiet live stream must not become snapshot polling.
        let response = tokio::time::timeout(
            REQUEST_TIMEOUT,
            self.connection
                .request(Method::GET, &["event"])?
                .header(reqwest::header::ACCEPT, "text/event-stream")
                .send(),
        )
        .await
        .map_err(|_| failure("event connection timed out"))?
        .map_err(request_error)?;
        if response.status() != reqwest::StatusCode::OK {
            return Err(failure(&format!("API HTTP {}", response.status().as_u16())));
        }
        if response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(';').next())
            != Some("text/event-stream")
        {
            return Err(invalid("event content type"));
        }
        Ok(IslandEvents {
            response,
            buffer: Vec::new(),
            offset: 0,
            frame: Frame::default(),
        })
    }
}

impl IslandEvents {
    pub(crate) async fn next(&mut self) -> Result<IslandChange, AppError> {
        loop {
            while self.offset < self.buffer.len() {
                let byte = self.buffer[self.offset];
                self.offset += 1;
                if let Some(data) = self.frame.byte(byte)? {
                    if !data.is_empty() {
                        if let Some(change) = classify(data)? {
                            return Ok(change);
                        }
                    }
                }
            }
            self.buffer.clear();
            self.offset = 0;
            let chunk = self
                .response
                .chunk()
                .await
                .map_err(request_error)?
                .ok_or_else(|| failure("event stream ended"))?;
            if chunk.len() > MAX_BYTES {
                return Err(failure("event chunk byte limit exceeded"));
            }
            self.buffer.extend_from_slice(&chunk);
        }
    }
}

pub(super) fn classify(bytes: &[u8]) -> Result<Option<IslandChange>, AppError> {
    let event: Value = transport::decode_json(bytes, "island event")?;
    let kind = event
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("event type"))?;
    let snapshot = matches!(
        kind,
        "server.connected"
            | "session.created"
            | "session.deleted"
            | "session.forked"
            | "session.renamed"
            | "session.moved"
            | "session.status"
            | "session.idle"
            | "session.execution.started"
            | "session.execution.succeeded"
            | "session.execution.failed"
            | "session.execution.interrupted"
            | "permission.asked"
            | "permission.replied"
            | "form.created"
            | "form.replied"
            | "form.cancelled"
            | "location.shutdown"
    );
    let messages = matches!(
        kind,
        "session.text.started"
            | "session.text.delta"
            | "session.text.ended"
            | "session.message.content.updated"
            | "session.inbox.delivered"
    );
    if !snapshot && !messages {
        return Ok(None);
    }
    let data = event
        .get("data")
        .filter(|value| value.is_object())
        .ok_or_else(|| invalid("event data"))?;
    // Snapshot invalidation needs no routing ID; native lifecycle payloads differ by event.
    if snapshot && !kind.starts_with("form.") {
        return Ok(Some(IslandChange::Snapshot));
    }
    let data = if kind == "form.created" {
        data.get("form").ok_or_else(|| invalid("form event"))?
    } else {
        data
    };
    let session_id = data
        .get("sessionID")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("event session"))?;
    if session_id == "global" && kind.starts_with("form.") {
        return Ok(None);
    }
    identifier(session_id, "ses_")?;
    if snapshot {
        Ok(Some(IslandChange::Snapshot))
    } else {
        Ok(Some(IslandChange::Messages {
            session_id: session_id.to_owned(),
        }))
    }
}
