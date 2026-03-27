
mod shell;
mod file_read;
mod file_write;
mod get_list;
mod http_get;
mod set_id;
mod skill_read;

use shell::ShellTool;
use file_read::FileReadTool;
use file_write::FileWriteTool;
use get_list::GetListTool;
use http_get::HttpGetTool;
use set_id::SetIdTool;
use skill_read::SkillReadTool;


use super::memory;
use crate::core;
pub use crate::protos::npc2::ToolAct;

use prost::Message;
use tokio::io::{AsyncWrite, AsyncRead, AsyncReadExt, AsyncWriteExt};
use std::fmt::Write;
use std::sync::Arc;
use std::time::Duration;

const RPC_TIMEOUT_SECS: u64 = 60;/// rpc执行的最大超时时间（秒）


pub fn all_tools(
    memory: Arc<dyn memory::Memory>,
    app: &core::AppState,
) -> Vec<Box<dyn Tool>>{

    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(MemoryStoreTool::new(memory.clone())),
        Box::new(MemoryRecallTool::new(memory.clone())),
        Box::new(MemoryForgetTool::new(memory)),
        Box::new(SetIdTool::new()),
        Box::new(SkillReadTool::new()),
        Box::new(ShellTool::new(app.clone())),
        Box::new(FileReadTool::new(app.clone())),
        Box::new(FileWriteTool::new(app.clone())),
        Box::new(GetListTool::new(app.clone())),
        Box::new(HttpGetTool::new(app.clone())),
    ];

    tools
}

pub async fn rpc_exec_timeout<S>(mut conn: &mut S, act: ToolAct) -> anyhow::Result<ToolAct> 
where S: AsyncRead + AsyncWrite + Unpin {
    let result = tokio::time::timeout(
        Duration::from_secs(RPC_TIMEOUT_SECS), 
        rpc_exec(&mut conn, act)
    );

    Ok(result.await??)
}


pub async fn rpc_exec<S>(mut conn: &mut S, act: ToolAct) -> anyhow::Result<ToolAct> 
where S: AsyncRead + AsyncWrite + Unpin {
    let mut data = Vec::new();
    let _ = act.encode(&mut data)?;
    let header_length = data.len() as u32;
    let header = header_length.to_be_bytes();
    let _ = conn.write_all(&header).await?;
    let _ = conn.write_all(&data).await?;
    let _ = conn.flush().await?;

    let mut length = [0u8; 4];
    let _ = conn.read_exact(&mut length).await?;
    let length_value = u32::from_be_bytes(length);
    let mut data = Vec::new();
    let mut buffer = [0u8; 1024];
    let mut total_read = 0;
    while total_read < length_value as usize {
        let bytes_read = conn.read(&mut buffer).await?;
        data.extend_from_slice(&buffer[..bytes_read]);
        total_read += bytes_read;
    }
    let mut cursor = std::io::Cursor::new(data);
    let event = ToolAct::decode(&mut cursor)?;
    Ok(event)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;
    async fn execute(&self, aid: &str, args: serde_json::Value) -> anyhow::Result<ToolResult>;
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters_schema(),
        }
    }
}

pub struct MemoryForgetTool {
    memory: Arc<dyn memory::Memory>,
}

impl MemoryForgetTool {
    pub fn new(memory: Arc<dyn memory::Memory>) -> Self {
        Self { memory }
    }
}

#[async_trait::async_trait]
impl Tool for MemoryForgetTool {
    fn name(&self) -> &str {
        "memory_forget"
    }

    fn description(&self) -> &str {
        "Remove a memory by key. Use to delete outdated facts or sensitive data. Returns whether the memory was found and removed."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "key": {
                    "type": "string",
                    "description": "The key of the memory to forget"
                }
            },
            "required": ["key"]
        })
    }

    async fn execute(&self, aid: &str, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let key = args
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'key' parameter"))?;

        match self.memory.forget(key).await {
            Ok(true) => Ok(ToolResult {
                success: true,
                output: format!("Forgot memory: {key}"),
                error: None,
            }),
            Ok(false) => Ok(ToolResult {
                success: true,
                output: format!("No memory found with key: {key}"),
                error: None,
            }),
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to forget memory: {e}")),
            }),
        }
    }
}


pub struct MemoryRecallTool {
    memory: Arc<dyn memory::Memory>,
}

impl MemoryRecallTool {
    pub fn new(memory: Arc<dyn memory::Memory>) -> Self {
        Self { memory }
    }
}

#[async_trait::async_trait]
impl Tool for MemoryRecallTool {
    fn name(&self) -> &str {
        "memory_recall"
    }

    fn description(&self) -> &str {
        "Search long-term memory for relevant facts, preferences, or context. Returns scored results ranked by relevance."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Keywords or phrase to search for in memory"
                },
                "limit": {
                    "type": "integer",
                    "description": "Max results to return (default: 5)"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, aid: &str, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' parameter"))?;

        #[allow(clippy::cast_possible_truncation)]
        let limit = args
            .get("limit")
            .and_then(serde_json::Value::as_u64)
            .map_or(5, |v| v as usize);

        match self.memory.recall(query, limit).await {
            Ok(entries) if entries.is_empty() => Ok(ToolResult {
                success: true,
                output: "No memories found matching that query.".into(),
                error: None,
            }),
            Ok(entries) => {
                let mut output = format!("Found {} memories:\n", entries.len());
                for entry in &entries {
                    let score = entry
                        .score
                        .map_or_else(String::new, |s| format!(" [{s:.0}%]"));
                    let _ = writeln!(
                        output,
                        "- [{}] {}: {}{score}",
                        entry.category, entry.key, entry.content
                    );
                }
                Ok(ToolResult {
                    success: true,
                    output,
                    error: None,
                })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Memory recall failed: {e}")),
            }),
        }
    }
}


pub struct MemoryStoreTool {
    memory: Arc<dyn memory::Memory>,
}

impl MemoryStoreTool {
    pub fn new(memory: Arc<dyn memory::Memory>) -> Self {
        Self { memory }
    }
}

#[async_trait::async_trait]
impl Tool for MemoryStoreTool {
    fn name(&self) -> &str {
        "memory_store"
    }

    fn description(&self) -> &str {
        "Store a fact, preference, or note in long-term memory. Use category 'core' for permanent facts, 'daily' for session notes, 'conversation' for chat context."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "key": {
                    "type": "string",
                    "description": "Unique key for this memory (e.g. 'user_lang', 'project_stack')"
                },
                "content": {
                    "type": "string",
                    "description": "The information to remember"
                },
                "category": {
                    "type": "string",
                    "enum": ["core", "daily", "conversation"],
                    "description": "Memory category: core (permanent), daily (session), conversation (chat)"
                }
            },
            "required": ["key", "content"]
        })
    }

    async fn execute(&self, aid: &str, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let key = args
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'key' parameter"))?;

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'content' parameter"))?;

        let category = match args.get("category").and_then(|v| v.as_str()) {
            Some("daily") => memory::MemoryCategory::Daily,
            Some("conversation") => memory::MemoryCategory::Conversation,
            _ => memory::MemoryCategory::Core,
        };

        match self.memory.store(key, content, category).await {
            Ok(()) => Ok(ToolResult {
                success: true,
                output: format!("Stored memory: {key}"),
                error: None,
            }),
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to store memory: {e}")),
            }),
        }
    }
}

