use app_core::{AcpMessage, ConversationId};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{mpsc, oneshot, Mutex};

#[derive(Debug, Error)]
pub enum AcpError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("missing stdio")]
    MissingStdio,
    #[error("json-rpc error: {0}")]
    JsonRpc(String),
    #[error("response channel closed")]
    ResponseClosed,
    #[error("missing ACP client for {0}")]
    MissingClient(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcEnvelope {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
}

impl JsonRpcEnvelope {
    pub fn request(id: u64, method: &str, params: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: Some(id.into()),
            method: Some(method.into()),
            params: Some(params),
            result: None,
        }
    }
    pub fn notification(method: &str, params: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: None,
            method: Some(method.into()),
            params: Some(params),
            result: None,
        }
    }
    pub fn response(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: Some(id),
            method: None,
            params: None,
            result: Some(result),
        }
    }
}

type PendingMap = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, AcpError>>>>>;

pub struct AcpTransport {
    child: Mutex<Child>,
    stdin: Mutex<ChildStdin>,
    next_id: AtomicU64,
    pending: PendingMap,
}

impl AcpTransport {
    pub async fn spawn(
        conversation_id: ConversationId,
        cwd: impl AsRef<Path>,
        event_tx: mpsc::UnboundedSender<(ConversationId, AcpMessage)>,
    ) -> Result<Self, AcpError> {
        let binary = std::env::var("CURSOR_AGENT_BINARY").unwrap_or_else(|_| "agent".into());
        let mut command = Command::new(binary);
        command
            .arg("acp")
            .current_dir(cwd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit());
        #[cfg(unix)]
        unsafe {
            command.pre_exec(|| {
                libc::setpgid(0, 0);
                Ok(())
            });
        }
        let mut child = command.spawn()?;

        let stdin = child.stdin.take().ok_or(AcpError::MissingStdio)?;
        let stdout = child.stdout.take().ok_or(AcpError::MissingStdio)?;
        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let reader_pending = pending.clone();

        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<Value>(&line) {
                    Ok(raw) => {
                        if let Some(id) = raw.get("id").and_then(|v| v.as_u64()) {
                            if raw.get("result").is_some() || raw.get("error").is_some() {
                                let tx = reader_pending.lock().await.remove(&id);
                                if let Some(tx) = tx {
                                    let result = if let Some(err) = raw.get("error") {
                                        Err(AcpError::JsonRpc(err.to_string()))
                                    } else {
                                        Ok(raw.get("result").cloned().unwrap_or(Value::Null))
                                    };
                                    let _ = tx.send(result);
                                    continue;
                                }
                            }
                        }
                        let _ =
                            event_tx.send((conversation_id.clone(), AcpMessage::from_value(raw)));
                    }
                    Err(error) => {
                        let raw = json!({ "method": "session/update", "params": { "update": { "sessionUpdate": "agent_message_chunk", "content": { "text": format!("ACP parse error: {}", error) } } } });
                        let _ =
                            event_tx.send((conversation_id.clone(), AcpMessage::from_value(raw)));
                    }
                }
            }
        });

        Ok(Self {
            child: Mutex::new(child),
            stdin: Mutex::new(stdin),
            next_id: AtomicU64::new(1),
            pending,
        })
    }

    pub async fn send_request(&self, method: &str, params: Value) -> Result<Value, AcpError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let envelope = JsonRpcEnvelope::request(id, method, params);
        let line = serde_json::to_string(&envelope)? + "\n";
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        let mut stdin = self.stdin.lock().await;
        stdin.write_all(line.as_bytes()).await?;
        stdin.flush().await?;
        drop(stdin);
        rx.await.map_err(|_| AcpError::ResponseClosed)?
    }

    pub async fn send_notification(&self, method: &str, params: Value) -> Result<(), AcpError> {
        let line = serde_json::to_string(&JsonRpcEnvelope::notification(method, params))? + "\n";
        let mut stdin = self.stdin.lock().await;
        stdin.write_all(line.as_bytes()).await?;
        stdin.flush().await?;
        Ok(())
    }

    pub async fn respond(&self, id: Value, result: Value) -> Result<(), AcpError> {
        let line = serde_json::to_string(&JsonRpcEnvelope::response(id, result))? + "\n";
        let mut stdin = self.stdin.lock().await;
        stdin.write_all(line.as_bytes()).await?;
        stdin.flush().await?;
        Ok(())
    }

    pub async fn root_pid(&self) -> Option<i32> {
        self.child.lock().await.id().map(|pid| pid as i32)
    }

    pub async fn kill_child(&self) {
        let mut child = self.child.lock().await;
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
}

