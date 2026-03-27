
use super::{ToolAct, rpc_exec_timeout};
use super::{Tool, ToolResult};

use crate::core::{AppState,RsshSession};
use std::time::Duration;

pub struct ShellTool {
    app: AppState,
}

impl ShellTool {
    pub fn new(app: AppState) -> Self {
        Self {app}
    }
}

#[async_trait::async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "执行 shell 命令"
    }

    /// 获取工具参数模式
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "要执行的 shell 命令"
                }
            },
            "required": ["command"]
        })
    }

    /// 执行工具
    async fn execute(&self, aid: &str, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少 'command' 参数"))?;

        let mut req_act = ToolAct::default();
        req_act.act = "shell".to_string();
        req_act.args = args.to_string();

        let rssh = RsshSession::new(self.app.clone());
        let channel = rssh.agent(aid).await?;
        let mut stream = channel.into_stream();

        match rpc_exec_timeout(&mut stream, req_act).await {
            Ok(res_act) => Ok(ToolResult {
                success: res_act.act.as_str() == "true",
                output: String::from_utf8_lossy(&res_act.data).to_string(),
                error: if res_act.act.as_str() == "true" { None} else { Some(res_act.args) },
            }),
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("命令执行错误 {}",e)),
            })
        }
    }
}

