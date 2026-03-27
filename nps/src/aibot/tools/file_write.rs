
use super::{RPC_TIMEOUT_SECS, ToolAct, rpc_exec_timeout};
use super::{Tool, ToolResult};

use crate::core::{AppState,RsshSession};


pub struct FileWriteTool {
    app: AppState,
}

impl FileWriteTool {
    pub fn new(app: AppState) -> Self {
        Self {app}
    }
}

#[async_trait::async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "文件写入"
    }

    /// 获取工具参数模式
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path to the file within the workspace"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    /// 执行工具
    async fn execute(&self, aid: &str, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'content' parameter"))?;

        let mut req_act = ToolAct::default();
        req_act.act = "file_write".to_string();
        req_act.args = args.to_string();

        let rssh = RsshSession::new(self.app.clone());
        let channel = rssh.agent(aid).await?;
        let mut stream = channel.into_stream();

        match rpc_exec_timeout(&mut stream, req_act).await {
            Ok(res_act) => Ok(ToolResult {
                success: res_act.act.as_str() == "true",
                output: String::from_utf8_lossy(&res_act.data).to_string(),
                error: None,
            }),
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("文件写入失败: {e}")),
            })
        }
    }
}