pub struct CursorAcpClient {
    transport: AcpTransport,
    cwd: PathBuf,
    session_id: Mutex<Option<String>>,
}

impl CursorAcpClient {
    pub async fn start(
        conversation_id: ConversationId,
        cwd: impl AsRef<Path>,
        event_tx: mpsc::UnboundedSender<(ConversationId, AcpMessage)>,
    ) -> Result<Self, AcpError> {
        let cwd = cwd.as_ref().to_path_buf();
        let transport = AcpTransport::spawn(conversation_id, &cwd, event_tx).await?;
        Ok(Self {
            transport,
            cwd,
            session_id: Mutex::new(None),
        })
    }

    pub async fn initialize(&self) -> Result<Value, AcpError> {
        self.transport.send_request("initialize", json!({
            "protocolVersion": 1,
            "clientCapabilities": { "fs": { "readTextFile": false, "writeTextFile": false }, "terminal": false },
            "clientInfo": { "name": "mac-agent-cockpit", "version": env!("CARGO_PKG_VERSION") }
        })).await
    }

    pub async fn authenticate(&self) -> Result<Value, AcpError> {
        self.transport
            .send_request("authenticate", json!({ "methodId": "cursor_login" }))
            .await
    }

    pub async fn session_new(&self) -> Result<(String, Value), AcpError> {
        let result = self
            .transport
            .send_request(
                "session/new",
                json!({ "cwd": self.cwd.to_string_lossy(), "mcpServers": [] }),
            )
            .await?;
        let session_id = parse_session_id(&result);
        *self.session_id.lock().await = Some(session_id.clone());
        Ok((session_id, result))
    }

    pub async fn session_load(&self, session_id: &str) -> Result<Value, AcpError> {
        let result = self
            .transport
            .send_request(
                "session/load",
                json!({
                    "sessionId": session_id,
                    "cwd": self.cwd.to_string_lossy(),
                    "mcpServers": []
                }),
            )
            .await?;
        *self.session_id.lock().await = Some(session_id.to_string());
        Ok(result)
    }

    pub async fn session_resume(&self, session_id: &str) -> Result<Value, AcpError> {
        let result = self
            .transport
            .send_request(
                "session/resume",
                json!({
                    "sessionId": session_id,
                    "cwd": self.cwd.to_string_lossy(),
                    "mcpServers": []
                }),
            )
            .await?;
        *self.session_id.lock().await = Some(session_id.to_string());
        Ok(result)
    }

    pub async fn set_mode(&self, session_id: Option<&str>, mode_id: &str) -> Result<Value, AcpError> {
        let owned;
        let sid = if let Some(s) = session_id {
            s
        } else {
            owned = self.session_id.lock().await.clone().unwrap_or_default();
            owned.as_str()
        };
        self.transport
            .send_request(
                "session/set_mode",
                json!({ "sessionId": sid, "modeId": mode_id }),
            )
            .await
    }

    pub async fn set_config_option(
        &self,
        session_id: Option<&str>,
        config_id: &str,
        value: &str,
    ) -> Result<Value, AcpError> {
        let owned;
        let sid = if let Some(s) = session_id {
            s
        } else {
            owned = self.session_id.lock().await.clone().unwrap_or_default();
            owned.as_str()
        };
        self.transport
            .send_request(
                "session/set_config_option",
                json!({
                    "sessionId": sid,
                    "optionId": config_id,
                    "configId": config_id,
                    "valueId": value,
                    "value": value
                }),
            )
            .await
    }

