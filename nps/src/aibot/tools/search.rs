
use super::{RPC_TIMEOUT_SECS, ToolAct, rpc_exec_timeout};
use super::{Tool, ToolResult};

use crate::core::{AppState,RsshSession};


pub struct SearchTool {
    app: AppState,
}

impl SearchTool {
    pub fn new(app: AppState) -> Self {
        Self {app}
    }
}

#[async_trait::async_trait]
impl Tool for SearchTool {
    fn name(&self) -> &str {
        "search"
    }

    fn description(&self) -> &str {
        "通过搜索引擎搜索内容"
    }

    /// 获取工具参数模式
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "keyword": {
                    "type": "string",
                    "description": "需要搜索的关键字"
                }
            },
            "required": ["keyword"]
        })
    }

    /// 执行工具
    async fn execute(&self, aid: &str, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let keyword = args
            .get("keyword")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'keyword' parameter"))?;



        Ok(ToolResult{
            suss
        })
    }
}

