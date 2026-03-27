
use super::{RPC_TIMEOUT_SECS, ToolAct, rpc_exec};
use super::{Tool, ToolResult};

use crate::core::AppState;

pub struct SetIdTool {
}

impl SetIdTool {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Tool for SetIdTool {
    fn name(&self) -> &str {
        "set_id"
    }

    fn description(&self) -> &str {
        "设置agent的id, 当找不到agent id时调用此工具"
    }

    /// 获取工具参数模式
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "需要设置的npc的id"
                }
            },
            "required": ["id"]
        })
    }


    async fn execute(&self, _aid: &str, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult {
            success: true,
            output: format!("ID 设置成功"),
            error: None,
        })
    }
}