    pub async fn prompt(&self, session_id: Option<&str>, text: &str) -> Result<Value, AcpError> {
        let owned;
        let sid = if let Some(s) = session_id {
            s
        } else {
            owned = self.session_id.lock().await.clone().unwrap_or_default();
            owned.as_str()
        };
        self.transport
            .send_request(
                "session/prompt",
                json!({
                    "sessionId": sid,
                    "prompt": [{ "type": "text", "text": text }]
                }),
            )
            .await
    }

    pub async fn steer(&self, session_id: Option<&str>, text: &str) -> Result<Value, AcpError> {
        let owned;
        let sid = if let Some(s) = session_id {
            s
        } else {
            owned = self.session_id.lock().await.clone().unwrap_or_default();
            owned.as_str()
        };
        self.transport
            .send_notification(
                "session/steer",
                json!({
                    "sessionId": sid,
                    "prompt": [{ "type": "text", "text": text }]
                }),
            )
            .await?;
        Ok(Value::Null)
    }

    pub async fn cancel(&self, session_id: Option<&str>) -> Result<(), AcpError> {
        let owned;
        let sid = if let Some(s) = session_id {
            s
        } else {
            owned = self.session_id.lock().await.clone().unwrap_or_default();
            owned.as_str()
        };
        self.transport
            .send_notification("session/cancel", json!({ "sessionId": sid }))
            .await
    }

    pub async fn respond_permission(
        &self,
        acp_request_id: Value,
        option_id: &str,
    ) -> Result<(), AcpError> {
        self.transport
            .respond(
                acp_request_id,
                json!({ "outcome": { "outcome": "selected", "optionId": option_id } }),
            )
            .await
    }

    pub async fn root_pid(&self) -> Option<i32> {
        self.transport.root_pid().await
    }

    pub async fn stop(&self, session_id: Option<&str>) -> Result<(), AcpError> {
        let _ = self.cancel(session_id).await;
        self.transport.kill_child().await;
        Ok(())
    }
}

pub struct AgentManager {
    sessions: Mutex<HashMap<ConversationId, Arc<CursorAcpClient>>>,
    event_tx: mpsc::UnboundedSender<(ConversationId, AcpMessage)>,
}

