pub mod connectors;
pub mod manager;
pub mod registry;

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelEvent {
    pub event_type: ChannelEventType,
    pub source: String,
    pub channel_name: String,
    pub content: String,
    pub author: Option<String>,
    pub message_id: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelEventType {
    Message,
    Edit,
    Delete,
    Reaction,
    ThreadReply,
    FileUpload,
    MemberJoin,
    MemberLeave,
    Custom(String),
}

impl ChannelEventType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Message => "message",
            Self::Edit => "edit",
            Self::Delete => "delete",
            Self::Reaction => "reaction",
            Self::ThreadReply => "thread_reply",
            Self::FileUpload => "file_upload",
            Self::MemberJoin => "member_join",
            Self::MemberLeave => "member_leave",
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "message" => Self::Message,
            "edit" => Self::Edit,
            "delete" => Self::Delete,
            "reaction" => Self::Reaction,
            "thread_reply" => Self::ThreadReply,
            "file_upload" => Self::FileUpload,
            "member_join" => Self::MemberJoin,
            "member_leave" => Self::MemberLeave,
            other => Self::Custom(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectorStatus {
    Disconnected,
    Connecting,
    Connected,
    Polling,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorConfig {
    pub params: HashMap<String, String>,
}

impl ConnectorConfig {
    pub fn new() -> Self {
        Self {
            params: HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(|s| s.as_str())
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.params.insert(key.into(), value.into());
    }
}

impl Default for ConnectorConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
pub trait Connector: Send + Sync {
    fn name(&self) -> &str;
    fn source_type(&self) -> &str;
    fn supported_events(&self) -> Vec<ChannelEventType>;
    fn status(&self) -> ConnectorStatus;
    async fn connect(&mut self, config: &ConnectorConfig) -> anyhow::Result<()>;
    async fn disconnect(&mut self) -> anyhow::Result<()>;
    async fn poll(&self) -> anyhow::Result<Vec<ChannelEvent>>;
}
