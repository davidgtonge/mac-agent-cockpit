use app_core::{PreviewId, PreviewRuntimeState, PreviewStatus, ProjectId};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PreviewError {
    #[error("preview not found: {0}")]
    NotFound(String),
    #[error("invalid url: {0}")]
    InvalidUrl(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevServerCandidate {
    pub url: String,
    pub port: u16,
    pub source: String,
}

#[derive(Default)]
pub struct PreviewManager {
    previews: BTreeMap<PreviewId, PreviewStatus>,
}

impl PreviewManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(
        &mut self,
        project_id: ProjectId,
        url: String,
    ) -> Result<PreviewStatus, PreviewError> {
        validate_url(&url)?;
        let last_detected_port = detect_port_hint(&url);
        let status = PreviewStatus {
            preview_id: PreviewId::new(),
            project_id,
            url,
            state: PreviewRuntimeState::Open,
            dev_server_pid: None,
            last_detected_port,
        };
        self.previews
            .insert(status.preview_id.clone(), status.clone());
        Ok(status)
    }

    pub fn suspend(&mut self, preview_id: &PreviewId) -> Result<PreviewStatus, PreviewError> {
        self.set_state(preview_id, PreviewRuntimeState::Suspended)
    }

    pub fn close(&mut self, preview_id: &PreviewId) -> Result<PreviewStatus, PreviewError> {
        self.set_state(preview_id, PreviewRuntimeState::Destroyed)
    }

    pub fn all(&self) -> Vec<PreviewStatus> {
        self.previews.values().cloned().collect()
    }

    fn set_state(
        &mut self,
        preview_id: &PreviewId,
        state: PreviewRuntimeState,
    ) -> Result<PreviewStatus, PreviewError> {
        let p = self
            .previews
            .get_mut(preview_id)
            .ok_or_else(|| PreviewError::NotFound(preview_id.to_string()))?;
        p.state = state;
        Ok(p.clone())
    }
}

pub fn detect_localhost_urls(log: &str) -> Vec<DevServerCandidate> {
    let re = Regex::new(r#"https?://(?:localhost|127\.0\.0\.1):([0-9]{2,5})(?:/[^\s'\"]*)?"#)
        .expect("valid regex");
    let mut out = Vec::new();
    for cap in re.captures_iter(log) {
        let Some(full) = cap.get(0) else {
            continue;
        };
        let Some(port) = cap.get(1).and_then(|m| m.as_str().parse::<u16>().ok()) else {
            continue;
        };
        if !out.iter().any(|c: &DevServerCandidate| c.port == port) {
            out.push(DevServerCandidate {
                url: full.as_str().to_string(),
                port,
                source: "log".into(),
            });
        }
    }
    out
}

fn validate_url(url: &str) -> Result<(), PreviewError> {
    if url.starts_with("http://localhost:")
        || url.starts_with("https://localhost:")
        || url.starts_with("http://127.0.0.1:")
        || url.starts_with("https://127.0.0.1:")
    {
        Ok(())
    } else {
        Err(PreviewError::InvalidUrl(url.into()))
    }
}

fn detect_port_hint(url: &str) -> Option<u16> {
    let re = Regex::new(r#":([0-9]{2,5})"#).ok()?;
    re.captures(url)
        .and_then(|cap| cap.get(1))
        .and_then(|m| m.as_str().parse().ok())
}