impl AgentManager {
    pub fn new(event_tx: mpsc::UnboundedSender<(ConversationId, AcpMessage)>) -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            event_tx,
        }
    }

    pub async fn start_new_session(
        &self,
        conversation_id: ConversationId,
        cwd: impl AsRef<Path>,
    ) -> Result<(Option<i32>, String, Value), AcpError> {
        let (root_pid, session_id, meta, _) = self
            .start_or_resume_session(conversation_id, cwd, None)
            .await?;
        Ok((root_pid, session_id, meta))
    }

    pub async fn start_or_resume_session(
        &self,
        conversation_id: ConversationId,
        cwd: impl AsRef<Path>,
        resume_session_id: Option<String>,
    ) -> Result<(Option<i32>, String, Value, bool), AcpError> {
        let client = Arc::new(
            CursorAcpClient::start(conversation_id.clone(), cwd, self.event_tx.clone()).await?,
        );
        let root_pid = client.root_pid().await;
        let init = client.initialize().await?;
        client.authenticate().await?;

        if let Some(existing_id) = resume_session_id.filter(|id| !id.is_empty()) {
            if supports_load_session(&init) {
                if let Ok(meta) = client.session_load(&existing_id).await {
                    self.sessions.lock().await.insert(conversation_id, client);
                    return Ok((root_pid, existing_id, meta, true));
                }
            }
            if supports_session_resume(&init) {
                if let Ok(meta) = client.session_resume(&existing_id).await {
                    self.sessions.lock().await.insert(conversation_id, client);
                    return Ok((root_pid, existing_id, meta, false));
                }
            }
        }

        let (session_id, meta) = client.session_new().await?;
        self.sessions.lock().await.insert(conversation_id, client);
        Ok((root_pid, session_id, meta, false))
    }

    pub async fn set_mode(
        &self,
        conversation_id: &ConversationId,
        cursor_session_id: Option<&str>,
        mode_id: &str,
    ) -> Result<Value, AcpError> {
        let client = self
            .sessions
            .lock()
            .await
            .get(conversation_id)
            .cloned()
            .ok_or_else(|| AcpError::MissingClient(conversation_id.to_string()))?;
        client.set_mode(cursor_session_id, mode_id).await
    }

    pub async fn set_config_option(
        &self,
        conversation_id: &ConversationId,
        cursor_session_id: Option<&str>,
        config_id: &str,
        value_id: &str,
    ) -> Result<Value, AcpError> {
        let client = self
            .sessions
            .lock()
            .await
            .get(conversation_id)
            .cloned()
            .ok_or_else(|| AcpError::MissingClient(conversation_id.to_string()))?;
        client
            .set_config_option(cursor_session_id, config_id, value_id)
            .await
    }

    pub async fn send_prompt(
        &self,
        conversation_id: &ConversationId,
        cursor_session_id: Option<&str>,
        text: &str,
    ) -> Result<Value, AcpError> {
        let client = self
            .sessions
            .lock()
            .await
            .get(conversation_id)
            .cloned()
            .ok_or_else(|| AcpError::MissingClient(conversation_id.to_string()))?;
        client.prompt(cursor_session_id, text).await
    }

    pub async fn cancel(
        &self,
        conversation_id: &ConversationId,
        cursor_session_id: Option<&str>,
    ) -> Result<(), AcpError> {
        let client = self
            .sessions
            .lock()
            .await
            .get(conversation_id)
            .cloned()
            .ok_or_else(|| AcpError::MissingClient(conversation_id.to_string()))?;
        client.cancel(cursor_session_id).await
    }

    pub async fn respond_permission(
        &self,
        conversation_id: &ConversationId,
        acp_request_id: Value,
        option_id: &str,
    ) -> Result<(), AcpError> {
        let client = self
            .sessions
            .lock()
            .await
            .get(conversation_id)
            .cloned()
            .ok_or_else(|| AcpError::MissingClient(conversation_id.to_string()))?;
        client.respond_permission(acp_request_id, option_id).await
    }

    pub async fn steer_prompt(
        &self,
        conversation_id: &ConversationId,
        cursor_session_id: Option<&str>,
        text: &str,
    ) -> Result<Value, AcpError> {
        let client = self
            .sessions
            .lock()
            .await
            .get(conversation_id)
            .cloned()
            .ok_or_else(|| AcpError::MissingClient(conversation_id.to_string()))?;
        client.steer(cursor_session_id, text).await
    }

    pub async fn stop_session(
        &self,
        conversation_id: &ConversationId,
        cursor_session_id: Option<&str>,
    ) -> Result<(), AcpError> {
        if let Some(client) = self.sessions.lock().await.remove(conversation_id) {
            client.stop(cursor_session_id).await?;
        }
        Ok(())
    }
}

fn parse_session_id(result: &Value) -> String {
    result
        .get("sessionId")
        .or_else(|| result.get("session_id"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn agent_capabilities(init: &Value) -> &Value {
    init.get("agentCapabilities").unwrap_or(init)
}

fn supports_load_session(init: &Value) -> bool {
    match agent_capabilities(init).get("loadSession") {
        Some(Value::Bool(true)) => true,
        Some(Value::Object(_)) => true,
        _ => false,
    }
}

fn supports_session_resume(init: &Value) -> bool {
    agent_capabilities(init)
        .get("sessionCapabilities")
        .and_then(|caps| caps.get("resume"))
        .map(|value| !value.is_null())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_load_and_resume_capabilities() {
        let init = json!({
            "agentCapabilities": {
                "loadSession": true,
                "sessionCapabilities": { "resume": {} }
            }
        });
        assert!(supports_load_session(&init));
        assert!(supports_session_resume(&init));
    }

    #[test]
    fn parse_session_id_accepts_common_aliases() {
        assert_eq!(
            parse_session_id(&json!({ "sessionId": "abc" })),
            "abc"
        );
        assert_eq!(
            parse_session_id(&json!({ "session_id": "def" })),
            "def"
        );
    }
}
